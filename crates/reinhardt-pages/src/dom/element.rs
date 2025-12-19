//! Element Wrapper
//!
//! Provides a safe, ergonomic wrapper around `web_sys::Element`.
//!
//! ## Key Features
//!
//! - **Attribute Management**: Type-safe attribute operations
//! - **Event Listeners**: RAII-based automatic cleanup via `EventHandle`
//! - **Reactive Binding**: Integration with Signal system for automatic updates
//!
//! ## RAII Pattern for Event Listeners
//!
//! Event listeners are automatically removed when `EventHandle` is dropped:
//!
//! ```ignore
//! let element = document().create_element("button")?;
//! {
//!     let handle = element.add_event_listener("click", || {
//!         console::log_1(&"Clicked!".into());
//!     });
//!     // handle.drop() is called here, removing the event listener
//! }
//! ```

use wasm_bindgen::{JsCast, closure::Closure};
use web_sys;

use crate::reactive::{Effect, Signal};

/// Thin wrapper around `web_sys::Element`
///
/// This struct provides a safe, ergonomic API for DOM manipulation while
/// maintaining compatibility with the underlying web-sys types.
#[derive(Clone)]
pub struct Element {
	/// The underlying web-sys Element
	inner: web_sys::Element,
}

impl Element {
	/// Create a new Element wrapper from a web-sys Element
	///
	/// # Arguments
	///
	/// * `element` - The web-sys Element to wrap
	///
	/// # Example
	///
	/// ```ignore
	/// use web_sys;
	/// use reinhardt_pages::dom::Element;
	///
	/// let web_element: web_sys::Element = /* ... */;
	/// let element = Element::new(web_element);
	/// ```
	pub fn new(element: web_sys::Element) -> Self {
		Self { inner: element }
	}

	/// Get a reference to the underlying web-sys Element
	///
	/// This is useful when you need to pass the element to web-sys APIs directly.
	pub fn as_web_sys(&self) -> &web_sys::Element {
		&self.inner
	}

	/// Consume self and return the underlying web-sys Element
	pub fn into_web_sys(self) -> web_sys::Element {
		self.inner
	}

	/// Set an attribute on this element
	///
	/// # Arguments
	///
	/// * `name` - Attribute name
	/// * `value` - Attribute value
	///
	/// # Example
	///
	/// ```ignore
	/// element.set_attribute("class", "btn btn-primary")?;
	/// element.set_attribute("id", "submit-button")?;
	/// ```
	pub fn set_attribute(&self, name: &str, value: &str) -> Result<(), String> {
		self.inner
			.set_attribute(name, value)
			.map_err(|e| format!("Failed to set attribute '{}': {:?}", name, e))
	}

	/// Get an attribute from this element
	///
	/// Returns `None` if the attribute doesn't exist.
	///
	/// # Arguments
	///
	/// * `name` - Attribute name
	///
	/// # Example
	///
	/// ```ignore
	/// if let Some(class) = element.get_attribute("class")? {
	///     println!("Element has class: {}", class);
	/// }
	/// ```
	pub fn get_attribute(&self, name: &str) -> Option<String> {
		self.inner.get_attribute(name)
	}

	/// Remove an attribute from this element
	///
	/// # Arguments
	///
	/// * `name` - Attribute name
	///
	/// # Example
	///
	/// ```ignore
	/// element.remove_attribute("disabled")?;
	/// ```
	pub fn remove_attribute(&self, name: &str) -> Result<(), String> {
		self.inner
			.remove_attribute(name)
			.map_err(|e| format!("Failed to remove attribute '{}': {:?}", name, e))
	}

	/// Set a reactive attribute that automatically updates when a Signal changes
	///
	/// This creates an Effect that tracks the Signal and updates the attribute
	/// whenever the Signal's value changes.
	///
	/// # Arguments
	///
	/// * `name` - Attribute name
	/// * `signal` - Signal to bind to
	///
	/// # Example
	///
	/// ```ignore
	/// use reinhardt_pages::{reactive::Signal, dom::Element};
	///
	/// let count = Signal::new(0);
	/// let element = document().create_element("div")?;
	///
	/// // Attribute automatically updates when count changes
	/// element.set_reactive_attribute("data-count", count.clone());
	///
	/// count.set(42); // Attribute is now "42"
	/// ```
	///
	/// # Note
	///
	/// The Effect is not returned, so it will live for the duration of the program.
	/// For fine-grained control, use Effect::new() directly.
	pub fn set_reactive_attribute<T>(&self, name: &str, signal: Signal<T>)
	where
		T: ToString + Clone + 'static,
	{
		let element = self.clone();
		let name = name.to_string();

		Effect::new(move || {
			let value = signal.get().to_string();
			let _ = element.set_attribute(&name, &value);
		});
	}

