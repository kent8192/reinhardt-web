//! Document Wrapper
//!
//! Provides a safe wrapper around `web_sys::Document` for DOM creation and querying.
//!
//! ## Thread-local Caching
//!
//! The global Document instance is cached in thread-local storage for efficiency:
//!
//! ```no_run
//! use reinhardt_pages::dom::Document;
//!
//! let doc = Document::global(); // Cached access
//! let div = doc.create_element("div")?;
//! ```

use std::cell::RefCell;

use wasm_bindgen::JsCast;
use web_sys;

use super::Element;

thread_local! {
	static CACHED_DOCUMENT: RefCell<Option<Document>> = const { RefCell::new(None) };
}

/// Thin wrapper around `web_sys::Document`
///
/// Provides ergonomic methods for creating and querying DOM elements.
#[derive(Clone)]
pub struct Document {
	/// The underlying web-sys Document
	inner: web_sys::Document,
}

impl Document {
	/// Create a new Document wrapper from a web-sys Document
	///
	/// # Arguments
	///
	/// * `document` - The web-sys Document to wrap
	pub fn new(document: web_sys::Document) -> Self {
		Self { inner: document }
	}

	/// Get the global Document instance
	///
	/// This retrieves the document from the global window object.
	/// The result is cached in thread-local storage for efficiency.
	///
	/// # Panics
	///
	/// Panics if there is no global window or document (non-browser environment).
	///
	/// # Example
	///
	/// ```no_run
	/// use reinhardt_pages::dom::Document;
	///
	/// let doc = Document::global();
	/// let div = doc.create_element("div").unwrap();
	/// ```
	pub fn global() -> Self {
		CACHED_DOCUMENT.with(|cache| {
			let mut cache_mut = cache.borrow_mut();
			if let Some(ref doc) = *cache_mut {
				doc.clone()
			} else {
				let window = web_sys::window().expect("No global window exists");
				let document = window.document().expect("Window should have a document");
				let doc = Self::new(document);
				*cache_mut = Some(doc.clone());
				doc
			}
		})
	}

	/// Get a reference to the underlying web-sys Document
	pub fn as_web_sys(&self) -> &web_sys::Document {
		&self.inner
	}

	/// Create a new element with the given tag name
	///
	/// # Arguments
	///
	/// * `tag_name` - HTML tag name (e.g., "div", "button", "input")
	///
	/// # Returns
	///
	/// A wrapped Element on success, or an error string on failure.
	///
	/// # Example
	///
	/// ```no_run
	/// let doc = Document::global();
	/// let div = doc.create_element("div")?;
	/// let button = doc.create_element("button")?;
	/// ```
	pub fn create_element(&self, tag_name: &str) -> Result<Element, String> {
		self.inner
			.create_element(tag_name)
			.map(Element::new)
			.map_err(|e| format!("Failed to create element '{}': {:?}", tag_name, e))
	}

	/// Query for a single element matching the CSS selector
	///
	/// # Arguments
	///
	/// * `selector` - CSS selector string
	///
	/// # Returns
	///
	/// `Some(Element)` if found, `None` if not found, or `Err` if selector is invalid.
	///
	/// # Example
	///
	/// ```no_run
	/// let doc = Document::global();
	/// if let Some(button) = doc.query_selector("#submit-button")? {
	///     button.set_attribute("disabled", "true")?;
	/// }
	/// ```
	pub fn query_selector(&self, selector: &str) -> Result<Option<Element>, String> {
		self.inner
			.query_selector(selector)
			.map(|opt| opt.map(Element::new))
			.map_err(|e| format!("Failed to query selector '{}': {:?}", selector, e))
	}

	/// Get the document body
	///
	/// # Returns
	///
	/// The body element, or `None` if the document has no body.
	///
	/// # Example
	///
	/// ```no_run
	/// let doc = Document::global();
	/// if let Some(body) = doc.body() {
	///     let div = doc.create_element("div")?;
	///     body.as_web_sys().append_child(div.as_web_sys())?;
	/// }
	/// ```
	pub fn body(&self) -> Option<Element> {
		self.inner
			.body()
			.map(|body| Element::new(body.unchecked_into()))
	}

	/// Get the document head
	///
	/// # Returns
	///
	/// The head element, or `None` if the document has no head.
	///
	/// # Note
	///
	/// This uses `query_selector` internally as web-sys doesn't expose a direct `head()` method.
	pub fn head(&self) -> Option<Element> {
		self.query_selector("head").ok().flatten()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use wasm_bindgen_test::*;

	wasm_bindgen_test_configure!(run_in_browser);

	#[wasm_bindgen_test]
	fn test_document_global() {
		let doc = Document::global();
		assert!(doc.as_web_sys().body().is_some());
	}

	#[wasm_bindgen_test]
	fn test_document_create_element() {
		let doc = Document::global();
		let div = doc.create_element("div").unwrap();
		assert!(div.as_web_sys().tag_name().eq_ignore_ascii_case("div"));
	}

	#[wasm_bindgen_test]
	fn test_document_body() {
		let doc = Document::global();
		let body = doc.body();
		assert!(body.is_some());
	}

	#[wasm_bindgen_test]
	fn test_document_head() {
		let doc = Document::global();
		let head = doc.head();
		assert!(head.is_some());
	}
}
