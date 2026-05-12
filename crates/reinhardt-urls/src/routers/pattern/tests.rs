//! Tests for the `pattern` submodules.

use super::matcher::{MatchingMode, PathMatcher};
use super::path_pattern::PathPattern;
use super::radix::RadixRouter;
use super::validation::{
	MAX_PATH_SEGMENTS, MAX_PATTERN_LENGTH, validate_path_param, validate_reverse_param,
};
use std::collections::HashMap;

#[test]
fn test_simple_pattern() {
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/")).unwrap();
	assert!(pattern.regex.is_match("/users/"));
	assert!(!pattern.regex.is_match("/users/123/"));
}

#[test]
fn test_parameter_pattern() {
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/{id}/")).unwrap();
	assert_eq!(pattern.param_names(), &["id"]);
	assert!(pattern.regex.is_match("/users/123/"));
	assert!(!pattern.regex.is_match("/users/"));
}

#[test]
fn test_pattern_multiple_parameters() {
	let pattern = PathPattern::new(reinhardt_routers_macros::path!(
		"/users/{user_id}/posts/{post_id}/"
	))
	.unwrap();
	assert_eq!(pattern.param_names(), &["user_id", "post_id"]);
	assert!(pattern.regex.is_match("/users/123/posts/456/"));
}

#[test]
fn test_path_matcher() {
	let mut matcher = PathMatcher::new();
	matcher
		.add_pattern(
			PathPattern::new(reinhardt_routers_macros::path!("/users/")).unwrap(),
			"users_list".to_string(),
		)
		.unwrap();
	matcher
		.add_pattern(
			PathPattern::new(reinhardt_routers_macros::path!("/users/{id}/")).unwrap(),
			"users_detail".to_string(),
		)
		.unwrap();

	let result = matcher.match_path("/users/123/");
	assert!(result.is_some());
	let (handler_id, params) = result.unwrap();
	assert_eq!(handler_id, "users_detail");
	assert_eq!(params.get("id"), Some(&"123".to_string()));
}

// ===================================================================
// URL Reversal Tests with Aho-Corasick
// ===================================================================

#[test]
fn test_reverse_simple_pattern_no_params() {
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/")).unwrap();
	let params = HashMap::new();

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, "/users/");
}

#[test]
fn test_reverse_single_parameter() {
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/{id}/")).unwrap();
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123".to_string());

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, "/users/123/");
}

#[test]
fn test_reverse_multiple_parameters() {
	let pattern = PathPattern::new(reinhardt_routers_macros::path!(
		"/users/{user_id}/posts/{post_id}/"
	))
	.unwrap();
	let mut params = HashMap::new();
	params.insert("user_id".to_string(), "42".to_string());
	params.insert("post_id".to_string(), "100".to_string());

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, "/users/42/posts/100/");
}

#[test]
fn test_reverse_many_parameters() {
	// Test with 10+ parameters to demonstrate Aho-Corasick performance
	let pattern =
		PathPattern::new("/api/{p1}/{p2}/{p3}/{p4}/{p5}/{p6}/{p7}/{p8}/{p9}/{p10}/{p11}/{p12}/")
			.unwrap();

	let mut params = HashMap::new();
	params.insert("p1".to_string(), "v1".to_string());
	params.insert("p2".to_string(), "v2".to_string());
	params.insert("p3".to_string(), "v3".to_string());
	params.insert("p4".to_string(), "v4".to_string());
	params.insert("p5".to_string(), "v5".to_string());
	params.insert("p6".to_string(), "v6".to_string());
	params.insert("p7".to_string(), "v7".to_string());
	params.insert("p8".to_string(), "v8".to_string());
	params.insert("p9".to_string(), "v9".to_string());
	params.insert("p10".to_string(), "v10".to_string());
	params.insert("p11".to_string(), "v11".to_string());
	params.insert("p12".to_string(), "v12".to_string());

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, "/api/v1/v2/v3/v4/v5/v6/v7/v8/v9/v10/v11/v12/");
}

#[test]
fn test_reverse_consecutive_placeholders() {
	let pattern = PathPattern::new("/{a}{b}/").unwrap();
	let mut params = HashMap::new();
	params.insert("a".to_string(), "1".to_string());
	params.insert("b".to_string(), "2".to_string());

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, "/12/");
}

#[test]
fn test_reverse_missing_parameter() {
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/{id}/")).unwrap();
	let params = HashMap::new();

	let result = pattern.reverse(&params);
	assert!(result.is_err());
	assert!(
		result
			.unwrap_err()
			.contains("Missing required parameter: id")
	);
}

