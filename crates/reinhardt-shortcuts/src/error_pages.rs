//! Custom error page rendering
//!
//! This module provides Django-style custom error page rendering with automatic
//! template selection based on HTTP status codes (404.html, 500.html, etc.).

#[cfg(feature = "templates")]
use crate::template_inheritance::get_tera_engine;
#[cfg(feature = "templates")]
use reinhardt_core::http::{Request, Response};
#[cfg(feature = "templates")]
use serde::Serialize;
#[cfg(feature = "templates")]
use std::backtrace::{Backtrace, BacktraceStatus};
#[cfg(feature = "templates")]
use std::collections::HashMap;
#[cfg(feature = "templates")]
use tera::Context;

/// Render a custom error page based on HTTP status code
///
/// This function attempts to render a custom error template based on the status code.
/// It follows Django's error template naming convention:
/// - 404.html for Not Found errors
/// - 500.html for Internal Server Error
/// - 403.html for Forbidden
/// - etc.
///
/// If no custom template exists, it falls back to a default error page.
///
/// # Arguments
///
/// * `request` - The HTTP request
/// * `status_code` - The HTTP status code (e.g., 404, 500)
/// * `context` - Optional context variables for the error template
///
/// # Returns
///
/// An HTTP Response with the rendered error page
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::error_pages::render_error_page;
/// use std::collections::HashMap;
///
/// async fn my_handler(request: Request) -> Result<Response, Response> {
///     let mut context = HashMap::new();
///     context.insert("error_message", "Page not found");
///
///     Err(render_error_page(&request, 404, Some(context)))
/// }
/// ```
#[cfg(feature = "templates")]
pub fn render_error_page<K, V>(
	request: &Request,
	status_code: u16,
	context: Option<HashMap<K, V>>,
) -> Response
where
	K: AsRef<str>,
	V: Serialize,
{
	let tera = get_tera_engine();
	let template_name = format!("{}.html", status_code);

	// Create base context with error information
	let mut tera_context = Context::new();
	tera_context.insert("status_code", &status_code);
	tera_context.insert("request_path", &request.uri.path());

	// Add user-provided context if any
	if let Some(user_context) = context {
		for (key, value) in user_context {
			if let Ok(json_value) = serde_json::to_value(value) {
				tera_context.insert(key.as_ref(), &json_value);
			}
		}
	}

	// Try to render custom error template
	let html = match tera.render(&template_name, &tera_context) {
		Ok(html) => html,
		Err(_) => {
			// Fallback to default error page
			render_default_error_page(status_code, request.uri.path())
		}
	};

	let status = hyper::StatusCode::from_u16(status_code)
		.unwrap_or(hyper::StatusCode::INTERNAL_SERVER_ERROR);

	let mut response = Response::new(status);
	response.headers.insert(
		hyper::header::CONTENT_TYPE,
		hyper::header::HeaderValue::from_static("text/html; charset=utf-8"),
	);
	response.body = bytes::Bytes::from(html);

	response
}

/// Render the default error page (used when no custom template exists)
#[cfg(feature = "templates")]
fn render_default_error_page(status_code: u16, path: &str) -> String {
	let (title, message) = match status_code {
		400 => (
			"Bad Request",
			"The request could not be understood by the server.",
		),
		401 => (
			"Unauthorized",
			"Authentication is required to access this resource.",
		),
		403 => (
			"Forbidden",
			"You don't have permission to access this resource.",
		),
		404 => ("Not Found", "The requested page could not be found."),
		405 => (
			"Method Not Allowed",
			"The request method is not supported for this resource.",
		),
		500 => (
			"Internal Server Error",
			"An error occurred while processing your request.",
		),
		502 => (
			"Bad Gateway",
			"The server received an invalid response from an upstream server.",
		),
		503 => (
			"Service Unavailable",
			"The server is currently unable to handle the request.",
		),
		_ => ("Error", "An error occurred while processing your request."),
	};

	// Use external template file
	let tera = get_tera_engine();
	let mut context = Context::new();
	context.insert("status_code", &status_code);
	context.insert("title", &title);
	context.insert("message", &message);
	context.insert("path", &path);

	tera.render("error_page.tpl", &context)
        .unwrap_or_else(|e| {
            // Fallback in case template cannot be rendered
            eprintln!("Warning: Failed to render error_page.tpl template: {}", e);
            eprintln!("Using fallback HTML");
            format!(
                "<!DOCTYPE html><html><head><title>{} - {}</title></head><body><h1>{}</h1><p>{}</p><p>Path: {}</p></body></html>",
                status_code, title, title, message, path
            )
        })
}

