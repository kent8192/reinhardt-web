#![cfg(feature = "templates")]

use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_http::Request;
use reinhardt_shortcuts::error_pages::{
    bad_request, page_not_found, permission_denied, render_error_page, server_error,
};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

fn create_test_request(path: &str) -> Request {
    Request::new(
        Method::GET,
        Uri::try_from(path).unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    )
}

fn setup_custom_error_templates() -> PathBuf {
    let test_dir = PathBuf::from("/tmp/reinhardt_error_templates");

    // Clean up any existing test directory
    if test_dir.exists() {
        fs::remove_dir_all(&test_dir).unwrap();
    }

    // Create fresh test directory
    fs::create_dir_all(&test_dir).unwrap();

    // Create custom 404 template
    fs::write(
        test_dir.join("404.html"),
        r#"<!DOCTYPE html>
<html>
<head><title>Custom 404</title></head>
<body>
    <h1>Custom 404 Page</h1>
    <p>Path: {{ request_path }}</p>
    {% if custom_message %}
    <p>Message: {{ custom_message }}</p>
    {% endif %}
</body>
</html>"#,
    )
    .unwrap();

    // Create custom 500 template
    fs::write(
        test_dir.join("500.html"),
        r#"<!DOCTYPE html>
<html>
<head><title>Custom 500</title></head>
<body>
    <h1>Custom Server Error</h1>
    <p>Status: {{ status_code }}</p>
    {% if debug_info %}
    <pre>{{ debug_info }}</pre>
    {% endif %}
</body>
</html>"#,
    )
    .unwrap();

    test_dir
}

fn cleanup_test_templates(test_dir: &PathBuf) {
    if test_dir.exists() {
        fs::remove_dir_all(test_dir).unwrap();
    }
}

#[test]
fn test_page_not_found_default() {
    let request = create_test_request("/missing/page");
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
    assert!(body.contains("&#x2F;missing&#x2F;page") || body.contains("/missing/page"));
    assert!(body.contains("<!DOCTYPE html>") || body.contains("<!doctype html>"));
}

#[test]
fn test_server_error_default() {
    let request = create_test_request("/api/endpoint");
    let response = server_error::<String, String>(&request, None);

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);

    let body = String::from_utf8(response.body.to_vec()).unwrap();
    assert!(body.contains("500"));
    assert!(body.contains("Internal Server Error"));
    // Path is HTML-escaped by Tera (/ becomes &#x2F;)
    assert!(body.contains("&#x2F;api&#x2F;endpoint") || body.contains("/api/endpoint"));
}

#[test]
fn test_permission_denied_default() {
    let request = create_test_request("/admin/secret");
    let response = permission_denied::<String, String>(&request, None);

    assert_eq!(response.status, StatusCode::FORBIDDEN);

    let body = String::from_utf8(response.body.to_vec()).unwrap();
    assert!(body.contains("403"));
    assert!(body.contains("Forbidden"));
    // Path is HTML-escaped by Tera (/ becomes &#x2F;)
    assert!(body.contains("&#x2F;admin&#x2F;secret") || body.contains("/admin/secret"));
}

#[test]
fn test_bad_request_default() {
    let request = create_test_request("/form/submit");
    let response = bad_request::<String, String>(&request, None);

    assert_eq!(response.status, StatusCode::BAD_REQUEST);

    let body = String::from_utf8(response.body.to_vec()).unwrap();
    assert!(body.contains("400"));
    assert!(body.contains("Bad Request"));
}

#[test]
fn test_render_error_page_with_custom_context() {
    let request = create_test_request("/test/path");
    let mut context = HashMap::new();
    context.insert("error_detail", serde_json::json!("Something went wrong"));
    context.insert("request_id", serde_json::json!("12345"));

    let response = render_error_page(&request, 404, Some(context));

    assert_eq!(response.status, StatusCode::NOT_FOUND);

    let body = String::from_utf8(response.body.to_vec()).unwrap();
    assert!(body.contains("404"));
    // Path is HTML-escaped by Tera (/ becomes &#x2F;)
    assert!(body.contains("&#x2F;test&#x2F;path") || body.contains("/test/path"));
}

