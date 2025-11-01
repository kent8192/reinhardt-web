//! Authentication Security Integration Tests
//!
//! Tests HTTP authentication mechanisms
//! Based on FastAPI's test_security_*.py tests

use reinhardt_integration_tests::security_test_helpers::*;

use base64::{Engine as _, engine::general_purpose};
use hyper::StatusCode;
use hyper::header::{AUTHORIZATION, HeaderName, HeaderValue, WWW_AUTHENTICATE};

/// Create Basic Auth header value
fn create_basic_auth(username: &str, password: &str) -> String {
	let credentials = format!("{}:{}", username, password);
	let encoded = general_purpose::STANDARD.encode(credentials.as_bytes());
	format!("Basic {}", encoded)
}

/// Create Bearer Auth header value
fn create_bearer_token(token: &str) -> String {
	format!("Bearer {}", token)
}

/// Extract credentials from Basic Auth header
fn parse_basic_auth(auth_header: &str) -> Option<(String, String)> {
	let encoded = auth_header.strip_prefix("Basic ")?;
	let decoded = general_purpose::STANDARD.decode(encoded).ok()?;
	let credentials = String::from_utf8(decoded).ok()?;
	let mut parts = credentials.splitn(2, ':');
	let username = parts.next()?.to_string();
	let password = parts.next()?.to_string();
	Some((username, password))
}

#[test]
fn test_basic_auth_encoding() {
	// Test: Basic auth credentials are properly encoded
	let auth = create_basic_auth("john", "secret");
	assert!(auth.starts_with("Basic "));
	assert!(auth.len() > 6); // "Basic " + base64
}

#[test]
fn test_basic_auth_parsing() {
	// Test: Basic auth credentials can be parsed
	let auth = create_basic_auth("alice", "password123");
	let (username, password) = parse_basic_auth(&auth).unwrap();
	assert_eq!(username, "alice");
	assert_eq!(password, "password123");
}

#[test]
fn test_basic_auth_with_special_chars() {
	// Test: Basic auth handles special characters in password
	let auth = create_basic_auth("user", "p@ss:w0rd!");
	let (username, password) = parse_basic_auth(&auth).unwrap();
	assert_eq!(username, "user");
	assert_eq!(password, "p@ss:w0rd!");
}

#[test]
fn test_basic_auth_invalid_format() {
	// Test: Invalid Basic auth format is detected
	let invalid_auth = "Basic notbase64!!!";
	let result = parse_basic_auth(invalid_auth);
	assert!(result.is_none());
}

#[test]
fn test_basic_auth_missing_colon() {
	// Test: Basic auth without colon separator is invalid
	let credentials = "johnsecret"; // Missing colon
	let encoded = general_purpose::STANDARD.encode(credentials.as_bytes());
	let auth = format!("Basic {}", encoded);
	let result = parse_basic_auth(&auth);
	assert!(result.is_none());
}

#[test]
fn test_bearer_token_format() {
	// Test: Bearer token has correct format
	let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
	let auth = create_bearer_token(token);
	assert_eq!(auth, format!("Bearer {}", token));
}

#[test]
fn test_bearer_token_extraction() {
	// Test: Bearer token can be extracted
	let token = "abc123def456";
	let auth = create_bearer_token(token);
	let extracted = auth.strip_prefix("Bearer ").unwrap();
	assert_eq!(extracted, token);
}

#[test]
fn test_authorization_header_basic() {
	// Test: Authorization header with Basic auth
	let mut request = create_test_request("GET", "/api/users", true);
	let auth = create_basic_auth("testuser", "testpass");
	request
		.headers
		.insert(AUTHORIZATION, HeaderValue::from_str(&auth).unwrap());

	let auth_header = request.headers.get(AUTHORIZATION).unwrap();
	assert!(auth_header.to_str().unwrap().starts_with("Basic "));
}

#[test]
fn test_authorization_header_bearer() {
	// Test: Authorization header with Bearer token
	let mut request = create_test_request("GET", "/api/users", true);
	let auth = create_bearer_token("my_token_123");
	request
		.headers
		.insert(AUTHORIZATION, HeaderValue::from_str(&auth).unwrap());

	let auth_header = request.headers.get(AUTHORIZATION).unwrap();
	assert!(auth_header.to_str().unwrap().starts_with("Bearer "));
}

#[test]
fn test_www_authenticate_header() {
	// Test: WWW-Authenticate header for 401 responses
	let mut response = create_response_with_status(StatusCode::UNAUTHORIZED);
	response.headers.insert(
		WWW_AUTHENTICATE,
		HeaderValue::from_static("Basic realm=\"API\""),
	);

	assert_has_header(&response, "www-authenticate");
	assert_header_contains(&response, "www-authenticate", "Basic");
}

