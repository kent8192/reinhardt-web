//! In-memory Page-backed tree for native component testing.

use std::cell::RefCell;
use std::rc::Rc;

use reinhardt_core::types::page::{
	ControlBinding, ControlBindingError, ControlKind, ControlValue, ControlWriteOutcome, EventName,
	NativeEventFile, NativeEventTarget, Page, PageEventHandler,
};

use super::fixture::{EventFixtureError, TargetStatePatch};
use super::scheduler::SchedulerScope;
#[cfg(feature = "msw")]
use super::server_fn_mock::SharedServerFnMocks;

/// Stable identifier for a node in the native test DOM.
pub(crate) type NodeId = usize;

/// Shared mutable screen state used by handles.
pub(crate) struct ScreenInner {
	/// Rendered native test DOM.
	pub dom: TestDom,
	/// Harness scheduler active for this screen.
	pub scheduler: Rc<SchedulerScope>,
	/// Server function mocks registered for this screen.
	#[cfg(feature = "msw")]
	pub mocks: SharedServerFnMocks,
}

pub(crate) struct TestDom {
	nodes: Vec<TestNode>,
	root: NodeId,
}

#[derive(Clone)]
pub(crate) struct ElementNode {
	pub tag: String,
	attrs: Vec<(String, String)>,
	pub children: Vec<NodeId>,
	parent: Option<NodeId>,
	is_void: bool,
	event_handlers: Vec<(EventName, PageEventHandler)>,
	value: Option<String>,
	checked: bool,
	selected_values: Vec<String>,
	files: Vec<NativeEventFile>,
	content_editable: bool,
	control_binding: Option<ControlBinding>,
	option_value: Option<String>,
	is_composing: bool,
	pending_raw: Option<String>,
	last_committed_raw: Option<String>,
	last_observed_control_value: Option<ControlValue>,
}

enum TestNode {
	Removed,
	Root {
		children: Vec<NodeId>,
	},
	Element(Box<ElementNode>),
	Text {
		text: String,
		parent: Option<NodeId>,
	},
	ReactiveAnchor {
		children: Vec<NodeId>,
		parent: Option<NodeId>,
		render: Rc<dyn Fn() -> Page + 'static>,
	},
}

pub(crate) struct PendingControlBindingWrite {
	node_id: NodeId,
	binding: ControlBinding,
	value: ControlValue,
	raw: Option<String>,
	dedupe_next_input: bool,
}

pub(crate) struct CompletedControlBindingWrite {
	node_id: NodeId,
	binding: ControlBinding,
	outcome: ControlWriteOutcome,
	raw: Option<String>,
	dedupe_next_input: bool,
}

impl PendingControlBindingWrite {
	pub(crate) fn execute(self) -> Result<CompletedControlBindingWrite, ControlBindingError> {
		let outcome = self.binding.write(self.value)?;
		Ok(CompletedControlBindingWrite {
			node_id: self.node_id,
			binding: self.binding,
			outcome,
			raw: self.raw,
			dedupe_next_input: self.dedupe_next_input,
		})
	}
}

impl TestDom {
	/// Renders a Page into an in-memory native test DOM.
	pub(crate) fn render(page: Page) -> Self {
		let mut dom = Self {
			nodes: vec![TestNode::Root {
				children: Vec::new(),
			}],
			root: 0,
		};
		dom.append_page(dom.root, page);
		dom
	}

	pub(crate) fn root(&self) -> NodeId {
		self.root
	}

	pub(crate) fn contains(&self, node_id: NodeId) -> bool {
		self.nodes
			.get(node_id)
			.is_some_and(|node| !matches!(node, TestNode::Removed))
	}

	pub(crate) fn element(&self, node_id: NodeId) -> Option<&ElementNode> {
		match self.nodes.get(node_id)? {
			TestNode::Element(node) => Some(node),
			_ => None,
		}
	}

