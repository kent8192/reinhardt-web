//! Fixtures for reinhardt-pages integration tests
//!
//! This module provides reusable fixtures for testing reinhardt-pages functionality,
//! including Server Functions, CSRF protection, and API model CRUD operations.

use rstest::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// CSRF Token Fixtures
// ============================================================================

/// Provides a test CSRF token
#[fixture]
pub fn csrf_token() -> String {
	"test-csrf-token-abc123xyz789".to_string()
}

/// Provides a cookie string containing a CSRF token
#[fixture]
pub fn csrf_cookie_string() -> String {
	"sessionid=abc123; csrftoken=test-csrf-token; other=value".to_string()
}

/// Provides HTML with a CSRF token in a meta tag
#[fixture]
pub fn csrf_html_with_meta() -> String {
	r#"<!DOCTYPE html>
<html>
<head>
    <meta name="csrf-token" content="meta-csrf-token-xyz789">
</head>
<body></body>
</html>"#
		.to_string()
}

/// Provides HTML with a CSRF token in a hidden input field
#[fixture]
pub fn csrf_html_with_input() -> String {
	r#"<!DOCTYPE html>
<html>
<body>
    <form>
        <input type="hidden" name="csrfmiddlewaretoken" value="input-csrf-token-def456">
    </form>
</body>
</html>"#
		.to_string()
}

/// Provides an invalid cookie string for error testing
#[fixture]
pub fn invalid_cookie_string() -> String {
	"malformed cookie string without = sign".to_string()
}

// ============================================================================
// Server Function Payload Fixtures
// ============================================================================

/// Test data structure for server function payloads
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestPayload {
	pub id: u32,
	pub name: String,
	pub active: bool,
}

impl TestPayload {
	pub fn new(id: u32, name: impl Into<String>, active: bool) -> Self {
		Self {
			id,
			name: name.into(),
			active,
		}
	}
}

/// Provides a basic test payload for server functions
#[fixture]
pub fn test_payload() -> TestPayload {
	TestPayload::new(42, "test-payload", true)
}

/// Provides a test payload with special characters
#[fixture]
pub fn test_payload_with_special_chars() -> TestPayload {
	TestPayload::new(1, "<>&\"'`!@#$%", true)
}

/// Provides a large test payload (for boundary testing)
#[fixture]
pub fn large_test_payload() -> TestPayload {
	TestPayload::new(999, "x".repeat(10000), false)
}

/// Provides an empty payload (for edge case testing)
#[fixture]
pub fn empty_payload() -> TestPayload {
	TestPayload::new(0, "", false)
}

// ============================================================================
// API Filter Fixtures
// ============================================================================

/// Test model structure for API operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestModel {
	pub id: i32,
	pub title: String,
	pub content: String,
	pub published: bool,
	pub view_count: i32,
}

impl TestModel {
	pub fn new(
		id: i32,
		title: impl Into<String>,
		content: impl Into<String>,
		published: bool,
		view_count: i32,
	) -> Self {
		Self {
			id,
			title: title.into(),
			content: content.into(),
			published,
			view_count,
		}
	}
}

/// Provides a test model for CRUD operations
#[fixture]
pub fn test_model() -> TestModel {
	TestModel::new(1, "Test Title", "Test content goes here", true, 100)
}

/// Provides multiple test models for list operations
#[fixture]
pub fn test_models() -> Vec<TestModel> {
	vec![
		TestModel::new(1, "First Post", "Content 1", true, 100),
		TestModel::new(2, "Second Post", "Content 2", true, 50),
		TestModel::new(3, "Draft Post", "Content 3", false, 0),
		TestModel::new(4, "Popular Post", "Content 4", true, 1000),
		TestModel::new(5, "Another Draft", "Content 5", false, 5),
	]
}

// ============================================================================
// Specialized String Fixtures
// ============================================================================

/// Provides a string with special characters that need escaping
#[fixture]
pub fn special_chars_string() -> String {
	r#"<script>alert("XSS")</script>&<>'"#.to_string()
}

