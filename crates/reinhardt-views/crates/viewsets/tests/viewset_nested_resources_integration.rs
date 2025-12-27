//! ViewSet Nested Resources Integration Tests
//!
//! Tests nested resource functionality for ViewSets:
//! - Create nested resource (POST /articles/1/comments/)
//! - List nested resources (GET /articles/1/comments/)
//! - Retrieve nested resource (GET /articles/1/comments/1/)
//! - Update nested resource (PUT /articles/1/comments/1/)
//! - Delete nested resource (DELETE /articles/1/comments/1/)
//! - Invalid parent ID handling
//! - Nested URL generation helpers
//! - Deep nesting (3 levels)
//!
//! **Test Category**: Decision Table (Decision Table Testing)
//!
//! **Note**: This test focuses on NestedResource configuration and URL patterns.
//! Full nested CRUD with database integration is tested separately.

use bytes::Bytes;
use hyper::{HeaderMap, Method, Version};
use reinhardt_core::http::Request;
use reinhardt_viewsets::{
	NestedResource, NestedResourcePath, NestedViewSet, nested_detail_url, nested_url,
};
use rstest::*;
use serial_test::serial;

// ============================================================================
// Test Structures
// ============================================================================

/// Mock ViewSet for nested resource testing
#[derive(Debug, Clone)]
struct MockViewSet {
	name: String,
}

impl MockViewSet {
	fn new(name: impl Into<String>) -> Self {
		Self { name: name.into() }
	}
}

// ============================================================================
// Tests
// ============================================================================

/// Test: NestedResource configuration for create operation
#[rstest]
#[tokio::test]
#[serial(viewset_nested)]
async fn test_nested_resource_create_config() {
	// Configure nested resource: /articles/{article_id}/comments/
	let nested = NestedResource::new("articles", "article_id", "article_id");

	assert_eq!(nested.parent, "articles");
	assert_eq!(nested.parent_id_param, "article_id");
	assert_eq!(nested.lookup_field, "article_id");

	// Verify URL pattern for create (POST)
	let create_url = nested_url("articles", "1", "comments");
	assert_eq!(create_url, "articles/1/comments/");
}

/// Test: NestedResource configuration for list operation
#[rstest]
#[tokio::test]
#[serial(viewset_nested)]
async fn test_nested_resource_list_config() {
	let nested = NestedResource::new("articles", "article_id", "article_id");
	let inner_viewset = MockViewSet::new("comments");
	let nested_viewset = NestedViewSet::new(inner_viewset, nested);

	// Verify nested configuration
	assert_eq!(nested_viewset.nested_config().parent, "articles");
	assert_eq!(nested_viewset.nested_config().parent_id_param, "article_id");

	// URL pattern for list: GET /articles/1/comments/
	let list_url = nested_url("articles", "1", "comments");
	assert_eq!(list_url, "articles/1/comments/");
}

/// Test: NestedResource configuration for retrieve operation
#[rstest]
#[tokio::test]
#[serial(viewset_nested)]
async fn test_nested_resource_retrieve_config() {
	let nested = NestedResource::new("articles", "article_id", "article_id");

	// URL pattern for retrieve: GET /articles/1/comments/5/
	let retrieve_url = nested_detail_url("articles", "1", "comments", "5");
	assert_eq!(retrieve_url, "articles/1/comments/5/");
}

