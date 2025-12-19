//! IntoView trait and View enum for component rendering.

use crate::dom::{Element, EventType};
use std::borrow::Cow;
use std::sync::Arc;

/// Type alias for event handler functions.
#[cfg(target_arch = "wasm32")]
pub type ViewEventHandler = Arc<dyn Fn(web_sys::Event) + 'static>;

/// Type alias for event handler functions (non-WASM placeholder).
#[cfg(not(target_arch = "wasm32"))]
pub type ViewEventHandler = Arc<dyn Fn() + Send + Sync + 'static>;

/// Error type for mounting views to the DOM.
#[derive(Debug, Clone)]
pub enum MountError {
	/// Window object not available.
	NoWindow,
	/// Document object not available.
	NoDocument,
	/// Failed to create an element.
	CreateElementFailed,
	/// Failed to set an attribute.
	SetAttributeFailed,
	/// Failed to append a child element.
	AppendChildFailed,
}

impl std::fmt::Display for MountError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			MountError::NoWindow => write!(f, "Window object not available"),
			MountError::NoDocument => write!(f, "Document object not available"),
			MountError::CreateElementFailed => write!(f, "Failed to create element"),
			MountError::SetAttributeFailed => write!(f, "Failed to set attribute"),
			MountError::AppendChildFailed => write!(f, "Failed to append child"),
		}
	}
}

impl std::error::Error for MountError {}

/// A unified representation of renderable content.
///
/// View is the core abstraction for all UI elements in the component system.
/// It can represent DOM elements, text nodes, fragments, or reactive content.
#[derive(Debug)]
pub enum View {
	/// A DOM element.
	Element(ElementView),
	/// A text node.
	Text(Cow<'static, str>),
	/// A fragment containing multiple views (no wrapper element).
	Fragment(Vec<View>),
	/// An empty view (renders nothing).
	Empty,
}

/// Represents a DOM element in the view tree.
pub struct ElementView {
	/// The tag name (e.g., "div", "span").
	tag: Cow<'static, str>,
	/// HTML attributes.
	attrs: Vec<(Cow<'static, str>, Cow<'static, str>)>,
	/// Child views.
	children: Vec<View>,
	/// Whether this is a void element (no closing tag).
	is_void: bool,
	/// Event handlers attached to this element.
	event_handlers: Vec<(EventType, ViewEventHandler)>,
}

impl std::fmt::Debug for ElementView {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ElementView")
			.field("tag", &self.tag)
			.field("attrs", &self.attrs)
			.field("children", &self.children)
			.field("is_void", &self.is_void)
			.field("event_handlers_count", &self.event_handlers.len())
			.finish()
	}
}

impl ElementView {
	/// Creates a new element view.
	pub fn new(tag: impl Into<Cow<'static, str>>) -> Self {
		let tag = tag.into();
		let is_void = matches!(
			tag.as_ref(),
			"area"
				| "base" | "br"
				| "col" | "embed"
				| "hr" | "img"
				| "input" | "link"
				| "meta" | "source"
				| "track" | "wbr"
		);
		Self {
			tag,
			attrs: Vec::new(),
			children: Vec::new(),
			is_void,
			event_handlers: Vec::new(),
		}
	}

	/// Adds an attribute.
	pub fn attr(
		mut self,
		name: impl Into<Cow<'static, str>>,
		value: impl Into<Cow<'static, str>>,
	) -> Self {
		self.attrs.push((name.into(), value.into()));
		self
	}

	/// Adds a child view.
	pub fn child(mut self, child: impl IntoView) -> Self {
		self.children.push(child.into_view());
		self
	}

	/// Adds multiple child views.
	pub fn children(mut self, children: impl IntoIterator<Item = impl IntoView>) -> Self {
		self.children
			.extend(children.into_iter().map(|c| c.into_view()));
		self
	}

	/// Adds an event handler.
	pub fn on(mut self, event_type: EventType, handler: ViewEventHandler) -> Self {
		self.event_handlers.push((event_type, handler));
		self
	}

	/// Returns the tag name.
	pub fn tag_name(&self) -> &str {
		&self.tag
	}

	/// Returns the attributes.
	pub fn attrs(&self) -> &[(Cow<'static, str>, Cow<'static, str>)] {
		&self.attrs
	}

	/// Returns the child views.
	pub fn child_views(&self) -> &[View] {
		&self.children
	}

	/// Returns whether this is a void element.
	pub fn is_void(&self) -> bool {
		self.is_void
	}

	/// Returns the event handlers.
	pub fn event_handlers(&self) -> &[(EventType, ViewEventHandler)] {
		&self.event_handlers
	}
}

impl View {
	/// Creates an element view.
	pub fn element(tag: impl Into<Cow<'static, str>>) -> ElementView {
		ElementView::new(tag)
	}

	/// Creates a text view.
	pub fn text(content: impl Into<Cow<'static, str>>) -> Self {
		Self::Text(content.into())
	}