	pub(crate) fn text_content(&self, node_id: NodeId) -> String {
		match self.nodes.get(node_id) {
			Some(TestNode::Removed) => String::new(),
			Some(TestNode::Root { children }) => self.children_text(children),
			Some(TestNode::Element(node)) => self.children_text(&node.children),
			Some(TestNode::Text { text, .. }) => text.clone(),
			Some(TestNode::ReactiveAnchor { children, .. }) => self.children_text(children),
			None => String::new(),
		}
	}

	pub(crate) fn visible_text_content(&self, node_id: NodeId) -> String {
		match self.nodes.get(node_id) {
			Some(TestNode::Removed) => String::new(),
			Some(TestNode::Root { children }) => self.children_visible_text(children),
			Some(TestNode::Element(_node)) if self.is_hidden(node_id) => String::new(),
			Some(TestNode::Element(node)) => self.children_visible_text(&node.children),
			Some(TestNode::Text { text, .. }) => text.clone(),
			Some(TestNode::ReactiveAnchor { children, .. }) => self.children_visible_text(children),
			None => String::new(),
		}
	}

	pub(crate) fn all_elements(&self) -> Vec<NodeId> {
		let mut nodes = Vec::new();
		self.collect_elements(self.root, &mut nodes);
		nodes
	}

	pub(crate) fn visible_elements(&self) -> Vec<NodeId> {
		self.all_elements()
			.into_iter()
			.filter(|node_id| !self.is_hidden(*node_id))
			.collect()
	}

	pub(crate) fn is_hidden(&self, node_id: NodeId) -> bool {
		let mut current = Some(node_id);
		while let Some(id) = current {
			if let Some(node) = self.element(id) {
				if node.has_attr("hidden") || node.attr("aria-hidden") == Some("true") {
					return true;
				}
				current = node.parent;
			} else {
				current = self.parent(id);
			}
		}
		false
	}

	pub(crate) fn find_element_by_id(&self, id: &str) -> Option<NodeId> {
		self.all_elements()
			.into_iter()
			.find(|node_id| self.element(*node_id).and_then(|node| node.attr("id")) == Some(id))
	}

	pub(crate) fn label_for(&self, id: &str) -> Option<NodeId> {
		self.all_elements().into_iter().find(|node_id| {
			self.element(*node_id).is_some_and(|node| {
				node.tag == "label" && node.attr("for") == Some(id) && !self.is_hidden(*node_id)
			})
		})
	}

	pub(crate) fn closest_label(&self, node_id: NodeId) -> Option<NodeId> {
		let mut current = self.parent(node_id);
		while let Some(id) = current {
			if self.element(id).is_some_and(|node| node.tag == "label") {
				return Some(id);
			}
			current = self.parent(id);
		}
		None
	}

	pub(crate) fn event_handlers(
		&self,
		node_id: NodeId,
		event_name: &EventName,
		bubbles: bool,
	) -> Vec<(NodeId, PageEventHandler)> {
		let mut handlers = Vec::new();
		let mut current = Some(node_id);
		while let Some(id) = current {
			if let Some(node) = self.element(id) {
				handlers.extend(
					node.event_handlers
						.iter()
						.filter(|(candidate, _)| candidate.as_str() == event_name.as_str())
						.map(|(_, handler)| (id, handler.clone())),
				);
			}
			if !bubbles {
				break;
			}
			current = self.parent(id);
		}
		handlers
	}

	pub(crate) fn event_target(&self, node_id: NodeId) -> Option<NativeEventTarget> {
		let node = self.element(node_id)?;
		let mut target = node
			.attrs
			.iter()
			.fold(
				NativeEventTarget::new(&node.tag),
				|target, (name, value)| target.with_attribute(name, value),
			)
			.with_text_content(self.text_content(node_id));

		if let Some(value) = &node.value {
			target = target.with_value(value);
		} else if node.content_editable {
			target = target.with_value(self.text_content(node_id));
		}

		if node.tag == "input" && matches!(node.attr("type"), Some("checkbox" | "radio")) {
			target = target.with_checked(node.checked);
		}

		target = target.with_selected_values(node.selected_values.clone());
		target = target.with_files(node.files.clone());
		Some(target.with_content_editable(node.content_editable))
	}