/// Render a debug error page for development environments
///
/// This function provides detailed error information including automatically
/// captured stack traces and request details, similar to Django's debug error page.
///
/// **Note**: Stack traces are automatically captured using `std::backtrace::Backtrace`.
/// For stack traces to work, you must compile with `RUST_BACKTRACE=1` or
/// `RUST_BACKTRACE=full` environment variable.
///
/// **Warning**: Only use this in development. Never expose detailed error
/// information in production as it may leak sensitive data.
///
/// # Arguments
///
/// * `request` - The HTTP request
/// * `status_code` - The HTTP status code
/// * `error_message` - Detailed error message
/// * `context` - Optional additional debug context
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::error_pages::render_debug_error_page;
/// use std::collections::HashMap;
///
/// async fn my_handler(request: Request) -> Result<Response, Response> {
///     let mut context = HashMap::new();
///     context.insert("local_vars", debug_info);
///
///     // Stack trace is automatically captured
///     Err(render_debug_error_page(
///         &request,
///         500,
///         "Database connection failed",
///         Some(context)
///     ))
/// }
/// ```
#[cfg(feature = "templates")]
pub fn render_debug_error_page<K, V>(
	request: &Request,
	status_code: u16,
	error_message: &str,
	context: Option<HashMap<K, V>>,
) -> Response
where
	K: AsRef<str>,
	V: Serialize,
{
	let tera = get_tera_engine();
	let template_name = "debug_error.html";

	// Create comprehensive debug context
	let mut tera_context = Context::new();
	tera_context.insert("status_code", &status_code);
	tera_context.insert("error_message", &error_message);
	tera_context.insert("request_path", &request.uri.path());
	tera_context.insert("request_method", &request.method.as_str());

	// Add request headers for debugging
	let headers: Vec<(String, String)> = request
		.headers
		.iter()
		.map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
		.collect();
	tera_context.insert("request_headers", &headers);

	// Add user-provided debug context
	if let Some(user_context) = context {
		for (key, value) in user_context {
			if let Ok(json_value) = serde_json::to_value(value) {
				tera_context.insert(key.as_ref(), &json_value);
			}
		}
	}

	// Capture and add stack trace
	let backtrace = Backtrace::capture();
	let stack_trace = if backtrace.status() == BacktraceStatus::Captured {
		format!("{}", backtrace)
	} else {
		"Stack trace not available (compile with RUST_BACKTRACE=1)".to_string()
	};
	tera_context.insert("stack_trace", &stack_trace);

	// Try to render debug template
	let html = match tera.render(template_name, &tera_context) {
		Ok(html) => html,
		Err(_) => {
			// Fallback to simple debug page if template doesn't exist
			render_simple_debug_page(status_code, error_message, request)
		}
	};

	let status = hyper::StatusCode::from_u16(status_code)
		.unwrap_or(hyper::StatusCode::INTERNAL_SERVER_ERROR);

	let mut response = Response::new(status);
	response.headers.insert(
		hyper::header::CONTENT_TYPE,
		hyper::header::HeaderValue::from_static("text/html; charset=utf-8"),
	);
	response.body = bytes::Bytes::from(html);

	response
}

