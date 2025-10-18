// Router registration and configuration tests
// Inspired by Django REST Framework's test_routers.py

use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_apps::{Handler, Request, Response, Result};
use reinhardt_routers::{path, re_path, DefaultRouter, Router};
use std::sync::Arc;

// Simple mock handler for testing
struct TestHandler;

#[async_trait]
impl Handler for TestHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        Ok(Response::ok().with_body(Bytes::from("test")))
    }
}

// Test: Basic route registration (inspired by DRF's TestSimpleRouter)
#[test]
fn test_router_registration_basic() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/items/", handler.clone()).with_name("items-list"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].path, "/items/");
    assert_eq!(routes[0].name.as_deref(), Some("items-list"));
}

// Test: Multiple route registration (inspired by DRF's test_register_after_accessing_urls)
#[test]
fn test_router_registration_multiple() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/items/", handler.clone()).with_name("items-list"));

    assert_eq!(router.get_routes().len(), 1);

    router.add_route(path("/users/", handler.clone()).with_name("users-list"));

    assert_eq!(router.get_routes().len(), 2);
}

// Test: Route with path parameters (inspired by DRF's TestCustomLookupFields)
#[test]
fn test_router_registration_with_parameters() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/items/{id}/", handler.clone()).with_name("items-detail"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].path, "/items/{id}/");
}

// Test: Include routes with prefix (inspired by DRF and FastAPI's test_include_route)
#[test]
fn test_router_registration_include_prefix() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    let sub_routes = vec![
        path("/", handler.clone()).with_name("list"),
        path("/{id}/", handler.clone()).with_name("detail"),
    ];

    router.include("/items", sub_routes, None);

    let routes = router.get_routes();
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0].path, "/items/");
    assert_eq!(routes[1].path, "/items/{id}/");
}

// Test: Include routes with namespace (inspired by DRF's TestRootView)
#[test]
fn test_router_registration_include_namespace() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    let sub_routes = vec![
        path("/", handler.clone()).with_name("list"),
        path("/{id}/", handler.clone()).with_name("detail"),
    ];

    router.include("/items", sub_routes, Some("items".to_string()));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0].namespace.as_deref(), Some("items"));
    assert_eq!(routes[1].namespace.as_deref(), Some("items"));
}

// Test: Empty prefix (inspired by DRF's TestEmptyPrefix and FastAPI's test_empty_router)
#[test]
fn test_router_registration_empty_prefix() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/", handler.clone()).with_name("root"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].path, "/");
}

// Test: Regex path conversion (inspired by DRF's TestRegexUrlPath)
#[test]
fn test_router_registration_regex_path() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    // Django-style regex pattern with named group
    router.add_route(re_path(r"^items/(?P<id>\d+)/$", handler.clone()).with_name("items-detail"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    // The pattern should be converted to our {param} format
    assert_eq!(routes[0].path, "items/{id}/");
}

// Test: Trailing slash handling (inspired by DRF's TestTrailingSlashIncluded)
#[test]
fn test_router_registration_trailing_slash() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/items/", handler.clone()).with_name("items"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert!(routes[0].path.ends_with('/'));
}

// Test: Routes without trailing slash (inspired by DRF's TestTrailingSlashRemoved)
#[test]
fn test_router_registration_no_trailing_slash() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/items", handler.clone()).with_name("items"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert!(!routes[0].path.ends_with('/'));
}

// Test: Custom route names (inspired by DRF's TestNameableRoot)
#[test]
fn test_router_registration_custom_names() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/custom/", handler.clone()).with_name("custom-route-name"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].name.as_deref(), Some("custom-route-name"));
}

// Test: Multiple routes with same prefix but different paths
#[test]
fn test_router_registration_same_prefix() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/api/items/", handler.clone()).with_name("items"));
    router.add_route(path("/api/users/", handler.clone()).with_name("users"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0].path, "/api/items/");
    assert_eq!(routes[1].path, "/api/users/");
}

// Test: Complex nested path structure
#[test]
fn test_complex_nested_paths() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(
        path("/api/v1/users/{user_id}/posts/{post_id}/", handler.clone())
            .with_name("user-post-detail"),
    );

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].path, "/api/v1/users/{user_id}/posts/{post_id}/");
}

// Test: Multiple includes with different namespaces
#[test]
fn test_multiple_includes_with_namespaces() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    let items_routes = vec![
        path("/", handler.clone()).with_name("list"),
        path("/{id}/", handler.clone()).with_name("detail"),
    ];

    let users_routes = vec![
        path("/", handler.clone()).with_name("list"),
        path("/{id}/", handler.clone()).with_name("detail"),
    ];

    router.include("/items", items_routes, Some("items".to_string()));
    router.include("/users", users_routes, Some("users".to_string()));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 4);

    // Check items namespace
    assert_eq!(routes[0].namespace.as_deref(), Some("items"));
    assert_eq!(routes[1].namespace.as_deref(), Some("items"));

    // Check users namespace
    assert_eq!(routes[2].namespace.as_deref(), Some("users"));
    assert_eq!(routes[3].namespace.as_deref(), Some("users"));
}

// Test: Routes with mixed parameter types
#[test]
fn test_routes_with_mixed_parameters() {
    let mut router = DefaultRouter::new();
    let handler = Arc::new(TestHandler);

    router.add_route(path("/files/{path}/{filename}", handler.clone()).with_name("file-detail"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].path, "/files/{path}/{filename}");
}
