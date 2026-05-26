//! Tests for the `reverse` submodules.

use super::super::Route;
use super::reverser::UrlReverser;
use super::runtime::{
	ReverseError, extract_param_names, try_reverse_single_pass, try_reverse_with_aho_corasick,
};
use crate::routers_macros::path;
use async_trait::async_trait;
use reinhardt_core::exception::Error;
use reinhardt_http::{Handler, Request, Response, Result as CoreResult};
use rstest::rstest;
use std::collections::HashMap;
use std::sync::Arc;

// Simple test handler
struct TestHandler;

#[async_trait]
impl Handler for TestHandler {
	async fn handle(&self, _request: Request) -> CoreResult<Response> {
		Ok(Response::ok())
	}
}

#[rstest]
fn test_reverse_simple_path() {
	let mut reverser = UrlReverser::new();

	let mut route = Route::new(path!("/users/"), Arc::new(TestHandler));
	route.name = Some("users-list".to_string());

	reverser.register(route).unwrap();

	let url = reverser.reverse("users-list", &HashMap::new()).unwrap();
	assert_eq!(url, path!("/users/"));
}

#[rstest]
fn test_reverse_with_parameters() {
	let mut reverser = UrlReverser::new();

	let mut route = Route::new(path!("/users/{id}/"), Arc::new(TestHandler));
	route.name = Some("users-detail".to_string());

	reverser.register(route).unwrap();

	let mut params = HashMap::new();
	params.insert("id".to_string(), "123".to_string());

	let url = reverser.reverse("users-detail", &params).unwrap();
	assert_eq!(url, "/users/123/");
}

#[rstest]
fn test_reverse_with_namespace() {
	let mut reverser = UrlReverser::new();

	let mut route =
		Route::new(path!("/users/{id}/"), Arc::new(TestHandler)).with_namespace("users");
	route.name = Some("detail".to_string());

	reverser.register(route).unwrap();

	let mut params = HashMap::new();
	params.insert("id".to_string(), "456".to_string());

	let url = reverser.reverse("users:detail", &params).unwrap();
	assert_eq!(url, "/users/456/");
}

#[rstest]
fn test_reverse_missing_parameter() {
	let mut reverser = UrlReverser::new();

	let mut route = Route::new(path!("/users/{id}/"), Arc::new(TestHandler));
	route.name = Some("users-detail".to_string());

	reverser.register(route).unwrap();

	let result = reverser.reverse("users-detail", &HashMap::new());
	assert!(result.is_err());
	assert!(matches!(result.unwrap_err(), ReverseError::Validation(_)));
}

#[rstest]
fn test_reverse_not_found() {
	let reverser = UrlReverser::new();

	let result = reverser.reverse("nonexistent", &HashMap::new());
	assert!(result.is_err());
	assert!(matches!(result.unwrap_err(), ReverseError::NotFound(_)));
}

#[rstest]
fn test_reverse_with_helper() {
	let mut reverser = UrlReverser::new();

	let mut route = Route::new(path!("/users/{id}/posts/{post_id}/"), Arc::new(TestHandler));
	route.name = Some("user-posts".to_string());

	reverser.register(route).unwrap();

	let url = reverser
		.reverse_with("user-posts", &[("id", "123"), ("post_id", "456")])
		.unwrap();

	assert_eq!(url, "/users/123/posts/456/");
}

#[rstest]
fn test_has_route() {
	let mut reverser = UrlReverser::new();

	let mut route = Route::new(path!("/users/"), Arc::new(TestHandler));
	route.name = Some("users-list".to_string());

	reverser.register(route).unwrap();

	assert!(reverser.has_route("users-list"));
	assert!(!reverser.has_route("nonexistent"));
}

// Single-pass algorithm tests
#[rstest]
fn test_single_pass_basic() {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123".to_string());

	let result = try_reverse_single_pass("/users/{id}/", &params).unwrap();
	assert_eq!(result, "/users/123/");
}

