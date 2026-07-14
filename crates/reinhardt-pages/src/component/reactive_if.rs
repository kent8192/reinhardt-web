//! Reactive conditional rendering DOM management.
//!
//! This module provides the `ReactiveIfNode` which manages DOM updates
//! for conditional rendering based on Signal changes.

#[cfg(wasm)]
use crate::component::into_page::PageExt;
#[cfg(wasm)]
use crate::reactive::effect::Effect;
#[cfg(wasm)]
use crate::reactive::runtime::{EffectTiming, with_runtime};
#[cfg(wasm)]
use reinhardt_core::types::page::{BOOLEAN_ATTRS, MountError, Page, is_boolean_attr_truthy};
#[cfg(wasm)]
use std::cell::Cell;
use std::cell::RefCell;
#[cfg(native)]
use std::future::Future;
use std::rc::Rc;

pub(crate) type ReactiveNodeStore = Rc<RefCell<Vec<Box<dyn std::any::Any>>>>;

// Thread-local storage for reactive nodes to prevent them from being dropped.
//
// When a reactive node or retained hook effect is created during view mounting,
// it must be kept alive for the lifetime of the current mounted view. This
// storage prevents premature cleanup while still allowing route and portal
// teardown to drop stored values through RAII.
thread_local! {
	static ROOT_REACTIVE_NODES: ReactiveNodeStore = Rc::new(RefCell::new(Vec::new()));
	#[cfg(wasm)]
	static ACTIVE_REACTIVE_NODE_STORE: RefCell<Option<ReactiveNodeStore>> = RefCell::new(None);
}

#[cfg(native)]
tokio::task_local! {
	static SSR_REACTIVE_NODE_STORE: ReactiveNodeStore;
}

#[cfg(wasm)]
struct ActiveReactiveNodeStoreGuard {
	previous: Option<ReactiveNodeStore>,
}

#[cfg(wasm)]
impl Drop for ActiveReactiveNodeStoreGuard {
	fn drop(&mut self) {
		ACTIVE_REACTIVE_NODE_STORE.with(|active| {
			active.replace(self.previous.take());
		});
	}
}

fn root_reactive_node_store() -> ReactiveNodeStore {
	ROOT_REACTIVE_NODES.with(Clone::clone)
}

#[cfg(wasm)]
fn current_reactive_node_store() -> ReactiveNodeStore {
	ACTIVE_REACTIVE_NODE_STORE
		.with(|active| active.borrow().clone())
		.unwrap_or_else(root_reactive_node_store)
}

#[cfg(native)]
fn current_reactive_node_store() -> ReactiveNodeStore {
	SSR_REACTIVE_NODE_STORE
		.try_with(Clone::clone)
		.unwrap_or_else(|_| root_reactive_node_store())
}

pub(crate) fn new_reactive_node_store() -> ReactiveNodeStore {
	Rc::new(RefCell::new(Vec::new()))
}

pub(crate) fn clear_reactive_node_store(store: &ReactiveNodeStore) {
	let _stored_nodes = {
		let mut stored_nodes = store.borrow_mut();
		std::mem::take(&mut *stored_nodes)
	};
}

#[cfg(wasm)]
pub(crate) fn with_reactive_node_store<R>(store: &ReactiveNodeStore, f: impl FnOnce() -> R) -> R {
	let previous = ACTIVE_REACTIVE_NODE_STORE.with(|active| active.replace(Some(store.clone())));
	let _guard = ActiveReactiveNodeStoreGuard { previous };
	f()
}

#[cfg(any(wasm, test))]
struct ReactiveNodeTransaction {
	destination: ReactiveNodeStore,
	staged: ReactiveNodeStore,
	committed: bool,
}

#[cfg(any(wasm, test))]
impl ReactiveNodeTransaction {
	fn new(destination: ReactiveNodeStore) -> Self {
		Self {
			destination,
			staged: new_reactive_node_store(),
			committed: false,
		}
	}

	fn store(&self) -> ReactiveNodeStore {
		self.staged.clone()
	}

	fn commit(&mut self) {
		self.destination
			.borrow_mut()
			.append(&mut self.staged.borrow_mut());
		self.committed = true;
	}
}

#[cfg(any(wasm, test))]
impl Drop for ReactiveNodeTransaction {
	fn drop(&mut self) {
		if !self.committed {
			clear_reactive_node_store(&self.staged);
		}
	}
}

