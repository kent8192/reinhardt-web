//! Hydration Runtime
//!
//! This module provides the main entry point for client-side hydration,
//! connecting reactive state with SSR-rendered DOM elements.

use crate::component::Component;
use crate::ssr::SsrState;
use std::collections::HashMap;

#[cfg(wasm)]
use crate::dom::{Element, document};

#[cfg(wasm)]
use crate::component::{
	Page, new_reactive_node_store, store_reactive_node, with_reactive_node_store,
};

#[cfg(wasm)]
use crate::ssr::HYDRATION_ATTR_ID;

/// Errors that can occur during hydration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HydrationError {
	/// The hydration root element was not found.
	RootNotFound(String),
	/// SSR state could not be parsed.
	StateParseError(String),
	/// A hydration marker was not found.
	MarkerNotFound(String),
	/// DOM structure doesn't match expected structure.
	StructureMismatch {
		/// The hydration ID.
		id: String,
		/// Expected element.
		expected: String,
		/// Actual element.
		actual: String,
	},
	/// Event attachment failed.
	EventAttachmentFailed(String),
}

impl std::fmt::Display for HydrationError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::RootNotFound(id) => write!(f, "Hydration root element not found: {}", id),
			Self::StateParseError(msg) => write!(f, "Failed to parse SSR state: {}", msg),
			Self::MarkerNotFound(id) => write!(f, "Hydration marker not found: {}", id),
			Self::StructureMismatch {
				id,
				expected,
				actual,
			} => {
				write!(
					f,
					"DOM structure mismatch at {}: expected {}, found {}",
					id, expected, actual
				)
			}
			Self::EventAttachmentFailed(msg) => write!(f, "Event attachment failed: {}", msg),
		}
	}
}

impl std::error::Error for HydrationError {}

/// Context for hydration operations.
#[derive(Debug)]
pub struct HydrationContext {
	/// The restored SSR state.
	state: SsrState,
	/// Mapping of hydration IDs to signal values.
	signals: HashMap<String, serde_json::Value>,
	/// Mapping of hydration IDs to component props.
	props: HashMap<String, serde_json::Value>,
	/// Whether hydration has been completed.
	hydrated: bool,
}

impl Default for HydrationContext {
	fn default() -> Self {
		Self::new()
	}
}

impl HydrationContext {
	/// Creates a new hydration context.
	pub fn new() -> Self {
		Self {
			state: SsrState::new(),
			signals: HashMap::new(),
			props: HashMap::new(),
			hydrated: false,
		}
	}

	/// Creates a context from SSR state.
	pub fn from_state(state: SsrState) -> Self {
		Self {
			state,
			signals: HashMap::new(),
			props: HashMap::new(),
			hydrated: false,
		}
	}

	/// Restores state from the `<script id="ssr-state">` element's JSON content.
	#[cfg(wasm)]
	pub fn from_window() -> Result<Self, HydrationError> {
		let window = web_sys::window()
			.ok_or_else(|| HydrationError::StateParseError("Window not available".to_string()))?;

		let document = window
			.document()
			.ok_or_else(|| HydrationError::StateParseError("Document not available".to_string()))?;

		let element = document.get_element_by_id("ssr-state").ok_or_else(|| {
			HydrationError::StateParseError("SSR state element not found".to_string())
		})?;

		let json = element.text_content().ok_or_else(|| {
			HydrationError::StateParseError("SSR state element is empty".to_string())
		})?;

		if json.trim().is_empty() {
			return Ok(Self::new());
		}

		let state = SsrState::from_json(&json)
			.map_err(|e| HydrationError::StateParseError(e.to_string()))?;

		Ok(Self::from_state(state))
	}

	/// Non-WASM version that returns an empty context.
	#[cfg(native)]
	pub fn from_window() -> Result<Self, HydrationError> {
		Ok(Self::new())
	}

	/// Gets a signal value by its hydration ID.
	pub fn get_signal(&self, id: &str) -> Option<&serde_json::Value> {
		self.signals.get(id).or_else(|| self.state.get_signal(id))
	}

	/// Gets component props by its hydration ID.
	pub fn get_props(&self, id: &str) -> Option<&serde_json::Value> {
		self.props.get(id).or_else(|| self.state.get_props(id))
	}