#[rstest]
fn test_single_pass_multiple_params() {
	let mut params = HashMap::new();
	params.insert("user_id".to_string(), "42".to_string());
	params.insert("post_id".to_string(), "100".to_string());

	let result = try_reverse_single_pass("/users/{user_id}/posts/{post_id}/", &params).unwrap();
	assert_eq!(result, "/users/42/posts/100/");
}

#[rstest]
fn test_single_pass_many_params() {
	// Test with 10+ parameters to demonstrate performance improvement
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

	let pattern = "/api/{p1}/{p2}/{p3}/{p4}/{p5}/{p6}/{p7}/{p8}/{p9}/{p10}/";
	let result = try_reverse_single_pass(pattern, &params).unwrap();
	assert_eq!(result, "/api/v1/v2/v3/v4/v5/v6/v7/v8/v9/v10/");
}

#[rstest]
fn test_single_pass_missing_param() {
	let params = HashMap::new();

	let result = try_reverse_single_pass("/users/{id}/", &params).unwrap();
	// Missing parameter should preserve placeholder
	assert_eq!(result, "/users/{id}/");
}

#[rstest]
fn test_single_pass_no_params() {
	let params = HashMap::new();

	let result = try_reverse_single_pass("/users/", &params).unwrap();
	assert_eq!(result, "/users/");
}

#[rstest]
fn test_single_pass_empty_pattern() {
	let params = HashMap::new();

	let result = try_reverse_single_pass("", &params).unwrap();
	assert_eq!(result, "");
}

#[rstest]
fn test_single_pass_consecutive_params() {
	let mut params = HashMap::new();
	params.insert("a".to_string(), "1".to_string());
	params.insert("b".to_string(), "2".to_string());

	let result = try_reverse_single_pass("/{a}{b}/", &params).unwrap();
	assert_eq!(result, "/12/");
}

#[rstest]
fn test_single_pass_special_chars_in_values() {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "foo-bar_123".to_string());

	let result = try_reverse_single_pass("/items/{id}/", &params).unwrap();
	assert_eq!(result, "/items/foo-bar_123/");
}

#[rstest]
fn test_single_pass_numeric_values() {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "12345".to_string());

	let result = try_reverse_single_pass("/items/{id}/", &params).unwrap();
	assert_eq!(result, "/items/12345/");
}

#[rstest]
fn test_single_pass_empty_value() {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "".to_string());

	let result = try_reverse_single_pass("/items/{id}/", &params).unwrap();
	assert_eq!(result, "/items//");
}

#[rstest]
fn test_single_pass_pattern_with_no_placeholder() {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123".to_string());

	let result = try_reverse_single_pass("/static/path/", &params).unwrap();
	assert_eq!(result, "/static/path/");
}

#[rstest]
fn test_single_pass_mixed_content() {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123".to_string());
	params.insert("action".to_string(), "edit".to_string());

	let result = try_reverse_single_pass("/items/{id}/actions/{action}/execute", &params).unwrap();
	assert_eq!(result, "/items/123/actions/edit/execute");
}

#[rstest]
fn test_single_pass_param_at_start() {
	let mut params = HashMap::new();
	params.insert("lang".to_string(), "ja".to_string());

	let result = try_reverse_single_pass("{lang}/users/", &params).unwrap();
	assert_eq!(result, "ja/users/");
}

#[rstest]
fn test_single_pass_param_at_end() {
	let mut params = HashMap::new();
	params.insert("format".to_string(), "json".to_string());

	let result = try_reverse_single_pass("/api/data.{format}", &params).unwrap();
	assert_eq!(result, "/api/data.json");
}

#[rstest]
fn test_single_pass_unicode_values() {
	let mut params = HashMap::new();
	params.insert("name".to_string(), "ユーザー".to_string());

	let result = try_reverse_single_pass("/users/{name}/", &params).unwrap();
	assert_eq!(result, "/users/ユーザー/");
}

