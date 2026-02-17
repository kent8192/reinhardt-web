//! Template rendering shortcut functions
//!
//! Provides convenient functions for rendering templates and creating HTTP responses.

use bytes::Bytes;
use reinhardt_http::Response;
use serde::Serialize;

/// Render data as JSON and return an HTTP 200 response
///
/// This is a convenient shortcut for creating JSON responses without needing
/// to manually create Response objects and set headers.
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::render_json;
/// use serde_json::json;
///
/// let data = json!({
///     "status": "success",
///     "message": "Hello, world!"
/// });
///
/// let response = render_json(&data);
/// ```
///
/// # Arguments
///
/// * `data` - The data to serialize as JSON
///
/// # Returns
///
/// A `Response` with HTTP 200 status, JSON content-type, and the serialized data as body.
pub fn render_json<T: Serialize>(data: &T) -> Response {
	let json_string = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());

	let mut response = Response::ok();
	response.body = Bytes::from(json_string);
	response
		.headers
		.insert("content-type", "application/json".parse().unwrap());

	response
}

/// Render data as JSON with pretty printing and return an HTTP 200 response
///
/// Same as `render_json` but with pretty-printed output (indented).
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::render_json_pretty;
/// use serde_json::json;
///
/// let data = json!({"key": "value"});
/// let response = render_json_pretty(&data);
/// ```
pub fn render_json_pretty<T: Serialize>(data: &T) -> Response {
	let json_string = serde_json::to_string_pretty(data).unwrap_or_else(|_| "{}".to_string());

	let mut response = Response::ok();
	response.body = Bytes::from(json_string);
	response
		.headers
		.insert("content-type", "application/json".parse().unwrap());

	response
}

/// Render a simple HTML string and return an HTTP 200 response
///
/// Creates a response with HTML content-type header.
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::render_html;
///
/// let html = "<h1>Hello, World!</h1>";
/// let response = render_html(html);
/// ```
///
/// # Arguments
///
/// * `html` - The HTML content to render
///
/// # Returns
///
/// A `Response` with HTTP 200 status, HTML content-type, and the HTML as body.
pub fn render_html(html: impl Into<String>) -> Response {
	let mut response = Response::ok();
	response.body = Bytes::from(html.into());
	response
		.headers
		.insert("content-type", "text/html; charset=utf-8".parse().unwrap());

	response
}

/// Render a simple text string and return an HTTP 200 response
///
/// Creates a response with plain text content-type header.
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::render_text;
///
/// let text = "Hello, World!";
/// let response = render_text(text);
/// ```
///
/// # Arguments
///
/// * `text` - The plain text content to render
///
/// # Returns
///
/// A `Response` with HTTP 200 status, text content-type, and the text as body.
pub fn render_text(text: impl Into<String>) -> Response {
	let mut response = Response::ok();
	response.body = Bytes::from(text.into());
	response
		.headers
		.insert("content-type", "text/plain; charset=utf-8".parse().unwrap());

	response
}

#[cfg(test)]
mod tests {
	use super::*;
	use hyper::StatusCode;
	use rstest::rstest;
	use serde_json::json;

	#[rstest]
	fn test_render_json() {
		let data = json!({"name": "test", "value": 123});
		let response = render_json(&data);

		assert_eq!(response.status, StatusCode::OK);
		assert_eq!(
			response
				.headers
				.get("content-type")
				.unwrap()
				.to_str()
				.unwrap(),
			"application/json"
		);

		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		assert!(body_str.contains("test"));
		assert!(body_str.contains("123"));
	}

	#[rstest]
	fn test_render_json_pretty() {
		let data = json!({"name": "test"});
		let response = render_json_pretty(&data);

		assert_eq!(response.status, StatusCode::OK);
		let body_str = String::from_utf8(response.body.to_vec()).unwrap();

		// Pretty printed JSON should have newlines
		assert!(body_str.contains('\n'));
	}

	#[rstest]
	fn test_render_html() {
		let html = "<h1>Title</h1><p>Content</p>";
		let response = render_html(html);

		assert_eq!(response.status, StatusCode::OK);
		assert_eq!(
			response
				.headers
				.get("content-type")
				.unwrap()
				.to_str()
				.unwrap(),
			"text/html; charset=utf-8"
		);

		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body_str, "<h1>Title</h1><p>Content</p>");
	}

	#[rstest]
	fn test_render_text() {
		let text = "Plain text content";
		let response = render_text(text);

		assert_eq!(response.status, StatusCode::OK);
		assert_eq!(
			response
				.headers
				.get("content-type")
				.unwrap()
				.to_str()
				.unwrap(),
			"text/plain; charset=utf-8"
		);

		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body_str, "Plain text content");
	}

	#[rstest]
	fn test_render_json_with_custom_struct() {
		#[derive(serde::Serialize)]
		struct User {
			name: String,
			age: u32,
		}

		let user = User {
			name: "Alice".to_string(),
			age: 30,
		};

		let response = render_json(&user);
		let body_str = String::from_utf8(response.body.to_vec()).unwrap();

		assert!(body_str.contains("Alice"));
		assert!(body_str.contains("30"));
	}
}