#[cfg(wasm)]
pub(crate) fn with_reactive_node_transaction<T, E>(
	f: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
	let mut transaction = ReactiveNodeTransaction::new(current_reactive_node_store());
	let result = with_reactive_node_store(&transaction.store(), f);
	if result.is_ok() {
		transaction.commit();
	}
	result
}

#[cfg(native)]
pub(crate) async fn scope_reactive_node_store<R>(future: impl Future<Output = R>) -> R {
	SSR_REACTIVE_NODE_STORE
		.scope(new_reactive_node_store(), future)
		.await
}

/// Stores a reactive node to keep it alive.
#[cfg(wasm)]
pub fn store_reactive_node<T: 'static>(node: T) {
	current_reactive_node_store()
		.borrow_mut()
		.push(Box::new(node));
}

/// Stores a reactive node to keep it alive.
#[cfg(native)]
pub(crate) fn store_reactive_node<T: 'static>(node: T) {
	current_reactive_node_store()
		.borrow_mut()
		.push(Box::new(node));
}

/// Cleanup function to release all reactive nodes.
///
/// This should be called when the application is being torn down or
/// when a complete re-render is needed.
pub fn cleanup_reactive_nodes() {
	clear_reactive_node_store(&root_reactive_node_store());
}

/// Manages DOM updates for reactive conditional rendering.
///
/// ReactiveIfNode creates a comment node as a marker in the DOM, and uses
/// an Effect to monitor condition changes. When the condition changes,
/// it removes the old DOM nodes and mounts new ones.
#[cfg(wasm)]
pub struct ReactiveIfNode {
	/// Marker comment node in DOM (used as insertion point reference)
	#[allow(dead_code)] // Kept for potential future use
	marker: web_sys::Comment,
	/// Stable start marker for hydrated DOM ranges.
	start_marker: Option<web_sys::Comment>,
	/// Currently mounted DOM nodes
	#[allow(dead_code)] // Kept for potential future use
	current_nodes: Rc<RefCell<Vec<web_sys::Node>>>,
	/// Last evaluated condition value (for change detection)
	#[allow(dead_code)] // Kept for potential future use
	last_condition: Rc<RefCell<Option<bool>>>,
	/// Nested reactive nodes owned by the current branch.
	reactive_nodes: ReactiveNodeStore,
	/// Effect handle (kept alive to maintain reactivity)
	effect: Option<Effect>,
}

#[cfg(wasm)]
impl ReactiveIfNode {
	/// Creates a new ReactiveIfNode and mounts it with reactive updates.
	///
	/// # Arguments
	///
	/// * `parent` - The parent DOM element to mount the conditional content into
	/// * `condition` - Closure that returns the condition value
	/// * `then_view` - Closure that returns the view when condition is true
	/// * `else_view` - Closure that returns the view when condition is false
	pub fn new(
		parent: &crate::dom::Element,
		condition: std::sync::Arc<dyn Fn() -> bool + 'static>,
		then_view: std::sync::Arc<dyn Fn() -> Page + 'static>,
		else_view: std::sync::Arc<dyn Fn() -> Page + 'static>,
	) -> Self {
		// Create a comment node as a marker/anchor point
		let document = web_sys::window()
			.expect("window should be available")
			.document()
			.expect("document should be available");
		let marker = document.create_comment("reactive-if");

		// Append marker to parent
		parent
			.inner()
			.append_child(&marker)
			.expect("should append marker");

		// Shared state for the Effect
		let current_nodes: Rc<RefCell<Vec<web_sys::Node>>> = Rc::new(RefCell::new(Vec::new()));
		let last_condition: Rc<RefCell<Option<bool>>> = Rc::new(RefCell::new(None));

		// Clone references for the Effect closure
		let current_nodes_clone = current_nodes.clone();
		let last_condition_clone = last_condition.clone();
		let marker_clone = marker.clone();
		let reactive_nodes = new_reactive_node_store();
		let effect_reactive_node_store = current_reactive_node_store();
		let branch_reactive_node_store = reactive_nodes.clone();

		// Create the Effect that will re-run when condition dependencies change
		let effect = Effect::new_with_timing(
			move || {
				with_reactive_node_store(&effect_reactive_node_store, || {
					// Evaluate the condition (this tracks Signal dependencies)
					let new_condition = condition();

					// Check if condition has changed
					let mut last = last_condition_clone.borrow_mut();
					if *last == Some(new_condition) {
						// Condition hasn't changed, skip DOM update
						return;
					}
					*last = Some(new_condition);
					drop(last);

					clear_reactive_node_store(&branch_reactive_node_store);

					// Refs #5100: remove old nodes before mounting the replacement view. The
					// mount path may synchronously run layout effects, so do not
					// hold this RefCell borrow across `mount_before_marker`.
					let old_nodes = {
						let mut nodes = current_nodes_clone.borrow_mut();
						nodes.drain(..).collect::<Vec<_>>()
					};
					for node in old_nodes {
						if let Some(parent_node) = node.parent_node() {
							let _ = parent_node.remove_child(&node);
						}
					}

					let new_nodes = with_reactive_node_store(&branch_reactive_node_store, || {
						// Generate the appropriate view
						let view = if new_condition {
							then_view()
						} else {
							else_view()
						};

						// Mount new nodes before the marker
						mount_before_marker(&marker_clone, view)
					});
					*current_nodes_clone.borrow_mut() = new_nodes;
				});
			},
			EffectTiming::Layout, // Use Layout timing for synchronous DOM updates
		);

		Self {
			marker,
			start_marker: None,
			current_nodes,
			last_condition,
			reactive_nodes,
			effect: Some(effect),
		}
	}