#[test]
#[ignore = "Tera engine is initialized as global singleton, cannot change template directory at runtime"]
fn test_custom_404_template() {
    let test_dir = setup_custom_error_templates();

    unsafe {
        env::set_var("REINHARDT_TEMPLATE_DIR", test_dir.to_str().unwrap());
    }

    let request = create_test_request("/custom/missing");
    let mut context = HashMap::new();
    context.insert(
        "custom_message",
        serde_json::json!("This is a custom message"),
    );

    let response = render_error_page(&request, 404, Some(context));

    assert_eq!(response.status, StatusCode::NOT_FOUND);

    let body = String::from_utf8(response.body.to_vec()).unwrap();
    assert!(body.contains("Custom 404 Page"));
    assert!(body.contains("/custom/missing"));
    assert!(body.contains("This is a custom message"));

    cleanup_test_templates(&test_dir);
    unsafe {
        env::remove_var("REINHARDT_TEMPLATE_DIR");
    }
}

#[test]
#[ignore = "Tera engine is initialized as global singleton, cannot change template directory at runtime"]
fn test_custom_500_template() {
    let test_dir = setup_custom_error_templates();

    unsafe {
        env::set_var("REINHARDT_TEMPLATE_DIR", test_dir.to_str().unwrap());
    }

    let request = create_test_request("/api/crash");
    let mut context = HashMap::new();
    context.insert(
        "debug_info",
        serde_json::json!("Stack trace: line 42 in module.rs"),
    );

    let response = render_error_page(&request, 500, Some(context));

    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);

    let body = String::from_utf8(response.body.to_vec()).unwrap();
    assert!(body.contains("Custom Server Error"));
    assert!(body.contains("Stack trace"));

    cleanup_test_templates(&test_dir);
    unsafe {
        env::remove_var("REINHARDT_TEMPLATE_DIR");
    }
}

#[test]
#[ignore = "Tera engine is initialized as global singleton, cannot change template directory at runtime"]
fn test_fallback_to_default_when_custom_template_missing() {
    let test_dir = setup_custom_error_templates();

    unsafe {
        env::set_var("REINHARDT_TEMPLATE_DIR", test_dir.to_str().unwrap());
    }

    let request = create_test_request("/forbidden/resource");
    // 403.html doesn't exist, should fallback to default
    let response = permission_denied::<String, String>(&request, None);

    assert_eq!(response.status, StatusCode::FORBIDDEN);

    let body = String::from_utf8(response.body.to_vec()).unwrap();
    // Should be default error page
    assert!(body.contains("403"));
    assert!(body.contains("Forbidden"));
    assert!(!body.contains("Custom")); // Not the custom template

    cleanup_test_templates(&test_dir);
    unsafe {
        env::remove_var("REINHARDT_TEMPLATE_DIR");
    }
}

#[test]
fn test_multiple_error_types() {
    let request = create_test_request("/test");

    let errors = vec![
        (400, StatusCode::BAD_REQUEST),
        (401, StatusCode::UNAUTHORIZED),
        (403, StatusCode::FORBIDDEN),
        (404, StatusCode::NOT_FOUND),
        (500, StatusCode::INTERNAL_SERVER_ERROR),
        (502, StatusCode::BAD_GATEWAY),
        (503, StatusCode::SERVICE_UNAVAILABLE),
    ];

    for (code, expected_status) in errors {
        let response = render_error_page::<String, String>(&request, code, None);
        assert_eq!(response.status, expected_status);

        let body = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body.contains(&code.to_string()));
    }
}

#[test]
fn test_error_page_html_structure() {
    let request = create_test_request("/test");
    let response = page_not_found::<String, String>(&request, None);

    let body = String::from_utf8(response.body.to_vec()).unwrap();

    // Check for proper HTML structure
    assert!(body.contains("<!DOCTYPE html>") || body.contains("<!doctype html>"));
    assert!(body.contains("<html lang=\"en\">"));
    assert!(body.contains("<head>"));
    // Meta tag can be self-closing: <meta charset="UTF-8" /> or <meta charset="UTF-8">
    assert!(body.contains("charset=\"UTF-8\""));
    assert!(body.contains("<meta name=\"viewport\""));
    assert!(body.contains("</head>"));
    assert!(body.contains("<body>"));
    assert!(body.contains("</body>"));
    assert!(body.contains("</html>"));
}

#[test]
fn test_error_page_responsive_design() {
    let request = create_test_request("/test");
    let response = page_not_found::<String, String>(&request, None);

    let body = String::from_utf8(response.body.to_vec()).unwrap();

    // Check for mobile responsiveness
    assert!(body.contains("viewport"));
    assert!(body.contains("width=device-width"));
}

#[test]
fn test_error_page_accessibility() {
    let request = create_test_request("/test");
    let response = page_not_found::<String, String>(&request, None);

    let body = String::from_utf8(response.body.to_vec()).unwrap();

    // Check for accessibility features
    assert!(body.contains("lang=\"en\""));
    assert!(body.contains("<title>"));
}