	pub(crate) fn suppresses_events(&self, node_id: NodeId) -> bool {
		let mut current = Some(node_id);
		while let Some(id) = current {
			if self
				.element(id)
				.is_some_and(ElementNode::is_disabled_form_control)
			{
				return true;
			}
			current = self.parent(id);
		}
		false
	}

	pub(crate) fn apply_target_state(
		&mut self,
		node_id: NodeId,
		patch: &TargetStatePatch,
	) -> Result<(), EventFixtureError> {
		let node = match self.nodes.get_mut(node_id) {
			Some(TestNode::Element(node)) => node,
			_ => return Ok(()),
		};
		let final_content_editable = patch.content_editable.unwrap_or(node.content_editable);
		let supports_final_value =
			node.supports_value_with_content_editable(final_content_editable);
		let unsupported_property = if patch.value.is_some() && !supports_final_value {
			Some("value")
		} else if patch.checked.is_some()
			&& !(node.tag == "input" && matches!(node.attr("type"), Some("checkbox" | "radio")))
		{
			Some("checked")
		} else if patch.selected_values.is_some() && node.tag != "select" {
			Some("selected_values")
		} else if patch.files.is_some()
			&& !(node.tag == "input" && node.attr("type") == Some("file"))
		{
			Some("files")
		} else {
			None
		};
		if let Some(property) = unsupported_property {
			return Err(EventFixtureError::UnsupportedTargetState {
				property,
				actual_tag: node.tag.clone(),
			});
		}

		node.content_editable = final_content_editable;
		if let Some(selected_values) = &patch.selected_values {
			node.value = if node.tag == "select" {
				Some(selected_values.first().cloned().unwrap_or_default())
			} else {
				selected_values.first().cloned()
			};
			node.selected_values.clone_from(selected_values);
		} else if let Some(value) = &patch.value {
			node.value = Some(value.clone());
			if node.tag == "select" {
				node.selected_values = vec![value.clone()];
			}
		}
		if let Some(checked) = patch.checked {
			node.checked = checked;
		}
		if let Some(files) = &patch.files {
			node.files.clone_from(files);
		}
		let refresh_selected_options =
			node.tag == "select" && (patch.value.is_some() || patch.selected_values.is_some());
		if refresh_selected_options {
			self.refresh_selected_options(node_id);
		}
		Ok(())
	}

	pub(crate) fn validate_control_binding(
		&self,
		node_id: NodeId,
	) -> Result<(), ControlBindingError> {
		let Some(node) = self.element(node_id) else {
			return Ok(());
		};
		if let Some(binding) = &node.control_binding {
			node.validate_control_binding(binding)?;
		}
		Ok(())
	}