	// DOM boundary coordinates, the precreated owner store, and the hydrated baseline must stay
	// explicit so hydration adopts the exact server-rendered range without reconstructing state.
	#[allow(clippy::too_many_arguments)]
	pub(crate) fn hydrate_at(
		parent: web_sys::Node,
		next_sibling: Option<web_sys::Node>,
		existing_nodes: Vec<web_sys::Node>,
		hydrated_condition: bool,
		condition: std::sync::Arc<dyn Fn() -> bool + 'static>,
		then_view: std::sync::Arc<dyn Fn() -> Page + 'static>,
		else_view: std::sync::Arc<dyn Fn() -> Page + 'static>,
		reactive_nodes: ReactiveNodeStore,
	) -> Option<Self> {
		let document = web_sys::window()
			.expect("window should be available")
			.document()
			.expect("document should be available");
		let start_marker = document.create_comment("reactive-if-start");
		let marker = document.create_comment("reactive-if");
		let start_anchor = existing_nodes.first().or(next_sibling.as_ref());
		let _ = parent.insert_before(&start_marker, start_anchor);
		let _ = parent.insert_before(&marker, next_sibling.as_ref());

		let current_nodes: Rc<RefCell<Vec<web_sys::Node>>> = Rc::new(RefCell::new(existing_nodes));
		let last_condition: Rc<RefCell<Option<bool>>> =
			Rc::new(RefCell::new(Some(hydrated_condition)));
		let current_nodes_clone = current_nodes.clone();
		let last_condition_clone = last_condition.clone();
		let start_marker_clone = Some(start_marker.clone());
		let marker_clone = marker.clone();
		let effect_reactive_node_store = current_reactive_node_store();
		let branch_reactive_node_store = reactive_nodes.clone();
		let first_run = Rc::new(Cell::new(true));
		let first_run_clone = first_run.clone();
		let hydration_mismatch = Rc::new(Cell::new(false));
		let hydration_mismatch_clone = hydration_mismatch.clone();
		#[cfg(feature = "i18n")]
		let i18n_context = crate::i18n::current_i18n_callback_context();

		let effect = Effect::new_with_timing(
			move || {
				let update = || {
					with_reactive_node_store(&effect_reactive_node_store, || {
						let new_condition = condition();

						if first_run_clone.replace(false) {
							hydration_mismatch_clone.set(new_condition != hydrated_condition);
							return;
						}

						let mut last = last_condition_clone.borrow_mut();
						if *last == Some(new_condition) {
							return;
						}
						*last = Some(new_condition);
						drop(last);

						clear_reactive_node_store(&branch_reactive_node_store);

						refresh_current_nodes_before_marker(
							start_marker_clone.as_ref(),
							&marker_clone,
							&current_nodes_clone,
						);
						let old_nodes = {
							let mut nodes = current_nodes_clone.borrow_mut();
							nodes.drain(..).collect::<Vec<_>>()
						};
						for node in old_nodes {
							if let Some(parent_node) = node.parent_node() {
								let _ = parent_node.remove_child(&node);
							}
						}

						let new_nodes =
							with_reactive_node_store(&branch_reactive_node_store, || {
								let view = if new_condition {
									then_view()
								} else {
									else_view()
								};
								mount_before_marker(&marker_clone, view)
							});
						*current_nodes_clone.borrow_mut() = new_nodes;
					});
				};
				#[cfg(feature = "i18n")]
				crate::i18n::with_optional_i18n_context(i18n_context.as_ref(), update);
				#[cfg(not(feature = "i18n"))]
				update();
			},
			EffectTiming::Layout,
		);
		if hydration_mismatch.get() {
			with_runtime(|runtime| runtime.schedule_update(effect.id()));
		}

		Some(Self {
			marker,
			start_marker: Some(start_marker),
			current_nodes,
			last_condition,
			reactive_nodes,
			effect: Some(effect),
		})
	}

