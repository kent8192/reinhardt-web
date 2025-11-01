// Router and ViewSet integration tests
// Inspired by Django REST Framework's ViewSet router registration tests

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
use reinhardt_apps::{Handler, Request, Response, Result};
use reinhardt_routers::{DefaultRouter, Router};
use reinhardt_viewsets::{Action, ModelViewSet, ViewSet};
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct TestModel {
	id: i64,
	name: String,
}

#[derive(Debug, Clone)]
struct TestSerializer;

// Test: ViewSet registration with router (inspired by DRF's TestSimpleRouter)
#[tokio::test]
async fn test_viewset_registration() {
	let mut router = DefaultRouter::new();
	let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("users"));

	router.register_viewset("users", viewset);

	let routes = router.get_routes();
	assert_eq!(routes.len(), 2); // list and detail routes

	// Check list route
	assert_eq!(routes[0].path, "/users/");
	assert_eq!(routes[0].name.as_deref(), Some("users-list"));

	// Check detail route
	assert_eq!(routes[1].path, "/users/{id}/");
	assert_eq!(routes[1].name.as_deref(), Some("users-detail"));
}

// Test: Multiple ViewSet registration
#[tokio::test]
async fn test_multiple_viewset_registration() {
	let mut router = DefaultRouter::new();

	let users_viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("users"));
	let posts_viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("posts"));

	router.register_viewset("users", users_viewset);
	router.register_viewset("posts", posts_viewset);

	let routes = router.get_routes();
	assert_eq!(routes.len(), 4); // 2 ViewSets * 2 routes each
}

// Test: ViewSet list route matching
#[tokio::test]
async fn test_viewset_list_route_matching() {
	let mut router = DefaultRouter::new();
	let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("users"));

	router.register_viewset("users", viewset);

	let request = Request::new(
		Method::GET,
		"/users/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let response = router.route(request).await;
	assert!(response.is_ok());
	assert_eq!(response.unwrap().status, StatusCode::OK);
}

// Test: ViewSet detail route matching
#[tokio::test]
async fn test_viewset_detail_route_matching() {
	let mut router = DefaultRouter::new();
	let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("users"));

	router.register_viewset("users", viewset);

	let request = Request::new(
		Method::GET,
		"/users/123/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let response = router.route(request).await;
	assert!(response.is_ok());
	assert_eq!(response.unwrap().status, StatusCode::OK);
}

// Test: ViewSet create action (POST to list route)
#[tokio::test]
async fn test_viewset_create_action() {
	let mut router = DefaultRouter::new();
	let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("users"));

	router.register_viewset("users", viewset);

	let request = Request::new(
		Method::POST,
		"/users/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::from(r#"{"name": "test"}"#),
	);

	let response = router.route(request).await;
	assert!(response.is_ok());
	assert_eq!(response.unwrap().status, StatusCode::CREATED);
}

// Test: ViewSet update action (PUT to detail route)
#[tokio::test]
async fn test_viewset_update_action() {
	let mut router = DefaultRouter::new();
	let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("users"));

	router.register_viewset("users", viewset);

	let request = Request::new(
		Method::PUT,
		"/users/123/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::from(r#"{"name": "updated"}"#),
	);

	let response = router.route(request).await;
	assert!(response.is_ok());
	assert_eq!(response.unwrap().status, StatusCode::OK);
}

// Test: ViewSet destroy action (DELETE to detail route)
#[tokio::test]
async fn test_viewset_destroy_action() {
	let mut router = DefaultRouter::new();
	let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("users"));

	router.register_viewset("users", viewset);

	let request = Request::new(
		Method::DELETE,
		"/users/123/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let response = router.route(request).await;
	assert!(response.is_ok());
	assert_eq!(response.unwrap().status, StatusCode::NO_CONTENT);
}

