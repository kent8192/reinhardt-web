//! In-memory Page-backed tree for native component testing.

use std::cell::RefCell;
use std::rc::Rc;

use reinhardt_core::types::page::{EventType, Page, PageEventHandler};

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
	event_handlers: Vec<(EventType, PageEventHandler)>,
	value: Option<String>,
}

enum TestNode {
	Removed,
	Root {
		children: Vec<NodeId>,
	},
	Element(ElementNode),
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

	pub(crate) fn event_handler(
		&self,
		node_id: NodeId,
		event_type: EventType,
	) -> Option<PageEventHandler> {
		self.element(node_id).and_then(|node| {
			node.event_handlers
				.iter()
				.find(|(candidate, _)| *candidate == event_type)
				.map(|(_, handler)| handler.clone())
		})
	}

	pub(crate) fn set_value(&mut self, node_id: NodeId, value: String) -> bool {
		match self.nodes.get_mut(node_id) {
			Some(TestNode::Element(node)) if node.supports_value() => {
				node.value = Some(value);
				true
			}
			_ => false,
		}
	}

	pub(crate) fn value(&self, node_id: NodeId) -> Option<String> {
		self.element(node_id).and_then(|node| node.value.clone())
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
				let (tag, attrs, children, is_void, event_handlers) = element.into_parts();
				let attrs = attrs
					.into_iter()
					.map(|(name, value)| (name.into_owned(), value.into_owned()))
					.collect::<Vec<_>>();
				let value = attrs
					.iter()
					.find(|(name, _)| name == "value")
					.map(|(_, value)| value.clone());
				let node_id = self.push_node(
					parent,
					TestNode::Element(ElementNode {
						tag: tag.into_owned(),
						attrs,
						children: Vec::new(),
						parent: Some(parent),
						is_void,
						event_handlers,
						value,
					}),
				);
				for child in children {
					self.append_page(node_id, child);
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
			Page::Reactive(reactive) => {
				let render_arc = reactive.into_render();
				let render: Rc<dyn Fn() -> Page + 'static> = Rc::new(move || render_arc());
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
		}
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
}

impl ElementNode {
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
		matches!(self.tag.as_str(), "input" | "textarea" | "select")
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