	pub(crate) fn reactive_node_store(&self) -> ReactiveNodeStore {
		self.reactive_nodes.clone()
	}

	pub(crate) fn refresh_hydrated_current_nodes(&self) {
		refresh_current_nodes_before_marker(
			self.start_marker.as_ref(),
			&self.marker,
			&self.current_nodes,
		);
	}
}

#[cfg(wasm)]
impl Drop for ReactiveIfNode {
	fn drop(&mut self) {
		let _marker_removal = MarkerRemovalGuard::new(self.start_marker.as_ref(), &self.marker);
		drop(self.effect.take());
		clear_reactive_node_store(&self.reactive_nodes);
	}
}

/// Manages DOM updates for reactive view rendering.
///
/// ReactiveNode is similar to ReactiveIfNode but handles a single render
/// closure rather than conditional branches. It creates a comment node as
/// a marker and uses an Effect to monitor Signal changes.
#[cfg(wasm)]
pub struct ReactiveNode {
	/// Marker comment node in DOM (used as insertion point reference)
	#[allow(dead_code)] // Kept for potential future use
	marker: web_sys::Comment,
	/// Stable start marker for hydrated DOM ranges.
	start_marker: Option<web_sys::Comment>,
	/// Currently mounted DOM nodes
	#[allow(dead_code)] // Kept for potential future use
	current_nodes: Rc<RefCell<Vec<web_sys::Node>>>,
	/// Nested reactive nodes owned by the current render.
	reactive_nodes: ReactiveNodeStore,
	/// Effect handle (kept alive to maintain reactivity)
	effect: Option<Effect>,
}

#[cfg(wasm)]
impl ReactiveNode {
	/// Creates a new ReactiveNode and mounts it with reactive updates.
	///
	/// # Arguments
	///
	/// * `parent` - The parent DOM element to mount the reactive content into
	/// * `render` - Closure that returns the view to render
	pub fn new(
		parent: &crate::dom::Element,
		render: std::sync::Arc<dyn Fn() -> Page + 'static>,
	) -> Self {
		// Create a comment node as a marker/anchor point
		let document = web_sys::window()
			.expect("window should be available")
			.document()
			.expect("document should be available");
		let marker = document.create_comment("reactive");

		// Append marker to parent
		parent
			.inner()
			.append_child(&marker)
			.expect("should append marker");

		// Shared state for the Effect
		let current_nodes: Rc<RefCell<Vec<web_sys::Node>>> = Rc::new(RefCell::new(Vec::new()));

		// Clone references for the Effect closure
		let current_nodes_clone = current_nodes.clone();
		let marker_clone = marker.clone();
		let reactive_nodes = new_reactive_node_store();
		let effect_reactive_node_store = current_reactive_node_store();
		let render_reactive_node_store = new_reactive_node_store();
		let mount_reactive_node_store = reactive_nodes.clone();
		#[cfg(feature = "i18n")]
		let i18n_context = crate::i18n::current_i18n_callback_context();

		// Create the Effect that will re-run when dependencies change
		let effect = Effect::new_with_timing(
			move || {
				let update = || {
					with_reactive_node_store(&effect_reactive_node_store, || {
						clear_reactive_node_store(&render_reactive_node_store);
						// Render the view (this tracks Signal dependencies)
						let view =
							with_reactive_node_store(&render_reactive_node_store, || render());

						if update_activity_boundary_attrs(&current_nodes_clone, &view) {
							return;
						}

						clear_reactive_node_store(&mount_reactive_node_store);

						// Refs #5100: remove old nodes before mounting the replacement view. The
						// mount path may synchronously run layout effects, so do not
						// hold this RefCell borrow across `mount_before_marker`.
						let old_nodes = {
							let mut nodes = current_nodes_clone.borrow_mut();
							nodes.drain(..).collect::<Vec<_>>()
						};
						for node in old_nodes {
							if let Some(parent_node) = node.parent_node() {
								let _ = parent_node.remove_child(&node);
							}
						}

						// Mount new nodes before the marker
						let new_nodes =
							with_reactive_node_store(&mount_reactive_node_store, || {
								mount_before_marker(&marker_clone, view)
							});
						*current_nodes_clone.borrow_mut() = new_nodes;
					});
				};
				#[cfg(feature = "i18n")]
				crate::i18n::with_optional_i18n_context(i18n_context.as_ref(), update);
				#[cfg(not(feature = "i18n"))]
				update();
			},
			EffectTiming::Layout, // Use Layout timing for synchronous DOM updates
		);

		Self {
			marker,
			start_marker: None,
			current_nodes,
			reactive_nodes,
			effect: Some(effect),
		}
	}

