//! Shared helpers for parsing session cookies from an HTTP `Cookie` header.
//!
//! Centralizing this avoids drift between the `SessionMiddleware` request
//! ingestion path and the `Injectable` DI path.

use reinhardt_http::Request;

/// Find the value of the cookie named `cookie_name` in the request's
/// `Cookie` header. Returns `None` if the header is missing, not valid
/// UTF-8, or does not contain a cookie with the requested name.
///
/// Cookie matching is case-sensitive on the name and uses `splitn(2, '=')`
/// so that values containing `=` are preserved.
pub(super) fn find_cookie_value(request: &Request, cookie_name: &str) -> Option<String> {
	let cookie_header = request.headers.get(hyper::header::COOKIE)?;
	let cookie_str = cookie_header.to_str().ok()?;
	for cookie in cookie_str.split(';') {
		let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
		if parts.len() == 2 && parts[0] == cookie_name {
			return Some(parts[1].to_string());
		}
	}
	None
}