	/// Gets SSR metadata by key.
	pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
		self.state.get_metadata(key)
	}

	/// Gets resource state by deterministic resource ID.
	pub fn get_resource_state(&self, id: &str) -> Option<&serde_json::Value> {
		self.state.get_resource_state(id)
	}

	/// Marks hydration as complete.
	pub fn mark_hydrated(&mut self) {
		self.hydrated = true;
	}

	/// Checks if hydration is complete.
	pub fn is_hydrated(&self) -> bool {
		self.hydrated
	}
}

/// Hydrates a component into the specified root element.
#[cfg(wasm)]
pub fn hydrate<C: Component>(component: &C, root: &Element) -> Result<(), HydrationError> {
	use super::events::EventRegistry;
	use super::reconcile::reconcile;

	web_sys::console::log_1(&"[Hydration] Starting...".into());

	// 1. Restore SSR state
	let mut context = HydrationContext::from_window()?;
	let scope = reinhardt_core::reactive::ReactiveScope::new();
	scope.enter(|| {
		#[cfg(feature = "i18n")]
		let i18n_guard = crate::i18n::provide_i18n_from_hydration_context(&context).map_err(|e| {
			HydrationError::StateParseError(format!("Failed to hydrate i18n state: {}", e))
		})?;

		let prepass_store = new_reactive_node_store();
		let view = with_reactive_node_store(&prepass_store, || -> Result<_, HydrationError> {
			// 2. Render the component to get expected structure
			let view = component.render();
			let resource_counter_offset =
				crate::reactive::resource::current_client_resource_counter();
			let id_counter_offset = crate::reactive::hooks::id::id_counter_snapshot();
			web_sys::console::log_1(&"[Hydration] View rendered".into());

			// 3. Reconcile DOM structure
			crate::reactive::resource::set_client_resource_counter(resource_counter_offset);
			crate::reactive::hooks::id::restore_id_counter(id_counter_offset);
			reconcile(root, &view).map_err(|e| {
				HydrationError::StateParseError(format!("Reconciliation failed: {}", e))
			})?;
			web_sys::console::log_1(&"[Hydration] Reconciliation complete".into());

			// 4. Attach event handlers
			crate::reactive::resource::set_client_resource_counter(resource_counter_offset);
			crate::reactive::hooks::id::restore_id_counter(id_counter_offset);
			let mut registry = EventRegistry::new();
			attach_events_recursive(root, &view, &mut registry)?;
			crate::reactive::resource::set_client_resource_counter(resource_counter_offset);
			crate::reactive::hooks::id::restore_id_counter(id_counter_offset);
			web_sys::console::log_1(&"[Hydration] Events attached".into());
			Ok(view)
		})?;

		// 5. Install reactive DOM owners for hydrated reactive views
		install_hydrated_reactive_nodes(root, &view);
		web_sys::console::log_1(&"[Hydration] Reactive nodes installed".into());

		#[cfg(feature = "i18n")]
		if let Some(i18n_guard) = i18n_guard {
			crate::i18n::retain_hydrated_i18n_context(i18n_guard);
		}

		Ok(())
	})?;
	crate::component::store_reactive_scope(scope);

	// 6. Mark hydration complete
	context.mark_hydrated();
	mark_hydration_complete_internal();
	web_sys::console::log_1(&"[Hydration] Complete!".into());

	Ok(())
}

#[cfg(wasm)]
fn with_hydration_prepass_store<R>(f: impl FnOnce() -> R) -> R {
	let store = new_reactive_node_store();
	with_reactive_node_store(&store, f)
}

