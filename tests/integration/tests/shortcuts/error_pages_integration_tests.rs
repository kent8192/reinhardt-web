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
	// Verify exact HTML structure for 404 error page
	assert!(
		body.contains("<title>404 - Not Found</title>"),
		"Error page should have exact title with format '404 - Not Found', got: {}",
		&body[..300.min(body.len())]
	);
	assert!(
		body.contains("<div class=\"error-code\">404</div>"),
		"Error page should have error-code div with status code, got: {}",
		&body[..1500.min(body.len())]
	);
	assert!(
		body.contains("<div class=\"error-title\">Not Found</div>"),
		"Error page should have error-title div with status text, got: {}",
		&body[..1500.min(body.len())]
	);
	// Path is HTML-escaped by Tera (/ becomes &#x2F;)
	assert!(
		body.contains("Path: &#x2F;missing&#x2F;page") || body.contains("Path: /missing/page"),
		"Error page should contain request path in error-path div, got: {}",
		&body[..2000.min(body.len())]
	);
	assert!(
		body.contains("<!DOCTYPE html>") || body.contains("<!doctype html>"),
		"Error page should have DOCTYPE declaration, got: {}",
		&body[..100.min(body.len())]
	);
}

#[test]
fn test_server_error_default() {
	let request = create_test_request("/api/endpoint");
	let response = server_error::<String, String>(&request, None);

	assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);

	let body = String::from_utf8(response.body.to_vec()).unwrap();
	// Verify exact HTML structure for 500 error page
	assert!(
		body.contains("<title>500 - Internal Server Error</title>"),
		"Error page should have exact title with format '500 - Internal Server Error', got: {}",
		&body[..300.min(body.len())]
	);
	assert!(
		body.contains("<div class=\"error-code\">500</div>"),
		"Error page should have error-code div with status code, got: {}",
		&body[..1500.min(body.len())]
	);
	assert!(
		body.contains("<div class=\"error-title\">Internal Server Error</div>"),
		"Error page should have error-title div with status text, got: {}",
		&body[..1500.min(body.len())]
	);
	// Path is HTML-escaped by Tera (/ becomes &#x2F;)
	assert!(
		body.contains("Path: &#x2F;api&#x2F;endpoint") || body.contains("Path: /api/endpoint"),
		"Error page should contain request path in error-path div, got: {}",
		&body[..2000.min(body.len())]
	);
}

#[test]
fn test_permission_denied_default() {
	let request = create_test_request("/admin/secret");
	let response = permission_denied::<String, String>(&request, None);

	assert_eq!(response.status, StatusCode::FORBIDDEN);

	let body = String::from_utf8(response.body.to_vec()).unwrap();
	// Verify exact HTML structure for 403 error page
	assert!(
		body.contains("<title>403 - Forbidden</title>"),
		"Error page should have exact title with format '403 - Forbidden', got: {}",
		&body[..300.min(body.len())]
	);
	assert!(
		body.contains("<div class=\"error-code\">403</div>"),
		"Error page should have error-code div with status code, got: {}",
		&body[..1500.min(body.len())]
	);
	assert!(
		body.contains("<div class=\"error-title\">Forbidden</div>"),
		"Error page should have error-title div with status text, got: {}",
		&body[..1500.min(body.len())]
	);
	// Path is HTML-escaped by Tera (/ becomes &#x2F;)
	assert!(
		body.contains("Path: &#x2F;admin&#x2F;secret") || body.contains("Path: /admin/secret"),
		"Error page should contain request path in error-path div, got: {}",
		&body[..2000.min(body.len())]
	);
}