#[test]
fn test_reverse_partial_parameters() {
	let pattern = PathPattern::new(reinhardt_routers_macros::path!(
		"/users/{user_id}/posts/{post_id}/"
	))
	.unwrap();
	let mut params = HashMap::new();
	params.insert("user_id".to_string(), "42".to_string());
	// Missing post_id

	let result = pattern.reverse(&params);
	assert!(result.is_err());
	assert!(result.unwrap_err().contains("Missing required parameter"));
}

#[test]
fn test_reverse_special_chars_in_values() {
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/items/{id}/")).unwrap();
	let mut params = HashMap::new();
	params.insert("id".to_string(), "foo-bar_123".to_string());

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, "/items/foo-bar_123/");
}

#[test]
fn test_reverse_numeric_values() {
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/items/{id}/")).unwrap();
	let mut params = HashMap::new();
	params.insert("id".to_string(), "12345".to_string());

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, "/items/12345/");
}

#[test]
fn test_reverse_unicode_values() {
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/{name}/")).unwrap();
	let mut params = HashMap::new();
	params.insert("name".to_string(), "ユーザー".to_string());

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, "/users/ユーザー/");
}

#[test]
fn test_reverse_param_at_start() {
	let pattern = PathPattern::new("{lang}/users/").unwrap();
	let mut params = HashMap::new();
	params.insert("lang".to_string(), "ja".to_string());

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, "ja/users/");
}

#[test]
fn test_reverse_param_at_end() {
	let pattern = PathPattern::new("/api/data.{format}").unwrap();
	let mut params = HashMap::new();
	params.insert("format".to_string(), "json".to_string());

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, "/api/data.json");
}

#[test]
fn test_reverse_complex_mixed_content() {
	let pattern = PathPattern::new("/items/{id}/actions/{action}/execute").unwrap();
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123".to_string());
	params.insert("action".to_string(), "edit".to_string());

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, "/items/123/actions/edit/execute");
}

#[test]
fn test_reverse_long_value() {
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/items/{id}/")).unwrap();
	let mut params = HashMap::new();
	let long_id = "a".repeat(1000);
	params.insert("id".to_string(), long_id.clone());

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, format!("/items/{}/", long_id));
}

#[test]
fn test_reverse_empty_value() {
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/items/{id}/")).unwrap();
	let mut params = HashMap::new();
	params.insert("id".to_string(), "".to_string());

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, "/items//");
}

#[test]
fn test_reverse_extra_parameters() {
	// Extra parameters should be ignored
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/{id}/")).unwrap();
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123".to_string());
	params.insert("extra".to_string(), "ignored".to_string());

	let result = pattern.reverse(&params).unwrap();
	assert_eq!(result, "/users/123/");
}

// ===================================================================
// RadixRouter Tests
// ===================================================================

#[test]
fn test_radix_router_basic_matching() {
	let mut router = RadixRouter::new();
	router
		.add_route("/users/", "users_list".to_string())
		.unwrap();
	router
		.add_route("/users/{id}/", "users_detail".to_string())
		.unwrap();

	// Match list route
	let result = router.match_path("/users/");
	assert!(result.is_some());
	let (handler_id, params) = result.unwrap();
	assert_eq!(handler_id, "users_list");
	assert!(params.is_empty());

	// Match detail route
	let result = router.match_path("/users/123/");
	assert!(result.is_some());
	let (handler_id, params) = result.unwrap();
	assert_eq!(handler_id, "users_detail");
	assert_eq!(params.get("id"), Some(&"123".to_string()));
}

#[test]
fn test_radix_router_multiple_parameters() {
	let mut router = RadixRouter::new();
	router
		.add_route("/users/{id}/posts/{post_id}/", "post_detail".to_string())
		.unwrap();

	let result = router.match_path("/users/123/posts/456/");
	assert!(result.is_some());
	let (handler_id, params) = result.unwrap();
	assert_eq!(handler_id, "post_detail");
	assert_eq!(params.get("id"), Some(&"123".to_string()));
	assert_eq!(params.get("post_id"), Some(&"456".to_string()));
}

#[test]
fn test_radix_router_wildcard() {
	let mut router = RadixRouter::new();
	router
		.add_route("/files/{*path}", "serve_file".to_string())
		.unwrap();

	let result = router.match_path("/files/images/logo.png");
	assert!(result.is_some());
	let (handler_id, params) = result.unwrap();
	assert_eq!(handler_id, "serve_file");
	assert_eq!(params.get("path"), Some(&"images/logo.png".to_string()));
}

