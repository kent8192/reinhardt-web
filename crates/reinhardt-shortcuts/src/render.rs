//! Template rendering shortcut functions
//!
//! Provides convenient functions for rendering templates and creating HTTP responses.

use bytes::Bytes;
use reinhardt_http::Response;
use serde::Serialize;

/// Render data as JSON and return an HTTP 200 response, or an error if serialization fails
///
/// This is a convenient shortcut for creating JSON responses without needing
/// to manually create Response objects and set headers.
///
/// Returns an error instead of partial output if serialization fails, ensuring
/// the caller always receives either a fully valid response or a clear error.
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
/// let response = render_json(&data).unwrap();
/// ```
///
/// # Arguments
///
/// * `data` - The data to serialize as JSON
///
/// # Returns
///
/// Either a `Response` with HTTP 200 status, JSON content-type, and the serialized data as body,
/// or a `serde_json::Error` if serialization fails.
///
/// # Errors
///
/// Returns `Err(serde_json::Error)` if the data cannot be serialized to JSON.
pub fn render_json<T: Serialize>(data: &T) -> Result<Response, serde_json::Error> {
	let json_string = serde_json::to_string(data)?;

	let mut response = Response::ok();
	response.body = Bytes::from(json_string);
	response
		.headers
		.insert("content-type", "application/json".parse().unwrap());

	Ok(response)
}

/// Render data as JSON with pretty printing and return an HTTP 200 response, or an error if serialization fails
///
/// Same as `render_json` but with pretty-printed output (indented).
///
/// Returns an error instead of partial output if serialization fails, ensuring
/// the caller always receives either a fully valid response or a clear error.
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::render_json_pretty;
/// use serde_json::json;
///
/// let data = json!({"key": "value"});
/// let response = render_json_pretty(&data).unwrap();
/// ```
///
/// # Errors
///
/// Returns `Err(serde_json::Error)` if the data cannot be serialized to JSON.
pub fn render_json_pretty<T: Serialize>(data: &T) -> Result<Response, serde_json::Error> {
	let json_string = serde_json::to_string_pretty(data)?;

	let mut response = Response::ok();
	response.body = Bytes::from(json_string);
	response
		.headers
		.insert("content-type", "application/json".parse().unwrap());

	Ok(response)
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
		// Arrange
		let data = json!({"name": "test", "value": 123});

		// Act
		let response = render_json(&data).expect("render_json should succeed for valid data");

		// Assert
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
		// Arrange
		let data = json!({"name": "test"});

		// Act
		let response =
			render_json_pretty(&data).expect("render_json_pretty should succeed for valid data");

		// Assert
		assert_eq!(response.status, StatusCode::OK);
		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		// Pretty printed JSON should have newlines
		assert!(body_str.contains('\n'));
	}

	#[rstest]
	fn test_render_json_returns_err_on_unserializable_value() {
		// Arrange - serde_json::Value::Number with f64::INFINITY cannot be serialized
		let data = serde_json::Value::Number(
			serde_json::Number::from_f64(f64::INFINITY).unwrap_or(serde_json::Number::from(0)),
		);
		// f64::INFINITY cannot be represented in JSON; fall back to a known-unserializable type
		// by using a custom struct that fails serialization
		struct AlwaysFails;
		impl serde::Serialize for AlwaysFails {
			fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
				Err(serde::ser::Error::custom("intentional failure"))
			}
		}
		let bad_data = AlwaysFails;

		// Act
		let result = render_json(&bad_data);

		// Assert - error is returned, no partial output is produced
		assert!(
			result.is_err(),
			"render_json must return Err for unserializable data, not partial output"
		);
		// Suppress unused variable warning
		let _ = data;
	}

	#[rstest]
	fn test_render_json_pretty_returns_err_on_unserializable_value() {
		// Arrange
		struct AlwaysFails;
		impl serde::Serialize for AlwaysFails {
			fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
				Err(serde::ser::Error::custom("intentional failure"))
			}
		}
		let bad_data = AlwaysFails;

		// Act
		let result = render_json_pretty(&bad_data);

		// Assert - error is returned, no partial output is produced
		assert!(
			result.is_err(),
			"render_json_pretty must return Err for unserializable data, not partial output"
		);
	}

	#[rstest]
	fn test_render_html() {
		// Arrange
		let html = "<h1>Title</h1><p>Content</p>";

		// Act
		let response = render_html(html);

		// Assert
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
		// Arrange
		let text = "Plain text content";

		// Act
		let response = render_text(text);

		// Assert
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
		// Arrange
		#[derive(serde::Serialize)]
		struct User {
			name: String,
			age: u32,
		}
		let user = User {
			name: "Alice".to_string(),
			age: 30,
		};

		// Act
		let response = render_json(&user).expect("render_json should succeed for User struct");
		let body_str = String::from_utf8(response.body.to_vec()).unwrap();

		// Assert
		assert!(body_str.contains("Alice"));
		assert!(body_str.contains("30"));
	}
}
