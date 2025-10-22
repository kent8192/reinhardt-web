//! Integration tests for template rendering
//!
//! These tests use actual template files from the tests/templates directory
//! to verify end-to-end template rendering functionality.

use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_http::Request;
use reinhardt_shortcuts::{render_template, render_to_response};
use std::collections::HashMap;
use std::env;

fn create_test_request() -> Request {
    Request::new(
        Method::GET,
        Uri::from_static("/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    )
}

fn setup_template_dir() {
    // Set the template directory to the test templates folder
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let template_dir = format!("{}/tests/templates", manifest_dir);
    unsafe {
        env::set_var("REINHARDT_TEMPLATE_DIR", template_dir);
    }
}

#[test]
fn test_render_simple_template() {
    setup_template_dir();
    let request = create_test_request();

    let mut context = HashMap::new();
    context.insert("title", "Test Page");
    context.insert("heading", "Welcome");
    context.insert("content", "This is a test page with variables.");

    let result = render_template(&request, "simple.html", context);
    assert!(result.is_ok());

    match result {
        Ok(response) => {
            assert_eq!(response.status, StatusCode::OK);
            let body = String::from_utf8(response.body.to_vec()).unwrap();
            assert!(body.contains("<title>Test Page</title>"));
            assert!(body.contains("<h1>Welcome</h1>"));
            assert!(body.contains("<p>This is a test page with variables.</p>"));
        }
        Err(_) => panic!("Expected Ok result"),
    }
}

#[test]
fn test_render_static_template() {
    setup_template_dir();
    let request = create_test_request();
    let context: HashMap<String, String> = HashMap::new();

    let result = render_template(&request, "static.html", context);
    assert!(result.is_ok());

    match result {
        Ok(response) => {
            assert_eq!(response.status, StatusCode::OK);
            let body = String::from_utf8(response.body.to_vec()).unwrap();
            assert!(body.contains("<title>Static Page</title>"));
            assert!(body.contains("This is a static template"));
            assert!(body.contains("No variables here!"));
        }
        Err(_) => panic!("Expected Ok result"),
    }
}

#[test]
fn test_render_greeting_template() {
    setup_template_dir();
    let request = create_test_request();

    let mut context = HashMap::new();
    context.insert("name", "Alice");
    context.insert("site_name", "Reinhardt Framework");

    let result = render_template(&request, "greeting.html", context);
    assert!(result.is_ok());

    match result {
        Ok(response) => {
            assert_eq!(response.status, StatusCode::OK);
            let body = String::from_utf8(response.body.to_vec()).unwrap();
            assert!(body.contains("Hello, Alice!"));
            assert!(body.contains("Welcome to Reinhardt Framework."));
        }
        Err(_) => panic!("Expected Ok result"),
    }
}

#[test]
fn test_render_template_missing_variables() {
    setup_template_dir();
    let request = create_test_request();

    let mut context = HashMap::new();
    context.insert("name", "Bob");
    // Missing site_name - Tera will error in strict mode

    let result = render_template(&request, "greeting.html", context);
    // Tera returns error for missing variables in strict mode
    assert!(result.is_err());

    if let Err(response) = result {
        assert_eq!(response.status, hyper::StatusCode::INTERNAL_SERVER_ERROR);
        let body = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body.contains("Template rendering failed"));
    }
}

#[test]
fn test_render_to_response_allows_customization() {
    setup_template_dir();
    let request = create_test_request();

    let mut context = HashMap::new();
    context.insert("title", "Custom Response");
    context.insert("heading", "Modified");
    context.insert("content", "Test content");

    let result = render_to_response(&request, "simple.html", context);
    assert!(result.is_ok());

    match result {
        Ok(mut response) => {
            // Verify we can customize the response
            response.status = StatusCode::CREATED;
            assert_eq!(response.status, StatusCode::CREATED);

            response.headers.insert(
                hyper::header::CACHE_CONTROL,
                hyper::header::HeaderValue::from_static("no-cache"),
            );

            assert_eq!(
                response.headers.get(hyper::header::CACHE_CONTROL),
                Some(&hyper::header::HeaderValue::from_static("no-cache"))
            );
        }
        Err(_) => panic!("Expected Ok result"),
    }
}

#[test]
fn test_render_nonexistent_template() {
    setup_template_dir();
    let request = create_test_request();
    let context: HashMap<String, String> = HashMap::new();

    let result = render_template(&request, "nonexistent.html", context);
    assert!(result.is_err());

    if let Err(response) = result {
        assert_eq!(response.status, StatusCode::NOT_FOUND);
        let body = String::from_utf8(response.body.to_vec()).unwrap();
        assert!(body.contains("Template not found"));
    }
}

#[test]
fn test_render_template_with_content_type() {
    setup_template_dir();
    let request = create_test_request();
    let context: HashMap<String, String> = HashMap::new();

    let result = render_template(&request, "static.html", context);
    assert!(result.is_ok());

    match result {
        Ok(response) => {
            assert_eq!(
                response.headers.get(hyper::header::CONTENT_TYPE),
                Some(&hyper::header::HeaderValue::from_static(
                    "text/html; charset=utf-8"
                ))
            );
        }
        Err(_) => panic!("Expected Ok result"),
    }
}

#[test]
fn test_render_template_with_special_characters() {
    setup_template_dir();
    let request = create_test_request();

    let mut context = HashMap::new();
    context.insert("name", "Alice & Bob");
    context.insert("site_name", "Test <Site>");

    let result = render_template(&request, "greeting.html", context);
    assert!(result.is_ok());

    match result {
        Ok(response) => {
            let body = String::from_utf8(response.body.to_vec()).unwrap();
            // Tera automatically escapes HTML by default for security
            assert!(body.contains("Alice &amp; Bob"));
            assert!(body.contains("Test &lt;Site&gt;"));
        }
        Err(_) => panic!("Expected Ok result"),
    }
}