	/// Add an event listener to this element
	///
	/// Returns an `EventHandle` that automatically removes the listener when dropped.
	///
	/// # Arguments
	///
	/// * `event_type` - Event type (e.g., "click", "input")
	/// * `callback` - Closure to call when event fires
	///
	/// # Example
	///
	/// ```ignore
	/// let handle = element.add_event_listener("click", || {
	///     console::log_1(&"Button clicked!".into());
	/// });
	///
	/// // Keep `handle` alive as long as you want the listener active
	/// // When `handle` is dropped, the listener is automatically removed
	/// ```
	pub fn add_event_listener<F>(&self, event_type: &str, mut callback: F) -> EventHandle
	where
		F: FnMut() + 'static,
	{
		let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
			callback();
		}) as Box<dyn FnMut(web_sys::Event)>);

		self.inner
			.add_event_listener_with_callback(event_type, closure.as_ref().unchecked_ref())
			.expect("Failed to add event listener");

		EventHandle {
			element: self.inner.clone(),
			event_type: event_type.to_string(),
			closure: Some(closure),
		}
	}

	/// Set text content of this element
	///
	/// # Arguments
	///
	/// * `text` - Text content to set
	///
	/// # Example
	///
	/// ```ignore
	/// element.set_text_content("Hello, World!");
	/// ```
	pub fn set_text_content(&self, text: &str) {
		self.inner.set_text_content(Some(text));
	}

	/// Get text content of this element
	///
	/// # Example
	///
	/// ```ignore
	/// let text = element.text_content();
	/// println!("Element text: {:?}", text);
	/// ```
	pub fn text_content(&self) -> Option<String> {
		self.inner.text_content()
	}

	/// Append a child element to this element
	///
	/// # Arguments
	///
	/// * `child` - Child element to append
	///
	/// # Example
	///
	/// ```ignore
	/// let parent = document().create_element("div")?;
	/// let child = document().create_element("p")?;
	/// parent.append_child(child)?;
	/// ```
	pub fn append_child(&self, child: Element) -> Result<(), String> {
		self.inner
			.append_child(&child.inner)
			.map_err(|e| format!("Failed to append child: {:?}", e))?;
		Ok(())
	}
}

/// RAII wrapper for event listeners
///
/// When dropped, this handle automatically removes the event listener from the element.
/// This prevents memory leaks from forgotten event listeners.
///
/// ## Example
///
/// ```ignore
/// {
///     let handle = element.add_event_listener("click", || {
///         console::log_1(&"Clicked!".into());
///     });
///     // Listener is active here
/// } // handle.drop() called here - listener automatically removed
/// ```
pub struct EventHandle {
	/// The element this listener is attached to
	element: web_sys::Element,
	/// Event type ("click", "input", etc.)
	event_type: String,
	/// The closure that handles the event
	/// Wrapped in Option so we can take() it in Drop
	closure: Option<Closure<dyn FnMut(web_sys::Event)>>,
}

impl Drop for EventHandle {
	fn drop(&mut self) {
		if let Some(closure) = self.closure.take() {
			// Remove the event listener
			let _ = self.element.remove_event_listener_with_callback(
				&self.event_type,
				closure.as_ref().unchecked_ref(),
			);
			// Closure is dropped here, cleaning up the JavaScript callback
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use wasm_bindgen_test::*;

	wasm_bindgen_test_configure!(run_in_browser);

	#[wasm_bindgen_test]
	fn test_element_set_attribute() {
		let document = web_sys::window().unwrap().document().unwrap();
		let web_element = document.create_element("div").unwrap();
		let element = Element::new(web_element);

		element.set_attribute("id", "test-div").unwrap();
		assert_eq!(element.get_attribute("id"), Some("test-div".to_string()));
	}

	#[wasm_bindgen_test]
	fn test_element_remove_attribute() {
		let document = web_sys::window().unwrap().document().unwrap();
		let web_element = document.create_element("div").unwrap();
		let element = Element::new(web_element);

		element.set_attribute("data-test", "value").unwrap();
		assert!(element.get_attribute("data-test").is_some());

		element.remove_attribute("data-test").unwrap();
		assert!(element.get_attribute("data-test").is_none());
	}

	#[wasm_bindgen_test]
	fn test_element_text_content() {
		let document = web_sys::window().unwrap().document().unwrap();
		let web_element = document.create_element("p").unwrap();
		let element = Element::new(web_element);

		element.set_text_content("Hello, World!");
		assert_eq!(element.text_content(), Some("Hello, World!".to_string()));
	}
}