	pub(crate) fn prepare_control_binding_commit(
		&mut self,
		node_id: NodeId,
		event_name: &EventName,
		input_is_composing: bool,
	) -> Result<(bool, Option<PendingControlBindingWrite>), ControlBindingError> {
		let node = match self.nodes.get_mut(node_id) {
			Some(TestNode::Element(node)) => node,
			_ => return Ok((false, None)),
		};
		let Some(binding) = node.control_binding.clone() else {
			return Ok((false, None));
		};
		node.validate_control_binding(&binding)?;

		let event_name = event_name.as_str();
		match binding.kind() {
			ControlKind::Text | ControlKind::Number => match event_name {
				"compositionstart" => {
					node.is_composing = true;
					node.pending_raw.clone_from(&node.value);
					Ok((true, None))
				}
				"compositionend" => {
					node.is_composing = false;
					let raw = node
						.value
						.clone()
						.or_else(|| node.pending_raw.clone())
						.ok_or(ControlBindingError::MissingProperty {
							control: binding.kind(),
							property: "value",
						})?;
					Ok((
						true,
						Some(PendingControlBindingWrite {
							node_id,
							binding,
							value: ControlValue::Text(raw.clone()),
							raw: Some(raw),
							dedupe_next_input: true,
						}),
					))
				}
				"input" => {
					let raw = node
						.value
						.clone()
						.ok_or(ControlBindingError::MissingProperty {
							control: binding.kind(),
							property: "value",
						})?;
					if node.is_composing || input_is_composing {
						node.last_committed_raw = None;
						node.pending_raw = Some(raw);
						return Ok((true, None));
					}
					if node.last_committed_raw.take().as_deref() == Some(raw.as_str()) {
						let current_value = binding.read();
						if current_value != ControlValue::Text(raw) {
							node.pending_raw = None;
							node.apply_control_value(current_value.clone());
							node.last_observed_control_value = Some(current_value);
						}
						return Ok((true, None));
					}
					Ok((
						true,
						Some(PendingControlBindingWrite {
							node_id,
							binding,
							value: ControlValue::Text(raw.clone()),
							raw: Some(raw),
							dedupe_next_input: false,
						}),
					))
				}
				_ => Ok((false, None)),
			},
			ControlKind::Checkbox | ControlKind::Radio if event_name == "change" => Ok((
				true,
				Some(PendingControlBindingWrite {
					node_id,
					binding,
					value: ControlValue::Checked(node.checked),
					raw: None,
					dedupe_next_input: false,
				}),
			)),
			ControlKind::SelectOne if event_name == "change" => Ok((
				true,
				Some(PendingControlBindingWrite {
					node_id,
					binding,
					value: ControlValue::Text(node.value.clone().unwrap_or_default()),
					raw: None,
					dedupe_next_input: false,
				}),
			)),
			ControlKind::SelectMany if event_name == "change" => Ok((
				true,
				Some(PendingControlBindingWrite {
					node_id,
					binding,
					value: ControlValue::SelectedValues(node.selected_values.clone()),
					raw: None,
					dedupe_next_input: false,
				}),
			)),
			_ => Ok((false, None)),
		}
	}

	pub(crate) fn record_control_binding_commit(
		&mut self,
		completed: CompletedControlBindingWrite,
	) {
		let Some(TestNode::Element(node)) = self.nodes.get_mut(completed.node_id) else {
			return;
		};
		node.last_committed_raw = completed
			.dedupe_next_input
			.then(|| completed.raw.clone())
			.flatten();
		node.record_write_outcome(&completed.binding, completed.outcome, completed.raw);
	}

	pub(crate) fn refresh_control_bindings(&mut self) {
		let mut selects = Vec::new();
		for (node_id, node) in self.nodes.iter_mut().enumerate() {
			let TestNode::Element(element) = node else {
				continue;
			};
			let Some(binding) = element.control_binding.clone() else {
				continue;
			};
			let value = binding.read();
			let retain_invalid_raw = binding.kind() == ControlKind::Number
				&& element.pending_raw.is_some()
				&& element.last_observed_control_value.as_ref() == Some(&value);
			if !retain_invalid_raw {
				element.pending_raw = None;
				element.apply_control_value(value.clone());
			}
			element.last_observed_control_value = Some(value);
			if matches!(
				binding.kind(),
				ControlKind::SelectOne | ControlKind::SelectMany
			) {
				selects.push(node_id);
			}
		}
		for select in selects {
			self.refresh_selected_options(select);
		}
	}

	pub(crate) fn value(&self, node_id: NodeId) -> Option<String> {
		self.element(node_id).and_then(|node| {
			node.value
				.clone()
				.or_else(|| node.content_editable.then(|| self.text_content(node_id)))
		})
	}

	pub(crate) fn children(&self, node_id: NodeId) -> &[NodeId] {
		match self.nodes.get(node_id) {
			Some(TestNode::Removed) => &[],
			Some(TestNode::Root { children }) => children,
			Some(TestNode::Element(node)) => &node.children,
			Some(TestNode::ReactiveAnchor { children, .. }) => children,
			_ => &[],
		}
	}

	pub(crate) fn text_node(&self, node_id: NodeId) -> Option<&str> {
		match self.nodes.get(node_id) {
			Some(TestNode::Text { text, .. }) => Some(text),
			_ => None,
		}
	}

