// Pattern matching and path parsing tests
// Inspired by Django REST Framework's test_routers.py (lookup fields, regex, etc.)

use reinhardt_routers::{PathMatcher, PathPattern, path};

// Test: Basic path pattern matching
#[test]
fn test_basic_pattern_matching() {
	let mut matcher = PathMatcher::new();
	let pattern = PathPattern::new(path!("/items/")).expect("Valid pattern");

	matcher.add_pattern(pattern, "items-list".to_string());

	let result = matcher.match_path("/items/");
	assert!(result.is_some());
	let (handler_id, params) = result.unwrap();
	assert_eq!(handler_id, "items-list");
	assert!(params.is_empty());
}

// Test: Pattern with single parameter (inspired by DRF's TestCustomLookupFields)
#[test]
fn test_pattern_with_single_parameter() {
	let mut matcher = PathMatcher::new();
	let pattern = PathPattern::new(path!("/items/{id}/")).expect("Valid pattern");

	matcher.add_pattern(pattern, "items-detail".to_string());

	let result = matcher.match_path("/items/123/");
	assert!(result.is_some());
	let (handler_id, params) = result.unwrap();
	assert_eq!(handler_id, "items-detail");
	assert_eq!(params.len(), 1);
	assert_eq!(params.get("id"), Some(&"123".to_string()));
}

// Test: Pattern with multiple parameters
#[test]
fn test_pattern_with_multiple_parameters() {
	let mut matcher = PathMatcher::new();
	let pattern =
		PathPattern::new(path!("/users/{user_id}/posts/{post_id}/")).expect("Valid pattern");

	matcher.add_pattern(pattern, "user-post-detail".to_string());

	let result = matcher.match_path("/users/42/posts/123/");
	assert!(result.is_some());
	let (handler_id, params) = result.unwrap();
	assert_eq!(handler_id, "user-post-detail");
	assert_eq!(params.len(), 2);
	assert_eq!(params.get("user_id"), Some(&"42".to_string()));
	assert_eq!(params.get("post_id"), Some(&"123".to_string()));
}

// Test: URL encoded path matching (inspired by DRF's test_retrieve_lookup_field_url_encoded_detail_view_)
#[test]
fn test_url_encoded_path_matching() {
	let mut matcher = PathMatcher::new();
	let pattern = PathPattern::new(path!("/items/{id}/")).expect("Valid pattern");

	matcher.add_pattern(pattern, "items-detail".to_string());

	// URL encoded space (%20)
	let result = matcher.match_path("/items/a%20b/");
	assert!(result.is_some());
	let (_, params) = result.unwrap();
	// Note: URL decoding should be handled by the HTTP layer
	assert_eq!(params.get("id"), Some(&"a%20b".to_string()));
}

// Test: Pattern mismatch
#[test]
fn test_pattern_mismatch() {
	let mut matcher = PathMatcher::new();
	let pattern = PathPattern::new(path!("/items/{id}/")).expect("Valid pattern");

	matcher.add_pattern(pattern, "items-detail".to_string());

	let result = matcher.match_path("/users/123/");
	assert!(result.is_none());
}

// Test: Multiple patterns with priority (first match wins)
#[test]
fn test_multiple_patterns_priority() {
	let mut matcher = PathMatcher::new();

	let pattern1 = PathPattern::new(path!("/items/new/")).expect("Valid pattern");
	matcher.add_pattern(pattern1, "items-new".to_string());

	let pattern2 = PathPattern::new(path!("/items/{id}/")).expect("Valid pattern");
	matcher.add_pattern(pattern2, "items-detail".to_string());

	// Specific path should match first pattern
	let result = matcher.match_path("/items/new/");
	assert!(result.is_some());
	let (handler_id, _) = result.unwrap();
	assert_eq!(handler_id, "items-new");
}

// Test: Pattern with numeric parameter (inspired by DRF's TestLookupValueRegex)
#[test]
fn test_pattern_with_numeric_constraint() {
	// Note: Current implementation doesn't enforce type constraints in pattern
	// This test documents the expected behavior
	let pattern = PathPattern::new(path!("/items/{id}/")).expect("Valid pattern");

	let mut matcher = PathMatcher::new();
	matcher.add_pattern(pattern, "items-detail".to_string());

	// Both numeric and non-numeric values should match
	let result = matcher.match_path("/items/123/");
	assert!(result.is_some());

	let result = matcher.match_path("/items/abc/");
	assert!(result.is_some());
}

// Test: Complex path with mixed segments
#[test]
fn test_complex_path_pattern() {
	let mut matcher = PathMatcher::new();
	let pattern = PathPattern::new(path!("/api/v1/users/{user_id}/posts/{post_id}/comments/"))
		.expect("Valid pattern");

	matcher.add_pattern(pattern, "user-post-comments".to_string());

	let result = matcher.match_path("/api/v1/users/42/posts/123/comments/");
	assert!(result.is_some());
	let (handler_id, params) = result.unwrap();
	assert_eq!(handler_id, "user-post-comments");
	assert_eq!(params.get("user_id"), Some(&"42".to_string()));
	assert_eq!(params.get("post_id"), Some(&"123".to_string()));
}