#[test]
fn test_bad_request_default() {
	let request = create_test_request("/form/submit");
	let response = bad_request::<String, String>(&request, None);

	assert_eq!(response.status, StatusCode::BAD_REQUEST);

	let body = String::from_utf8(response.body.to_vec()).unwrap();
	// Verify exact HTML structure for 400 error page
	assert!(
		body.contains("<title>400 - Bad Request</title>"),
		"Error page should have exact title with format '400 - Bad Request', got: {}",
		&body[..300.min(body.len())]
	);
	assert!(
		body.contains("<div class=\"error-code\">400</div>"),
		"Error page should have error-code div with status code, got: {}",
		&body[..1500.min(body.len())]
	);
	assert!(
		body.contains("<div class=\"error-title\">Bad Request</div>"),
		"Error page should have error-title div with status text, got: {}",
		&body[..1500.min(body.len())]
	);
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
	// Verify custom context is rendered in error page
	assert!(
		body.contains("<title>404 - Not Found</title>"),
		"Error page with custom context should have exact title with format '404 - Not Found', got: {}",
		&body[..300.min(body.len())]
	);
	assert!(
		body.contains("<div class=\"error-code\">404</div>"),
		"Error page should have error-code div with status code, got: {}",
		&body[..1500.min(body.len())]
	);
	// Path is HTML-escaped by Tera (/ becomes &#x2F;)
	assert!(
		body.contains("Path: &#x2F;test&#x2F;path") || body.contains("Path: /test/path"),
		"Error page should contain request path in error-path div, got: {}",
		&body[..2000.min(body.len())]
	);
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
	// Verify custom template elements
	assert!(
		body.contains("<title>Custom 404</title>"),
		"Custom template should have exact title, got: {}",
		&body[..200.min(body.len())]
	);
	assert!(
		body.contains("<h1>Custom 404 Page</h1>"),
		"Custom template should have exact h1, got: {}",
		&body[..500.min(body.len())]
	);
	assert!(
		body.contains("<p>Path: /custom/missing</p>"),
		"Custom template should have path paragraph, got: {}",
		&body[..800.min(body.len())]
	);
	assert!(
		body.contains("<p>Message: This is a custom message</p>"),
		"Custom template should have custom message, got: {}",
		&body[..800.min(body.len())]
	);

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
	// Verify custom 500 template elements
	assert!(
		body.contains("<title>Custom 500</title>"),
		"Custom 500 template should have exact title, got: {}",
		&body[..200.min(body.len())]
	);
	assert!(
		body.contains("<h1>Custom Server Error</h1>"),
		"Custom 500 template should have exact h1, got: {}",
		&body[..500.min(body.len())]
	);
	assert!(
		body.contains("<pre>Stack trace: line 42 in module.rs</pre>"),
		"Custom 500 template should have debug info in pre tag, got: {}",
		&body[..800.min(body.len())]
	);

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
	// Should be default error page, not custom template
	assert!(
		body.contains("<title>403 - Forbidden</title>"),
		"Default 403 page should have exact title with format '403 - Forbidden', got: {}",
		&body[..300.min(body.len())]
	);
	assert!(
		body.contains("<div class=\"error-code\">403</div>"),
		"Default 403 page should have error-code div with status code, got: {}",
		&body[..1500.min(body.len())]
	);
	assert!(
		body.contains("<div class=\"error-title\">Forbidden</div>"),
		"Default 403 page should have error-title div with status text, got: {}",
		&body[..1500.min(body.len())]
	);
	assert!(
		!body.contains("Custom"),
		"Should not contain custom template marker, got: {}",
		&body[..1500.min(body.len())]
	);

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
		// Verify error page contains error-code div with status code
		let expected_div = format!("<div class=\"error-code\">{}</div>", code);
		assert!(
			body.contains(&expected_div),
			"Error page for {} should have error-code div with status code, got: {}",
			code,
			&body[..1500.min(body.len())]
		);
	}
}

#[test]
fn test_error_page_html_structure() {
	let request = create_test_request("/test");
	let response = page_not_found::<String, String>(&request, None);

	let body = String::from_utf8(response.body.to_vec()).unwrap();

	// Check for proper HTML structure elements
	assert!(
		body.contains("<!DOCTYPE html>") || body.contains("<!doctype html>"),
		"HTML should start with DOCTYPE, got: {}",
		&body[..100.min(body.len())]
	);
	assert!(
		body.contains("<html lang=\"en\">"),
		"HTML element should have lang attribute, got: {}",
		&body[..300.min(body.len())]
	);
	assert!(
		body.contains("<head>"),
		"HTML should have head element, got: {}",
		&body[..400.min(body.len())]
	);
	// Meta tag can be self-closing: <meta charset="UTF-8" /> or <meta charset="UTF-8">
	assert!(
		body.contains("<meta charset=\"UTF-8\""),
		"Head should have charset meta tag with exact format, got: {}",
		&body[..500.min(body.len())]
	);
	assert!(
		body.contains("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\""),
		"Head should have viewport meta tag with exact content, got: {}",
		&body[..600.min(body.len())]
	);
	assert!(
		body.contains("<title>404 - Not Found</title>"),
		"Head should have exact title, got: {}",
		&body[..600.min(body.len())]
	);
	assert!(
		body.contains("</head>"),
		"HTML should close head element, got structure around head closing"
	);
	assert!(
		body.contains("<body>"),
		"HTML should have body element, got structure around body opening"
	);
	assert!(body.contains("</body>"), "HTML should close body element");
	assert!(body.contains("</html>"), "HTML should close html element");
}

#[test]
fn test_error_page_responsive_design() {
	let request = create_test_request("/test");
	let response = page_not_found::<String, String>(&request, None);

	let body = String::from_utf8(response.body.to_vec()).unwrap();

	// Check for mobile responsiveness meta tag
	assert!(
		body.contains("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\""),
		"HTML should have viewport meta tag with exact content for responsive design, got: {}",
		&body[..500.min(body.len())]
	);
}

#[test]
fn test_error_page_accessibility() {
	let request = create_test_request("/test");
	let response = page_not_found::<String, String>(&request, None);

	let body = String::from_utf8(response.body.to_vec()).unwrap();

	// Check for accessibility features
	assert!(
		body.contains("<html lang=\"en\">"),
		"HTML should have lang attribute for accessibility, got: {}",
		&body[..300.min(body.len())]
	);
	assert!(
		body.contains("<title>404 - Not Found</title>"),
		"HTML should have descriptive title with format '404 - Not Found' for accessibility, got: {}",
		&body[..500.min(body.len())]
	);
}