	pub(crate) fn is_void(&self, node_id: NodeId) -> bool {
		self.element(node_id).is_some_and(|node| node.is_void)
	}

	fn append_page(&mut self, parent: NodeId, page: Page) {
		match page {
			Page::Element(element) => {
				let option_value = element
					.tag_name()
					.eq_ignore_ascii_case("option")
					.then(|| crate::ssr::control_binding::option_value(&element));
				let (tag, attrs, children, is_void, event_handlers, control_binding) =
					element.into_parts();
				let attrs = attrs
					.into_iter()
					.map(|(name, value)| (name.into_owned(), value.into_owned()))
					.collect::<Vec<_>>();
				let value = attrs
					.iter()
					.find(|(name, _)| name == "value")
					.map(|(_, value)| value.clone());
				let checked = attrs.iter().any(|(name, _)| name == "checked");
				let selected_values = value.clone().into_iter().collect();
				let content_editable = attrs
					.iter()
					.find(|(name, _)| name == "contenteditable")
					.is_some_and(|(_, value)| value != "false");
				let last_observed_control_value =
					control_binding.as_ref().map(ControlBinding::read);
				let refresh_controlled_select = control_binding.as_ref().is_some_and(|binding| {
					matches!(
						binding.kind(),
						ControlKind::SelectOne | ControlKind::SelectMany
					)
				});
				let mut element_node = ElementNode {
					tag: tag.into_owned(),
					attrs,
					children: Vec::new(),
					parent: Some(parent),
					is_void,
					event_handlers,
					value,
					checked,
					selected_values,
					files: Vec::new(),
					content_editable,
					control_binding,
					option_value,
					is_composing: false,
					pending_raw: None,
					last_committed_raw: None,
					last_observed_control_value: last_observed_control_value.clone(),
				};
				let binding_supported = element_node
					.control_binding
					.as_ref()
					.is_none_or(|binding| element_node.validate_control_binding(binding).is_ok());
				if binding_supported && let Some(value) = last_observed_control_value {
					element_node.apply_control_value(value);
				}
				let node_id = self.push_node(parent, TestNode::Element(Box::new(element_node)));
				for child in children {
					self.append_page(node_id, child);
				}
				if refresh_controlled_select {
					self.refresh_selected_options(node_id);
				}
			}
			Page::Text(text) => {
				self.push_node(
					parent,
					TestNode::Text {
						text: text.into_owned(),
						parent: Some(parent),
					},
				);
			}
			Page::Fragment(children) => {
				for child in children {
					self.append_page(parent, child);
				}
			}
			Page::KeyedFragment(children) => {
				for (_, child) in children {
					self.append_page(parent, child);
				}
			}
			Page::Outlet(outlet) => {
				let id = outlet.id().map(str::to_string);
				if let Some(child) = outlet.into_child() {
					self.append_page(parent, child);
				} else if let Some(id) = id {
					self.push_node(
						parent,
						TestNode::Element(Box::new(ElementNode {
							tag: "reinhardt-outlet".to_string(),
							attrs: vec![
								("data-rh-outlet-id".to_string(), id),
								("style".to_string(), "display: contents;".to_string()),
							],
							children: Vec::new(),
							parent: Some(parent),
							is_void: false,
							event_handlers: Default::default(),
							value: None,
							checked: false,
							selected_values: Vec::new(),
							files: Vec::new(),
							content_editable: false,
							control_binding: None,
							option_value: None,
							is_composing: false,
							pending_raw: None,
							last_committed_raw: None,
							last_observed_control_value: None,
						})),
					);
				}
			}
			Page::Empty => {}
			Page::WithHead { view, .. } => self.append_page(parent, *view),
			Page::ReactiveIf(reactive_if) => {
				let (condition, then_view, else_view) = reactive_if.into_parts();
				let render: Rc<dyn Fn() -> Page + 'static> = Rc::new(move || {
					if condition() {
						then_view()
					} else {
						else_view()
					}
				});
				self.append_reactive_anchor(parent, render);
			}
			Page::Reactive(reactive) => {
				let render_arc = reactive.into_render();
				let render: Rc<dyn Fn() -> Page + 'static> = Rc::new(move || render_arc());
				self.append_reactive_anchor(parent, render);
			}
			Page::Suspense(node) => {
				let render: Rc<dyn Fn() -> Page + 'static> = Rc::new(move || node.render_branch());
				self.append_reactive_anchor(parent, render);
			}
			Page::Deferred(node) => {
				let content = node.render_content();
				self.append_page(parent, content);
			}
		}
	}

	fn append_reactive_anchor(&mut self, parent: NodeId, render: Rc<dyn Fn() -> Page + 'static>) {
		let anchor = self.push_node(
			parent,
			TestNode::ReactiveAnchor {
				children: Vec::new(),
				parent: Some(parent),
				render: Rc::clone(&render),
			},
		);
		self.append_page(anchor, render());
	}

	fn push_node(&mut self, parent: NodeId, node: TestNode) -> NodeId {
		let node_id = self.nodes.len();
		self.nodes.push(node);
		match self.nodes.get_mut(parent) {
			Some(TestNode::Removed) => {}
			Some(TestNode::Root { children }) => children.push(node_id),
			Some(TestNode::Element(element)) => element.children.push(node_id),
			Some(TestNode::ReactiveAnchor { children, .. }) => children.push(node_id),
			_ => {}
		}
		node_id
	}

	fn children_text(&self, children: &[NodeId]) -> String {
		children
			.iter()
			.map(|child| self.text_content(*child))
			.collect::<String>()
	}

	fn children_visible_text(&self, children: &[NodeId]) -> String {
		children
			.iter()
			.map(|child| self.visible_text_content(*child))
			.collect::<String>()
	}

	fn collect_elements(&self, node_id: NodeId, output: &mut Vec<NodeId>) {
		if self.element(node_id).is_some() {
			output.push(node_id);
		}
		for child in self.children(node_id) {
			self.collect_elements(*child, output);
		}
	}

	fn parent(&self, node_id: NodeId) -> Option<NodeId> {
		match self.nodes.get(node_id) {
			Some(TestNode::Removed) => None,
			Some(TestNode::Element(node)) => node.parent,
			Some(TestNode::Text { parent, .. }) => *parent,
			Some(TestNode::ReactiveAnchor { parent, .. }) => *parent,
			_ => None,
		}
	}

	pub(crate) fn rerender_reactive_anchors(&mut self) {
		let anchors = self
			.nodes
			.iter()
			.enumerate()
			.filter_map(|(node_id, node)| match node {
				TestNode::ReactiveAnchor { render, .. } => Some((node_id, Rc::clone(render))),
				_ => None,
			})
			.collect::<Vec<_>>();

		for (anchor, render) in anchors {
			if !self.contains(anchor) {
				continue;
			}
			self.clear_children(anchor);
			self.append_page(anchor, render());
		}
	}

	fn clear_children(&mut self, node_id: NodeId) {
		let children = match self.nodes.get_mut(node_id) {
			Some(TestNode::Root { children }) => std::mem::take(children),
			Some(TestNode::Element(node)) => std::mem::take(&mut node.children),
			Some(TestNode::ReactiveAnchor { children, .. }) => std::mem::take(children),
			_ => Vec::new(),
		};
		for child in children {
			self.remove_subtree(child);
		}
	}

	fn remove_subtree(&mut self, node_id: NodeId) {
		let children = self.children(node_id).to_vec();
		for child in children {
			self.remove_subtree(child);
		}
		if let Some(node) = self.nodes.get_mut(node_id) {
			*node = TestNode::Removed;
		}
	}

	fn refresh_selected_options(&mut self, select_id: NodeId) {
		let Some(select) = self.element(select_id) else {
			return;
		};
		if !select.tag.eq_ignore_ascii_case("select") {
			return;
		}
		let requested_values = select.selected_values.clone();
		let multiple = select.has_attr("multiple");
		let children = select.children.clone();
		let mut selected_values = Vec::new();
		for child in children {
			self.refresh_selected_options_in_subtree(
				child,
				&requested_values,
				multiple,
				&mut selected_values,
			);
		}
		if let Some(TestNode::Element(select)) = self.nodes.get_mut(select_id) {
			select.value = Some(selected_values.first().cloned().unwrap_or_default());
			select.selected_values = selected_values;
		}
	}

	fn refresh_selected_options_in_subtree(
		&mut self,
		node_id: NodeId,
		requested_values: &[String],
		multiple: bool,
		selected_values: &mut Vec<String>,
	) {
		let children = self.children(node_id).to_vec();
		let effective_value = self.element(node_id).and_then(|node| {
			(node.tag.eq_ignore_ascii_case("option"))
				.then(|| node.option_value.clone().unwrap_or_default())
		});
		if let Some(TestNode::Element(node)) = self.nodes.get_mut(node_id)
			&& node.tag.eq_ignore_ascii_case("option")
		{
			let selected = effective_value.as_ref().is_some_and(|value| {
				requested_values.iter().any(|candidate| candidate == value)
					&& (multiple || selected_values.is_empty())
			});
			node.attrs.retain(|(name, _)| name != "selected");
			if selected {
				node.attrs
					.push(("selected".to_owned(), "selected".to_owned()));
				selected_values.push(effective_value.expect("option value should exist"));
			}
		}
		for child in children {
			self.refresh_selected_options_in_subtree(
				child,
				requested_values,
				multiple,
				selected_values,
			);
		}
	}
}