	pub(crate) fn hydrate_at(
		parent: web_sys::Node,
		next_sibling: Option<web_sys::Node>,
		existing_nodes: Vec<web_sys::Node>,
		render: std::sync::Arc<dyn Fn() -> Page + 'static>,
		render_reactive_node_store: ReactiveNodeStore,
		reactive_nodes: ReactiveNodeStore,
		refresh_after_control_adoption: bool,
	) -> Option<Self> {
		let document = web_sys::window()
			.expect("window should be available")
			.document()
			.expect("document should be available");
		let start_marker = document.create_comment("reactive-start");
		let marker = document.create_comment("reactive");
		let start_anchor = existing_nodes.first().or(next_sibling.as_ref());
		let _ = parent.insert_before(&start_marker, start_anchor);
		let _ = parent.insert_before(&marker, next_sibling.as_ref());

		let current_nodes: Rc<RefCell<Vec<web_sys::Node>>> = Rc::new(RefCell::new(existing_nodes));
		let current_nodes_clone = current_nodes.clone();
		let start_marker_clone = Some(start_marker.clone());
		let marker_clone = marker.clone();
		let effect_reactive_node_store = current_reactive_node_store();
		let mount_reactive_node_store = reactive_nodes.clone();
		let first_run = Rc::new(Cell::new(true));
		let first_run_clone = first_run.clone();
		#[cfg(feature = "i18n")]
		let i18n_context = crate::i18n::current_i18n_callback_context();

		let effect = Effect::new_with_timing(
			move || {
				let update = || {
					with_reactive_node_store(&effect_reactive_node_store, || {
						clear_reactive_node_store(&render_reactive_node_store);
						let first_run_resource_counter =
							crate::reactive::resource::current_client_resource_counter();
						let first_run_id_counter =
							crate::reactive::hooks::id::id_counter_snapshot();
						let view =
							with_reactive_node_store(&render_reactive_node_store, || render());

						let preserve_adopted_control =
							refresh_after_control_adoption && is_single_control_view(&view);
						if first_run_clone.replace(false)
							&& (!refresh_after_control_adoption || preserve_adopted_control)
						{
							crate::reactive::resource::set_client_resource_counter(
								first_run_resource_counter,
							);
							crate::reactive::hooks::id::restore_id_counter(first_run_id_counter);
							return;
						}

						if update_activity_boundary_attrs(&current_nodes_clone, &view) {
							return;
						}

						clear_reactive_node_store(&mount_reactive_node_store);

						refresh_current_nodes_before_marker(
							start_marker_clone.as_ref(),
							&marker_clone,
							&current_nodes_clone,
						);
						let old_nodes = {
							let mut nodes = current_nodes_clone.borrow_mut();
							nodes.drain(..).collect::<Vec<_>>()
						};
						for node in old_nodes {
							if let Some(parent_node) = node.parent_node() {
								let _ = parent_node.remove_child(&node);
							}
						}

						let new_nodes =
							with_reactive_node_store(&mount_reactive_node_store, || {
								mount_before_marker(&marker_clone, view)
							});
						*current_nodes_clone.borrow_mut() = new_nodes;
					});
				};
				#[cfg(feature = "i18n")]
				crate::i18n::with_optional_i18n_context(i18n_context.as_ref(), update);
				#[cfg(not(feature = "i18n"))]
				update();
			},
			EffectTiming::Layout,
		);

		Some(Self {
			marker,
			start_marker: Some(start_marker),
			current_nodes,
			reactive_nodes,
			effect: Some(effect),
		})
	}

