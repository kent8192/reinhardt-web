//! IntoView trait and View enum for component rendering.

use crate::component::Head;
#[cfg(target_arch = "wasm32")]
use crate::component::reactive_if::{ReactiveIfNode, ReactiveNode, store_reactive_node};
#[cfg(target_arch = "wasm32")]
use crate::dom::Element;
use crate::dom::EventType;
use std::borrow::Cow;
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;

/// Type alias for event handler functions.
#[cfg(target_arch = "wasm32")]
pub type ViewEventHandler = Arc<dyn Fn(web_sys::Event) + 'static>;

/// Dummy event type for non-WASM environments.
///
/// This type exists to maintain API compatibility between WASM and non-WASM builds.
/// In non-WASM environments, event handlers still accept an argument (this dummy type)
/// so that user code doesn't need conditional compilation for event handler signatures.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Default)]
pub struct DummyEvent;

/// Type alias for event handler functions (non-WASM placeholder).
///
/// Uses `DummyEvent` to maintain API compatibility with the WASM version,
/// allowing the same event handler signatures (e.g., `|_| { ... }`) to work
/// in both WASM and non-WASM environments.
#[cfg(not(target_arch = "wasm32"))]
pub type ViewEventHandler = Arc<dyn Fn(DummyEvent) + Send + Sync + 'static>;

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

/// HTML boolean attributes that should only be set when the value is truthy.
///
/// Boolean attributes in HTML are special: the presence of the attribute alone
/// makes it active, regardless of its value. For example:
/// - `<button disabled="">` is disabled
/// - `<button disabled="false">` is STILL disabled
/// - `<button>` is NOT disabled (attribute absent)
///
/// This list follows the HTML5 specification for boolean attributes.
const BOOLEAN_ATTRS: &[&str] = &[
	"allowfullscreen",
	"async",
	"autofocus",
	"autoplay",
	"checked",
	"controls",
	"default",
	"defer",
	"disabled",
	"formnovalidate",
	"hidden",
	"inert",
	"ismap",
	"itemscope",
	"loop",
	"multiple",
	"muted",
	"nomodule",
	"novalidate",
	"open",
	"playsinline",
	"readonly",
	"required",
	"reversed",
	"selected",
	"truespeed",
];

/// Checks if a boolean attribute value should result in the attribute being set.
///
/// Returns `true` if the value is non-empty and not "false" or "0".
/// Returns `false` for empty strings, "false", or "0", meaning the attribute
/// should NOT be set (to properly disable the boolean attribute).
fn is_boolean_attr_truthy(value: &str) -> bool {
	!value.is_empty() && value != "false" && value != "0"
}

/// Reactive conditional rendering.
///
/// This struct holds closures for condition evaluation and view generation,
/// enabling automatic DOM updates when the condition's dependencies change.
pub struct ReactiveIf {
	/// Condition closure that returns bool when called.
	/// This closure is re-evaluated whenever its Signal dependencies change.
	condition: Box<dyn Fn() -> bool + 'static>,
	/// View to render when condition is true.
	then_view: Box<dyn Fn() -> View + 'static>,
	/// View to render when condition is false.
	else_view: Box<dyn Fn() -> View + 'static>,
}

/// Reactive view that re-evaluates when Signal dependencies change.
///
/// This struct holds a single closure that generates a View, enabling
/// automatic DOM updates when any Signal accessed within the closure changes.
pub struct Reactive {
	/// View generation closure that returns a View when called.
	/// This closure is re-evaluated whenever its Signal dependencies change.
	render: Box<dyn Fn() -> View + 'static>,
}

impl std::fmt::Debug for Reactive {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Reactive")
			.field("render", &"<closure>")
			.finish()
	}
}

impl Reactive {
	/// Returns the rendered view.
	pub fn render(&self) -> View {
		(self.render)()
	}

	/// Consumes the Reactive and returns the render closure.
	pub fn into_render(self) -> Box<dyn Fn() -> View + 'static> {
		self.render
	}
}

impl std::fmt::Debug for ReactiveIf {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ReactiveIf")
			.field("condition", &"<closure>")
			.field("then_view", &"<closure>")
			.field("else_view", &"<closure>")
			.finish()
	}
}