#[test]
fn test_radix_router_no_match() {
	let mut router = RadixRouter::new();
	router
		.add_route("/users/", "users_list".to_string())
		.unwrap();

	let result = router.match_path("/posts/");
	assert!(result.is_none());
}

#[test]
fn test_path_matcher_radix_tree_mode() {
	let mut matcher = PathMatcher::with_mode(MatchingMode::RadixTree);
	matcher
		.add_pattern(
			PathPattern::new(reinhardt_routers_macros::path!("/users/")).unwrap(),
			"users_list".to_string(),
		)
		.unwrap();
	matcher
		.add_pattern(
			PathPattern::new(reinhardt_routers_macros::path!("/users/{id}/")).unwrap(),
			"users_detail".to_string(),
		)
		.unwrap();

	assert_eq!(matcher.mode(), MatchingMode::RadixTree);

	let result = matcher.match_path("/users/123/");
	assert!(result.is_some());
	let (handler_id, params) = result.unwrap();
	assert_eq!(handler_id, "users_detail");
	assert_eq!(params.get("id"), Some(&"123".to_string()));
}

#[test]
fn test_path_matcher_enable_radix_tree() {
	let mut matcher = PathMatcher::new();
	matcher
		.add_pattern(
			PathPattern::new(reinhardt_routers_macros::path!("/users/")).unwrap(),
			"users_list".to_string(),
		)
		.unwrap();

	// Initially in linear mode
	assert_eq!(matcher.mode(), MatchingMode::Linear);

	// Enable radix tree mode
	matcher.enable_radix_tree().unwrap();
	assert_eq!(matcher.mode(), MatchingMode::RadixTree);

	// Should still work after mode switch
	let result = matcher.match_path("/users/");
	assert!(result.is_some());
}

#[test]
fn test_path_matcher_linear_vs_radix() {
	// Create two matchers with same routes
	let mut linear_matcher = PathMatcher::new();
	let mut radix_matcher = PathMatcher::with_mode(MatchingMode::RadixTree);

	for i in 1..=10 {
		let pattern = PathPattern::new(format!("/route{}/{{id}}/", i)).unwrap();
		linear_matcher
			.add_pattern(pattern.clone(), format!("handler_{}", i))
			.unwrap();
		radix_matcher
			.add_pattern(pattern, format!("handler_{}", i))
			.unwrap();
	}

	// Both should produce the same results
	for i in 1..=10 {
		let path = format!("/route{}/123/", i);
		let linear_result = linear_matcher.match_path(&path);
		let radix_result = radix_matcher.match_path(&path);

		assert_eq!(linear_result, radix_result);
		assert!(linear_result.is_some());
	}
}

// ===================================================================
// Path traversal prevention tests (Issue #425)
// ===================================================================

#[test]
fn test_path_type_rejects_traversal() {
	// Arrange
	let pattern = PathPattern::new("/files/{<path:filepath>}").unwrap();

	// Act & Assert - should reject `..` segments
	assert!(
		pattern
			.extract_params("/files/../../../etc/passwd")
			.is_none(),
		"Path type should reject directory traversal"
	);
	assert!(
		pattern
			.extract_params("/files/foo/../../etc/passwd")
			.is_none(),
		"Path type should reject embedded directory traversal"
	);
}

#[test]
fn test_path_type_allows_valid_paths() {
	// Arrange
	let pattern = PathPattern::new("/files/{<path:filepath>}").unwrap();

	// Act
	let result = pattern.extract_params("/files/images/logo.png");

	// Assert
	assert!(result.is_some());
	let params = result.unwrap();
	assert_eq!(params.get("filepath"), Some(&"images/logo.png".to_string()));
}

#[test]
fn test_path_type_allows_dotfiles() {
	// Arrange
	let pattern = PathPattern::new("/files/{<path:filepath>}").unwrap();

	// Act
	let result = pattern.extract_params("/files/.gitignore");

	// Assert
	assert!(result.is_some());
	let params = result.unwrap();
	assert_eq!(params.get("filepath"), Some(&".gitignore".to_string()));
}