#[cfg(wasm)]
fn install_hydrated_reactive_nodes(element: &Element, view: &Page) {
	match view {
		Page::Element(element_view) => {
			install_hydrated_element_children(element, element_view.child_views());
		}
		Page::WithHead { view, .. } => install_hydrated_reactive_nodes(element, view),
		Page::Fragment(children) => install_hydrated_element_children(element, children),
		Page::KeyedFragment(children) => {
			let child_views = children
				.iter()
				.map(|(_, child)| child.clone())
				.collect::<Vec<_>>();
			install_hydrated_element_children(element, &child_views);
		}
		Page::Outlet(outlet) => {
			if let Some(child) = outlet.child() {
				install_hydrated_reactive_nodes(element, child);
			}
		}
		Page::Reactive(reactive) => {
			let render_store = new_reactive_node_store();
			let rendered = with_reactive_node_store(&render_store, || reactive.render());
			with_hydration_prepass_store(|| {
				split_coalesced_text_children(element, std::slice::from_ref(&rendered));
			});
			let nodes = relevant_child_nodes(element);
			let hydrated_node = crate::component::ReactiveNode::hydrate_at(
				element.as_web_sys().clone().into(),
				None,
				nodes.clone(),
				reactive.clone().into_render(),
				render_store,
			);
			let boundary_sibling = hydrated_node.as_ref().map(|node| node.marker_node());
			if let Some(node) = hydrated_node.as_ref() {
				with_reactive_node_store(&node.reactive_node_store(), || {
					install_hydrated_child_reactive_nodes(
						&element.as_web_sys().clone().into(),
						&nodes,
						boundary_sibling,
						&rendered,
					);
				});
			} else {
				install_hydrated_child_reactive_nodes(
					&element.as_web_sys().clone().into(),
					&nodes,
					boundary_sibling,
					&rendered,
				);
			}
			if let Some(node) = hydrated_node {
				node.refresh_hydrated_current_nodes();
				store_reactive_node(node);
			}
		}
		Page::ReactiveIf(reactive_if) => {
			let branch_store = new_reactive_node_store();
			let branch_view = with_reactive_node_store(&branch_store, || {
				if reactive_if.condition() {
					reactive_if.then_view()
				} else {
					reactive_if.else_view()
				}
			});
			with_hydration_prepass_store(|| {
				split_coalesced_text_children(element, std::slice::from_ref(&branch_view));
			});
			let nodes = relevant_child_nodes(element);
			let (condition, then_view, else_view) = reactive_if.clone().into_parts();
			let hydrated_node = crate::component::ReactiveIfNode::hydrate_at(
				element.as_web_sys().clone().into(),
				None,
				nodes.clone(),
				condition,
				then_view,
				else_view,
				branch_store,
			);
			let boundary_sibling = hydrated_node.as_ref().map(|node| node.marker_node());
			if let Some(node) = hydrated_node.as_ref() {
				with_reactive_node_store(&node.reactive_node_store(), || {
					install_hydrated_child_reactive_nodes(
						&element.as_web_sys().clone().into(),
						&nodes,
						boundary_sibling,
						&branch_view,
					);
				});
			} else {
				install_hydrated_child_reactive_nodes(
					&element.as_web_sys().clone().into(),
					&nodes,
					boundary_sibling,
					&branch_view,
				);
			}
			if let Some(node) = hydrated_node {
				node.refresh_hydrated_current_nodes();
				store_reactive_node(node);
			}
		}
		_ => {}
	}
}

#[cfg(wasm)]
fn install_hydrated_element_children(element: &Element, children: &[Page]) {
	with_hydration_prepass_store(|| split_coalesced_text_children(element, children));
	let actual_nodes = relevant_child_nodes(element);
	install_hydrated_children_reactive_nodes(
		&element.as_web_sys().clone().into(),
		&actual_nodes,
		None,
		children,
	);
}

#[cfg(wasm)]
fn install_hydrated_children_reactive_nodes(
	parent: &web_sys::Node,
	nodes: &[web_sys::Node],
	next_sibling: Option<web_sys::Node>,
	children: &[Page],
) {
	let mut index = 0;
	for child in children {
		let node_count = with_hydration_prepass_store(|| hydrated_node_count(child));
		let end = (index + node_count).min(nodes.len());
		let child_next_sibling = nodes.get(end).cloned().or_else(|| next_sibling.clone());
		install_hydrated_child_reactive_nodes(
			parent,
			&nodes[index..end],
			child_next_sibling,
			child,
		);
		index = end;
	}
}

