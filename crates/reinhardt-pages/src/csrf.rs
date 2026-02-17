//! CSRF Protection for Client-side WASM
//!
//! This module provides utilities for retrieving and injecting CSRF tokens
//! in client-side WASM applications. It integrates with Django's CSRF protection
//! mechanism and the `reinhardt-auth` session system.
//!
//! ## Token Retrieval
//!
//! CSRF tokens can be retrieved from multiple sources:
//! 1. **Cookie**: The `csrftoken` cookie set by Django
//! 2. **Meta tag**: `<meta name="csrf-token" content="...">` in the HTML head
//! 3. **Hidden input**: `<input name="csrfmiddlewaretoken">` in forms
//!
//! ## Usage
//!
//! ```ignore
//! use reinhardt_pages::csrf::{get_csrf_token, CsrfManager};
//!
//! // Simple retrieval
//! if let Some(token) = get_csrf_token() {
//!     // Use token for API calls
//! }
//!
//! // Using CsrfManager for more control
//! let manager = CsrfManager::new();
//! let token = manager.get_or_fetch_token();
//! ```

use crate::reactive::Signal;

/// The cookie name used by Django for CSRF tokens.
pub const CSRF_COOKIE_NAME: &str = "csrftoken";

/// The meta tag name for CSRF tokens.
pub const CSRF_META_NAME: &str = "csrf-token";

/// The header name used to send CSRF tokens in AJAX requests.
pub const CSRF_HEADER_NAME: &str = "X-CSRFToken";

/// The form field name for CSRF tokens.
pub const CSRF_FORM_FIELD: &str = "csrfmiddlewaretoken";

/// CSRF token manager for client-side token handling.
///
/// This struct provides a reactive interface to CSRF token management,
/// with automatic caching and multiple retrieval strategies.
#[derive(Debug, Clone)]
pub struct CsrfManager {
	/// Cached CSRF token as a reactive Signal.
	token: Signal<Option<String>>,
}

impl Default for CsrfManager {
	fn default() -> Self {
		Self::new()
	}
}

impl CsrfManager {
	/// Creates a new CSRF manager.
	///
	/// The token is not fetched until explicitly requested.
	pub fn new() -> Self {
		Self {
			token: Signal::new(None),
		}
	}

	/// Gets the cached token without fetching from the browser.
	pub fn cached_token(&self) -> Option<String> {
		self.token.get()
	}

	/// Gets or fetches the CSRF token.
	///
	/// This method will:
	/// 1. Return the cached token if available
	/// 2. Otherwise, try to fetch from cookie, meta tag, or form input
	/// 3. Cache the result for future use
	#[cfg(target_arch = "wasm32")]
	pub fn get_or_fetch_token(&self) -> Option<String> {
		// Return cached if available
		if let Some(token) = self.token.get() {
			return Some(token);
		}

		// Try to fetch and cache
		if let Some(token) = get_csrf_token() {
			self.token.set(Some(token.clone()));
			return Some(token);
		}

		None
	}

	/// Gets or fetches the CSRF token (non-WASM stub).
	#[cfg(not(target_arch = "wasm32"))]
	pub fn get_or_fetch_token(&self) -> Option<String> {
		self.token.get()
	}

	/// Forces a refresh of the CSRF token from the browser.
	#[cfg(target_arch = "wasm32")]
	pub fn refresh(&self) -> Option<String> {
		if let Some(token) = get_csrf_token() {
			self.token.set(Some(token.clone()));
			return Some(token);
		}
		self.token.set(None);
		None
	}

	/// Forces a refresh of the CSRF token (non-WASM stub).
	#[cfg(not(target_arch = "wasm32"))]
	pub fn refresh(&self) -> Option<String> {
		None
	}

	/// Sets the token manually.
	///
	/// This is useful when the token is received from the server
	/// (e.g., in an initial page response).
	pub fn set_token(&self, token: impl Into<String>) {
		self.token.set(Some(token.into()));
	}

	/// Clears the cached token.
	pub fn clear(&self) {
		self.token.set(None);
	}

	/// Returns a Signal that tracks the current token.
	///
	/// This can be used for reactive UI updates when the token changes.
	pub fn token_signal(&self) -> Signal<Option<String>> {
		self.token.clone()
	}
}

/// Retrieves the CSRF token from the browser.
///
/// This function tries multiple sources in order:
/// 1. Cookie (`csrftoken`)
/// 2. Meta tag (`<meta name="csrf-token">`)
/// 3. Hidden form input (`<input name="csrfmiddlewaretoken">`)
///
/// Returns `None` if no token is found.
#[cfg(target_arch = "wasm32")]
pub fn get_csrf_token() -> Option<String> {
	// Try cookie first (most common)
	if let Some(token) = get_csrf_token_from_cookie() {
		return Some(token);
	}

	// Try meta tag
	if let Some(token) = get_csrf_token_from_meta() {
		return Some(token);
	}

	// Try hidden input as last resort
	get_csrf_token_from_input()
}