/// Test: NestedResource configuration for update operation
#[rstest]
#[tokio::test]
#[serial(viewset_nested)]
async fn test_nested_resource_update_config() {
	let nested = NestedResource::new("articles", "article_id", "article_id");
	let inner_viewset = MockViewSet::new("comments");
	let nested_viewset = NestedViewSet::new(inner_viewset, nested);

	// URL pattern for update: PUT /articles/1/comments/5/
	let update_url = nested_detail_url("articles", "1", "comments", "5");
	assert_eq!(update_url, "articles/1/comments/5/");

	// Verify parent ID extraction
	let request = Request::builder()
		.method(Method::PUT)
		.uri("/articles/1/comments/5/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let parent_id = nested_viewset.get_parent_id(&request);
	assert_eq!(parent_id, Some("1".to_string()));
}

/// Test: NestedResource configuration for delete operation
#[rstest]
#[tokio::test]
#[serial(viewset_nested)]
async fn test_nested_resource_delete_config() {
	let nested = NestedResource::new("articles", "article_id", "article_id");
	let inner_viewset = MockViewSet::new("comments");
	let nested_viewset = NestedViewSet::new(inner_viewset, nested);

	// URL pattern for delete: DELETE /articles/1/comments/5/
	let delete_url = nested_detail_url("articles", "1", "comments", "5");
	assert_eq!(delete_url, "articles/1/comments/5/");

	// Verify configuration
	assert_eq!(nested_viewset.nested_config().lookup_field, "article_id");
}

/// Test: Invalid parent ID handling
#[rstest]
#[tokio::test]
#[serial(viewset_nested)]
async fn test_nested_resource_invalid_parent_id() {
	let nested = NestedResource::new("articles", "article_id", "article_id");
	let inner_viewset = MockViewSet::new("comments");
	let nested_viewset = NestedViewSet::new(inner_viewset, nested);

	// Request with non-numeric parent ID
	let request_alpha = Request::builder()
		.method(Method::GET)
		.uri("/articles/invalid/comments/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let parent_id_alpha = nested_viewset.get_parent_id(&request_alpha);
	assert_eq!(
		parent_id_alpha,
		Some("invalid".to_string()),
		"Should extract parent ID even if non-numeric"
	);

	// Request with missing parent ID
	let request_missing = Request::builder()
		.method(Method::GET)
		.uri("/comments/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let parent_id_missing = nested_viewset.get_parent_id(&request_missing);
	assert_eq!(
		parent_id_missing, None,
		"Should return None for missing parent ID"
	);
}

/// Test: Nested URL helper functions
#[rstest]
#[tokio::test]
#[serial(viewset_nested)]
async fn test_nested_url_helpers() {
	// List URL: /articles/1/comments/
	let list = nested_url("articles", "1", "comments");
	assert_eq!(list, "articles/1/comments/");

	// Detail URL: /articles/1/comments/5/
	let detail = nested_detail_url("articles", "1", "comments", "5");
	assert_eq!(detail, "articles/1/comments/5/");

	// Different resource names
	let users_posts = nested_url("users", "123", "posts");
	assert_eq!(users_posts, "users/123/posts/");

	let user_post_detail = nested_detail_url("users", "123", "posts", "456");
	assert_eq!(user_post_detail, "users/123/posts/456/");

	// URL encoding not applied (raw paths)
	let special = nested_url("items", "test-id", "children");
	assert_eq!(special, "items/test-id/children/");
}

/// Test: Deep nesting (3 levels)
#[rstest]
#[tokio::test]
#[serial(viewset_nested)]
async fn test_deep_nesting_three_levels() {
	// Three-level nesting: /organizations/{org_id}/teams/{team_id}/members/
	let path = NestedResourcePath::new()
		.add_segment("organizations", "org_id")
		.add_segment("teams", "team_id")
		.add_segment("members", "member_id");

	// Verify URL pattern
	let list_url = path.build_list_url();
	assert_eq!(list_url, "organizations/teams/members/");

	let detail_url = path.build_url();
	assert_eq!(
		detail_url,
		"organizations/{org_id}/teams/{team_id}/members/{member_id}/"
	);

	// Extract parent IDs from request
	let request = Request::builder()
		.method(Method::GET)
		.uri("/organizations/1/teams/2/members/3/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let ids = path.extract_parent_ids(&request);
	assert_eq!(ids.get("org_id"), Some(&"1".to_string()));
	assert_eq!(ids.get("team_id"), Some(&"2".to_string()));
	assert_eq!(ids.get("member_id"), Some(&"3".to_string()));
}

/// Test: NestedResourcePath single level
#[rstest]
#[tokio::test]
#[serial(viewset_nested)]
async fn test_nested_resource_path_single_level() {
	let path = NestedResourcePath::new().add_segment("users", "user_id");

	assert_eq!(path.build_url(), "users/{user_id}/");
	assert_eq!(path.build_list_url(), "users/");

	// Extract ID from request
	let request = Request::builder()
		.method(Method::GET)
		.uri("/users/123/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let ids = path.extract_parent_ids(&request);
	assert_eq!(ids.get("user_id"), Some(&"123".to_string()));
}

/// Test: NestedResourcePath two levels
#[rstest]
#[tokio::test]
#[serial(viewset_nested)]
async fn test_nested_resource_path_two_levels() {
	let path = NestedResourcePath::new()
		.add_segment("users", "user_id")
		.add_segment("posts", "post_id");

	assert_eq!(path.build_url(), "users/{user_id}/posts/{post_id}/");
	assert_eq!(path.build_list_url(), "users/posts/");

	// Extract IDs from request
	let request = Request::builder()
		.method(Method::GET)
		.uri("/users/123/posts/456/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let ids = path.extract_parent_ids(&request);
	assert_eq!(ids.get("user_id"), Some(&"123".to_string()));
	assert_eq!(ids.get("post_id"), Some(&"456".to_string()));
}

/// Test: NestedViewSet parent ID extraction
#[rstest]
#[tokio::test]
#[serial(viewset_nested)]
async fn test_nested_viewset_parent_id_extraction() {
	let nested = NestedResource::new("posts", "post_id", "post_id");
	let inner_viewset = MockViewSet::new("comments");
	let nested_viewset = NestedViewSet::new(inner_viewset, nested);

	// Test various URL patterns
	let test_cases = vec![
		("/posts/1/comments/", Some("1")),
		("/posts/123/comments/", Some("123")),
		("/posts/abc/comments/", Some("abc")),
		("/posts/1/comments/5/", Some("1")),
		("/comments/", None), // No parent
		("/posts/", None),    // Incomplete path
	];

	for (url, expected) in test_cases {
		let request = Request::builder()
			.method(Method::GET)
			.uri(url)
			.version(Version::HTTP_11)
			.headers(HeaderMap::new())
			.body(Bytes::new())
			.build()
			.unwrap();

		let parent_id = nested_viewset.get_parent_id(&request);
		assert_eq!(
			parent_id,
			expected.map(|s| s.to_string()),
			"URL: {} should extract parent_id: {:?}",
			url,
			expected
		);
	}
}

/// Test: Multiple nested resources with different configurations
#[rstest]
#[tokio::test]
#[serial(viewset_nested)]
async fn test_multiple_nested_resources() {
	// Article comments: /articles/{article_id}/comments/
	let article_comments = NestedResource::new("articles", "article_id", "article_id");
	assert_eq!(article_comments.parent, "articles");
	assert_eq!(article_comments.lookup_field, "article_id");

	// User posts: /users/{user_id}/posts/
	let user_posts = NestedResource::new("users", "user_id", "user_id");
	assert_eq!(user_posts.parent, "users");
	assert_eq!(user_posts.lookup_field, "user_id");

	// Organization teams: /organizations/{org_id}/teams/
	let org_teams = NestedResource::new("organizations", "org_id", "organization_id");
	assert_eq!(org_teams.parent, "organizations");
	assert_eq!(org_teams.parent_id_param, "org_id");
	assert_eq!(org_teams.lookup_field, "organization_id");

	// Verify each has unique configuration
	assert_ne!(article_comments.parent, user_posts.parent);
	assert_ne!(user_posts.parent, org_teams.parent);
}

/// Test: NestedResourcePath empty (edge case)
#[rstest]
#[tokio::test]
#[serial(viewset_nested)]
async fn test_nested_resource_path_empty() {
	let path = NestedResourcePath::new();

	assert_eq!(path.build_list_url(), "/");
	assert_eq!(path.build_url(), "/");

	// Extract from empty path
	let request = Request::builder()
		.method(Method::GET)
		.uri("/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let ids = path.extract_parent_ids(&request);
	assert_eq!(ids.len(), 0, "Empty path should extract no IDs");
}