	pub(crate) fn reactive_node_store(&self) -> ReactiveNodeStore {
		self.reactive_nodes.clone()
	}

	pub(crate) fn refresh_hydrated_current_nodes(&self) {
		refresh_current_nodes_before_marker(
			self.start_marker.as_ref(),
			&self.marker,
			&self.current_nodes,
		);
	}
}

#[cfg(wasm)]
fn is_single_control_view(view: &Page) -> bool {
	match view {
		Page::Element(element) => {
			element.bound_control().is_some() && element.child_views().is_empty()
		}
		Page::Fragment(children) => {
			children.len() == 1 && children.first().is_some_and(is_single_control_view)
		}
		_ => false,
	}
}

#[cfg(wasm)]
impl Drop for ReactiveNode {
	fn drop(&mut self) {
		let _marker_removal = MarkerRemovalGuard::new(self.start_marker.as_ref(), &self.marker);
		drop(self.effect.take());
		clear_reactive_node_store(&self.reactive_nodes);
	}
}

#[cfg(wasm)]
struct MarkerRemovalGuard {
	start_marker: Option<web_sys::Comment>,
	marker: web_sys::Comment,
}

#[cfg(wasm)]
impl MarkerRemovalGuard {
	fn new(start_marker: Option<&web_sys::Comment>, marker: &web_sys::Comment) -> Self {
		Self {
			start_marker: start_marker.cloned(),
			marker: marker.clone(),
		}
	}
}

#[cfg(wasm)]
impl Drop for MarkerRemovalGuard {
	fn drop(&mut self) {
		remove_marker_from_dom(self.start_marker.as_ref());
		remove_marker_from_dom(Some(&self.marker));
	}
}

#[cfg(wasm)]
fn remove_marker_from_dom(marker: Option<&web_sys::Comment>) {
	let Some(marker) = marker else { return };
	let marker_node: web_sys::Node = marker.clone().into();
	if let Some(parent) = marker_node.parent_node() {
		let _ = parent.remove_child(&marker_node);
	}
}

#[cfg(wasm)]
fn refresh_current_nodes_before_marker(
	start_marker: Option<&web_sys::Comment>,
	marker: &web_sys::Comment,
	current_nodes: &Rc<RefCell<Vec<web_sys::Node>>>,
) {
	let first_node = start_marker
		.and_then(|marker| {
			let marker_node: web_sys::Node = marker.clone().into();
			marker_node.next_sibling()
		})
		.or_else(|| current_nodes.borrow().first().cloned());

	let Some(first_node) = first_node else { return };
	let marker_node: web_sys::Node = marker.clone().into();
	let mut nodes = Vec::new();
	let mut next = Some(first_node);
	while let Some(node) = next {
		if node.is_same_node(Some(&marker_node)) {
			break;
		}
		next = node.next_sibling();
		nodes.push(node);
	}
	*current_nodes.borrow_mut() = nodes;
}

#[cfg(wasm)]
fn update_activity_boundary_attrs(
	current_nodes: &Rc<RefCell<Vec<web_sys::Node>>>,
	view: &Page,
) -> bool {
	use wasm_bindgen::JsCast;

	let Page::Element(element_view) = view else {
		return false;
	};

	let Some(activity_mode) = element_view
		.attrs()
		.iter()
		.find(|(name, _)| name.as_ref() == "data-rh-activity")
		.map(|(_, value)| value.as_ref())
	else {
		return false;
	};

	if !element_view
		.attrs()
		.iter()
		.any(|(name, value)| name.as_ref() == "data-rh-state-preserved" && value.as_ref() == "true")
	{
		return false;
	}

	let nodes = current_nodes.borrow();
	if nodes.len() != 1 {
		return false;
	}

	let Some(existing_element) = nodes[0].dyn_ref::<web_sys::Element>() else {
		return false;
	};

	if existing_element
		.get_attribute("data-rh-state-preserved")
		.as_deref()
		!= Some("true")
		|| existing_element.get_attribute("data-rh-activity").is_none()
	{
		return false;
	}

	let _ = existing_element.set_attribute("data-rh-activity", activity_mode);
	let _ = existing_element.set_attribute("data-rh-state-preserved", "true");

	if activity_mode == "hidden" {
		let _ = existing_element.set_attribute("hidden", "hidden");
		let _ = existing_element.set_attribute("aria-hidden", "true");
	} else {
		let _ = existing_element.remove_attribute("hidden");
		let _ = existing_element.remove_attribute("aria-hidden");
	}

	true
}