#[rstest]
fn test_single_pass_long_value() {
	let mut params = HashMap::new();
	let long_id = "a".repeat(1000);
	params.insert("id".to_string(), long_id.clone());

	let result = try_reverse_single_pass("/items/{id}/", &params).unwrap();
	assert_eq!(result, format!("/items/{}/", long_id));
}

// Aho-Corasick algorithm tests
#[rstest]
fn test_aho_corasick_basic() {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123".to_string());

	let result = try_reverse_with_aho_corasick("/users/{id}/", &params).unwrap();
	assert_eq!(result, "/users/123/");
}

#[rstest]
fn test_aho_corasick_multiple_params() {
	let mut params = HashMap::new();
	params.insert("user_id".to_string(), "42".to_string());
	params.insert("post_id".to_string(), "100".to_string());

	let result =
		try_reverse_with_aho_corasick("/users/{user_id}/posts/{post_id}/", &params).unwrap();
	assert_eq!(result, "/users/42/posts/100/");
}

#[rstest]
fn test_aho_corasick_many_params() {
	let mut params = HashMap::new();
	for i in 1..=10 {
		params.insert(format!("p{}", i), format!("v{}", i));
	}

	let pattern = "/api/{p1}/{p2}/{p3}/{p4}/{p5}/{p6}/{p7}/{p8}/{p9}/{p10}/";
	let result = try_reverse_with_aho_corasick(pattern, &params).unwrap();
	assert_eq!(result, "/api/v1/v2/v3/v4/v5/v6/v7/v8/v9/v10/");
}

#[rstest]
fn test_aho_corasick_missing_param() {
	let params = HashMap::new();

	let result = try_reverse_with_aho_corasick("/users/{id}/", &params).unwrap();
	// Missing parameter should preserve placeholder
	assert_eq!(result, "/users/{id}/");
}

#[rstest]
fn test_aho_corasick_no_params() {
	let params = HashMap::new();

	let result = try_reverse_with_aho_corasick("/users/", &params).unwrap();
	assert_eq!(result, "/users/");
}

#[rstest]
fn test_aho_corasick_empty_pattern() {
	let params = HashMap::new();

	let result = try_reverse_with_aho_corasick("", &params).unwrap();
	assert_eq!(result, "");
}

#[rstest]
fn test_aho_corasick_consecutive_params() {
	let mut params = HashMap::new();
	params.insert("a".to_string(), "1".to_string());
	params.insert("b".to_string(), "2".to_string());

	let result = try_reverse_with_aho_corasick("/{a}{b}/", &params).unwrap();
	assert_eq!(result, "/12/");
}

#[rstest]
fn test_aho_corasick_special_chars_in_values() {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "foo-bar_123".to_string());

	let result = try_reverse_with_aho_corasick("/items/{id}/", &params).unwrap();
	assert_eq!(result, "/items/foo-bar_123/");
}

#[rstest]
fn test_aho_corasick_unicode() {
	let mut params = HashMap::new();
	params.insert("name".to_string(), "ユーザー".to_string());

	let result = try_reverse_with_aho_corasick("/users/{name}/", &params).unwrap();
	assert_eq!(result, "/users/ユーザー/");
}

#[rstest]
fn test_extract_param_names_basic() {
	let names = extract_param_names("/users/{id}/");
	assert_eq!(names, vec!["id"]);
}

#[rstest]
fn test_extract_param_names_multiple() {
	let names = extract_param_names("/users/{user_id}/posts/{post_id}/");
	assert_eq!(names, vec!["user_id", "post_id"]);
}

#[rstest]
fn test_extract_param_names_no_params() {
	let names = extract_param_names("/users/");
	assert!(names.is_empty());
}

#[rstest]
fn test_extract_param_names_consecutive() {
	let names = extract_param_names("/{a}{b}/");
	assert_eq!(names, vec!["a", "b"]);
}