/// Render a simple debug error page (fallback)
#[cfg(feature = "templates")]
fn render_simple_debug_page(status_code: u16, error_message: &str, request: &Request) -> String {
	let headers_html = request
		.headers
		.iter()
		.map(|(k, v)| {
			format!(
				"<tr><td><strong>{}</strong></td><td>{}</td></tr>",
				k,
				v.to_str().unwrap_or("<binary>")
			)
		})
		.collect::<Vec<_>>()
		.join("\n");

	// Capture stack trace
	let backtrace = Backtrace::capture();
	let stack_trace_html = if backtrace.status() == BacktraceStatus::Captured {
		format!("{}", backtrace)
	} else {
		"Stack trace not available (compile with RUST_BACKTRACE=1)".to_string()
	};

	format!(
		r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Debug Error: {} - {}</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            background: #1a1a1a;
            color: #e0e0e0;
            margin: 0;
            padding: 20px;
        }}
        .container {{
            max-width: 1200px;
            margin: 0 auto;
        }}
        .error-header {{
            background: #c62828;
            color: white;
            padding: 20px;
            border-radius: 8px 8px 0 0;
        }}
        .error-code {{
            font-size: 48px;
            font-weight: 700;
            margin: 0;
        }}
        .error-message {{
            font-size: 24px;
            margin: 10px 0 0 0;
        }}
        .debug-section {{
            background: #2d2d2d;
            padding: 20px;
            margin-top: 2px;
        }}
        .debug-section h2 {{
            color: #64b5f6;
            margin-top: 0;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
            margin-top: 10px;
        }}
        td {{
            padding: 8px;
            border-bottom: 1px solid #404040;
        }}
        code {{
            background: #1a1a1a;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: monospace;
        }}
        .warning {{
            background: #f57c00;
            color: white;
            padding: 15px;
            margin: 20px 0;
            border-radius: 4px;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="error-header">
            <div class="error-code">{}</div>
            <div class="error-message">{}</div>
        </div>
        <div class="warning">
            <strong>⚠️ Development Mode Debug Page</strong><br>
            This page contains sensitive information. Never enable debug mode in production.
        </div>
        <div class="debug-section">
            <h2>Request Information</h2>
            <table>
                <tr><td><strong>Method</strong></td><td>{}</td></tr>
                <tr><td><strong>Path</strong></td><td>{}</td></tr>
            </table>
        </div>
        <div class="debug-section">
            <h2>Request Headers</h2>
            <table>{}</table>
        </div>
        <div class="debug-section">
            <h2>Stack Trace</h2>
            <pre style="background: #1a1a1a; padding: 15px; border-radius: 4px; overflow-x: auto; font-size: 12px; line-height: 1.5;">{}</pre>
        </div>
    </div>
</body>
</html>"#,
		status_code,
		error_message,
		status_code,
		error_message,
		request.method.as_str(),
		request.uri.path(),
		headers_html,
		stack_trace_html
	)
}

/// Shortcut for rendering a 404 Not Found error page
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::error_pages::page_not_found;
///
/// async fn my_handler(request: Request) -> Result<Response, Response> {
///     Err(page_not_found(&request, None))
/// }
/// ```
#[cfg(feature = "templates")]
pub fn page_not_found<K, V>(request: &Request, context: Option<HashMap<K, V>>) -> Response
where
	K: AsRef<str>,
	V: Serialize,
{
	render_error_page(request, 404, context)
}

/// Shortcut for rendering a 500 Internal Server Error page
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::error_pages::server_error;
///
/// async fn my_handler(request: Request) -> Result<Response, Response> {
///     let mut context = HashMap::new();
///     context.insert("debug_info", "Database connection failed");
///
///     Err(server_error(&request, Some(context)))
/// }
/// ```
#[cfg(feature = "templates")]
pub fn server_error<K, V>(request: &Request, context: Option<HashMap<K, V>>) -> Response
where
	K: AsRef<str>,
	V: Serialize,
{
	render_error_page(request, 500, context)
}

/// Shortcut for rendering a 403 Forbidden error page
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::error_pages::permission_denied;
///
/// async fn my_handler(request: Request) -> Result<Response, Response> {
///     Err(permission_denied(&request, None))
/// }
/// ```
#[cfg(feature = "templates")]
pub fn permission_denied<K, V>(request: &Request, context: Option<HashMap<K, V>>) -> Response
where
	K: AsRef<str>,
	V: Serialize,
{
	render_error_page(request, 403, context)
}

/// Shortcut for rendering a 400 Bad Request error page
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::error_pages::bad_request;
///
/// async fn my_handler(request: Request) -> Result<Response, Response> {
///     let mut context = HashMap::new();
///     context.insert("error", "Invalid form data");
///
///     Err(bad_request(&request, Some(context)))
/// }
/// ```
#[cfg(feature = "templates")]
pub fn bad_request<K, V>(request: &Request, context: Option<HashMap<K, V>>) -> Response
where
	K: AsRef<str>,
	V: Serialize,
{
	render_error_page(request, 400, context)
}

