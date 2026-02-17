//! CSP Helper Functions
//!
//! Provides utility functions for retrieving CSP nonce from Request extensions
//! for use in templates and views.

use reinhardt_http::Request;

use crate::csp::CspNonce;

/// Get the CSP nonce from the request extensions
///
/// Returns the nonce if it exists, otherwise returns None.
/// The nonce is stored by the CspMiddleware when `include_nonce` is enabled.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_middleware::csp_helpers::get_csp_nonce;
/// use reinhardt_http::Request;
///
/// fn my_view(request: &Request) -> String {
///     if let Some(nonce) = get_csp_nonce(request) {
///         format!("<script nonce=\"{}\">alert('Hello');</script>", nonce)
///     } else {
///         String::from("<script>alert('Hello');</script>")
///     }
/// }
/// ```
pub fn get_csp_nonce(request: &Request) -> Option<String> {
	request.extensions.get::<CspNonce>().map(|n| n.0.clone())
}

/// Get CSP nonce attribute for HTML tags
///
/// Returns a formatted nonce attribute string if nonce exists, otherwise returns empty string.
/// This is useful for directly inserting into HTML templates.
///
/// # Examples
///
/// ```rust,no_run
/// use reinhardt_middleware::csp_helpers::csp_nonce_attr;
/// use reinhardt_http::Request;
///
/// fn my_template(request: &Request) -> String {
///     format!("<script {}>alert('Hello');</script>", csp_nonce_attr(request))
/// }
/// // Output: <script nonce="AbCdEf123456">alert('Hello');</script>
/// // Or:     <script>alert('Hello');</script> (if nonce not enabled)
/// ```
pub fn csp_nonce_attr(request: &Request) -> String {
	if let Some(nonce) = get_csp_nonce(request) {
		format!("nonce=\"{}\"", nonce)
	} else {
		String::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Version};
	use rstest::rstest;

	#[rstest]
	fn test_get_csp_nonce_exists() {
		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		request
			.extensions
			.insert(CspNonce("test-nonce-123".to_string()));

		let nonce = get_csp_nonce(&request);
		assert_eq!(nonce, Some("test-nonce-123".to_string()));
	}

	#[rstest]
	fn test_get_csp_nonce_not_exists() {
		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let nonce = get_csp_nonce(&request);
		assert_eq!(nonce, None);
	}

	#[rstest]
	fn test_csp_nonce_attr_exists() {
		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		request.extensions.insert(CspNonce("abc123".to_string()));

		let attr = csp_nonce_attr(&request);
		assert_eq!(attr, "nonce=\"abc123\"");
	}

	#[rstest]
	fn test_csp_nonce_attr_not_exists() {
		let request = Request::builder()
			.method(Method::GET)
			.uri("/test")
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let attr = csp_nonce_attr(&request);
		assert_eq!(attr, "");
	}
}
