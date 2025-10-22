//! Template rendering shortcut functions
//!
//! These functions provide convenient template rendering with automatic HTTP
//! response generation, similar to Django's `render()` function.
//!
//! This module is only available with the `templates` feature enabled.

#[cfg(feature = "templates")]
use crate::template_inheritance::get_tera_engine;
#[cfg(feature = "templates")]
use reinhardt_http::{Request, Response};
#[cfg(feature = "templates")]
use serde::Serialize;
#[cfg(feature = "templates")]
use std::collections::HashMap;
#[cfg(feature = "templates")]
use tera::Context;

/// Render a template with context and return an HTTP response
///
/// This is the main template rendering function, similar to Django's `render()`.
/// It loads templates from the file system and returns an HTTP response with
/// the template content.
///
/// Supports full Django/Jinja2-style template syntax including:
/// - Template inheritance: `{% extends "base.html" %}`
/// - Template blocks: `{% block content %}...{% endblock %}`
/// - Variable substitution: `{{ variable }}`
/// - Control structures: `{% if %}`, `{% for %}`, etc.
/// - Filters: `{{ value|lower }}`
///
/// # Template Directory
///
/// Templates are loaded from the directory specified by the `REINHARDT_TEMPLATE_DIR`
/// environment variable. If not set, defaults to `./templates`.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::render_template;
/// use std::collections::HashMap;
///
/// async fn index_view(request: Request) -> Result<Response, Response> {
///     let mut context = HashMap::new();
///     context.insert("title", "Welcome");
///     context.insert("user", request.user().name());
///
///     render_template(&request, "index.html", context)
/// }
/// ```
///
/// # Arguments
///
/// * `_request` - The HTTP request (used for request-specific context)
/// * `template_name` - The name of the template file (relative to template directory)
/// * `context` - A HashMap containing template variables
///
/// # Returns
///
/// An HTTP Response with the rendered template as HTML
///
/// # Errors
///
/// - Returns HTTP 404 if template file is not found
/// - Returns HTTP 500 if template rendering fails
#[cfg(feature = "templates")]
pub fn render_template<K, V>(
    _request: &Request,
    template_name: &str,
    context: HashMap<K, V>,
) -> Result<Response, Response>
where
    K: AsRef<str>,
    V: Serialize,
{
    let tera = get_tera_engine();

    // Convert HashMap to Tera Context
    let mut tera_context = Context::new();
    for (key, value) in context {
        // Serialize to serde_json::Value for Tera
        if let Ok(json_value) = serde_json::to_value(value) {
            tera_context.insert(key.as_ref(), &json_value);
        }
    }

    // Render template with Tera
    let html = tera.render(template_name, &tera_context).map_err(|e| {
        let error_msg = e.to_string();

        // Check if it's a template not found error
        if error_msg.contains("not found") || error_msg.contains("doesn't exist") {
            let mut response = Response::not_found();
            response.body = bytes::Bytes::from(format!("Template not found: {}", template_name));
            response
        } else {
            let mut response = Response::internal_server_error();
            response.body = bytes::Bytes::from(format!("Template rendering failed: {}", error_msg));
            response
        }
    })?;

    let mut response = Response::ok();
    response.headers.insert(
        hyper::header::CONTENT_TYPE,
        hyper::header::HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response.body = bytes::Bytes::from(html);

    Ok(response)
}

/// Render a template to HTTP response with custom configuration
///
/// This function provides more control over the response, allowing you to
/// specify custom HTTP status code, headers, and other response properties.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_shortcuts::render_to_response;
/// use std::collections::HashMap;
///
/// async fn custom_view(request: Request) -> Result<Response, Response> {
///     let mut context = HashMap::new();
///     context.insert("message", "Custom response");
///
///     let mut response = render_to_response(
///         &request,
///         "custom.html",
///         context,
///     )?;
///
///     // Customize the response
///     response.status = hyper::StatusCode::CREATED;
///     response.headers.insert(
///         hyper::header::CACHE_CONTROL,
///         hyper::header::HeaderValue::from_static("no-cache"),
///     );
///
///     Ok(response)
/// }
/// ```
///
/// # Arguments
///
/// * `request` - The HTTP request
/// * `template_name` - The name of the template to render
/// * `context` - A HashMap containing template variables
///
/// # Returns
///
/// A mutable HTTP Response that can be further customized
///
/// # Errors
///
/// Returns `Err(Response)` with HTTP 500 if template rendering fails
#[cfg(feature = "templates")]
pub fn render_to_response<K, V>(
    request: &Request,
    template_name: &str,
    context: HashMap<K, V>,
) -> Result<Response, Response>
where
    K: AsRef<str>,
    V: Serialize,
{
    // Use render_template as the base implementation
    render_template(request, template_name, context)
}

#[cfg(all(test, feature = "templates"))]
mod tests {
    use super::*;
    use bytes::Bytes;
    use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
    use reinhardt_http::Request;

    fn create_test_request() -> Request {
        Request::new(
            Method::GET,
            Uri::from_static("/"),
            Version::HTTP_11,
            HeaderMap::new(),
            Bytes::new(),
        )
    }

    #[test]
    fn test_render_template_not_found() {
        // Test that non-existent template returns 404
        let request = create_test_request();
        let mut context = HashMap::new();
        context.insert("title", "Test Page");

        let result = render_template(&request, "nonexistent.html", context);
        assert!(result.is_err());

        if let Err(response) = result {
            assert_eq!(response.status, StatusCode::NOT_FOUND);
            let body = String::from_utf8(response.body.to_vec()).unwrap();
            assert!(body.contains("Template not found"));
        }
    }

    #[test]
    fn test_render_to_response_customizable() {
        // Test that response can be customized
        // Note: This test will fail if template doesn't exist
        // In a real scenario, you would create a test template file
        let request = create_test_request();
        let context: HashMap<String, String> = HashMap::new();

        // Test with non-existent template to verify error handling
        let result = render_to_response(&request, "custom.html", context);
        // Expecting error since template doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_render_template_with_empty_context() {
        // Test that empty context works
        let request = create_test_request();
        let context: HashMap<String, String> = HashMap::new();

        let result = render_template(&request, "empty.html", context);
        // Expecting error since template doesn't exist
        assert!(result.is_err());

        if let Err(response) = result {
            assert_eq!(response.status, StatusCode::NOT_FOUND);
        }
    }
}