#[test]
fn test_path_type_matcher_rejects_traversal() {
	// Arrange
	let mut matcher = PathMatcher::new();
	matcher
		.add_pattern(
			PathPattern::new("/files/{<path:filepath>}").unwrap(),
			"serve_file".to_string(),
		)
		.unwrap();

	// Act & Assert
	assert!(
		matcher.match_path("/files/../../../etc/passwd").is_none(),
		"PathMatcher should reject directory traversal in path params"
	);

	// Valid path should work
	let result = matcher.match_path("/files/css/style.css");
	assert!(result.is_some());
}

#[test]
fn test_validate_path_param_function() {
	// Normal paths should pass
	assert!(validate_path_param("images/logo.png"));
	assert!(validate_path_param("css/style.css"));
	assert!(validate_path_param(".gitignore"));
	assert!(validate_path_param("dir/.hidden"));

	// Traversal attacks should fail
	assert!(!validate_path_param("../etc/passwd"));
	assert!(!validate_path_param("foo/../../bar"));
	assert!(!validate_path_param(".."));
	assert!(!validate_path_param("foo/.."));

	// Null bytes should fail
	assert!(!validate_path_param("foo\0bar"));
}

// ===================================================================
// Encoded path traversal prevention tests (Issue #425)
// ===================================================================

#[test]
fn test_validate_path_param_rejects_encoded_traversal() {
	// Arrange & Act & Assert
	// Percent-encoded dot sequences (%2e = '.')
	assert!(!validate_path_param("%2e%2e/%2e%2e/etc/passwd"));
	assert!(!validate_path_param("foo/%2e%2e/bar"));
	assert!(!validate_path_param("%2E%2E/secret"));

	// Percent-encoded slash (%2f = '/')
	assert!(!validate_path_param("foo%2fbar"));
	assert!(!validate_path_param("..%2f..%2fetc%2fpasswd"));
	assert!(!validate_path_param("foo%2Fbar"));

	// Percent-encoded backslash (%5c = '\')
	assert!(!validate_path_param("foo%5cbar"));
	assert!(!validate_path_param("..%5C..%5Csecret"));

	// Percent-encoded null byte (%00)
	assert!(!validate_path_param("file%00.txt"));
}

#[test]
fn test_validate_path_param_rejects_absolute_paths() {
	// Arrange & Act & Assert
	assert!(!validate_path_param("/etc/passwd"));
	assert!(!validate_path_param("\\windows\\system32"));
}

#[test]
fn test_path_type_rejects_encoded_traversal() {
	// Arrange
	let pattern = PathPattern::new("/files/{<path:filepath>}").unwrap();

	// Act & Assert - percent-encoded traversal
	assert!(
		pattern
			.extract_params("/files/%2e%2e/%2e%2e/etc/passwd")
			.is_none(),
		"Path type should reject percent-encoded traversal"
	);
	assert!(
		pattern
			.extract_params("/files/..%2f..%2fetc%2fpasswd")
			.is_none(),
		"Path type should reject mixed encoded traversal"
	);
	assert!(
		pattern.extract_params("/files/foo%00bar").is_none(),
		"Path type should reject encoded null bytes"
	);
}

#[test]
fn test_path_type_rejects_absolute_path_param() {
	// Arrange
	let pattern = PathPattern::new("/files/{<path:filepath>}").unwrap();

	// Act & Assert - absolute paths in parameter value
	// Note: the regex `.+` will match, but validation rejects absolute paths
	assert!(
		pattern.extract_params("/files//etc/passwd").is_none(),
		"Path type should reject absolute path in parameter"
	);
}

#[test]
fn test_radix_tree_mode_rejects_traversal() {
	// Arrange
	let mut matcher = PathMatcher::with_mode(MatchingMode::RadixTree);
	matcher
		.add_pattern(
			PathPattern::new("/files/{<path:filepath>}").unwrap(),
			"serve_file".to_string(),
		)
		.unwrap();

	// Act & Assert - should reject traversal in RadixTree mode
	assert!(
		matcher.match_path("/files/../../../etc/passwd").is_none(),
		"RadixTree mode should reject directory traversal in path params"
	);
	assert!(
		matcher.match_path("/files/foo/../../etc/passwd").is_none(),
		"RadixTree mode should reject embedded directory traversal"
	);

	// Valid path should work
	let result = matcher.match_path("/files/css/style.css");
	assert!(result.is_some());
	let (handler_id, params) = result.unwrap();
	assert_eq!(handler_id, "serve_file");
	assert_eq!(params.get("filepath"), Some(&"css/style.css".to_string()));
}