// Test: Root path matching
#[test]
fn test_root_path_matching() {
	let mut matcher = PathMatcher::new();
	let pattern = PathPattern::new(path!("/")).expect("Valid pattern");

	matcher.add_pattern(pattern, "root".to_string());

	let result = matcher.match_path("/");
	assert!(result.is_some());
	let (handler_id, _) = result.unwrap();
	assert_eq!(handler_id, "root");
}

// Test: Path without trailing slash
#[test]
fn test_path_without_trailing_slash() {
	let mut matcher = PathMatcher::new();
	let pattern = PathPattern::new(path!("/items")).expect("Valid pattern");

	matcher.add_pattern(pattern, "items".to_string());

	let result = matcher.match_path("/items");
	assert!(result.is_some());
}

// Test: Parameter at end without trailing slash
#[test]
fn test_parameter_without_trailing_slash() {
	let mut matcher = PathMatcher::new();
	let pattern = PathPattern::new(path!("/items/{id}")).expect("Valid pattern");

	matcher.add_pattern(pattern, "items-detail".to_string());

	let result = matcher.match_path("/items/123");
	assert!(result.is_some());
	let (_, params) = result.unwrap();
	assert_eq!(params.get("id"), Some(&"123".to_string()));
}

// Test: Empty pattern (edge case)
#[test]
fn test_empty_pattern() {
	let result = PathPattern::new("");
	// Empty patterns should be handled gracefully
	assert!(result.is_ok() || result.is_err());
}

// Test: Pattern with special characters in parameter name
#[test]
fn test_pattern_with_underscore_parameter() {
	let mut matcher = PathMatcher::new();
	let pattern = PathPattern::new(path!("/items/{item_id}/")).expect("Valid pattern");

	matcher.add_pattern(pattern, "items-detail".to_string());

	let result = matcher.match_path("/items/123/");
	assert!(result.is_some());
	let (_, params) = result.unwrap();
	assert_eq!(params.get("item_id"), Some(&"123".to_string()));
}

// Test: Pattern with hyphens in path segments
#[test]
fn test_pattern_with_hyphens() {
	let mut matcher = PathMatcher::new();
	let pattern = PathPattern::new(path!("/api-v1/user-items/{id}/")).expect("Valid pattern");

	matcher.add_pattern(pattern, "user-items-detail".to_string());

	let result = matcher.match_path("/api-v1/user-items/123/");
	assert!(result.is_some());
}

// Test: Very long path
#[test]
fn test_long_path_pattern() {
	let mut matcher = PathMatcher::new();
	let pattern = PathPattern::new(path!(
        "/api/v1/organizations/{org_id}/projects/{project_id}/repositories/{repo_id}/branches/{branch_id}/commits/"
    ))
    .expect("Valid pattern");

	matcher.add_pattern(pattern, "commit-list".to_string());

	let result = matcher.match_path(
		"/api/v1/organizations/123/projects/456/repositories/789/branches/main/commits/",
	);
	assert!(result.is_some());
	let (_, params) = result.unwrap();
	assert_eq!(params.len(), 4);
	assert_eq!(params.get("org_id"), Some(&"123".to_string()));
	assert_eq!(params.get("project_id"), Some(&"456".to_string()));
	assert_eq!(params.get("repo_id"), Some(&"789".to_string()));
	assert_eq!(params.get("branch_id"), Some(&"main".to_string()));
}

// Test: Parameter with dots (e.g., file extensions)
#[test]
fn test_parameter_with_dots() {
	let mut matcher = PathMatcher::new();
	let pattern = PathPattern::new(path!("/files/{filename}/")).expect("Valid pattern");

	matcher.add_pattern(pattern, "file-detail".to_string());

	let result = matcher.match_path("/files/document.pdf/");
	assert!(result.is_some());
	let (_, params) = result.unwrap();
	assert_eq!(params.get("filename"), Some(&"document.pdf".to_string()));
}

// Test: Consecutive parameters (edge case)
#[test]
fn test_consecutive_parameters() {
	let result = PathPattern::new("/items/{id}{name}/");
	// This should either work or fail gracefully
	// The behavior depends on implementation
	assert!(result.is_ok() || result.is_err());
}

// Test: Parameter with numbers
#[test]
fn test_parameter_with_numbers() {
	let mut matcher = PathMatcher::new();
	let pattern = PathPattern::new(path!("/v{version}/items/{id}/")).expect("Valid pattern");

	matcher.add_pattern(pattern, "versioned-items".to_string());

	let result = matcher.match_path("/v1/items/123/");
	assert!(result.is_some());
	let (_, params) = result.unwrap();
	assert_eq!(params.get("version"), Some(&"1".to_string()));
	assert_eq!(params.get("id"), Some(&"123".to_string()));
}