#[rstest]
fn test_aho_corasick_vs_single_pass_consistency() {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123".to_string());
	params.insert("action".to_string(), "edit".to_string());

	let pattern = "/users/{id}/actions/{action}/";

	let result_single = try_reverse_single_pass(pattern, &params).unwrap();
	let result_aho = try_reverse_with_aho_corasick(pattern, &params).unwrap();

	assert_eq!(
		result_single, result_aho,
		"Both algorithms should produce identical results"
	);
}

#[rstest]
fn test_aho_corasick_complex_pattern() {
	let mut params = HashMap::new();
	params.insert("org".to_string(), "myorg".to_string());
	params.insert("repo".to_string(), "myrepo".to_string());
	params.insert("branch".to_string(), "main".to_string());
	params.insert("file".to_string(), "README.md".to_string());

	let pattern = "/repos/{org}/{repo}/contents/{file}?ref={branch}";
	let result = try_reverse_with_aho_corasick(pattern, &params).unwrap();
	assert_eq!(result, "/repos/myorg/myrepo/contents/README.md?ref=main");
}

#[rstest]
fn test_performance_comparison_many_params() {
	use std::time::Instant;

	// Create a pattern with 20 parameters
	let mut params = HashMap::new();
	let mut pattern_parts = vec!["/api".to_string()];
	for i in 1..=20 {
		params.insert(format!("p{}", i), format!("v{}", i));
		pattern_parts.push(format!("{{p{}}}", i));
	}
	let pattern = pattern_parts.join("/") + "/";

	// Warm up
	for _ in 0..10 {
		let _ = try_reverse_single_pass(&pattern, &params);
		let _ = try_reverse_with_aho_corasick(&pattern, &params);
	}

	// Measure single_pass
	let start = Instant::now();
	for _ in 0..1000 {
		let _ = try_reverse_single_pass(&pattern, &params);
	}
	let single_pass_duration = start.elapsed();

	// Measure aho_corasick
	let start = Instant::now();
	for _ in 0..1000 {
		let _ = try_reverse_with_aho_corasick(&pattern, &params);
	}
	let aho_corasick_duration = start.elapsed();

	// Verify both produce same result
	let result_single = try_reverse_single_pass(&pattern, &params).unwrap();
	let result_aho = try_reverse_with_aho_corasick(&pattern, &params).unwrap();
	assert_eq!(result_single, result_aho);

	// Print performance results (for informational purposes)
	println!("\nPerformance comparison (20 params, 1000 iterations):");
	println!("  Single-pass: {:?}", single_pass_duration);
	println!("  Aho-Corasick: {:?}", aho_corasick_duration);

	if aho_corasick_duration < single_pass_duration {
		let improvement =
			single_pass_duration.as_nanos() as f64 / aho_corasick_duration.as_nanos() as f64;
		println!("  Improvement: {:.2}x faster", improvement);
	}

	// Note: This test doesn't fail, it's for informational purposes
	// Actual performance may vary based on pattern complexity and parameter count
}

#[rstest]
fn test_performance_few_params() {
	use std::time::Instant;

	// Test with fewer parameters (where single-pass might be faster due to overhead)
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123".to_string());
	params.insert("action".to_string(), "edit".to_string());
	let pattern = "/users/{id}/actions/{action}/";

	// Warm up
	for _ in 0..10 {
		let _ = try_reverse_single_pass(pattern, &params);
		let _ = try_reverse_with_aho_corasick(pattern, &params);
	}

	// Measure single_pass
	let start = Instant::now();
	for _ in 0..10000 {
		let _ = try_reverse_single_pass(pattern, &params);
	}
	let single_pass_duration = start.elapsed();

	// Measure aho_corasick
	let start = Instant::now();
	for _ in 0..10000 {
		let _ = try_reverse_with_aho_corasick(pattern, &params);
	}
	let aho_corasick_duration = start.elapsed();

	// Verify both produce same result
	let result_single = try_reverse_single_pass(pattern, &params).unwrap();
	let result_aho = try_reverse_with_aho_corasick(pattern, &params).unwrap();
	assert_eq!(result_single, result_aho);

	// Print performance results
	println!("\nPerformance comparison (2 params, 10000 iterations):");
	println!("  Single-pass: {:?}", single_pass_duration);
	println!("  Aho-Corasick: {:?}", aho_corasick_duration);
}

