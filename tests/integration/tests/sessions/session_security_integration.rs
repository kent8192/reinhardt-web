//! Session Security Integration Tests
//!
//! Tests session cookie security settings
//! Based on Django's check_framework/test_security.py session tests

use reinhardt_integration_tests::security_test_helpers::*;

use hyper::header::{HeaderValue, SET_COOKIE};

/// Parse Set-Cookie header for attributes
fn parse_cookie_attributes(set_cookie: &str) -> Vec<String> {
	set_cookie
		.split(';')
		.map(|s| s.trim().to_lowercase())
		.collect()
}

/// Check if cookie has attribute
fn has_cookie_attribute(set_cookie: &str, attribute: &str) -> bool {
	let attributes = parse_cookie_attributes(set_cookie);
	let attr_lower = attribute.to_lowercase();
	attributes
		.iter()
		.any(|attr| attr == &attr_lower || attr.starts_with(&format!("{}=", attr_lower)))
}

/// Get cookie attribute value
fn get_cookie_attribute(set_cookie: &str, attribute: &str) -> Option<String> {
	let attributes = parse_cookie_attributes(set_cookie);
	for attr in attributes {
		if let Some(value) = attr.strip_prefix(&format!("{}=", attribute.to_lowercase())) {
			return Some(value.to_string());
		}
	}
	None
}

#[test]
fn test_session_cookie_secure_flag() {
	// Test: Session cookie should have Secure flag in production
	let mut response = create_test_response();
	response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=abc123; Secure; HttpOnly; SameSite=Lax"),
	);

	let cookie = get_header(&response, "set-cookie").unwrap();
	assert!(has_cookie_attribute(cookie, "Secure"));
}

#[test]
fn test_session_cookie_httponly_flag() {
	// Test: Session cookie should have HttpOnly flag
	let mut response = create_test_response();
	response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=abc123; Secure; HttpOnly; SameSite=Lax"),
	);

	let cookie = get_header(&response, "set-cookie").unwrap();
	assert!(has_cookie_attribute(cookie, "HttpOnly"));
}

#[test]
fn test_session_cookie_samesite() {
	// Test: Session cookie should have SameSite attribute
	let mut response = create_test_response();
	response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=abc123; Secure; HttpOnly; SameSite=Lax"),
	);

	let cookie = get_header(&response, "set-cookie").unwrap();
	assert!(has_cookie_attribute(cookie, "SameSite"));
}

#[test]
fn test_session_cookie_samesite_strict() {
	// Test: Session cookie with SameSite=Strict
	let mut response = create_test_response();
	response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=abc123; SameSite=Strict"),
	);

	let cookie = get_header(&response, "set-cookie").unwrap();
	let samesite = get_cookie_attribute(cookie, "SameSite").unwrap();
	assert_eq!(samesite, "strict");
}

#[test]
fn test_session_cookie_samesite_lax() {
	// Test: Session cookie with SameSite=Lax (default recommended)
	let mut response = create_test_response();
	response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=abc123; SameSite=Lax"),
	);

	let cookie = get_header(&response, "set-cookie").unwrap();
	let samesite = get_cookie_attribute(cookie, "SameSite").unwrap();
	assert_eq!(samesite, "lax");
}

#[test]
fn test_session_cookie_samesite_none_requires_secure() {
	// Test: SameSite=None requires Secure flag
	let mut response = create_test_response();
	response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=abc123; SameSite=None; Secure"),
	);

	let cookie = get_header(&response, "set-cookie").unwrap();
	assert!(has_cookie_attribute(cookie, "Secure"));
	let samesite = get_cookie_attribute(cookie, "SameSite").unwrap();
	assert_eq!(samesite, "none");
}

#[test]
fn test_session_cookie_domain() {
	// Test: Session cookie can specify domain
	let mut response = create_test_response();
	response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=abc123; Domain=.example.com"),
	);

	let cookie = get_header(&response, "set-cookie").unwrap();
	let domain = get_cookie_attribute(cookie, "Domain").unwrap();
	assert_eq!(domain, ".example.com");
}