impl ReactiveIf {
	/// Evaluates the condition closure.
	pub fn condition(&self) -> bool {
		(self.condition)()
	}

	/// Calls the then_view closure and returns the view.
	pub fn then_view(&self) -> View {
		(self.then_view)()
	}

	/// Calls the else_view closure and returns the view.
	pub fn else_view(&self) -> View {
		(self.else_view)()
	}

	/// Consumes the ReactiveIf and returns its parts.
	///
	/// Returns a tuple of (condition, then_view, else_view) closures.
	#[allow(clippy::type_complexity)] // Tuple decomposition is intentional for destructuring
	pub fn into_parts(
		self,
	) -> (
		Box<dyn Fn() -> bool + 'static>,
		Box<dyn Fn() -> View + 'static>,
		Box<dyn Fn() -> View + 'static>,
	) {
		(self.condition, self.then_view, self.else_view)
	}
}

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
	/// A view with associated head section.
	///
	/// This variant allows components to declare their own `<head>` requirements
	/// (title, meta tags, stylesheets, etc.) that will be collected during SSR.
	WithHead {
		/// The head section for this view.
		head: Head,
		/// The actual view content.
		view: Box<View>,
	},
	/// A reactive conditional view.
	///
	/// This variant enables automatic DOM updates when the condition's
	/// Signal dependencies change. The condition is re-evaluated and
	/// the appropriate branch is rendered.
	ReactiveIf(ReactiveIf),
	/// A reactive view that re-renders when Signal dependencies change.
	///
	/// This variant wraps any expression in a reactive context, enabling
	/// automatic DOM updates when Signal values accessed within the
	/// closure change.
	Reactive(Reactive),
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

	/// Adds a boolean attribute.
	///
	/// Boolean attributes in HTML are either present (true) or absent (false).
	/// When true, the attribute is added with the attribute name as its value
	/// (e.g., `disabled="disabled"`). When false, the attribute is not added.
	///
	/// # Example
	///
	/// ```ignore
	/// ElementView::new("button")
	///     .bool_attr("disabled", is_disabled)
	///     .child("Click me")
	/// ```
	pub fn bool_attr(self, name: impl Into<Cow<'static, str>>, value: bool) -> Self {
		if value {
			let name = name.into();
			// Boolean attributes use the attribute name as value (e.g., disabled="disabled")
			self.attr(name.clone(), name)
		} else {
			self
		}
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

	/// Adds an event listener using string event name (convenience method).
	///
	/// This is a convenience wrapper around [`on`] that accepts a string event name
	/// and a closure. The event name is parsed to [`EventType`] at runtime.
	///
	/// # Arguments
	///
	/// * `event_name` - The event name (e.g., "click", "submit", "input")
	/// * `handler` - The event handler closure
	///
	/// # Panics
	///
	/// Panics if the event name is not a recognized event type.
	///
	/// # Example
	///
	/// ```ignore
	/// ElementView::new("button")
	///     .listener("click", |event| {
	///         console::log_1(&"Button clicked!".into());
	///     })
	/// ```
	#[cfg(target_arch = "wasm32")]
	pub fn listener<F>(self, event_name: &str, handler: F) -> Self
	where
		F: Fn(web_sys::Event) + 'static,
	{
		use std::str::FromStr;
		let event_type = EventType::from_str(event_name)
			.unwrap_or_else(|_| panic!("Unknown event type: {}", event_name));
		self.on(event_type, Arc::new(handler))
	}

	/// Adds an event listener using string event name (non-WASM stub).
	///
	/// In non-WASM environments, this is a stub that stores the handler
	/// for API compatibility but won't actually attach to DOM events.
	#[cfg(not(target_arch = "wasm32"))]
	pub fn listener<F>(self, event_name: &str, handler: F) -> Self
	where
		F: Fn(DummyEvent) + Send + Sync + 'static,
	{
		use std::str::FromStr;
		let event_type = EventType::from_str(event_name)
			.unwrap_or_else(|_| panic!("Unknown event type: {}", event_name));
		self.on(event_type, Arc::new(handler))
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

	/// Consumes the element view and returns the children.
	pub fn into_children(self) -> Vec<View> {
		self.children
	}

	/// Consumes the element view and returns the event handlers.
	pub fn into_event_handlers(self) -> Vec<(EventType, ViewEventHandler)> {
		self.event_handlers
	}

	/// Consumes the element view and returns all parts.
	///
	/// Returns a tuple of (tag, attrs, children, is_void, event_handlers).
	#[allow(clippy::type_complexity)] // Tuple decomposition is intentional for destructuring
	pub fn into_parts(
		self,
	) -> (
		Cow<'static, str>,
		Vec<(Cow<'static, str>, Cow<'static, str>)>,
		Vec<View>,
		bool,
		Vec<(EventType, ViewEventHandler)>,
	) {
		(
			self.tag,
			self.attrs,
			self.children,
			self.is_void,
			self.event_handlers,
		)
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

	/// Attaches a head section to this view.
	///
	/// The head section contains metadata like title, meta tags, stylesheets,
	/// and scripts that should be included in the HTML `<head>` element.
	///
	/// # Example
	///
	/// ```ignore
	/// use reinhardt_pages::{View, Head};
	///
	/// let view = View::text("Hello, World!");
	/// let head = Head::new().title("My Page");
	/// let view_with_head = view.with_head(head);
	/// ```
	pub fn with_head(self, head: Head) -> Self {
		View::WithHead {
			head,
			view: Box::new(self),
		}
	}

	/// Creates a reactive conditional view.
	///
	/// The condition is re-evaluated whenever its Signal dependencies change,
	/// and the DOM is automatically updated to show the appropriate branch.
	///
	/// # Arguments
	///
	/// * `condition` - A closure that returns `true` or `false`. This closure
	///   should call `.get()` on Signals to establish reactive dependencies.
	/// * `then_view` - A closure that returns the View to render when condition is true.
	/// * `else_view` - A closure that returns the View to render when condition is false.
	///
	/// # Example
	///
	/// ```ignore
	/// use reinhardt_pages::component::View;
	/// use reinhardt_pages::reactive::hooks::use_state;
	///
	/// let (show_error, set_show_error) = use_state(false);
	///
	/// let view = View::reactive_if(
	///     move || show_error.get(),
	///     move || View::text("Error occurred!"),
	///     move || View::text("All good!"),
	/// );
	/// ```
	pub fn reactive_if<C, T, E>(condition: C, then_view: T, else_view: E) -> Self
	where
		C: Fn() -> bool + 'static,
		T: Fn() -> View + 'static,
		E: Fn() -> View + 'static,
	{
		View::ReactiveIf(ReactiveIf {
			condition: Box::new(condition),
			then_view: Box::new(then_view),
			else_view: Box::new(else_view),
		})
	}

	/// Creates a reactive view that re-renders when Signal dependencies change.
	///
	/// This wraps any view-generating closure in a reactive context. When any
	/// Signal accessed within the closure changes, the closure is re-evaluated
	/// and the DOM is updated accordingly.
	///
	/// # Arguments
	///
	/// * `render` - A closure that returns a `View`. This closure will be
	///   re-evaluated whenever its Signal dependencies change.
	///
	/// # Example
	///
	/// ```rust,ignore
	/// let (count, set_count) = use_state(0);
	///
	/// View::reactive(move || {
	///     if count.get() > 0 {
	///         View::text(format!("Count: {}", count.get()))
	///     } else {
	///         View::text("No count yet")
	///     }
	/// })
	/// ```
	pub fn reactive<F>(render: F) -> Self
	where
		F: Fn() -> View + 'static,
	{
		View::Reactive(Reactive {
			render: Box::new(render),
		})
	}

	/// Extracts the head section from this view if it has one.
	///
	/// Returns `Some(&Head)` if this view is a `WithHead` variant,
	/// or `None` for other variants.
	pub fn extract_head(&self) -> Option<&Head> {
		match self {
			View::WithHead { head, .. } => Some(head),
			_ => None,
		}
	}

	/// Finds the topmost head section in the view tree.
	///
	/// This method searches the view tree from the root and returns the first
	/// head section found. This ensures that the outermost (page-level) head
	/// takes precedence over inner component heads.
	///
	/// # Search Order
	///
	/// 1. If this view is a `WithHead`, returns its head
	/// 2. For `Fragment` views, searches children in order and returns the first found
	/// 3. For other variants, returns `None`
	pub fn find_topmost_head(&self) -> Option<&Head> {
		match self {
			View::WithHead { head, .. } => Some(head),
			View::Fragment(children) => children.iter().find_map(|v| v.find_topmost_head()),
			_ => None,
		}
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
					// Skip boolean attributes with falsy values (empty, "false", "0")
					let name_str: &str = name.as_ref();
					if BOOLEAN_ATTRS.contains(&name_str) && !is_boolean_attr_truthy(value) {
						continue;
					}

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
			View::WithHead { view, .. } => {
				// The head is extracted separately during SSR; here we just render the content
				view.render_to_string_inner(output);
			}
			View::ReactiveIf(reactive_if) => {
				// For SSR, evaluate condition once and render the appropriate branch
				let condition_result = (reactive_if.condition)();
				let view = if condition_result {
					(reactive_if.then_view)()
				} else {
					(reactive_if.else_view)()
				};
				view.render_to_string_inner(output);
			}
			View::Reactive(reactive) => {
				// For SSR, evaluate render once and render the result
				let view = reactive.render();
				view.render_to_string_inner(output);
			}
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
					// Skip boolean attributes with falsy values (empty, "false", "0")
					// This ensures `disabled: ""` doesn't set the attribute
					let name_str: &str = name.as_ref();
					let value_str: &str = value.as_ref();
					let is_boolean = BOOLEAN_ATTRS.contains(&name_str);
					let is_falsy = !is_boolean_attr_truthy(value_str);

					if is_boolean && is_falsy {
						continue;
					}

					element
						.set_attribute(&name, &value)
						.map_err(|err_str: String| {
							#[cfg(target_arch = "wasm32")]
							{
								// Log detailed error to browser console
								let msg: wasm_bindgen::JsValue = format!(
									"[SetAttributeFailed] attribute='{}', value='{}'",
									name, value
								)
								.into();
								let label: wasm_bindgen::JsValue = "Error message:".into();
								let err_msg: wasm_bindgen::JsValue = err_str.into();
								web_sys::console::error_3(&msg, &label, &err_msg);
							}
							MountError::SetAttributeFailed
						})?;
				}

				// Attach event handlers before mounting children
				for (event_type, handler) in el.event_handlers {
					#[cfg(target_arch = "wasm32")]
					{
						let handler_clone = handler.clone();
						let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
							handler_clone(event);
						}) as Box<dyn FnMut(web_sys::Event)>);

						element
							.inner()
							.add_event_listener_with_callback(
								event_type.as_str(),
								closure.as_ref().unchecked_ref(),
							)
							.expect("Failed to add event listener");

						// Prevent the closure from being dropped (memory leak, but intentional for app lifetime)
						closure.forget();
					}
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
			View::WithHead { view, .. } => {
				// On client-side, head is handled separately; just mount the content
				view.mount_inner(parent)?;
			}
			View::ReactiveIf(reactive_if) => {
				// Decompose the ReactiveIf to get the closures
				let (condition, then_view, else_view) = reactive_if.into_parts();

				// Create a ReactiveIfNode that manages DOM updates reactively.
				// The node uses an Effect to monitor condition changes and swaps
				// DOM nodes when the condition value changes.
				let node = ReactiveIfNode::new(
					parent,
					move || condition(),
					move || then_view(),
					move || else_view(),
				);
				// Store the node to keep it alive for the lifetime of the DOM element
				store_reactive_node(node);
			}
			View::Reactive(reactive) => {
				// Get the render closure from the Reactive
				let render = reactive.into_render();

				// Create a ReactiveNode that manages DOM updates reactively.
				// The node uses an Effect to monitor dependency changes and
				// re-renders when they change.
				let node = ReactiveNode::new(parent, move || render());
				// Store the node to keep it alive for the lifetime of the DOM element
				store_reactive_node(node);
			}
		}

		Ok(())
	}

	/// Mounts the view (non-WASM stub).
	///
	/// In non-WASM environments, this function is a no-op stub that always succeeds.
	/// The `_parent` parameter is unused and exists only for API compatibility.
	#[cfg(not(target_arch = "wasm32"))]
	pub fn mount<T>(self, _parent: &T) -> Result<(), MountError> {
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

impl IntoView for &String {
	fn into_view(self) -> View {
		View::Text(Cow::Owned(self.clone()))
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

	// Boolean attribute handling tests

	#[test]
	fn test_is_boolean_attr_truthy() {
		// Truthy values
		assert!(is_boolean_attr_truthy("true"));
		assert!(is_boolean_attr_truthy("1"));
		assert!(is_boolean_attr_truthy("disabled"));
		assert!(is_boolean_attr_truthy("yes"));

		// Falsy values
		assert!(!is_boolean_attr_truthy(""));
		assert!(!is_boolean_attr_truthy("false"));
		assert!(!is_boolean_attr_truthy("0"));
	}

	#[test]
	fn test_boolean_attr_disabled_empty_string_not_rendered() {
		// Empty string should NOT render the disabled attribute
		let view = ElementView::new("button").attr("disabled", "").into_view();
		let html = view.render_to_string();
		assert_eq!(html, "<button></button>");
		assert!(!html.contains("disabled"));
	}

	#[test]
	fn test_boolean_attr_disabled_false_not_rendered() {
		// "false" should NOT render the disabled attribute
		let view = ElementView::new("button")
			.attr("disabled", "false")
			.into_view();
		let html = view.render_to_string();
		assert_eq!(html, "<button></button>");
		assert!(!html.contains("disabled"));
	}

	#[test]
	fn test_boolean_attr_disabled_zero_not_rendered() {
		// "0" should NOT render the disabled attribute
		let view = ElementView::new("button").attr("disabled", "0").into_view();
		let html = view.render_to_string();
		assert_eq!(html, "<button></button>");
		assert!(!html.contains("disabled"));
	}

	#[test]
	fn test_boolean_attr_disabled_true_rendered() {
		// "true" should render the disabled attribute
		let view = ElementView::new("button")
			.attr("disabled", "true")
			.into_view();
		let html = view.render_to_string();
		assert!(html.contains("disabled=\"true\""));
	}

	#[test]
	fn test_boolean_attr_checked_empty_not_rendered() {
		// Empty string should NOT render the checked attribute
		let view = ElementView::new("input")
			.attr("type", "checkbox")
			.attr("checked", "")
			.into_view();
		let html = view.render_to_string();
		assert!(html.contains("type=\"checkbox\""));
		assert!(!html.contains("checked"));
	}

	#[test]
	fn test_boolean_attr_checked_true_rendered() {
		// "true" should render the checked attribute
		let view = ElementView::new("input")
			.attr("type", "checkbox")
			.attr("checked", "true")
			.into_view();
		let html = view.render_to_string();
		assert!(html.contains("checked=\"true\""));
	}

	#[test]
	fn test_non_boolean_attr_empty_string_rendered() {
		// Non-boolean attributes with empty string SHOULD be rendered
		let view = ElementView::new("input")
			.attr("placeholder", "")
			.into_view();
		let html = view.render_to_string();
		assert!(html.contains("placeholder=\"\""));
	}

	#[test]
	fn test_non_boolean_attr_false_rendered() {
		// Non-boolean attributes with "false" SHOULD be rendered as-is
		let view = ElementView::new("div")
			.attr("data-active", "false")
			.into_view();
		let html = view.render_to_string();
		assert!(html.contains("data-active=\"false\""));
	}

	#[test]
	fn test_multiple_boolean_attrs_mixed() {
		// Mix of boolean and non-boolean attributes
		let view = ElementView::new("input")
			.attr("type", "text")
			.attr("disabled", "")      // Should NOT be rendered
			.attr("readonly", "true")  // Should be rendered
			.attr("required", "false") // Should NOT be rendered
			.attr("placeholder", "")   // Should be rendered (non-boolean)
			.into_view();
		let html = view.render_to_string();

		assert!(html.contains("type=\"text\""));
		assert!(!html.contains("disabled"));
		assert!(html.contains("readonly=\"true\""));
		assert!(!html.contains("required"));
		assert!(html.contains("placeholder=\"\""));
	}
}