// ===================================================================
// URL reversal parameter injection prevention tests (Issue #423)
// ===================================================================

#[rstest]
fn test_reverser_rejects_path_separator_injection() {
	// Arrange
	let mut reverser = UrlReverser::new();
	let mut route = Route::new(path!("/users/{id}/"), Arc::new(TestHandler));
	route.name = Some("users-detail".to_string());
	reverser.register(route).unwrap();

	let mut params = HashMap::new();
	params.insert("id".to_string(), "123/../../admin".to_string());

	// Act
	let result = reverser.reverse("users-detail", &params);

	// Assert
	assert!(
		result.is_err(),
		"Reverser should reject path separator injection"
	);
}

#[rstest]
fn test_reverser_rejects_query_injection() {
	// Arrange
	let mut reverser = UrlReverser::new();
	let mut route = Route::new(path!("/users/{id}/"), Arc::new(TestHandler));
	route.name = Some("users-detail".to_string());
	reverser.register(route).unwrap();

	let mut params = HashMap::new();
	params.insert("id".to_string(), "123?admin=true".to_string());

	// Act
	let result = reverser.reverse("users-detail", &params);

	// Assert
	assert!(
		result.is_err(),
		"Reverser should reject query string injection"
	);
}

#[rstest]
fn test_reverser_rejects_fragment_injection() {
	// Arrange
	let mut reverser = UrlReverser::new();
	let mut route = Route::new(path!("/users/{id}/"), Arc::new(TestHandler));
	route.name = Some("users-detail".to_string());
	reverser.register(route).unwrap();

	let mut params = HashMap::new();
	params.insert("id".to_string(), "123#admin".to_string());

	// Act
	let result = reverser.reverse("users-detail", &params);

	// Assert
	assert!(result.is_err(), "Reverser should reject fragment injection");
}

#[rstest]
fn test_reverser_rejects_encoded_injection() {
	// Arrange
	let mut reverser = UrlReverser::new();
	let mut route = Route::new(path!("/users/{id}/"), Arc::new(TestHandler));
	route.name = Some("users-detail".to_string());
	reverser.register(route).unwrap();

	let mut params = HashMap::new();
	params.insert("id".to_string(), "123%2f..%2fadmin".to_string());

	// Act
	let result = reverser.reverse("users-detail", &params);

	// Assert
	assert!(
		result.is_err(),
		"Reverser should reject percent-encoded injection"
	);
}

#[rstest]
fn test_reverser_allows_safe_values() {
	// Arrange
	let mut reverser = UrlReverser::new();
	let mut route = Route::new(path!("/users/{id}/"), Arc::new(TestHandler));
	route.name = Some("users-detail".to_string());
	reverser.register(route).unwrap();

	let mut params = HashMap::new();
	params.insert("id".to_string(), "456".to_string());

	// Act
	let result = reverser.reverse("users-detail", &params);

	// Assert
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), "/users/456/");
}

#[rstest]
fn test_single_pass_rejects_path_separator() {
	// Arrange
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123/admin".to_string());

	// Act
	let result = try_reverse_single_pass("/users/{id}/", &params);

	// Assert
	let err = result.unwrap_err();
	assert!(matches!(err, Error::Validation(_)));
}

#[rstest]
fn test_aho_corasick_rejects_path_separator() {
	// Arrange
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123/admin".to_string());

	// Act
	let result = try_reverse_with_aho_corasick("/users/{id}/", &params);

	// Assert
	let err = result.unwrap_err();
	assert!(matches!(err, Error::Validation(_)));
}

