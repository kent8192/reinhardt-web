//! Redirect shortcut functions
//!
//! Provides convenient functions for creating HTTP redirects.

use reinhardt_core::http::Response;

/// Create a temporary redirect (HTTP 302) to the specified URL
///
/// This is the most common redirect type, indicating that the resource has temporarily
/// moved to another location. Search engines won't update their links.
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::redirect;
///
// Redirect to a different page
/// let response = redirect("/users/profile/");
/// ```
///
/// # Arguments
///
/// * `to` - The URL to redirect to. Can be a relative or absolute URL.
///
/// # Returns
///
/// A `Response` with HTTP 302 status and the Location header set.
pub fn redirect(to: impl AsRef<str>) -> Response {
	Response::temporary_redirect(to.as_ref())
}

/// Create a permanent redirect (HTTP 301) to the specified URL
///
/// Use this when a resource has permanently moved to a new location.
/// Search engines will update their indexes to point to the new URL.
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::redirect_permanent;
///
// Permanently redirect old URL to new one
/// let response = redirect_permanent("/new-page/");
/// ```
///
/// # Arguments
///
/// * `to` - The URL to redirect to. Can be a relative or absolute URL.
///
/// # Returns
///
/// A `Response` with HTTP 301 status and the Location header set.
pub fn redirect_permanent(to: impl AsRef<str>) -> Response {
	Response::permanent_redirect(to.as_ref())
}

#[cfg(test)]
mod tests {
	use super::*;
	use hyper::StatusCode;

	#[test]
	fn test_redirect_temporary() {
		let response = redirect("/users/");

		assert_eq!(response.status, StatusCode::FOUND);
		assert_eq!(
			response.headers.get("location").unwrap().to_str().unwrap(),
			"/users/"
		);
	}

	#[test]
	fn test_redirect_permanent() {
		let response = redirect_permanent("/new-location/");

		assert_eq!(response.status, StatusCode::MOVED_PERMANENTLY);
		assert_eq!(
			response.headers.get("location").unwrap().to_str().unwrap(),
			"/new-location/"
		);
	}

	#[test]
	fn test_redirect_absolute_url() {
		let response = redirect("https://example.com/page/");

		assert_eq!(
			response.headers.get("location").unwrap().to_str().unwrap(),
			"https://example.com/page/"
		);
	}

	#[test]
	fn test_redirect_with_query_params() {
		let response = redirect("/search/?q=test&page=2");

		assert_eq!(
			response.headers.get("location").unwrap().to_str().unwrap(),
			"/search/?q=test&page=2"
		);
	}
}