/// Mounts a Page before a marker node and returns the created DOM nodes.
///
/// This function recursively mounts the view tree and inserts all created
/// nodes before the marker comment node.
#[cfg(wasm)]
fn mount_before_marker(marker: &web_sys::Comment, view: Page) -> Vec<web_sys::Node> {
	use wasm_bindgen::JsCast;

	let document = web_sys::window()
		.expect("window should be available")
		.document()
		.expect("document should be available");

	let parent = marker.parent_node().expect("marker should have a parent");

	let mut nodes = Vec::new();

	match view {
		Page::Element(el) => {
			let mount_element = || {
				// Decompose the element to avoid ownership issues
				let (tag, attrs, children, _is_void, event_handlers, control_binding) =
					el.into_parts_with_control_binding();
				let element = document
					.create_element(&tag)
					.expect("should create element");

				// Set attributes
				for (name, value) in attrs {
					// Skip falsy boolean attributes
					let name_str: &str = name.as_ref();
					if BOOLEAN_ATTRS.contains(&name_str) && !is_boolean_attr_truthy(&value) {
						continue;
					}
					if crate::component::into_page::controlled_attribute_is_overridden(
						control_binding.as_ref(),
						name_str,
					) {
						continue;
					}
					let _ = element.set_attribute(&name, &value);
				}

				let element_wrapper = crate::dom::Element::new(element.clone());
				let mount_children_before_binding = tag == "select";
				let mut children = children.into_iter();
				if mount_children_before_binding {
					for child in children.by_ref() {
						child.mount(&element_wrapper)?;
					}
				}

				let binding_controller = control_binding
					.map(|binding| {
						crate::dom::control_binding::ControlBindingController::mount(
							element_wrapper.clone(),
							binding,
						)
					})
					.transpose()?;
				let mut event_handles = Vec::new();
				for (event_type, handler) in event_handlers {
					let handler_clone = handler.clone();
					#[cfg(feature = "i18n")]
					let i18n_context = crate::i18n::current_i18n_callback_context();
					event_handles.push(element_wrapper.add_event_listener_with_event(
						event_type.as_str(),
						move |event| {
							#[cfg(feature = "i18n")]
							{
								crate::i18n::with_optional_i18n_context(
									i18n_context.as_ref(),
									|| handler_clone(event),
								);
							}
							#[cfg(not(feature = "i18n"))]
							handler_clone(event);
						},
					));
				}

				if !mount_children_before_binding {
					let child_marker = document.create_comment("reactive-element-children");
					element
						.append_child(&child_marker)
						.map_err(|_| MountError::AppendChildFailed)?;
					for child in children {
						mount_before_marker(&child_marker, child);
					}
					let _ = element.remove_child(&child_marker);
				}

				parent
					.insert_before(&element, Some(marker))
					.map_err(|_| MountError::AppendChildFailed)?;
				store_reactive_node((binding_controller, event_handles));
				Ok::<_, MountError>(element.unchecked_into::<web_sys::Node>())
			};
			let Ok(element) = with_reactive_node_transaction(mount_element) else {
				return nodes;
			};
			nodes.push(element);
		}
		Page::Text(text) => {
			let text_node = document.create_text_node(&text);
			let _ = parent.insert_before(&text_node, Some(marker));
			nodes.push(text_node.unchecked_into());
		}
		Page::Fragment(children) => {
			for child in children {
				nodes.extend(mount_before_marker(marker, child));
			}
		}
		Page::KeyedFragment(children) => {
			for (_, child) in children {
				nodes.extend(mount_before_marker(marker, child));
			}
		}
		Page::Outlet(outlet) => {
			let id = outlet.id().map(str::to_string);
			if let Some(child) = outlet.into_child() {
				nodes.extend(mount_before_marker(marker, child));
			} else if let Some(id) = id {
				let element = document
					.create_element("reinhardt-outlet")
					.expect("should create outlet host");
				let _ = element.set_attribute("data-rh-outlet-id", &id);
				let _ = element.set_attribute("style", "display: contents;");
				let _ = parent.insert_before(&element, Some(marker));
				nodes.push(element.unchecked_into());
			}
		}
		Page::Empty => {}
		Page::WithHead { view, .. } => {
			// Head is handled separately; just mount the content
			nodes.extend(mount_before_marker(marker, *view));
		}
		Page::ReactiveIf(reactive_if) => {
			// Decompose the ReactiveIf to get the closures
			let (condition, then_view, else_view) = reactive_if.into_parts();

			// Create a nested ReactiveIfNode
			// First, create a new marker for this nested reactive if
			let nested_marker = document.create_comment("reactive-if-nested");
			let _ = parent.insert_before(&nested_marker, Some(marker));
			nodes.push(nested_marker.clone().unchecked_into());

			// Create a temporary parent wrapper to use ReactiveIfNode
			let temp_parent =
				crate::dom::Element::new(parent.clone().unchecked_into::<web_sys::Element>());

			// Use the nested marker as the anchor point
			let nested_node = ReactiveIfNode::new(&temp_parent, condition, then_view, else_view);

			// Store the nested node to keep it alive
			store_reactive_node(nested_node);
		}
		Page::Reactive(reactive) => {
			// Create a nested ReactiveNode
			// First, create a new marker for this nested reactive
			let nested_marker = document.create_comment("reactive-nested");
			let _ = parent.insert_before(&nested_marker, Some(marker));
			nodes.push(nested_marker.clone().unchecked_into());

			// Create a temporary parent wrapper to use ReactiveNode
			let temp_parent =
				crate::dom::Element::new(parent.clone().unchecked_into::<web_sys::Element>());

			// Get the render closure
			let render = reactive.into_render();

			// Create the nested ReactiveNode
			let nested_node = ReactiveNode::new(&temp_parent, render);

			// Store the nested node to keep it alive
			store_reactive_node(nested_node);
		}
		Page::Suspense(node) => {
			nodes.extend(mount_before_marker(marker, node.render_branch()));
		}
		Page::Deferred(node) => {
			nodes.extend(mount_before_marker(marker, node.content()));
		}
	}

	nodes
}