#[cfg(wasm)]
fn install_hydrated_child_reactive_nodes(
	parent: &web_sys::Node,
	nodes: &[web_sys::Node],
	next_sibling: Option<web_sys::Node>,
	view: &Page,
) {
	match view {
		Page::Reactive(reactive) => {
			let render_store = new_reactive_node_store();
			let rendered = with_reactive_node_store(&render_store, || reactive.render());
			let mut boundary_sibling = next_sibling.clone();
			let hydrated_node = crate::component::ReactiveNode::hydrate_at(
				parent.clone(),
				next_sibling.clone(),
				nodes.to_vec(),
				reactive.clone().into_render(),
				render_store,
			);
			if let Some(node) = hydrated_node.as_ref() {
				boundary_sibling = Some(node.marker_node());
			}
			if let Some(node) = hydrated_node.as_ref() {
				with_reactive_node_store(&node.reactive_node_store(), || {
					install_hydrated_child_reactive_nodes(
						parent,
						nodes,
						boundary_sibling,
						&rendered,
					);
				});
			} else {
				install_hydrated_child_reactive_nodes(parent, nodes, boundary_sibling, &rendered);
			}
			if let Some(node) = hydrated_node {
				node.refresh_hydrated_current_nodes();
				store_reactive_node(node);
			}
		}
		Page::ReactiveIf(reactive_if) => {
			let branch_store = new_reactive_node_store();
			let branch_view = with_reactive_node_store(&branch_store, || {
				if reactive_if.condition() {
					reactive_if.then_view()
				} else {
					reactive_if.else_view()
				}
			});
			let (condition, then_view, else_view) = reactive_if.clone().into_parts();
			let mut boundary_sibling = next_sibling.clone();
			let hydrated_node = crate::component::ReactiveIfNode::hydrate_at(
				parent.clone(),
				next_sibling.clone(),
				nodes.to_vec(),
				condition,
				then_view,
				else_view,
				branch_store,
			);
			if let Some(node) = hydrated_node.as_ref() {
				boundary_sibling = Some(node.marker_node());
			}
			if let Some(node) = hydrated_node.as_ref() {
				with_reactive_node_store(&node.reactive_node_store(), || {
					install_hydrated_child_reactive_nodes(
						parent,
						nodes,
						boundary_sibling,
						&branch_view,
					);
				});
			} else {
				install_hydrated_child_reactive_nodes(
					parent,
					nodes,
					boundary_sibling,
					&branch_view,
				);
			}
			if let Some(node) = hydrated_node {
				node.refresh_hydrated_current_nodes();
				store_reactive_node(node);
			}
		}
		Page::Element(_) => {
			if let Some(element) = nodes
				.first()
				.and_then(|node| wasm_bindgen::JsCast::dyn_ref::<web_sys::Element>(node))
			{
				install_hydrated_reactive_nodes(&Element::new(element.clone()), view);
			}
		}
		Page::WithHead { view, .. } => {
			install_hydrated_child_reactive_nodes(parent, nodes, next_sibling, view);
		}
		Page::Fragment(children) => {
			install_hydrated_children_reactive_nodes(parent, nodes, next_sibling, children)
		}
		Page::KeyedFragment(children) => {
			let child_views = children
				.iter()
				.map(|(_, child)| child.clone())
				.collect::<Vec<_>>();
			install_hydrated_children_reactive_nodes(parent, nodes, next_sibling, &child_views);
		}
		Page::Outlet(outlet) => {
			if let Some(child) = outlet.child() {
				install_hydrated_child_reactive_nodes(parent, nodes, next_sibling, child);
			}
		}
		Page::Suspense(node) => {
			let branch_view = node.render_branch();
			install_hydrated_child_reactive_nodes(parent, nodes, next_sibling, &branch_view);
		}
		Page::Deferred(node) => {
			let content_view = node.content();
			install_hydrated_child_reactive_nodes(parent, nodes, next_sibling, &content_view);
		}
		Page::Text(_) | Page::Empty => {}
	}
}

#[cfg(wasm)]
#[derive(Clone, Debug, PartialEq, Eq)]
enum ExpectedDomChild {
	Text(String),
	Node,
}

#[cfg(wasm)]
fn hydrated_node_count(view: &Page) -> usize {
	match view {
		Page::Text(text) => usize::from(!normalize_whitespace(text.as_ref()).is_empty()),
		Page::Element(_) => 1,
		Page::Fragment(children) => children.iter().map(hydrated_node_count).sum(),
		Page::KeyedFragment(children) => children
			.iter()
			.map(|(_, child)| hydrated_node_count(child))
			.sum(),
		Page::Outlet(outlet) => outlet.child().map(hydrated_node_count).unwrap_or(0),
		Page::WithHead { view, .. } => hydrated_node_count(view),
		Page::ReactiveIf(reactive_if) => {
			let branch_view = if reactive_if.condition() {
				reactive_if.then_view()
			} else {
				reactive_if.else_view()
			};
			hydrated_node_count(&branch_view)
		}
		Page::Reactive(reactive) => hydrated_node_count(&reactive.render()),
		Page::Suspense(node) => hydrated_node_count(&node.render_branch()),
		Page::Deferred(node) => hydrated_node_count(&node.content()),
		Page::Empty => 0,
	}
}