	/// Creates a fragment view.
	pub fn fragment(children: impl IntoIterator<Item = impl IntoView>) -> Self {
		Self::Fragment(children.into_iter().map(|c| c.into_view()).collect())
	}

	/// Creates an empty view.
	pub fn empty() -> Self {
		Self::Empty
	}

	/// Renders the view to an HTML string.
	///
	/// This is the core SSR method that converts the view tree to HTML.
	pub fn render_to_string(&self) -> String {
		let mut output = String::new();
		self.render_to_string_inner(&mut output);
		output
	}

	fn render_to_string_inner(&self, output: &mut String) {
		match self {
			View::Element(el) => {
				output.push('<');
				output.push_str(el.tag_name());

				for (name, value) in el.attrs() {
					output.push(' ');
					output.push_str(name);
					output.push_str("=\"");
					output.push_str(&html_escape(value));
					output.push('"');
				}

				if el.is_void() {
					output.push_str(" />");
				} else {
					output.push('>');
					for child in el.child_views() {
						child.render_to_string_inner(output);
					}
					output.push_str("</");
					output.push_str(el.tag_name());
					output.push('>');
				}
			}
			View::Text(text) => {
				output.push_str(&html_escape(text));
			}
			View::Fragment(children) => {
				for child in children {
					child.render_to_string_inner(output);
				}
			}
			View::Empty => {}
		}
	}

	/// Mounts the view to a DOM element (client-side only).
	#[cfg(target_arch = "wasm32")]
	pub fn mount(self, parent: &Element) -> Result<(), MountError> {
		self.mount_inner(parent)
	}

	#[cfg(target_arch = "wasm32")]
	fn mount_inner(self, parent: &Element) -> Result<(), MountError> {
		use crate::dom::document;

		match self {
			View::Element(el) => {
				let doc = document();
				let element = doc
					.create_element(&el.tag)
					.map_err(|_| MountError::CreateElementFailed)?;

				for (name, value) in el.attrs {
					element
						.set_attribute(&name, &value)
						.map_err(|_| MountError::SetAttributeFailed)?;
				}

				for child in el.children {
					child.mount_inner(&element)?;
				}

				parent
					.append_child(element)
					.map_err(|_| MountError::AppendChildFailed)?;
			}
			View::Text(text) => {
				let window = web_sys::window().ok_or(MountError::NoWindow)?;
				let document = window.document().ok_or(MountError::NoDocument)?;
				let text_node = document.create_text_node(&text);
				parent
					.inner()
					.append_child(&text_node)
					.map_err(|_| MountError::AppendChildFailed)?;
			}
			View::Fragment(children) => {
				for child in children {
					child.mount_inner(parent)?;
				}
			}
			View::Empty => {}
		}

		Ok(())
	}

	/// Mounts the view (non-WASM stub).
	#[cfg(not(target_arch = "wasm32"))]
	pub fn mount(self, _parent: &Element) -> Result<(), MountError> {
		Ok(())
	}
}

/// Trait for types that can be converted into a View.
///
/// This is the primary abstraction for renderable content.
/// Implementing this trait allows any type to be used in the view tree.
pub trait IntoView {
	/// Converts self into a View.
	fn into_view(self) -> View;
}

// Core implementations

impl IntoView for View {
	fn into_view(self) -> View {
		self
	}
}

impl IntoView for ElementView {
	fn into_view(self) -> View {
		View::Element(self)
	}
}

impl IntoView for String {
	fn into_view(self) -> View {
		View::Text(Cow::Owned(self))
	}
}

impl IntoView for &'static str {
	fn into_view(self) -> View {
		View::Text(Cow::Borrowed(self))
	}
}

impl<T: IntoView> IntoView for Option<T> {
	fn into_view(self) -> View {
		match self {
			Some(v) => v.into_view(),
			None => View::Empty,
		}
	}
}

impl<T: IntoView> IntoView for Vec<T> {
	fn into_view(self) -> View {
		View::Fragment(self.into_iter().map(|v| v.into_view()).collect())
	}
}

impl IntoView for () {
	fn into_view(self) -> View {
		View::Empty
	}
}

// Tuple implementations for fragments

impl<A: IntoView, B: IntoView> IntoView for (A, B) {
	fn into_view(self) -> View {
		View::Fragment(vec![self.0.into_view(), self.1.into_view()])
	}
}

impl<A: IntoView, B: IntoView, C: IntoView> IntoView for (A, B, C) {
	fn into_view(self) -> View {
		View::Fragment(vec![
			self.0.into_view(),
			self.1.into_view(),
			self.2.into_view(),
		])
	}
}

impl<A: IntoView, B: IntoView, C: IntoView, D: IntoView> IntoView for (A, B, C, D) {
	fn into_view(self) -> View {
		View::Fragment(vec![
			self.0.into_view(),
			self.1.into_view(),
			self.2.into_view(),
			self.3.into_view(),
		])
	}
}

// Integration with existing Element type

impl IntoView for Element {
	fn into_view(self) -> View {
		// Convert Element to ElementView by extracting tag and attributes
		// This is a simplified implementation - in practice, we'd need
		// to extract the actual DOM state
		View::Empty // Placeholder - actual implementation would serialize the element
	}
}