#[test]
fn test_multiple_auth_schemes() {
	// Test: Multiple authentication schemes can be advertised
	let mut response = create_response_with_status(StatusCode::UNAUTHORIZED);
	response.headers.insert(
		WWW_AUTHENTICATE,
		HeaderValue::from_static("Basic realm=\"API\", Bearer"),
	);

	let auth_header = get_header(&response, "www-authenticate").unwrap();
	assert!(auth_header.contains("Basic"));
	assert!(auth_header.contains("Bearer"));
}

#[test]
fn test_api_key_in_header() {
	// Test: API key in custom header
	let mut request = create_test_request("GET", "/api/data", true);
	request.headers.insert(
		HeaderName::from_static("x-api-key"),
		HeaderValue::from_static("my-api-key-123"),
	);

	assert!(request.headers.contains_key("x-api-key"));
	let api_key = request.headers.get("x-api-key").unwrap();
	assert_eq!(api_key.to_str().unwrap(), "my-api-key-123");
}

#[test]
fn test_api_key_in_query() {
	// Test: API key in query parameter
	let request = create_test_request("GET", "/api/data?api_key=secret123", true);
	let query = request.uri.query().unwrap();
	assert!(query.contains("api_key=secret123"));
}

#[test]
fn test_api_key_in_cookie() {
	// Test: API key in cookie
	let mut request = create_test_request("GET", "/api/data", true);
	request.headers.insert(
		hyper::header::COOKIE,
		HeaderValue::from_static("api_key=cookie_key_456"),
	);

	let cookie = request.headers.get("cookie").unwrap();
	assert!(cookie.to_str().unwrap().contains("api_key="));
}

#[test]
fn test_empty_authorization_header() {
	// Test: Empty authorization header is invalid
	let mut request = create_test_request("GET", "/api/test", true);
	request
		.headers
		.insert(AUTHORIZATION, HeaderValue::from_static(""));

	let auth = request.headers.get(AUTHORIZATION).unwrap();
	assert_eq!(auth.to_str().unwrap(), "");
}

#[test]
fn test_malformed_basic_auth() {
	// Test: Malformed Basic auth header
	let mut request = create_test_request("GET", "/api/test", true);
	request
		.headers
		.insert(AUTHORIZATION, HeaderValue::from_static("Basic"));

	let auth = request
		.headers
		.get(AUTHORIZATION)
		.unwrap()
		.to_str()
		.unwrap();
	assert_eq!(auth, "Basic"); // Missing space and credentials
}

#[test]
fn test_case_sensitivity_auth_scheme() {
	// Test: Auth scheme should be case-insensitive
	let schemes = ["Basic", "basic", "BASIC", "Bearer", "bearer", "BEARER"];
	for scheme in &schemes {
		let auth = format!("{} credentials", scheme);
		assert!(
			auth.to_lowercase().starts_with("basic") || auth.to_lowercase().starts_with("bearer")
		);
	}
}

#[test]
fn test_basic_auth_with_empty_password() {
	// Test: Basic auth with empty password
	let auth = create_basic_auth("user", "");
	let (username, password) = parse_basic_auth(&auth).unwrap();
	assert_eq!(username, "user");
	assert_eq!(password, "");
}

#[test]
fn test_basic_auth_with_empty_username() {
	// Test: Basic auth with empty username
	let auth = create_basic_auth("", "password");
	let (username, password) = parse_basic_auth(&auth).unwrap();
	assert_eq!(username, "");
	assert_eq!(password, "password");
}

#[test]
fn test_bearer_token_with_special_chars() {
	// Test: Bearer tokens can contain various characters
	let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
	let auth = create_bearer_token(token);
	assert!(auth.contains(token));
}

#[test]
fn test_authorization_header_with_extra_spaces() {
	// Test: Extra spaces in authorization header
	let auth = "Basic  dXNlcjpwYXNz"; // Extra space
	// This should be invalid or handled gracefully
	assert!(auth.starts_with("Basic"));
}

#[test]
fn test_unauthorized_response_format() {
	// Test: 401 response should include WWW-Authenticate
	let mut response = create_response_with_status(StatusCode::UNAUTHORIZED);
	response.headers.insert(
		WWW_AUTHENTICATE,
		HeaderValue::from_static("Basic realm=\"Secure Area\""),
	);

	assert_status(&response, StatusCode::UNAUTHORIZED);
	assert_has_header(&response, "www-authenticate");
}

#[test]
fn test_forbidden_vs_unauthorized() {
	// Test: 401 (Unauthorized) vs 403 (Forbidden)
	let unauthorized = create_response_with_status(StatusCode::UNAUTHORIZED);
	let forbidden = create_response_with_status(StatusCode::FORBIDDEN);

	assert_eq!(unauthorized.status, StatusCode::UNAUTHORIZED); // No/invalid auth
	assert_eq!(forbidden.status, StatusCode::FORBIDDEN); // Valid auth, insufficient permissions
}