// Test: ViewSet basename in routes
#[tokio::test]
async fn test_viewset_basename_in_routes() {
	let mut router = DefaultRouter::new();
	let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("custom-basename"));

	router.register_viewset("items", viewset);

	let routes = router.get_routes();
	assert_eq!(routes[0].name.as_deref(), Some("custom-basename-list"));
	assert_eq!(routes[1].name.as_deref(), Some("custom-basename-detail"));
}

// Test: ViewSet reverse URL lookup
#[tokio::test]
async fn test_viewset_reverse_url() {
	let mut router = DefaultRouter::new();
	let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("users"));

	router.register_viewset("users", viewset);

	// Reverse list URL
	let list_url = router.reverse("users-list", &std::collections::HashMap::new());
	assert!(list_url.is_ok());
	assert_eq!(list_url.unwrap(), "/users/");

	// Reverse detail URL
	let mut params = std::collections::HashMap::new();
	params.insert("id".to_string(), "42".to_string());
	let detail_url = router.reverse("users-detail", &params);
	assert!(detail_url.is_ok());
	assert_eq!(detail_url.unwrap(), "/users/42/");
}

// Test: Nested ViewSet routes with prefix
#[tokio::test]
async fn test_nested_viewset_routes() {
	let mut router = DefaultRouter::new();
	let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("users"));

	router.register_viewset("api/v1/users", viewset);

	let routes = router.get_routes();
	assert_eq!(routes[0].path, "/api/v1/users/");
	assert_eq!(routes[1].path, "/api/v1/users/{id}/");
}

// Test: ViewSet with custom lookup field
#[tokio::test]
async fn test_viewset_custom_lookup_field() {
	let mut router = DefaultRouter::new();
	let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("users").with_lookup_field("username"));

	router.register_viewset("users", viewset);

	// Verify that the route uses 'username' instead of 'id'
	let routes = router.get_routes();
	assert_eq!(routes.len(), 2);
	assert_eq!(routes[1].path, "/users/{username}/");

	// Test that the lookup field parameter is correctly used
	let request = Request::new(
		Method::GET,
		"/users/alice/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let response = router.route(request).await;
	assert!(response.is_ok());
	assert_eq!(response.unwrap().status, StatusCode::OK);
}

// Test: Multiple HTTP methods on same ViewSet route
#[tokio::test]
async fn test_viewset_multiple_http_methods() {
	let mut router = DefaultRouter::new();
	let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("users"));

	router.register_viewset("users", viewset);

	// GET (retrieve)
	let get_request = Request::new(
		Method::GET,
		"/users/1/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);
	let get_response = router.route(get_request).await;
	assert!(get_response.is_ok());

	// PUT (update)
	let put_request = Request::new(
		Method::PUT,
		"/users/1/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::from(r#"{"name": "updated"}"#),
	);
	let put_response = router.route(put_request).await;
	assert!(put_response.is_ok());

	// DELETE (destroy)
	let delete_request = Request::new(
		Method::DELETE,
		"/users/1/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);
	let delete_response = router.route(delete_request).await;
	assert!(delete_response.is_ok());
}

// Test: ViewSet route not found for invalid path
#[tokio::test]
async fn test_viewset_route_not_found() {
	let mut router = DefaultRouter::new();
	let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("users"));

	router.register_viewset("users", viewset);

	let request = Request::new(
		Method::GET,
		"/invalid/path/".parse::<Uri>().unwrap(),
		Version::HTTP_11,
		HeaderMap::new(),
		Bytes::new(),
	);

	let response = router.route(request).await;
	assert!(response.is_err());
}

// Test: ViewSet registration with trailing slashes
#[tokio::test]
async fn test_viewset_trailing_slashes() {
	let mut router = DefaultRouter::new();
	let viewset: Arc<ModelViewSet<TestModel, TestSerializer>> =
		Arc::new(ModelViewSet::new("users"));

	router.register_viewset("users", viewset);

	let routes = router.get_routes();
	// Both routes should have trailing slashes
	assert!(routes[0].path.ends_with('/'));
	assert!(routes[1].path.ends_with('/'));
}