#[cfg(all(test, feature = "templates"))]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, StatusCode, Uri, Version};

	fn create_test_request() -> Request {
		Request::new(
			Method::GET,
			Uri::from_static("/test/path"),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		)
	}

	#[test]
	fn test_render_default_404_page() {
		let request = create_test_request();
		let response = page_not_found::<String, String>(&request, None);

		assert_eq!(response.status, StatusCode::NOT_FOUND);
		assert_eq!(
			response.headers.get(hyper::header::CONTENT_TYPE),
			Some(&hyper::header::HeaderValue::from_static(
				"text/html; charset=utf-8"
			))
		);

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		// Verify HTML structure and content
		assert_eq!(body.matches("404").count() >= 2, true); // Status code appears at least twice
		assert_eq!(body.matches("Not Found").count() >= 1, true);
		// Path is HTML-escaped by Tera (/ becomes &#x2F;)
		assert_eq!(
			body.contains("&#x2F;test&#x2F;path") || body.contains("/test/path"),
			true
		);
	}

	#[test]
	fn test_render_default_500_page() {
		let request = create_test_request();
		let response = server_error::<String, String>(&request, None);

		assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body.matches("500").count() >= 2, true);
		assert_eq!(body.matches("Internal Server Error").count() >= 1, true);
	}

	#[test]
	fn test_render_default_403_page() {
		let request = create_test_request();
		let response = permission_denied::<String, String>(&request, None);

		assert_eq!(response.status, StatusCode::FORBIDDEN);

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body.matches("403").count() >= 2, true);
		assert_eq!(body.matches("Forbidden").count() >= 1, true);
	}

	#[test]
	fn test_render_default_400_page() {
		let request = create_test_request();
		let response = bad_request::<String, String>(&request, None);

		assert_eq!(response.status, StatusCode::BAD_REQUEST);

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body.matches("400").count() >= 2, true);
		assert_eq!(body.matches("Bad Request").count() >= 1, true);
	}

	#[test]
	fn test_render_error_page_with_context() {
		let request = create_test_request();
		let mut context = HashMap::new();
		context.insert("custom_message", serde_json::json!("Test error message"));

		let response = render_error_page(&request, 404, Some(context));

		assert_eq!(response.status, StatusCode::NOT_FOUND);

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body.matches("404").count() >= 2, true);
	}

	#[test]
	fn test_render_unknown_status_code() {
		let request = create_test_request();
		let response = render_error_page::<String, String>(&request, 999, None);

		// Unknown status codes should still render a page
		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body.matches("999").count() >= 2, true);
		assert_eq!(body.matches("Error").count() >= 1, true);
	}

	#[test]
	fn test_default_error_page_structure() {
		let html = render_default_error_page(404, "/test/path");

		// Check HTML structure
		let has_doctype = html.contains("<!DOCTYPE html>") || html.contains("<!doctype html>");
		assert_eq!(has_doctype, true);
		assert_eq!(html.contains("<html") && html.contains("lang=\"en\""), true);
		assert_eq!(html.matches("404").count() >= 2, true);
		assert_eq!(html.matches("Not Found").count() >= 1, true);
		// Path is HTML-escaped by Tera (/ becomes &#x2F;)
		assert_eq!(
			html.contains("&#x2F;test&#x2F;path") || html.contains("/test/path"),
			true
		);
		assert_eq!(html.matches("Go to Home").count() >= 1, true);
	}

	#[test]
	fn test_different_error_codes() {
		let codes = vec![400, 401, 403, 404, 405, 500, 502, 503];

		for code in codes {
			let html = render_default_error_page(code, "/test");
			assert_eq!(html.matches(&code.to_string()).count() >= 2, true);
			// Path is HTML-escaped by Tera (/ becomes &#x2F;)
			assert_eq!(html.contains("&#x2F;test") || html.contains("/test"), true);
		}
	}

	#[test]
	fn test_render_debug_error_page() {
		let request = create_test_request();
		let response = render_debug_error_page::<String, String>(
			&request,
			500,
			"Database connection failed",
			None,
		);

		assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body.matches("500").count() >= 2, true);
		assert_eq!(
			body.matches("Database connection failed").count() >= 1,
			true
		);
		assert_eq!(
			body.matches("Development Mode Debug Page").count() >= 1,
			true
		);
	}

	#[test]
	fn test_render_debug_error_page_with_context() {
		let request = create_test_request();
		let mut context = HashMap::new();
		context.insert("stack_trace", serde_json::json!("Error at line 42"));

		let response = render_debug_error_page(&request, 500, "Critical error", Some(context));

		assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);

		let body = String::from_utf8(response.body.to_vec()).unwrap();
		assert_eq!(body.matches("Critical error").count() >= 1, true);
	}

	#[test]
	fn test_debug_page_includes_request_details() {
		let request = create_test_request();
		let html = render_simple_debug_page(404, "Not found", &request);

		// Should include method and path
		assert_eq!(html.matches("GET").count() >= 1, true);
		assert_eq!(
			html.contains("/test/path") || html.contains("&#x2F;test&#x2F;path"),
			true
		);
		assert_eq!(html.matches("Request Information").count() >= 1, true);
		assert_eq!(html.matches("Request Headers").count() >= 1, true);
	}
}