#[test]
fn test_radix_tree_mode_rejects_encoded_traversal() {
	// Arrange
	let mut matcher = PathMatcher::with_mode(MatchingMode::RadixTree);
	matcher
		.add_pattern(
			PathPattern::new("/files/{<path:filepath>}").unwrap(),
			"serve_file".to_string(),
		)
		.unwrap();

	// Act & Assert - percent-encoded traversal
	assert!(
		matcher
			.match_path("/files/%2e%2e/%2e%2e/etc/passwd")
			.is_none(),
		"RadixTree mode should reject percent-encoded traversal"
	);
	assert!(
		matcher
			.match_path("/files/..%2f..%2fetc%2fpasswd")
			.is_none(),
		"RadixTree mode should reject mixed encoded traversal"
	);

	// Null byte injection
	assert!(
		matcher.match_path("/files/foo%00bar").is_none(),
		"RadixTree mode should reject encoded null bytes"
	);
}

// ===================================================================
// URL reversal parameter injection prevention tests (Issue #423)
// ===================================================================

#[test]
fn test_reverse_rejects_path_separator_injection() {
	// Arrange
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/{id}/")).unwrap();
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123/../../admin".to_string());

	// Act
	let result = pattern.reverse(&params);

	// Assert
	assert!(
		result.is_err(),
		"Reverse should reject path separators in parameter values"
	);
}

#[test]
fn test_reverse_rejects_query_string_injection() {
	// Arrange
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/{id}/")).unwrap();
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123?admin=true".to_string());

	// Act
	let result = pattern.reverse(&params);

	// Assert
	assert!(
		result.is_err(),
		"Reverse should reject query string delimiters in parameter values"
	);
}

#[test]
fn test_reverse_rejects_fragment_injection() {
	// Arrange
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/{id}/")).unwrap();
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123#fragment".to_string());

	// Act
	let result = pattern.reverse(&params);

	// Assert
	assert!(
		result.is_err(),
		"Reverse should reject fragment identifiers in parameter values"
	);
}

#[test]
fn test_reverse_rejects_encoded_injection() {
	// Arrange
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/{id}/")).unwrap();
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123%2f..%2f..%2fadmin".to_string());

	// Act
	let result = pattern.reverse(&params);

	// Assert
	assert!(
		result.is_err(),
		"Reverse should reject percent-encoded dangerous characters"
	);
}

#[test]
fn test_reverse_allows_safe_values() {
	// Arrange
	let pattern =
		PathPattern::new(reinhardt_routers_macros::path!("/users/{id}/posts/{slug}/")).unwrap();
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123".to_string());
	params.insert("slug".to_string(), "my-blog-post".to_string());

	// Act
	let result = pattern.reverse(&params);

	// Assert
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), "/users/123/posts/my-blog-post/");
}

#[test]
fn test_reverse_allows_unicode_values() {
	// Arrange
	let pattern = PathPattern::new(reinhardt_routers_macros::path!("/users/{name}/")).unwrap();
	let mut params = HashMap::new();
	params.insert("name".to_string(), "ユーザー".to_string());

	// Act
	let result = pattern.reverse(&params);

	// Assert
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), "/users/ユーザー/");
}

#[test]
fn test_validate_reverse_param_function() {
	// Arrange & Act & Assert

	// Safe values should pass
	assert!(validate_reverse_param("123"));
	assert!(validate_reverse_param("my-slug"));
	assert!(validate_reverse_param("foo_bar"));
	assert!(validate_reverse_param("ユーザー"));
	assert!(validate_reverse_param("hello-world-123"));

	// Path separators should fail
	assert!(!validate_reverse_param("foo/bar"));
	assert!(!validate_reverse_param("foo\\bar"));

	// URL-special characters should fail
	assert!(!validate_reverse_param("foo?bar=1"));
	assert!(!validate_reverse_param("foo#bar"));

	// Null bytes should fail
	assert!(!validate_reverse_param("foo\0bar"));

	// Encoded sequences should fail
	assert!(!validate_reverse_param("foo%2fbar"));
	assert!(!validate_reverse_param("foo%2ebar"));
	assert!(!validate_reverse_param("foo%5cbar"));
	assert!(!validate_reverse_param("foo%3fbar"));
	assert!(!validate_reverse_param("foo%23bar"));
	assert!(!validate_reverse_param("foo%00bar"));
}

// ===================================================================
// ReDoS prevention tests (Issue #430)
// ===================================================================