#[cfg(wasm)]
fn split_coalesced_text_children(element: &Element, children: &[Page]) {
	use wasm_bindgen::JsCast;

	let mut expected = Vec::new();
	for child in children {
		collect_expected_dom_children(child, &mut expected);
	}

	let mut actual_nodes = relevant_child_nodes(element);
	let document = web_sys::window()
		.and_then(|window| window.document())
		.expect("document should be available");
	let mut actual_index = 0;

	for expected_child in expected {
		match expected_child {
			ExpectedDomChild::Node => {
				actual_index += 1;
			}
			ExpectedDomChild::Text(expected_text) => {
				if expected_text.is_empty() {
					continue;
				}
				let Some(node) = actual_nodes.get(actual_index).cloned() else {
					return;
				};
				if node.node_type() != web_sys::Node::TEXT_NODE {
					actual_index += 1;
					continue;
				}

				let actual_text = node.text_content().unwrap_or_default();
				if actual_text == expected_text {
					actual_index += 1;
					continue;
				}
				if !actual_text.starts_with(&expected_text) {
					actual_index += 1;
					continue;
				}

				let remainder = actual_text[expected_text.len()..].to_string();
				node.set_text_content(Some(&expected_text));
				if !remainder.is_empty() {
					let remainder_node = document.create_text_node(&remainder);
					if let Some(parent) = node.parent_node() {
						let next = node.next_sibling();
						let _ = parent.insert_before(&remainder_node, next.as_ref());
						actual_nodes.insert(actual_index + 1, remainder_node.unchecked_into());
					}
				}
				actual_index += 1;
			}
		}
	}
}

#[cfg(wasm)]
fn collect_expected_dom_children(view: &Page, children: &mut Vec<ExpectedDomChild>) {
	match view {
		Page::Empty => {}
		Page::Text(text) => {
			if !text.is_empty() {
				children.push(ExpectedDomChild::Text(text.to_string()));
			}
		}
		Page::Fragment(fragment_children) => {
			for child in fragment_children {
				collect_expected_dom_children(child, children);
			}
		}
		Page::KeyedFragment(keyed_children) => {
			for (_, child) in keyed_children {
				collect_expected_dom_children(child, children);
			}
		}
		Page::Outlet(outlet) => {
			if let Some(child) = outlet.child() {
				collect_expected_dom_children(child, children);
			}
		}
		Page::WithHead { view, .. } => collect_expected_dom_children(view, children),
		Page::ReactiveIf(reactive_if) => {
			let branch_view = if reactive_if.condition() {
				reactive_if.then_view()
			} else {
				reactive_if.else_view()
			};
			collect_expected_dom_children(&branch_view, children);
		}
		Page::Reactive(reactive) => {
			let rendered_view = reactive.render();
			collect_expected_dom_children(&rendered_view, children);
		}
		Page::Suspense(node) => {
			let branch_view = node.render_branch();
			collect_expected_dom_children(&branch_view, children);
		}
		Page::Deferred(node) => {
			let content_view = node.content();
			collect_expected_dom_children(&content_view, children);
		}
		Page::Element(_) => children.push(ExpectedDomChild::Node),
	}
}

#[cfg(wasm)]
fn relevant_child_nodes(element: &Element) -> Vec<web_sys::Node> {
	let child_nodes = element.as_web_sys().child_nodes();
	(0..child_nodes.length())
		.filter_map(|index| child_nodes.item(index))
		.filter(is_relevant_child_node)
		.collect()
}

#[cfg(wasm)]
fn is_relevant_child_node(node: &web_sys::Node) -> bool {
	match node.node_type() {
		web_sys::Node::ELEMENT_NODE => true,
		web_sys::Node::TEXT_NODE => {
			!normalize_whitespace(&node.text_content().unwrap_or_default()).is_empty()
		}
		_ => false,
	}
}