/// Retrieves the CSRF token (non-WASM stub).
#[cfg(not(target_arch = "wasm32"))]
pub fn get_csrf_token() -> Option<String> {
	None
}

/// Retrieves the CSRF token from the cookie.
#[cfg(target_arch = "wasm32")]
pub fn get_csrf_token_from_cookie() -> Option<String> {
	use wasm_bindgen::JsCast;
	use web_sys::{HtmlDocument, window};

	let window = window()?;
	let document = window.document()?;
	let html_doc = document.dyn_ref::<HtmlDocument>()?;
	let cookie_str = html_doc.cookie().ok()?;

	parse_cookie_value(&cookie_str, CSRF_COOKIE_NAME)
}

/// Retrieves the CSRF token from cookie (non-WASM stub).
#[cfg(not(target_arch = "wasm32"))]
pub fn get_csrf_token_from_cookie() -> Option<String> {
	None
}

/// Retrieves the CSRF token from a meta tag.
#[cfg(target_arch = "wasm32")]
pub fn get_csrf_token_from_meta() -> Option<String> {
	use web_sys::window;

	let window = window()?;
	let document = window.document()?;

	// Try <meta name="csrf-token" content="...">
	let selector = format!("meta[name=\"{}\"]", CSRF_META_NAME);
	let meta = document.query_selector(&selector).ok()??;
	meta.get_attribute("content")
}

/// Retrieves the CSRF token from meta tag (non-WASM stub).
#[cfg(not(target_arch = "wasm32"))]
pub fn get_csrf_token_from_meta() -> Option<String> {
	None
}

/// Retrieves the CSRF token from a hidden form input.
#[cfg(target_arch = "wasm32")]
pub fn get_csrf_token_from_input() -> Option<String> {
	use wasm_bindgen::JsCast;
	use web_sys::{HtmlInputElement, window};

	let window = window()?;
	let document = window.document()?;

	// Try <input name="csrfmiddlewaretoken">
	let selector = format!("input[name=\"{}\"]", CSRF_FORM_FIELD);
	let input = document.query_selector(&selector).ok()??;
	let input: HtmlInputElement = input.dyn_into().ok()?;
	Some(input.value())
}

/// Retrieves the CSRF token from hidden input (non-WASM stub).
#[cfg(not(target_arch = "wasm32"))]
pub fn get_csrf_token_from_input() -> Option<String> {
	None
}

/// Parses a cookie value from a cookie string.
///
/// The cookie string format is: "name1=value1; name2=value2; ..."
// Exposed as public for testing purposes
pub fn parse_cookie_value(cookie_str: &str, name: &str) -> Option<String> {
	for part in cookie_str.split(';') {
		let part = part.trim();
		if let Some((key, value)) = part.split_once('=')
			&& key.trim() == name
		{
			return Some(value.trim().to_string());
		}
	}
	None
}

/// Creates HTTP headers with CSRF token for AJAX requests.
///
/// Returns a tuple of (header_name, header_value) if a token is available.
#[cfg(target_arch = "wasm32")]
pub fn csrf_headers() -> Option<(&'static str, String)> {
	get_csrf_token().map(|token| (CSRF_HEADER_NAME, token))
}

/// Creates HTTP headers with CSRF token (non-WASM stub).
#[cfg(not(target_arch = "wasm32"))]
pub fn csrf_headers() -> Option<(&'static str, String)> {
	None
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_parse_cookie_value() {
		let cookie_str = "sessionid=abc123; csrftoken=xyz789; other=value";
		assert_eq!(
			parse_cookie_value(cookie_str, "csrftoken"),
			Some("xyz789".to_string())
		);
		assert_eq!(
			parse_cookie_value(cookie_str, "sessionid"),
			Some("abc123".to_string())
		);
		assert_eq!(parse_cookie_value(cookie_str, "nonexistent"), None);
	}

	#[rstest]
	fn test_parse_cookie_value_with_spaces() {
		let cookie_str = " csrftoken = token123 ; other = value ";
		assert_eq!(
			parse_cookie_value(cookie_str, "csrftoken"),
			Some("token123".to_string())
		);
	}

	#[rstest]
	fn test_csrf_manager_creation() {
		let manager = CsrfManager::new();
		assert!(manager.cached_token().is_none());
	}

	#[rstest]
	fn test_csrf_manager_set_token() {
		let manager = CsrfManager::new();
		manager.set_token("test-token");
		assert_eq!(manager.cached_token(), Some("test-token".to_string()));
	}

	#[rstest]
	fn test_csrf_manager_clear() {
		let manager = CsrfManager::new();
		manager.set_token("test-token");
		manager.clear();
		assert!(manager.cached_token().is_none());
	}

	#[rstest]
	fn test_csrf_manager_default() {
		let manager = CsrfManager::default();
		assert!(manager.cached_token().is_none());
	}
}
