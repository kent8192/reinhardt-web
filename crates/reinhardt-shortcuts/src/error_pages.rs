//! Custom error page rendering
//!
//! This module provides Django-style custom error page rendering with automatic
//! template selection based on HTTP status codes (404.html, 500.html, etc.).

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

    tera.render("error_page.html", &context)
        .unwrap_or_else(|e| {
            // Fallback in case template cannot be rendered
            eprintln!("Warning: Failed to render error_page.html template: {}", e);
            eprintln!("Using fallback HTML");
            format!(
                "<!DOCTYPE html><html><head><title>{} - {}</title></head><body><h1>{}</h1><p>{}</p><p>Path: {}</p></body></html>",
                status_code, title, title, message, path
            )
        })
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
        assert!(body.contains("404"));
        assert!(body.contains("Not Found"));
        // Path is HTML-escaped by Tera (/ becomes &#x2F;)
        assert!(body.contains("&#x2F;test&#x2F;path") || body.contains("/test/path"));
    }

    #[test]
    fn test_render_default_500_page() {
        let request = create_test_request();
        let response = server_error::<String, String>(&request, None);

        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);

        let body = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body.contains("500"));
        assert!(body.contains("Internal Server Error"));
    }

    #[test]
    fn test_render_default_403_page() {
        let request = create_test_request();
        let response = permission_denied::<String, String>(&request, None);

        assert_eq!(response.status, StatusCode::FORBIDDEN);

        let body = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body.contains("403"));
        assert!(body.contains("Forbidden"));
    }

    #[test]
    fn test_render_default_400_page() {
        let request = create_test_request();
        let response = bad_request::<String, String>(&request, None);

        assert_eq!(response.status, StatusCode::BAD_REQUEST);

        let body = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body.contains("400"));
        assert!(body.contains("Bad Request"));
    }

    #[test]
    fn test_render_error_page_with_context() {
        let request = create_test_request();
        let mut context = HashMap::new();
        context.insert("custom_message", serde_json::json!("Test error message"));

        let response = render_error_page(&request, 404, Some(context));

        assert_eq!(response.status, StatusCode::NOT_FOUND);

        let body = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body.contains("404"));
    }

    #[test]
    fn test_render_unknown_status_code() {
        let request = create_test_request();
        let response = render_error_page::<String, String>(&request, 999, None);

        // Unknown status codes should still render a page
        let body = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body.contains("999"));
        assert!(body.contains("Error"));
    }

    #[test]
    fn test_default_error_page_structure() {
        let html = render_default_error_page(404, "/test/path");

        // Check HTML structure
        assert!(html.contains("<!DOCTYPE html>") || html.contains("<!doctype html>"));
        assert!(html.contains("<html") && html.contains("lang=\"en\""));
        assert!(html.contains("404"));
        assert!(html.contains("Not Found"));
        // Path is HTML-escaped by Tera (/ becomes &#x2F;)
        assert!(html.contains("&#x2F;test&#x2F;path") || html.contains("/test/path"));
        assert!(html.contains("Go to Home"));
    }

    #[test]
    fn test_different_error_codes() {
        let codes = vec![400, 401, 403, 404, 405, 500, 502, 503];

        for code in codes {
            let html = render_default_error_page(code, "/test");
            assert!(html.contains(&code.to_string()));
            // Path is HTML-escaped by Tera (/ becomes &#x2F;)
            assert!(html.contains("&#x2F;test") || html.contains("/test"));
        }
    }
}