#[cfg(wasm)]
fn normalize_whitespace(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Non-WASM version for testing.
#[cfg(native)]
pub fn hydrate<C: Component>(_component: &C, _root: &str) -> Result<(), HydrationError> {
	Ok(())
}

/// Hydrates a component at the default root element (#app).
#[cfg(wasm)]
pub fn hydrate_root<C: Component + Default>() -> Result<(), HydrationError> {
	let component = C::default();
	let doc = document();
	let root = doc
		.query_selector("#app")
		.map_err(|e| HydrationError::StateParseError(format!("Query selector failed: {}", e)))?
		.ok_or_else(|| HydrationError::RootNotFound("#app".to_string()))?;

	hydrate(&component, &root)
}

/// Non-WASM version for testing.
#[cfg(native)]
pub fn hydrate_root<C: Component + Default>() -> Result<(), HydrationError> {
	Ok(())
}

/// Attaches event handlers to a mounted view (CSR mode).
///
/// This is a convenience function for client-side rendering (CSR) applications.
/// After mounting a view with `view.mount()`, call this function to attach event handlers.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::hydration::attach_events_to_mounted_view;
/// use reinhardt_pages::dom::Element;
///
/// // Mount the view
/// let view = my_component();
/// let root = Element::new(root_element);
/// view.mount(&root)?;
///
/// // Attach event handlers
/// attach_events_to_mounted_view(&root, &view)?;
/// ```
#[cfg(wasm)]
pub fn attach_events_to_mounted_view(
	element: &Element,
	view: &crate::component::Page,
) -> Result<(), HydrationError> {
	use super::events::EventRegistry;

	web_sys::console::log_1(&"[CSR] Attaching events to mounted view...".into());

	let mut registry = EventRegistry::new();
	attach_events_recursive(element, view, &mut registry)?;

	web_sys::console::log_1(&"[CSR] Events attached successfully!".into());

	Ok(())
}

/// Recursively attaches event handlers to DOM elements.
///
/// This function can be used for both SSR+Hydration and CSR (client-side rendering only) scenarios.
/// For CSR, call this after mounting a view to attach event handlers.
#[cfg(wasm)]
pub(crate) fn attach_events_recursive(
	element: &Element,
	view: &crate::component::Page,
	registry: &mut super::events::EventRegistry,
) -> Result<(), HydrationError> {
	use super::events::attach_event;
	use crate::component::Page;

	match view {
		Page::Element(el_view) => {
			let tag = el_view.tag_name();
			let event_count = el_view.event_handlers().len();

			if event_count > 0 {
				web_sys::console::log_1(
					&format!("[attach_events] {} has {} event handlers", tag, event_count).into(),
				);
			}

			// Attach events from the view's event handlers
			for (event_type, handler) in el_view.event_handlers() {
				web_sys::console::log_1(
					&format!("[attach_events] Attaching {:?} to {}", event_type, tag).into(),
				);

				attach_event(element, event_type, handler.clone(), registry)
					.map_err(|e| HydrationError::EventAttachmentFailed(e.to_string()))?;
			}

			// Recursively process children
			let children = element.children();
			let view_children = el_view.child_views();

			for (i, child_view) in view_children.iter().enumerate() {
				if i < children.len() {
					attach_events_recursive(&children[i], child_view, registry)?;
				}
			}
		}
		Page::Fragment(views) => {
			let children = element.children();
			for (i, child_view) in views.iter().enumerate() {
				if i < children.len() {
					attach_events_recursive(&children[i], child_view, registry)?;
				}
			}
		}
		Page::KeyedFragment(views) => {
			let children = element.children();
			for (i, (_, child_view)) in views.iter().enumerate() {
				if i < children.len() {
					attach_events_recursive(&children[i], child_view, registry)?;
				}
			}
		}
		Page::Outlet(outlet) => {
			if let Some(child) = outlet.child() {
				attach_events_recursive(element, child, registry)?;
			}
		}
		Page::Text(_) | Page::Empty => {
			// No events to attach
		}
		Page::WithHead { view, .. } => {
			// Head section doesn't have event handlers
			// Attach events to the inner view
			attach_events_recursive(element, view, registry)?;
		}
		Page::ReactiveIf(reactive_if) => {
			// For hydration, evaluate the condition and attach events to the rendered branch
			let branch_view = if reactive_if.condition() {
				reactive_if.then_view()
			} else {
				reactive_if.else_view()
			};
			attach_events_recursive(element, &branch_view, registry)?;
		}
		Page::Reactive(reactive) => {
			// For hydration, evaluate the render closure and attach events to the resulting view
			let rendered_view = reactive.render();
			attach_events_recursive(element, &rendered_view, registry)?;
		}
		Page::Suspense(node) => {
			let branch_view = node.render_branch();
			attach_events_recursive(element, &branch_view, registry)?;
		}
		Page::Deferred(node) => {
			let content_view = node.content();
			attach_events_recursive(element, &content_view, registry)?;
		}
	}

	Ok(())
}

/// Finds all elements with hydration markers in the given root.
#[cfg(wasm)]
// Allow dead_code: WASM hydration helper reserved for future hydration runtime
#[allow(dead_code)]
pub(super) fn find_hydration_markers(root: &Element) -> Vec<(String, Element)> {
	let mut markers = Vec::new();
	find_markers_recursive(root, &mut markers);
	markers
}

#[cfg(wasm)]
fn find_markers_recursive(element: &Element, markers: &mut Vec<(String, Element)>) {
	if let Some(id) = element.get_attribute(HYDRATION_ATTR_ID) {
		markers.push((id, element.clone()));
	}

	for child in element.children() {
		find_markers_recursive(&child, markers);
	}
}

/// Non-WASM version for testing.
#[cfg(native)]
// Allow dead_code: non-WASM stub for hydration marker scanning
#[allow(dead_code)]
pub(super) fn find_hydration_markers(_root: &str) -> Vec<(String, String)> {
	Vec::new()
}

// Global hydration state management
type HydrationListener = Box<dyn Fn(bool) + 'static>;
type HydrationListeners = Vec<HydrationListener>;

thread_local! {
	static HYDRATION_COMPLETE: std::cell::RefCell<bool> = const { std::cell::RefCell::new(false) };
	static HYDRATION_LISTENERS: std::cell::RefCell<HydrationListeners> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Initialize hydration state (called before hydration starts)
pub fn init_hydration_state() {
	HYDRATION_COMPLETE.with(|state| {
		*state.borrow_mut() = false;
	});
}

/// Check if hydration is complete
pub fn is_hydration_complete() -> bool {
	HYDRATION_COMPLETE.with(|state| *state.borrow())
}

/// Register a callback to be called when hydration completes
pub fn on_hydration_complete<F>(callback: F)
where
	F: Fn(bool) + 'static,
{
	HYDRATION_LISTENERS.with(|listeners| {
		listeners.borrow_mut().push(Box::new(callback));
	});
}

/// Mark hydration as complete and notify all listeners (internal)
#[cfg(wasm)]
fn mark_hydration_complete_internal() {
	HYDRATION_COMPLETE.with(|state| {
		*state.borrow_mut() = true;
	});

	// Notify all listeners
	HYDRATION_LISTENERS.with(|listeners| {
		for listener in listeners.borrow().iter() {
			listener(true);
		}
	});
}

/// Manually mark hydration as complete (public API)
///
/// This function can be called explicitly to mark hydration as complete
/// when not using the automatic hydration process (e.g., when using mount() instead of hydrate()).
#[cfg(wasm)]
pub fn mark_hydration_complete() {
	mark_hydration_complete_internal();
}

#[cfg(test)]
mod tests {
	use super::*;
	#[cfg(wasm)]
	use crate::component::{IntoPage, PageElement, cleanup_reactive_nodes};
	#[cfg(wasm)]
	use crate::reactive::hooks::use_retained_effect;
	#[cfg(wasm)]
	use crate::reactive::{Signal, with_runtime};
	#[cfg(wasm)]
	use std::cell::RefCell;
	#[cfg(wasm)]
	use std::rc::Rc;
	#[cfg(wasm)]
	use wasm_bindgen_test::*;

	#[cfg(wasm)]
	wasm_bindgen_test_configure!(run_in_browser);

	#[test]
	fn test_hydration_context_new() {
		let ctx = HydrationContext::new();
		assert!(!ctx.is_hydrated());
	}

	#[test]
	fn test_hydration_context_mark_hydrated() {
		let mut ctx = HydrationContext::new();
		assert!(!ctx.is_hydrated());
		ctx.mark_hydrated();
		assert!(ctx.is_hydrated());
	}

	#[test]
	fn test_hydration_context_from_state() {
		let mut state = SsrState::new();
		state.add_signal("count", 42);
		let ctx = HydrationContext::from_state(state);
		assert_eq!(ctx.get_signal("count"), Some(&serde_json::json!(42)));
	}

	#[test]
	fn test_hydration_context_get_resource_state() {
		let mut state = SsrState::new();
		state.add_resource_state("rh-res-0", serde_json::json!({"Success": {"name": "Ada"}}));
		let ctx = HydrationContext::from_state(state);
		assert_eq!(
			ctx.get_resource_state("rh-res-0"),
			Some(&serde_json::json!({"Success": {"name": "Ada"}}))
		);
	}

	#[test]
	fn test_hydration_error_display() {
		let err = HydrationError::RootNotFound("#app".to_string());
		assert_eq!(err.to_string(), "Hydration root element not found: #app");

		let err = HydrationError::StructureMismatch {
			id: "rh-0".to_string(),
			expected: "div".to_string(),
			actual: "span".to_string(),
		};
		assert!(err.to_string().contains("DOM structure mismatch"));
	}

	#[test]
	fn test_hydration_context_from_window_non_wasm() {
		// Non-WASM version should return empty context
		let ctx = HydrationContext::from_window().unwrap();
		assert!(!ctx.is_hydrated());
	}

	#[cfg(wasm)]
	#[wasm_bindgen_test]
	fn hydration_preview_replaces_retained_effect_in_same_render_store() {
		cleanup_reactive_nodes();
		let document = web_sys::window().unwrap().document().unwrap();
		let root = document.create_element("div").unwrap();
		root.set_inner_html("<span>value:0</span>");
		document.body().unwrap().append_child(&root).unwrap();

		let render_signal = Signal::new(0_i32);
		let effect_signal = Signal::new(0_i32);
		let effect_log = Rc::new(RefCell::new(Vec::new()));
		let view = Page::reactive({
			let render_signal = render_signal.clone();
			let effect_signal = effect_signal.clone();
			let effect_log = Rc::clone(&effect_log);
			move || {
				let render_value = render_signal.get();
				use_retained_effect(
					{
						let effect_signal = effect_signal.clone();
						let effect_log = Rc::clone(&effect_log);
						move || {
							let value = effect_signal.get();
							effect_log.borrow_mut().push(format!("run:{value}"));
							let effect_log = Rc::clone(&effect_log);
							Some(move || effect_log.borrow_mut().push("cleanup".to_string()))
						}
					},
					(effect_signal.clone(),),
				);
				PageElement::new("span")
					.child(format!("value:{render_value}"))
					.into_page()
			}
		});

		install_hydrated_reactive_nodes(&Element::new(root.clone()), &view);
		effect_signal.set(1);
		with_runtime(|runtime| runtime.flush_updates());

		let log = effect_log.borrow();
		assert_eq!(
			log.iter().filter(|entry| entry.as_str() == "run:1").count(),
			1,
			"only the tracked hydration render should retain an effect: {log:?}"
		);
		drop(log);
		cleanup_reactive_nodes();
		root.remove();
	}

	#[cfg(wasm)]
	#[wasm_bindgen_test]
	fn hydration_element_child_prepasses_do_not_retain_effects() {
		cleanup_reactive_nodes();
		let document = web_sys::window().unwrap().document().unwrap();
		let root = document.create_element("div").unwrap();
		root.set_inner_html("<span>value:0</span>");
		document.body().unwrap().append_child(&root).unwrap();

		let effect_signal = Signal::new(0_i32);
		let effect_log = Rc::new(RefCell::new(Vec::new()));
		let view = PageElement::new("div")
			.child(Page::reactive({
				let effect_signal = effect_signal.clone();
				let effect_log = Rc::clone(&effect_log);
				move || {
					use_retained_effect(
						{
							let effect_signal = effect_signal.clone();
							let effect_log = Rc::clone(&effect_log);
							move || {
								let value = effect_signal.get();
								effect_log.borrow_mut().push(format!("run:{value}"));
								let effect_log = Rc::clone(&effect_log);
								Some(move || effect_log.borrow_mut().push("cleanup".to_string()))
							}
						},
						(effect_signal.clone(),),
					);
					PageElement::new("span").child("value:0").into_page()
				}
			}))
			.into_page();

		install_hydrated_reactive_nodes(&Element::new(root.clone()), &view);
		effect_signal.set(1);
		with_runtime(|runtime| runtime.flush_updates());

		let log = effect_log.borrow();
		assert_eq!(
			log.iter().filter(|entry| entry.as_str() == "run:1").count(),
			1,
			"hydration prepasses must not retain duplicate effects: {log:?}"
		);
		drop(log);
		cleanup_reactive_nodes();
		root.remove();
	}
}