impl ElementNode {
	fn validate_control_binding(
		&self,
		binding: &ControlBinding,
	) -> Result<(), ControlBindingError> {
		let supported = match binding.kind() {
			ControlKind::Text => {
				self.tag.eq_ignore_ascii_case("textarea")
					|| (self.tag.eq_ignore_ascii_case("input")
						&& has_effective_text_type(self.attr("type")))
			}
			ControlKind::Number => {
				self.tag.eq_ignore_ascii_case("input")
					&& self
						.attr("type")
						.is_some_and(|kind| kind.eq_ignore_ascii_case("number"))
			}
			ControlKind::Checkbox => {
				self.tag.eq_ignore_ascii_case("input")
					&& self
						.attr("type")
						.is_some_and(|kind| kind.eq_ignore_ascii_case("checkbox"))
			}
			ControlKind::Radio => {
				self.tag.eq_ignore_ascii_case("input")
					&& self
						.attr("type")
						.is_some_and(|kind| kind.eq_ignore_ascii_case("radio"))
			}
			ControlKind::SelectOne => {
				self.tag.eq_ignore_ascii_case("select") && !self.has_attr("multiple")
			}
			ControlKind::SelectMany => {
				self.tag.eq_ignore_ascii_case("select") && self.has_attr("multiple")
			}
		};
		if supported {
			Ok(())
		} else {
			Err(ControlBindingError::UnsupportedElement {
				control: binding.kind(),
				actual_tag: self.tag.clone(),
			})
		}
	}