/// Provides a very long string for boundary testing
#[fixture]
pub fn long_string() -> String {
	"a".repeat(100_000)
}

/// Provides an empty string for edge case testing
#[fixture]
pub fn empty_string() -> String {
	String::new()
}

/// Provides a string with unicode characters
#[fixture]
pub fn unicode_string() -> String {
	"こんにちは世界🌍🚀".to_string()
}

// ============================================================================
// Numeric Boundary Fixtures
// ============================================================================

/// Provides various payload sizes for boundary testing
pub fn payload_sizes() -> Vec<usize> {
	vec![
		0,                // Empty
		64,               // Small
		1024,             // Typical (1KB)
		1024 * 1024,      // Large (1MB)
		10 * 1024 * 1024, // Very large (10MB)
	]
}

/// Provides various page sizes for pagination testing
pub fn page_sizes() -> Vec<usize> {
	vec![
		1,    // Minimum
		10,   // Typical
		100,  // Large
		1000, // Maximum
	]
}

/// Provides various offset values for pagination testing
pub fn offset_values() -> Vec<usize> {
	vec![
		0,     // Start
		10,    // Small offset
		100,   // Medium offset
		10000, // Large offset
	]
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generates a test JSON payload of specified size
pub fn generate_json_payload(size_bytes: usize) -> String {
	let chunk = r#"{"key":"value","num":42,"bool":true},"#;
	let repeats = size_bytes / chunk.len();
	let mut result = String::from("[");
	for i in 0..repeats {
		result.push_str(&chunk.replace("key", &format!("key{}", i)));
	}
	result.push(']');
	result
}

/// Generates a nested HTML structure of specified depth
pub fn generate_nested_html(depth: usize) -> String {
	let mut html = String::new();
	for i in 0..depth {
		html.push_str(&format!(r#"<div data-hyd-id="comp-{}">"#, i));
	}
	html.push_str("content");
	for _ in 0..depth {
		html.push_str("</div>");
	}
	html
}

/// Creates a complex cookie string with multiple key-value pairs
pub fn create_complex_cookie_string(pairs: &[(&str, &str)]) -> String {
	pairs
		.iter()
		.map(|(k, v)| format!("{}={}", k, v))
		.collect::<Vec<_>>()
		.join("; ")
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_csrf_token_fixture() {
		let token = csrf_token();
		assert!(!token.is_empty());
		assert!(token.starts_with("test-csrf-token"));
	}

	#[rstest]
	fn test_payload_fixture() {
		let payload = test_payload();
		assert_eq!(payload.id, 42);
		assert_eq!(payload.name, "test-payload");
		assert!(payload.active);
	}

	#[rstest]
	fn test_model_fixture() {
		let model = test_model();
		assert_eq!(model.id, 1);
		assert_eq!(model.title, "Test Title");
		assert!(model.published);
	}

	#[rstest]
	fn test_generate_json_payload() {
		let payload = generate_json_payload(1000);
		assert!(!payload.is_empty());
		assert!(payload.starts_with('['));
		assert!(payload.ends_with(']'));
	}

	#[rstest]
	fn test_generate_nested_html() {
		let html = generate_nested_html(3);
		assert!(html.contains(r#"data-hyd-id="comp-0""#));
		assert!(html.contains(r#"data-hyd-id="comp-1""#));
		assert!(html.contains(r#"data-hyd-id="comp-2""#));
		assert_eq!(html.matches("<div").count(), 3);
		assert_eq!(html.matches("</div>").count(), 3);
	}

	#[rstest]
	fn test_create_complex_cookie_string() {
		let cookie = create_complex_cookie_string(&[
			("session", "abc123"),
			("csrf", "xyz789"),
			("user", "john"),
		]);
		assert!(cookie.contains("session=abc123"));
		assert!(cookie.contains("csrf=xyz789"));
		assert!(cookie.contains("user=john"));
	}
}
