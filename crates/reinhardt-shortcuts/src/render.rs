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
/// # Safety
///
/// This function does **not** sanitize or escape the input HTML.
/// Passing untrusted or user-supplied content directly to this function
/// can lead to **Cross-Site Scripting (XSS)** vulnerabilities.
///
/// Callers **must** ensure that any dynamic or user-provided content
/// embedded in the HTML string is properly escaped before calling this
/// function. Consider using [`render_html_safe`] instead when the HTML
/// may contain untrusted dynamic content.
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::render_html;
///
/// // Safe: static HTML with no user input
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

/// Escape HTML special characters in a string to prevent XSS attacks.
///
/// Replaces the following characters with their HTML entity equivalents:
/// - `&` -> `&amp;`
/// - `<` -> `&lt;`
/// - `>` -> `&gt;`
/// - `"` -> `&quot;`
/// - `'` -> `&#x27;`
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::escape_html;
///
/// let escaped = escape_html("<script>alert('xss')</script>");
/// assert_eq!(escaped, "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
/// ```
pub fn escape_html(input: &str) -> String {
	let mut output = String::with_capacity(input.len());
	for ch in input.chars() {
		match ch {
			'&' => output.push_str("&amp;"),
			'<' => output.push_str("&lt;"),
			'>' => output.push_str("&gt;"),
			'"' => output.push_str("&quot;"),
			'\'' => output.push_str("&#x27;"),
			_ => output.push(ch),
		}
	}
	output
}

/// Render an HTML string with dynamic content escaped for XSS safety.
///
/// This is the safe alternative to [`render_html`]. It escapes all HTML
/// special characters in the provided content before embedding it into
/// a response, preventing Cross-Site Scripting (XSS) attacks.
///
/// Use this function when the HTML content may include untrusted or
/// user-supplied data.
///
/// # Examples
///
/// ```
/// use reinhardt_shortcuts::render_html_safe;
///
/// // User input is automatically escaped
/// let user_input = "<script>alert('xss')</script>";
/// let response = render_html_safe(user_input);
///
/// let body = String::from_utf8(response.body.to_vec()).unwrap();
/// assert!(!body.contains("<script>"));
/// assert!(body.contains("&lt;script&gt;"));
/// ```
///
/// # Arguments
///
/// * `content` - The content to escape and render as HTML
///
/// # Returns
///
/// A `Response` with HTTP 200 status, HTML content-type, and the escaped content as body.
pub fn render_html_safe(content: impl AsRef<str>) -> Response {
	let escaped = escape_html(content.as_ref());

	let mut response = Response::ok();
	response.body = Bytes::from(escaped);
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
		// Arrange
		let data = json!({"name": "test"});

		// Act
		let response = render_json_pretty(&data);

		// Assert
		assert_eq!(response.status, StatusCode::OK);
		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		assert!(body_str.contains('\n'));
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
		let response = render_json(&user);

		// Assert
		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		assert!(body_str.contains("Alice"));
		assert!(body_str.contains("30"));
	}

	#[rstest]
	fn test_escape_html_special_characters() {
		// Act & Assert
		assert_eq!(escape_html("&"), "&amp;");
		assert_eq!(escape_html("<"), "&lt;");
		assert_eq!(escape_html(">"), "&gt;");
		assert_eq!(escape_html("\""), "&quot;");
		assert_eq!(escape_html("'"), "&#x27;");
	}

	#[rstest]
	fn test_escape_html_script_tag() {
		// Arrange
		let malicious = "<script>alert('xss')</script>";

		// Act
		let escaped = escape_html(malicious);

		// Assert
		assert_eq!(
			escaped,
			"&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"
		);
	}

	#[rstest]
	fn test_escape_html_preserves_safe_text() {
		// Arrange
		let safe = "Hello, World! 123";

		// Act
		let escaped = escape_html(safe);

		// Assert
		assert_eq!(escaped, "Hello, World! 123");
	}

	#[rstest]
	fn test_escape_html_empty_string() {
		// Act & Assert
		assert_eq!(escape_html(""), "");
	}

	#[rstest]
	fn test_escape_html_mixed_content() {
		// Arrange
		let input = "Name: <b>\"O'Brien\"</b> & sons";

		// Act
		let escaped = escape_html(input);

		// Assert
		assert_eq!(
			escaped,
			"Name: &lt;b&gt;&quot;O&#x27;Brien&quot;&lt;/b&gt; &amp; sons"
		);
	}

	#[rstest]
	fn test_render_html_safe_escapes_xss() {
		// Arrange
		let user_input = "<script>alert('xss')</script>";

		// Act
		let response = render_html_safe(user_input);

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
		assert!(!body_str.contains("<script>"));
		assert!(body_str.contains("&lt;script&gt;"));
	}

	#[rstest]
	fn test_render_html_safe_preserves_plain_text() {
		// Arrange
		let text = "Hello, World!";

		// Act
		let response = render_html_safe(text);

		// Assert
		let body_str = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body_str, "Hello, World!");
	}
}