#[rstest]
fn test_reverse_with_helper_rejects_injection() {
	// Arrange
	let mut reverser = UrlReverser::new();
	let mut route = Route::new(path!("/users/{id}/"), Arc::new(TestHandler));
	route.name = Some("users-detail".to_string());
	reverser.register(route).unwrap();

	// Act
	let result = reverser.reverse_with("users-detail", &[("id", "123?admin=true")]);

	// Assert
	assert!(
		result.is_err(),
		"reverse_with should reject query injection"
	);
}

// ===================================================================
// Duplicate route name detection tests (Issue #3462)
// ===================================================================

#[rstest]
fn test_register_duplicate_name_returns_error() {
	// Arrange
	let mut reverser = UrlReverser::new();
	let mut route_a = Route::new(path!("/users/"), Arc::new(TestHandler));
	route_a.name = Some("users-list".to_string());
	let mut route_b = Route::new(path!("/people/"), Arc::new(TestHandler));
	route_b.name = Some("users-list".to_string());

	// Act
	let first = reverser.register(route_a);
	let second = reverser.register(route_b);

	// Assert
	assert!(first.is_ok());
	assert!(second.is_err());
	let err = second.unwrap_err();
	assert!(err.contains("Duplicate route name 'users-list'"));
	assert!(err.contains("/people/"));
	assert!(err.contains("/users/"));
}

#[rstest]
fn test_register_path_duplicate_name_returns_error() {
	// Arrange
	let mut reverser = UrlReverser::new();

	// Act
	let first = reverser.register_path("v1:users:detail", "/api/v1/users/{id}/");
	let second = reverser.register_path("v1:users:detail", "/api/v1/people/{id}/");

	// Assert
	assert!(first.is_ok());
	assert!(second.is_err());
	let err = second.unwrap_err();
	assert!(err.contains("Duplicate route name 'v1:users:detail'"));
}

#[rstest]
fn test_register_unique_names_succeeds() {
	// Arrange
	let mut reverser = UrlReverser::new();
	let mut route_a = Route::new(path!("/users/"), Arc::new(TestHandler));
	route_a.name = Some("users-list".to_string());
	let mut route_b = Route::new(path!("/posts/"), Arc::new(TestHandler));
	route_b.name = Some("posts-list".to_string());

	// Act & Assert
	assert!(reverser.register(route_a).is_ok());
	assert!(reverser.register(route_b).is_ok());
}

// --- Name alias tests (Issue #3526) ---

#[rstest]
fn test_alias_resolves_to_canonical() {
	// Arrange
	let mut reverser = UrlReverser::new();
	reverser
		.register_path("users:list", path!("/users/"))
		.unwrap();
	reverser.add_name_alias("user_list", "users:list");

	// Act
	let result = reverser
		.reverse_with("user_list", &[] as &[(&str, &str)])
		.unwrap();

	// Assert
	assert_eq!(result, path!("/users/"));
}

#[rstest]
fn test_alias_and_canonical_return_same_result() {
	// Arrange
	let mut reverser = UrlReverser::new();
	reverser
		.register_path("users:detail", path!("/users/{id}/"))
		.unwrap();
	reverser.add_name_alias("user_detail", "users:detail");

	// Act
	let canonical = reverser
		.reverse_with("users:detail", &[("id", "42")])
		.unwrap();
	let aliased = reverser
		.reverse_with("user_detail", &[("id", "42")])
		.unwrap();

	// Assert
	assert_eq!(canonical, aliased);
}

#[rstest]
fn test_alias_target_not_found() {
	// Arrange
	let mut reverser = UrlReverser::new();
	reverser.add_name_alias("old_name", "nonexistent:route");

	// Act
	let result = reverser.reverse_with("old_name", &[] as &[(&str, &str)]);

	// Assert
	assert!(result.is_err());
}