/// Escapes HTML special characters.
fn html_escape(s: &str) -> Cow<'_, str> {
	if s.contains(['&', '<', '>', '"', '\'']) {
		let mut escaped = String::with_capacity(s.len() + 8);
		for c in s.chars() {
			match c {
				'&' => escaped.push_str("&amp;"),
				'<' => escaped.push_str("&lt;"),
				'>' => escaped.push_str("&gt;"),
				'"' => escaped.push_str("&quot;"),
				'\'' => escaped.push_str("&#x27;"),
				_ => escaped.push(c),
			}
		}
		Cow::Owned(escaped)
	} else {
		Cow::Borrowed(s)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_element_view_creation() {
		let el = ElementView::new("div");
		assert_eq!(el.tag, "div");
		assert!(!el.is_void);
		assert!(el.attrs.is_empty());
		assert!(el.children.is_empty());
	}

	#[test]
	fn test_void_element_detection() {
		assert!(ElementView::new("br").is_void);
		assert!(ElementView::new("img").is_void);
		assert!(ElementView::new("input").is_void);
		assert!(!ElementView::new("div").is_void);
		assert!(!ElementView::new("span").is_void);
	}

	#[test]
	fn test_element_with_attrs() {
		let el = ElementView::new("div")
			.attr("class", "container")
			.attr("id", "main");
		assert_eq!(el.attrs.len(), 2);
	}

	#[test]
	fn test_element_with_children() {
		let el = ElementView::new("div").child("Hello").child("World");
		assert_eq!(el.children.len(), 2);
	}

	#[test]
	fn test_render_simple_element() {
		let view = ElementView::new("div").into_view();
		assert_eq!(view.render_to_string(), "<div></div>");
	}

	#[test]
	fn test_render_element_with_attrs() {
		let view = ElementView::new("div")
			.attr("class", "container")
			.attr("id", "main")
			.into_view();
		let html = view.render_to_string();
		assert!(html.contains("class=\"container\""));
		assert!(html.contains("id=\"main\""));
	}

	#[test]
	fn test_render_void_element() {
		let view = ElementView::new("br").into_view();
		assert_eq!(view.render_to_string(), "<br />");
	}

	#[test]
	fn test_render_element_with_children() {
		let view = ElementView::new("div")
			.child("Hello, ")
			.child(ElementView::new("strong").child("World"))
			.into_view();
		assert_eq!(
			view.render_to_string(),
			"<div>Hello, <strong>World</strong></div>"
		);
	}

	#[test]
	fn test_render_text() {
		let view = View::text("Hello");
		assert_eq!(view.render_to_string(), "Hello");
	}

	#[test]
	fn test_render_text_with_escaping() {
		let view = View::text("<script>alert('xss')</script>");
		assert_eq!(
			view.render_to_string(),
			"&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
		);
	}

	#[test]
	fn test_render_fragment() {
		let view = View::fragment(["One", "Two", "Three"]);
		assert_eq!(view.render_to_string(), "OneTwoThree");
	}

	#[test]
	fn test_render_empty() {
		let view = View::empty();
		assert_eq!(view.render_to_string(), "");
	}

	#[test]
	fn test_into_view_string() {
		let view = "Hello".into_view();
		assert_eq!(view.render_to_string(), "Hello");
	}

	#[test]
	fn test_into_view_option_some() {
		let view: View = Some("Hello").into_view();
		assert_eq!(view.render_to_string(), "Hello");
	}

	#[test]
	fn test_into_view_option_none() {
		let view: View = None::<String>.into_view();
		assert_eq!(view.render_to_string(), "");
	}

	#[test]
	fn test_into_view_vec() {
		let view = vec!["A", "B", "C"].into_view();
		assert_eq!(view.render_to_string(), "ABC");
	}

	#[test]
	fn test_into_view_tuple() {
		let view = ("Hello, ", "World!").into_view();
		assert_eq!(view.render_to_string(), "Hello, World!");
	}

	#[test]
	fn test_html_escape() {
		assert_eq!(html_escape("Hello"), Cow::Borrowed("Hello"));
		assert_eq!(
			html_escape("<div>"),
			Cow::<str>::Owned("&lt;div&gt;".to_string())
		);
		assert_eq!(
			html_escape("a & b"),
			Cow::<str>::Owned("a &amp; b".to_string())
		);
	}

	#[test]
	fn test_nested_elements() {
		let view = ElementView::new("html")
			.child(ElementView::new("head").child(ElementView::new("title").child("Test Page")))
			.child(
				ElementView::new("body")
					.child(ElementView::new("h1").child("Hello"))
					.child(ElementView::new("p").child("World")),
			)
			.into_view();

		let html = view.render_to_string();
		assert!(html.starts_with("<html>"));
		assert!(html.contains("<title>Test Page</title>"));
		assert!(html.contains("<h1>Hello</h1>"));
		assert!(html.ends_with("</html>"));
	}
}
