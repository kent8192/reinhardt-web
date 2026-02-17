//! URL type integration tests
//!
//! Tests URL validation, conversion, and various URL formats including
//! relative paths, absolute paths, query parameters, and error cases.

use reinhardt_shortcuts::{Url, UrlError};
use rstest::rstest;

/// Test: Valid relative URL paths
#[rstest]
fn test_valid_relative_urls() {
	// Simple relative paths
	let url = Url::new("/").unwrap();
	assert_eq!(url.as_str(), "/");

	let url = Url::new("/home").unwrap();
	assert_eq!(url.as_str(), "/home");

	let url = Url::new("/users/123").unwrap();
	assert_eq!(url.as_str(), "/users/123");

	// Nested paths
	let url = Url::new("/api/v1/users/profile").unwrap();
	assert_eq!(url.as_str(), "/api/v1/users/profile");
}

/// Test: Valid absolute URLs
#[rstest]
fn test_valid_absolute_urls() {
	// HTTP URLs
	let url = Url::new("http://example.com").unwrap();
	assert_eq!(url.as_str(), "http://example.com");

	// HTTPS URLs
	let url = Url::new("https://example.com/path").unwrap();
	assert_eq!(url.as_str(), "https://example.com/path");

	// With port
	let url = Url::new("http://localhost:8000").unwrap();
	assert_eq!(url.as_str(), "http://localhost:8000");

	// With subdomain
	let url = Url::new("https://api.example.com/v1/users").unwrap();
	assert_eq!(url.as_str(), "https://api.example.com/v1/users");
}

/// Test: URLs with query parameters
#[rstest]
fn test_urls_with_query_parameters() {
	// Single parameter
	let url = Url::new("/search?q=rust").unwrap();
	assert_eq!(url.as_str(), "/search?q=rust");

	// Multiple parameters
	let url = Url::new("/search?q=rust&page=2&limit=10").unwrap();
	assert_eq!(url.as_str(), "/search?q=rust&page=2&limit=10");

	// Absolute URL with query
	let url = Url::new("https://example.com/search?q=test").unwrap();
	assert_eq!(url.as_str(), "https://example.com/search?q=test");
}

/// Test: URLs with fragments
#[rstest]
fn test_urls_with_fragments() {
	// Fragment only
	let url = Url::new("/page#section").unwrap();
	assert_eq!(url.as_str(), "/page#section");

	// Query and fragment
	let url = Url::new("/page?id=123#section").unwrap();
	assert_eq!(url.as_str(), "/page?id=123#section");

	// Absolute URL with fragment
	let url = Url::new("https://example.com/docs#introduction").unwrap();
	assert_eq!(url.as_str(), "https://example.com/docs#introduction");
}

/// Test: Empty URL validation error
#[rstest]
fn test_empty_url_error() {
	let result = Url::new("");
	assert!(result.is_err());
	assert_eq!(result.unwrap_err(), UrlError::Empty);
}

/// Test: Whitespace-only URL validation error
#[rstest]
fn test_whitespace_only_url_error() {
	let result = Url::new("   ");
	assert!(result.is_err());
	assert_eq!(result.unwrap_err(), UrlError::Whitespace);

	let result = Url::new("\t\n  ");
	assert!(result.is_err());
	assert_eq!(result.unwrap_err(), UrlError::Whitespace);
}

/// Test: URL Display trait implementation
#[rstest]
fn test_url_display() {
	let url = Url::new("/home").unwrap();
	assert_eq!(format!("{}", url), "/home");

	let url = Url::new("https://example.com").unwrap();
	assert_eq!(format!("{}", url), "https://example.com");
}

/// Test: URL AsRef<str> implementation
#[rstest]
fn test_url_as_ref() {
	let url = Url::new("/page").unwrap();
	let url_ref: &str = url.as_ref();
	assert_eq!(url_ref, "/page");
}

/// Test: URL into_string conversion
#[rstest]
fn test_url_into_string() {
	let url = Url::new("/about").unwrap();
	let string: String = url.into_string();
	assert_eq!(string, "/about");
}

/// Test: URL From<String> conversion (backward compatibility)
#[rstest]
fn test_url_from_string() {
	let string = String::from("/users");
	let url: Url = string.into();
	assert_eq!(url.as_str(), "/users");

	// Even empty strings are allowed via From (no validation)
	let empty = String::from("");
	let url: Url = empty.into();
	assert_eq!(url.as_str(), "");
}

/// Test: URL From<&str> conversion (backward compatibility)
#[rstest]
fn test_url_from_str() {
	let url: Url = "/profile".into();
	assert_eq!(url.as_str(), "/profile");

	// Even whitespace-only strings are allowed via From (no validation)
	let url: Url = "   ".into();
	assert_eq!(url.as_str(), "   ");
}

/// Test: URL cloning
#[rstest]
fn test_url_clone() {
	let url1 = Url::new("/original").unwrap();
	let url2 = url1.clone();

	assert_eq!(url1.as_str(), url2.as_str());
	assert_eq!(url1, url2);
}

/// Test: URL equality
#[rstest]
fn test_url_equality() {
	let url1 = Url::new("/same").unwrap();
	let url2 = Url::new("/same").unwrap();
	let url3 = Url::new("/different").unwrap();

	assert_eq!(url1, url2);
	assert_ne!(url1, url3);
}

/// Test: UTF-8 URLs (encoded and unencoded)
#[rstest]
fn test_utf8_urls() {
	// Unencoded UTF-8 (for internal representation)
	let url = Url::new("/search?q=日本語").unwrap();
	assert_eq!(url.as_str(), "/search?q=日本語");

	// Percent-encoded UTF-8 (typical for actual HTTP URLs)
	let url = Url::new("/search?q=%E6%97%A5%E6%9C%AC%E8%AA%9E").unwrap();
	assert_eq!(url.as_str(), "/search?q=%E6%97%A5%E6%9C%AC%E8%AA%9E");
}

/// Test: Special characters in URLs
#[rstest]
fn test_special_characters_in_urls() {
	// Spaces (should be percent-encoded in real URLs, but URL type doesn't enforce)
	let url = Url::new("/path with spaces").unwrap();
	assert_eq!(url.as_str(), "/path with spaces");

	// Percent-encoded special characters
	let url = Url::new("/path%20with%20encoded%20spaces").unwrap();
	assert_eq!(url.as_str(), "/path%20with%20encoded%20spaces");

	// Ampersands in query
	let url = Url::new("/page?a=1&b=2&c=3").unwrap();
	assert_eq!(url.as_str(), "/page?a=1&b=2&c=3");
}

/// Test: Very long URLs
#[rstest]
fn test_long_urls() {
	let long_path = format!("/very/long/path/{}", "segment/".repeat(100));
	let url = Url::new(&long_path).unwrap();
	assert_eq!(url.as_str(), long_path);

	let long_query = format!(
		"/search?{}",
		(0..50)
			.map(|i| format!("param{}=value{}", i, i))
			.collect::<Vec<_>>()
			.join("&")
	);
	let url = Url::new(&long_query).unwrap();
	assert_eq!(url.as_str(), long_query);
}

/// Test: Error message formatting
#[rstest]
fn test_error_message_formatting() {
	let error = UrlError::Empty;
	assert_eq!(error.to_string(), "URL cannot be empty");

	let error = UrlError::Whitespace;
	assert_eq!(error.to_string(), "URL cannot contain only whitespace");
}
