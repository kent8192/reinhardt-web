// URL reverse (reverse routing) tests
// Inspired by Django REST Framework's URL reversal functionality

use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_apps::{Handler, Request, Response, Result};
use reinhardt_routers::{DefaultRouter, Router, path};
use std::collections::HashMap;
use std::sync::Arc;

// Simple mock handler
struct TestHandler;

#[async_trait]
impl Handler for TestHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        Ok(Response::ok().with_body(Bytes::from("test")))
    }
}

// Test: URL reverse without parameters
#[test]
fn test_url_reverse_simple() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/items/", handler.clone()).with_name("items-list"));

    let url = router.reverse("items-list", &HashMap::new());
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/items/");
}

// Test: URL reverse with single parameter
#[test]
fn test_url_reverse_with_parameter() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/items/{id}/", handler.clone()).with_name("items-detail"));

    let mut params = HashMap::new();
    params.insert("id".to_string(), "123".to_string());

    let url = router.reverse("items-detail", &params);
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/items/123/");
}

// Test: URL reverse with multiple parameters
#[test]
fn test_url_reverse_with_multiple_parameters() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(
        path("/users/{user_id}/posts/{post_id}/", handler.clone()).with_name("user-post-detail"),
    );

    let mut params = HashMap::new();
    params.insert("user_id".to_string(), "42".to_string());
    params.insert("post_id".to_string(), "123".to_string());

    let url = router.reverse("user-post-detail", &params);
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/users/42/posts/123/");
}

// Test: URL reverse with namespace
#[test]
fn test_reverse_url_with_namespace() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    let sub_routes = vec![
        path("/", handler.clone()).with_name("list"),
        path("/{id}/", handler.clone()).with_name("detail"),
    ];

    router.include("/items", sub_routes, Some("items".to_string()));

    // Reverse with namespace
    let url = router.reverse("items:list", &HashMap::new());
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/items/");

    // Reverse namespaced route with parameters
    let mut params = HashMap::new();
    params.insert("id".to_string(), "456".to_string());
    let url = router.reverse("items:detail", &params);
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/items/456/");
}

// Test: URL reverse with convenience method (reverse_with)
#[test]
fn test_url_reverse_with_convenience_method() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/items/{id}/", handler.clone()).with_name("items-detail"));

    let url = router.reverse_with("items-detail", &[("id", "789")]);
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/items/789/");
}

// Test: URL reverse for nonexistent route
#[test]
fn test_url_reverse_nonexistent_route() {
    let router = DefaultRouter::new();

    let url = router.reverse("nonexistent", &HashMap::new());
    assert!(url.is_err());
}

// Test: URL reverse with missing parameters
#[test]
fn test_url_reverse_missing_parameters() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/items/{id}/", handler.clone()).with_name("items-detail"));

    // Try to reverse without providing required parameter
    let url = router.reverse("items-detail", &HashMap::new());
    assert!(url.is_err());
}

// Test: URL reverse with extra parameters (should be ignored)
#[test]
fn test_url_reverse_with_extra_parameters() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/items/{id}/", handler.clone()).with_name("items-detail"));

    let mut params = HashMap::new();
    params.insert("id".to_string(), "123".to_string());
    params.insert("extra".to_string(), "ignored".to_string());

    let url = router.reverse("items-detail", &params);
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/items/123/");
}

// Test: URL reverse for root path
#[test]
fn test_url_reverse_root_path() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/", handler.clone()).with_name("root"));

    let url = router.reverse("root", &HashMap::new());
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/");
}

// Test: URL reverse with special characters in parameters
#[test]
fn test_url_reverse_with_special_chars() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/items/{id}/", handler.clone()).with_name("items-detail"));

    let mut params = HashMap::new();
    params.insert("id".to_string(), "foo-bar_123".to_string());

    let url = router.reverse("items-detail", &params);
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/items/foo-bar_123/");
}

// Test: URL reverse for complex nested paths
#[test]
fn test_url_reverse_complex_nested() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(
        path(
            "/api/v1/organizations/{org_id}/projects/{project_id}/",
            handler.clone(),
        )
        .with_name("project-detail"),
    );

    let mut params = HashMap::new();
    params.insert("org_id".to_string(), "org123".to_string());
    params.insert("project_id".to_string(), "proj456".to_string());

    let url = router.reverse("project-detail", &params);
    assert!(url.is_ok());
    assert_eq!(
        url.unwrap(),
        "/api/v1/organizations/org123/projects/proj456/"
    );
}

// Test: URL reverse with numeric parameters
#[test]
fn test_url_reverse_with_numeric_params() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/items/{id}/", handler.clone()).with_name("items-detail"));

    let url = router.reverse_with("items-detail", &[("id", "42")]);
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/items/42/");
}

// Test: Multiple namespaced routes reversal
#[test]
fn test_multiple_namespaced_routes_reversal() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    let items_routes = vec![path("/", handler.clone()).with_name("list")];
    let users_routes = vec![path("/", handler.clone()).with_name("list")];

    router.include("/items", items_routes, Some("items".to_string()));
    router.include("/users", users_routes, Some("users".to_string()));

    // Both namespaces should be resolvable independently
    let items_url = router.reverse("items:list", &HashMap::new());
    assert!(items_url.is_ok());
    assert_eq!(items_url.unwrap(), "/items/");

    let users_url = router.reverse("users:list", &HashMap::new());
    assert!(users_url.is_ok());
    assert_eq!(users_url.unwrap(), "/users/");
}

// Test: URL reverse with empty parameter value
#[test]
fn test_url_reverse_with_empty_parameter() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/items/{id}/", handler.clone()).with_name("items-detail"));

    let mut params = HashMap::new();
    params.insert("id".to_string(), "".to_string());

    // Empty parameter should still work (validation is separate concern)
    let url = router.reverse("items-detail", &params);
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/items//");
}