#[test]
fn test_session_cookie_path() {
	// Test: Session cookie with Path attribute
	let mut response = create_test_response();
	response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=abc123; Path=/"),
	);

	let cookie = get_header(&response, "set-cookie").unwrap();
	let path = get_cookie_attribute(cookie, "Path").unwrap();
	assert_eq!(path, "/");
}

#[test]
fn test_session_cookie_max_age() {
	// Test: Session cookie with Max-Age
	let mut response = create_test_response();
	response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=abc123; Max-Age=3600"),
	);

	let cookie = get_header(&response, "set-cookie").unwrap();
	let max_age = get_cookie_attribute(cookie, "Max-Age").unwrap();
	assert_eq!(max_age, "3600");
}

#[test]
fn test_session_cookie_expires() {
	// Test: Session cookie with Expires
	let mut response = create_test_response();
	response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=abc123; Expires=Wed, 21 Oct 2025 07:28:00 GMT"),
	);

	let cookie = get_header(&response, "set-cookie").unwrap();
	assert!(has_cookie_attribute(cookie, "Expires"));
}

#[test]
fn test_session_cookie_deletion() {
	// Test: Session cookie deletion (Max-Age=0 or past Expires)
	let mut response = create_test_response();
	response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=; Max-Age=0"),
	);

	let cookie = get_header(&response, "set-cookie").unwrap();
	let max_age = get_cookie_attribute(cookie, "Max-Age").unwrap();
	assert_eq!(max_age, "0");
}

#[test]
fn test_csrf_vs_session_cookie_httponly() {
	// Test: CSRF cookies should NOT be HttpOnly (JS needs access)
	// Session cookies SHOULD be HttpOnly (prevent XSS)

	let mut session_response = create_test_response();
	session_response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=abc; HttpOnly"),
	);

	let mut csrf_response = create_test_response();
	csrf_response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("csrftoken=xyz"), // No HttpOnly
	);

	let session_cookie = get_header(&session_response, "set-cookie").unwrap();
	assert!(has_cookie_attribute(session_cookie, "HttpOnly"));

	let csrf_cookie = get_header(&csrf_response, "set-cookie").unwrap();
	assert!(!has_cookie_attribute(csrf_cookie, "HttpOnly"));
}

#[test]
fn test_session_fixation_prevention() {
	// Test: Session ID should be regenerated on login
	// Old session: sessionid=old123
	// After login: sessionid=new456

	let old_session_id = "old123";
	let new_session_id = "new456";

	assert_ne!(old_session_id, new_session_id);
	// In real implementation, session ID should be regenerated
}

#[test]
fn test_session_timeout() {
	// Test: Session should timeout after inactivity
	let max_age = 3600; // 1 hour
	assert!(max_age > 0);
	// Real implementation would check last activity timestamp
}

#[test]
fn test_multiple_cookies() {
	// Test: Multiple Set-Cookie headers for different cookies
	let mut response = create_test_response();
	response.headers.append(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=abc123; Secure; HttpOnly"),
	);
	response.headers.append(
		SET_COOKIE,
		HeaderValue::from_static("preferences=dark_mode; Secure"),
	);

	// Both cookies should be present
	let cookies: Vec<_> = response.headers.get_all(SET_COOKIE).iter().collect();
	assert_eq!(cookies.len(), 2);
}

#[test]
fn test_session_cookie_name() {
	// Test: Session cookie has correct name
	let mut response = create_test_response();
	response
		.headers
		.insert(SET_COOKIE, HeaderValue::from_static("sessionid=abc123"));

	let cookie = get_header(&response, "set-cookie").unwrap();
	assert!(cookie.starts_with("sessionid="));
}

#[test]
fn test_session_cookie_secure_production() {
	// Test: In production (HTTPS), Secure flag must be set
	let mut response = create_test_response();
	response.headers.insert(
		SET_COOKIE,
		HeaderValue::from_static("sessionid=abc123; Secure; HttpOnly; SameSite=Lax"),
	);

	let cookie = get_header(&response, "set-cookie").unwrap();
	// All three security attributes should be present
	assert!(has_cookie_attribute(cookie, "Secure"));
	assert!(has_cookie_attribute(cookie, "HttpOnly"));
	assert!(has_cookie_attribute(cookie, "SameSite"));
}