// Note: is_boolean_attr_truthy and BOOLEAN_ATTRS are imported from reinhardt_core::types::page

#[cfg(all(test, native))]
mod tests {
	use std::cell::Cell;
	use std::panic::{AssertUnwindSafe, catch_unwind};
	use std::rc::Rc;

	use rstest::rstest;

	use super::*;

	struct CyclicDropProbe {
		drops: Rc<Cell<usize>>,
		_store_cycle: ReactiveNodeStore,
	}

	impl Drop for CyclicDropProbe {
		fn drop(&mut self) {
			self.drops.set(self.drops.get() + 1);
		}
	}

	#[rstest]
	fn reactive_node_transaction_rolls_back_cycles_during_unwind() {
		// Arrange
		let destination = new_reactive_node_store();
		let drops = Rc::new(Cell::new(0));
		let transaction_drops = Rc::clone(&drops);

		// Act
		let result = catch_unwind(AssertUnwindSafe(|| {
			let transaction = ReactiveNodeTransaction::new(destination.clone());
			let staged = transaction.store();
			staged.borrow_mut().push(Box::new(CyclicDropProbe {
				drops: transaction_drops,
				_store_cycle: staged.clone(),
			}));
			panic!("transaction rollback");
		}));

		// Assert
		assert!(result.is_err());
		assert_eq!(drops.get(), 1);
		assert!(destination.borrow().is_empty());
	}

	#[rstest]
	fn reactive_node_transaction_transfers_committed_owners() {
		// Arrange
		let destination = new_reactive_node_store();
		let drops = Rc::new(Cell::new(0));
		let mut transaction = ReactiveNodeTransaction::new(destination.clone());
		let staged = transaction.store();
		staged.borrow_mut().push(Box::new(CyclicDropProbe {
			drops: Rc::clone(&drops),
			_store_cycle: staged.clone(),
		}));

		// Act
		transaction.commit();
		drop(transaction);

		// Assert
		assert_eq!(drops.get(), 0);
		assert_eq!(destination.borrow().len(), 1);
		clear_reactive_node_store(&destination);
		assert_eq!(drops.get(), 1);
	}
}
