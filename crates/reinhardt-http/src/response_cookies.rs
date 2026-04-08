//! Response cookies for server function handlers.
//!
//! Allows server functions to set `Set-Cookie` headers on HTTP responses
//! by inserting a [`ResponseCookies`] value into the request's
//! [`Extensions`](crate::Extensions). The server function router
//! automatically extracts these cookies and applies them as `Set-Cookie`
//! headers on the outgoing HTTP response.
//!
//! # How it works
//!
//! [`Extensions`](crate::Extensions) uses `Arc<Mutex<HashMap>>` internally,
//! so cloning an `Extensions` value shares the same backing store. The
//! server function router clones the request's extensions *before* calling
//! the handler. Any [`ResponseCookies`] the handler inserts into
//! `request.extensions` are therefore visible through the cloned reference,
//! and the router can extract and apply them after the handler returns.
//!
//! # Usage in a handler
//!
//! Insert a [`ResponseCookies`] into the request's extensions inside your
//! server function handler. **Do not** construct `ResponseCookies`
//! separately and return it — it must be placed into the request's
//! extensions so the router can find it.
//!
//! ```
//! use reinhardt_http::ResponseCookies;
//!
//! // Inside a server function handler:
//! let mut cookies = ResponseCookies::new();
//! cookies.add("session=abc123; Path=/; HttpOnly".to_string());
//! // request.extensions.insert(cookies);
//!
//! assert_eq!(cookies.cookies().len(), 1);
//! ```

/// A collection of `Set-Cookie` header values to include in the HTTP response.
///
/// Server function handlers insert this into the request's
/// [`Extensions`](crate::Extensions) to communicate cookies back to the
/// response layer. Because `Extensions` is backed by `Arc<Mutex<HashMap>>`,
/// cloning it shares the same underlying map. The server function router
/// exploits this: it clones the extensions before invoking the handler, then
/// extracts `ResponseCookies` from the clone afterwards. Each cookie is
/// applied as a `Set-Cookie` header on the HTTP response.
///
/// **Important:** `ResponseCookies` must be inserted into the request's
/// extensions — not held separately — for the cookies to reach the
/// response.
#[derive(Debug, Clone, Default)]
pub struct ResponseCookies {
	/// Cookie header values to include in the response
	cookies: Vec<String>,
}

impl ResponseCookies {
	/// Creates a new empty `ResponseCookies`.
	pub fn new() -> Self {
		Self {
			cookies: Vec::new(),
		}
	}

	/// Adds a `Set-Cookie` header value.
	pub fn add(&mut self, cookie: String) {
		self.cookies.push(cookie);
	}

	/// Returns the cookie header values.
	pub fn cookies(&self) -> &[String] {
		&self.cookies
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_new_response_cookies_is_empty() {
		// Arrange & Act
		let cookies = ResponseCookies::new();

		// Assert
		assert!(cookies.cookies().is_empty());
	}

	#[rstest]
	fn test_add_single_cookie() {
		// Arrange
		let mut cookies = ResponseCookies::new();

		// Act
		cookies.add("session=abc; Path=/".to_string());

		// Assert
		assert_eq!(cookies.cookies().len(), 1);
		assert_eq!(cookies.cookies()[0], "session=abc; Path=/");
	}

	#[rstest]
	fn test_add_multiple_cookies() {
		// Arrange
		let mut cookies = ResponseCookies::new();

		// Act
		cookies.add("session=abc; Path=/".to_string());
		cookies.add("csrf=xyz; SameSite=Strict".to_string());

		// Assert
		assert_eq!(cookies.cookies().len(), 2);
		assert_eq!(cookies.cookies()[0], "session=abc; Path=/");
		assert_eq!(cookies.cookies()[1], "csrf=xyz; SameSite=Strict");
	}

	#[rstest]
	fn test_default_is_empty() {
		// Arrange & Act
		let cookies = ResponseCookies::default();

		// Assert
		assert!(cookies.cookies().is_empty());
	}
}