#[test]
fn test_pattern_rejects_excessive_length() {
	// Arrange: a pattern exceeding MAX_PATTERN_LENGTH (1024 bytes)
	let long_pattern = "/".to_string() + &"a".repeat(1025);

	// Act
	let result = PathPattern::new(long_pattern);

	// Assert
	assert!(result.is_err());
	assert!(
		result
			.unwrap_err()
			.contains("exceeds maximum allowed length")
	);
}

#[test]
fn test_pattern_accepts_within_length_limit() {
	// Arrange: a pattern within the limit
	let pattern = "/users/{id}/posts/{post_id}/";

	// Act
	let result = PathPattern::new(pattern);

	// Assert
	assert!(result.is_ok());
}

#[test]
fn test_pattern_rejects_at_boundary() {
	// Arrange: a pattern at exactly the boundary + 1
	let pattern = "/".to_string() + &"a/".repeat(512) + "end";
	if pattern.len() > MAX_PATTERN_LENGTH {
		// Act
		let result = PathPattern::new(pattern);

		// Assert
		assert!(result.is_err());
	}
}

// ===================================================================
// Path segment count limit tests (Issue #431)
// ===================================================================

#[test]
fn test_pattern_rejects_excessive_segments() {
	// Arrange: a pattern with more than MAX_PATH_SEGMENTS segments
	let segments: Vec<&str> = (0..35).map(|_| "seg").collect();
	let pattern = format!("/{}/", segments.join("/"));

	// Act
	let result = PathPattern::new(pattern);

	// Assert
	assert!(result.is_err());
	assert!(result.unwrap_err().contains("exceeding maximum"));
}

#[test]
fn test_pattern_accepts_within_segment_limit() {
	// Arrange: a pattern with few segments
	let pattern = "/a/b/c/d/e/";

	// Act
	let result = PathPattern::new(pattern);

	// Assert
	assert!(result.is_ok());
}

#[test]
fn test_pattern_accepts_at_segment_boundary() {
	// Arrange: a pattern at exactly the maximum segment count
	let segments: Vec<String> = (0..MAX_PATH_SEGMENTS - 2)
		.map(|i| format!("s{}", i))
		.collect();
	let pattern = format!("/{}/", segments.join("/"));

	// Act
	let result = PathPattern::new(&pattern);

	// Assert
	assert!(result.is_ok());
}

// ===================================================================
// PathMatcher fallible radix insertion (Issue #4345)
// ===================================================================

#[test]
fn test_add_pattern_returns_err_on_radix_conflict() {
	// Arrange — RadixTree mode so the second insert hits matchit's conflict
	// detection (matchit rejects two identical parameterized routes).
	let mut matcher = PathMatcher::with_mode(MatchingMode::RadixTree);
	matcher
		.add_pattern(
			PathPattern::new(reinhardt_routers_macros::path!("/items/{id}/")).unwrap(),
			"items_first".to_string(),
		)
		.expect("first insert must succeed");

	// Act — register a different name on the same matchit pattern. matchit
	// treats the two as a conflict and rejects the insertion.
	let result = matcher.add_pattern(
		PathPattern::new(reinhardt_routers_macros::path!("/items/{id}/")).unwrap(),
		"items_second".to_string(),
	);

	// Assert — error is surfaced instead of being silently dropped.
	assert!(result.is_err(), "expected radix insertion conflict, got Ok");
}

#[test]
fn test_enable_radix_tree_returns_err_on_conflict() {
	// Arrange — register two conflicting patterns in Linear mode (linear
	// mode does not enforce uniqueness, so this succeeds).
	let mut matcher = PathMatcher::new();
	matcher
		.add_pattern(
			PathPattern::new(reinhardt_routers_macros::path!("/items/{id}/")).unwrap(),
			"first".to_string(),
		)
		.unwrap();
	matcher
		.add_pattern(
			PathPattern::new(reinhardt_routers_macros::path!("/items/{id}/")).unwrap(),
			"second".to_string(),
		)
		.unwrap();

	// Act — switching to RadixTree mode must rebuild the radix router from
	// the existing patterns; the duplicate triggers a conflict.
	let result = matcher.enable_radix_tree();

	// Assert — error is propagated and the matcher stays in Linear mode so
	// Linear / RadixTree views can never silently diverge.
	assert!(result.is_err(), "expected radix rebuild conflict, got Ok");
	assert_eq!(
		matcher.mode(),
		MatchingMode::Linear,
		"matcher must remain in Linear mode after a failed upgrade"
	);
}