	fn record_write_outcome(
		&mut self,
		binding: &ControlBinding,
		outcome: ControlWriteOutcome,
		raw: Option<String>,
	) {
		match outcome {
			ControlWriteOutcome::Committed | ControlWriteOutcome::Ignored => {
				self.pending_raw = None;
				self.last_observed_control_value = Some(binding.read());
			}
			ControlWriteOutcome::Rejected(_) => {
				self.pending_raw = raw;
			}
		}
	}

	fn apply_control_value(&mut self, value: ControlValue) {
		match value {
			ControlValue::Text(value) => {
				self.value = Some(value.clone());
				if self.tag.eq_ignore_ascii_case("select") {
					self.selected_values = vec![value];
				}
			}
			ControlValue::Checked(checked) => self.checked = checked,
			ControlValue::SelectedValues(values) => {
				self.value = Some(values.first().cloned().unwrap_or_default());
				self.selected_values = values;
			}
		}
	}

	pub(crate) fn attr(&self, name: &str) -> Option<&str> {
		self.attrs
			.iter()
			.find(|(candidate, _)| candidate == name)
			.map(|(_, value)| value.as_str())
	}

	pub(crate) fn attrs(&self) -> &[(String, String)] {
		&self.attrs
	}

	pub(crate) fn has_attr(&self, name: &str) -> bool {
		self.attr(name).is_some()
	}