#[rstest]
fn test_duplicate_alias_last_write_wins() {
	// Arrange
	let mut reverser = UrlReverser::new();
	reverser
		.register_path("users:list", path!("/users/"))
		.unwrap();
	reverser
		.register_path("posts:list", path!("/posts/"))
		.unwrap();

	// Act
	reverser.add_name_alias("the_list", "users:list");
	reverser.add_name_alias("the_list", "posts:list");
	let result = reverser
		.reverse_with("the_list", &[] as &[(&str, &str)])
		.unwrap();

	// Assert
	assert_eq!(result, path!("/posts/"));
}

#[rstest]
fn test_alias_does_not_shadow_canonical_route() {
	// Arrange — register route "X", then add alias "X" -> "Y" (same key as route)
	let mut reverser = UrlReverser::new();
	reverser
		.register_path("users:list", path!("/users/"))
		.unwrap();
	reverser
		.register_path("posts:list", path!("/posts/"))
		.unwrap();
	// Alias "users:list" -> "posts:list" should NOT shadow the real "users:list" route
	reverser.add_name_alias("users:list", "posts:list");

	// Act
	let result = reverser.reverse("users:list", &HashMap::new()).unwrap();

	// Assert — canonical route takes precedence over alias
	assert_eq!(result, "/users/");
}

#[rstest]
fn test_alias_with_params() {
	// Arrange
	let mut reverser = UrlReverser::new();
	reverser
		.register_path("users:detail", path!("/users/{id}/"))
		.unwrap();
	reverser.add_name_alias("user_detail", "users:detail");

	// Act
	let result = reverser
		.reverse_with("user_detail", &[("id", "foo/bar")])
		.unwrap_err();

	// Assert — path separators are rejected by validation
	assert!(
		matches!(result, Error::Validation(_)),
		"expected Validation error for dangerous param"
	);
}

// ===================================================================
// Fallible reverse helpers (Issue #4345)
// ===================================================================

#[rstest]
fn test_try_reverse_with_aho_corasick_returns_err_on_invalid_param() {
	// Arrange — a dangerous parameter value containing a path separator
	let mut params = HashMap::new();
	params.insert("id".to_string(), "foo/bar".to_string());

	// Act
	let result = try_reverse_with_aho_corasick("/users/{id}/", &params);

	// Assert — returns Err instead of panicking
	let err = result.expect_err("dangerous param must return Err");
	assert!(
		matches!(err, Error::Validation(_)),
		"expected Validation error, got: {:?}",
		err
	);
}

#[rstest]
fn test_try_reverse_single_pass_returns_err_on_invalid_param() {
	// Arrange — a dangerous parameter value containing a query delimiter
	let mut params = HashMap::new();
	params.insert("id".to_string(), "abc?evil=1".to_string());

	// Act
	let result = try_reverse_single_pass("/users/{id}/", &params);

	// Assert — returns Err instead of panicking
	let err = result.expect_err("dangerous param must return Err");
	assert!(
		matches!(err, Error::Validation(_)),
		"expected Validation error, got: {:?}",
		err
	);
}

#[rstest]
fn test_try_reverse_with_aho_corasick_ok_on_valid_params() {
	// Arrange
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123".to_string());
	params.insert("post_id".to_string(), "456".to_string());

	// Act
	let result = try_reverse_with_aho_corasick("/users/{id}/posts/{post_id}/", &params).unwrap();

	// Assert
	assert_eq!(result, "/users/123/posts/456/");
}

#[rstest]
fn test_try_reverse_single_pass_ok_on_valid_params() {
	// Arrange
	let mut params = HashMap::new();
	params.insert("id".to_string(), "123".to_string());

	// Act
	let result = try_reverse_single_pass("/users/{id}/", &params).unwrap();

	// Assert
	assert_eq!(result, "/users/123/");
}
