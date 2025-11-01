//! Response Module Tests
//!
//! Tests inspired by Django Rest Framework status and response tests

use bytes::Bytes;
use hyper::StatusCode;
use reinhardt_apps::Response;
use serde_json::json;

#[test]
fn test_response_creation_ok() {
	let response = Response::ok();
	assert_eq!(response.status, StatusCode::OK);
}

#[test]
fn test_response_creation_created() {
	let response = Response::created();
	assert_eq!(response.status, StatusCode::CREATED);
}

#[test]
fn test_response_creation_no_content() {
	let response = Response::no_content();
	assert_eq!(response.status, StatusCode::NO_CONTENT);
}

#[test]
fn test_response_creation_bad_request() {
	let response = Response::bad_request();
	assert_eq!(response.status, StatusCode::BAD_REQUEST);
}

#[test]
fn test_response_creation_unauthorized() {
	let response = Response::unauthorized();
	assert_eq!(response.status, StatusCode::UNAUTHORIZED);
}

#[test]
fn test_response_creation_forbidden() {
	let response = Response::forbidden();
	assert_eq!(response.status, StatusCode::FORBIDDEN);
}

#[test]
fn test_response_creation_not_found() {
	let response = Response::not_found();
	assert_eq!(response.status, StatusCode::NOT_FOUND);
}

#[test]
fn test_response_creation_internal_server_error() {
	let response = Response::internal_server_error();
	assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_response_with_body() {
	let body_content = "Hello, world!";
	let response = Response::ok().with_body(body_content);

	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	assert_eq!(body_str, body_content);
}

#[test]
fn test_apps_response_with_json() {
	let data = json!({
		"message": "Hello, world!",
		"status": "success"
	});

	let response = Response::ok().with_json(&data).unwrap();

	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(body_str.contains("Hello, world!"));
	assert!(body_str.contains("success"));

	// Check content type header
	assert_eq!(
		response.headers.get(hyper::header::CONTENT_TYPE).unwrap(),
		"application/json"
	);
}

#[test]
fn test_response_with_header() {
	let response = Response::ok().with_header("content-type", "text/plain");

	assert_eq!(
		response.headers.get(hyper::header::CONTENT_TYPE).unwrap(),
		"text/plain"
	);
}

#[test]
fn test_response_new_with_custom_status() {
	let response = Response::new(StatusCode::ACCEPTED);
	assert_eq!(response.status, StatusCode::ACCEPTED);
}

#[test]
fn test_response_empty_body_by_default() {
	let response = Response::ok();
	assert!(response.body.is_empty());
}

#[test]
fn test_response_headers_empty_by_default() {
	let response = Response::ok();
	assert!(response.headers.is_empty());
}

/// Test HTTP status code categories
/// Inspired by DRF test_status.py
#[test]
fn test_status_categories_informational() {
	// 1xx: Informational
	assert!(!is_informational(99));
	assert!(is_informational(100));
	assert!(is_informational(199));
	assert!(!is_informational(200));
}

#[test]
fn test_status_categories_success() {
	// 2xx: Success
	assert!(!is_success(199));
	assert!(is_success(200));
	assert!(is_success(299));
	assert!(!is_success(300));
}

#[test]
fn test_status_categories_redirect() {
	// 3xx: Redirection
	assert!(!is_redirect(299));
	assert!(is_redirect(300));
	assert!(is_redirect(399));
	assert!(!is_redirect(400));
}

#[test]
fn test_status_categories_client_error() {
	// 4xx: Client Error
	assert!(!is_client_error(399));
	assert!(is_client_error(400));
	assert!(is_client_error(499));
	assert!(!is_client_error(500));
}

#[test]
fn test_status_categories_server_error() {
	// 5xx: Server Error
	assert!(!is_server_error(499));
	assert!(is_server_error(500));
	assert!(is_server_error(599));
	assert!(!is_server_error(600));
}

#[test]
fn test_response_chaining() {
	let response = Response::ok()
		.with_body("test")
		.with_header("content-type", "text/plain");

	assert_eq!(response.status, StatusCode::OK);
	assert_eq!(String::from_utf8(response.body.to_vec()).unwrap(), "test");
	assert_eq!(
		response.headers.get(hyper::header::CONTENT_TYPE).unwrap(),
		"text/plain"
	);
}

#[test]
fn test_response_with_multiple_headers() {
	let response = Response::ok()
		.with_header("content-type", "application/json")
		.with_header("cache-control", "no-cache");

	assert_eq!(
		response.headers.get(hyper::header::CONTENT_TYPE).unwrap(),
		"application/json"
	);
	assert_eq!(
		response.headers.get(hyper::header::CACHE_CONTROL).unwrap(),
		"no-cache"
	);
}

#[test]
fn test_response_body_bytes() {
	let body_bytes = Bytes::from("binary data");
	let response = Response::ok().with_body(body_bytes.clone());

	assert_eq!(response.body, body_bytes);
}

// Helper functions for status code category checking
fn is_informational(code: u16) -> bool {
	(100..200).contains(&code)
}

fn is_success(code: u16) -> bool {
	(200..300).contains(&code)
}

fn is_redirect(code: u16) -> bool {
	(300..400).contains(&code)
}

fn is_client_error(code: u16) -> bool {
	(400..500).contains(&code)
}

fn is_server_error(code: u16) -> bool {
	(500..600).contains(&code)
}