	pub(crate) fn supports_value(&self) -> bool {
		self.supports_value_with_content_editable(self.content_editable)
	}

	fn supports_value_with_content_editable(&self, content_editable: bool) -> bool {
		content_editable
			|| (["input", "textarea", "select"]
				.iter()
				.any(|tag| self.tag.eq_ignore_ascii_case(tag))
				&& !(self.tag.eq_ignore_ascii_case("input")
					&& self
						.attr("type")
						.is_some_and(|kind| kind.eq_ignore_ascii_case("hidden"))))
	}

	pub(crate) fn is_disabled_form_control(&self) -> bool {
		self.has_attr("disabled")
			&& [
				"button", "fieldset", "input", "optgroup", "option", "select", "textarea",
			]
			.iter()
			.any(|tag| self.tag.eq_ignore_ascii_case(tag))
	}
}

fn has_effective_text_type(input_type: Option<&str>) -> bool {
	let Some(input_type) = input_type else {
		return true;
	};
	input_type.eq_ignore_ascii_case("text")
		|| ![
			"button",
			"checkbox",
			"color",
			"date",
			"datetime-local",
			"email",
			"file",
			"hidden",
			"image",
			"month",
			"number",
			"password",
			"radio",
			"range",
			"reset",
			"search",
			"submit",
			"tel",
			"time",
			"url",
			"week",
		]
		.iter()
		.any(|known| input_type.eq_ignore_ascii_case(known))
}

#[cfg(test)]
mod case_normalization_tests {
	use super::*;
	use crate::reactive::Signal;

	fn element(tag: &str, input_type: Option<&str>) -> ElementNode {
		ElementNode {
			tag: tag.to_owned(),
			attrs: input_type
				.map(|value| vec![("type".to_owned(), value.to_owned())])
				.unwrap_or_default(),
			children: Vec::new(),
			parent: None,
			is_void: false,
			event_handlers: Vec::new(),
			value: None,
			checked: false,
			selected_values: Vec::new(),
			files: Vec::new(),
			content_editable: false,
			control_binding: None,
			option_value: None,
			is_composing: false,
			pending_raw: None,
			last_committed_raw: None,
			last_observed_control_value: None,
		}
	}

	#[test]
	fn native_control_binding_validation_normalizes_ascii_case() {
		assert!(
			element("INPUT", Some("RADIO"))
				.validate_control_binding(&ControlBinding::radio(
					Signal::new(String::new()),
					"choice".to_owned(),
				))
				.is_ok()
		);
		assert!(
			element("SELECT", None)
				.validate_control_binding(&ControlBinding::select_one(Signal::new(String::new())))
				.is_ok()
		);
	}

	#[test]
	fn native_value_support_normalizes_ascii_case() {
		assert!(element("TEXTAREA", None).supports_value());
		assert!(!element("INPUT", Some("HIDDEN")).supports_value());
	}
}

#[cfg(feature = "msw")]
pub(crate) fn shared_screen_inner(
	dom: TestDom,
	scheduler: Rc<SchedulerScope>,
	mocks: SharedServerFnMocks,
) -> Rc<RefCell<ScreenInner>> {
	Rc::new(RefCell::new(ScreenInner {
		dom,
		scheduler,
		mocks,
	}))
}

#[cfg(not(feature = "msw"))]
pub(crate) fn shared_screen_inner(
	dom: TestDom,
	scheduler: Rc<SchedulerScope>,
) -> Rc<RefCell<ScreenInner>> {
	Rc::new(RefCell::new(ScreenInner { dom, scheduler }))
}
