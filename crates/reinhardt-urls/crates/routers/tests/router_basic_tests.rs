// Basic router functionality tests
// Inspired by Django REST Framework's test_routers.py and FastAPI's router tests

use async_trait::async_trait;
use bytes::Bytes;
use hyper::{HeaderMap, Method, Uri, Version};
use reinhardt_apps::{Handler, Request, Response, Result};
use reinhardt_routers::{DefaultRouter, Router, path, re_path};
use std::sync::Arc;

// Mock handlers for testing
struct MockHandler {
    response_body: String,
}

impl MockHandler {
    fn new(response_body: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            response_body: response_body.into(),
        })
    }
}

#[async_trait]
impl Handler for MockHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        Ok(Response::ok().with_body(Bytes::from(self.response_body.clone())))
    }
}

// Test: Basic route registration (inspired by DRF's TestSimpleRouter)
#[tokio::test]
async fn test_router_basic_registration() {
    let mut router = DefaultRouter::new();
    let handler = MockHandler::new("list response");

    router.add_route(path("/items/", handler.clone()).with_name("items-list"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].path, "/items/");
    assert_eq!(routes[0].name.as_deref(), Some("items-list"));
}

// Test: Multiple route registration (inspired by DRF's test_register_after_accessing_urls)
#[tokio::test]
async fn test_router_basic_multiple() {
    let mut router = DefaultRouter::new();

    router.add_route(path("/items/", MockHandler::new("items")).with_name("items-list"));

    assert_eq!(router.get_routes().len(), 1);

    router.add_route(path("/users/", MockHandler::new("users")).with_name("users-list"));

    assert_eq!(router.get_routes().len(), 2);
}

// Test: Route with path parameters (inspired by DRF's TestCustomLookupFields)
#[tokio::test]
async fn test_router_basic_with_parameters() {
    let mut router = DefaultRouter::new();
    let handler = MockHandler::new("detail response");

    router.add_route(path("/items/{id}/", handler.clone()).with_name("items-detail"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].path, "/items/{id}/");
}

// Test: Include routes with prefix (inspired by DRF and FastAPI's test_include_route)
#[tokio::test]
async fn test_router_basic_include_prefix() {
    let mut router = DefaultRouter::new();

    let sub_routes = vec![
        path("/", MockHandler::new("list")).with_name("list"),
        path("/{id}/", MockHandler::new("detail")).with_name("detail"),
    ];

    router.include("/items", sub_routes, None);

    let routes = router.get_routes();
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0].path, "/items/");
    assert_eq!(routes[1].path, "/items/{id}/");
}

// Test: Include routes with namespace (inspired by DRF's TestRootView)
#[tokio::test]
async fn test_router_basic_include_namespace() {
    let mut router = DefaultRouter::new();

    let sub_routes = vec![
        path("/", MockHandler::new("list")).with_name("list"),
        path("/{id}/", MockHandler::new("detail")).with_name("detail"),
    ];

    router.include("/items", sub_routes, Some("items".to_string()));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0].namespace.as_deref(), Some("items"));
    assert_eq!(routes[1].namespace.as_deref(), Some("items"));
}

// Test: Empty prefix (inspired by DRF's TestEmptyPrefix and FastAPI's test_empty_router)
#[tokio::test]
async fn test_router_basic_empty_prefix() {
    let mut router = DefaultRouter::new();

    router.add_route(path("/", MockHandler::new("root")).with_name("root"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].path, "/");
}

// Test: Regex path conversion (inspired by DRF's TestRegexUrlPath)
#[tokio::test]
async fn test_router_basic_regex_path() {
    let mut router = DefaultRouter::new();

    // Django-style regex pattern with named group
    let handler = MockHandler::new("regex response");
    router.add_route(re_path(r"^items/(?P<id>\d+)/$", handler.clone()).with_name("items-detail"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    // The pattern should be converted to our {param} format
    assert_eq!(routes[0].path, "items/{id}/");
}

// Test: URL reverse lookup (inspired by DRF's reverse functionality)
#[tokio::test]
async fn test_url_reverse() {
    let mut router = DefaultRouter::new();

    router.add_route(path("/items/", MockHandler::new("list")).with_name("items-list"));

    router.add_route(path("/items/{id}/", MockHandler::new("detail")).with_name("items-detail"));

    // Test basic reverse
    let url = router.reverse("items-list", &Default::default());
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/items/");

    // Test reverse with parameters
    use std::collections::HashMap;
    let mut params = HashMap::new();
    params.insert("id".to_string(), "123".to_string());

    let url = router.reverse("items-detail", &params);
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/items/123/");
}

// Test: URL reverse with namespace (inspired by DRF's TestRootView)
#[tokio::test]
async fn test_router_basic_reverse_namespace() {
    let mut router = DefaultRouter::new();

    let sub_routes = vec![
        path("/", MockHandler::new("list")).with_name("list"),
        path("/{id}/", MockHandler::new("detail")).with_name("detail"),
    ];

    router.include("/items", sub_routes, Some("items".to_string()));

    // Reverse with namespace
    let url = router.reverse("items:list", &Default::default());
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/items/");
}

// Test: URL reverse with convenience method
#[tokio::test]
async fn test_url_reverse_with() {
    let mut router = DefaultRouter::new();

    router.add_route(path("/items/{id}/", MockHandler::new("detail")).with_name("items-detail"));

    let url = router.reverse_with("items-detail", &[("id", "456")]);
    assert!(url.is_ok());
    assert_eq!(url.unwrap(), "/items/456/");
}

// Test: Route matching (inspired by FastAPI's test_route_scope)
#[tokio::test]
async fn test_route_matching() {
    let mut router = DefaultRouter::new();

    router.add_route(path("/items/", MockHandler::new("list")).with_name("items-list"));

    let request = Request::new(
        Method::GET,
        "/items/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );
    let response = router.route(request).await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.body, Bytes::from("list"));
}

// Test: Route matching with parameters
#[tokio::test]
async fn test_route_matching_with_parameters() {
    let mut router = DefaultRouter::new();

    router.add_route(path("/items/{id}/", MockHandler::new("detail")).with_name("items-detail"));

    let request = Request::new(
        Method::GET,
        "/items/123/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );
    let response = router.route(request).await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.body, Bytes::from("detail"));
}

// Test: No route found (inspired by FastAPI's test_invalid_path_doesnt_match)
#[tokio::test]
async fn test_no_route_found() {
    let router = DefaultRouter::new();

    let request = Request::new(
        Method::GET,
        "/nonexistent/".parse::<Uri>().unwrap(),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );
    let response = router.route(request).await;

    assert!(response.is_err());
}

// Test: Trailing slash handling (inspired by DRF's TestTrailingSlashIncluded)
#[tokio::test]
async fn test_router_basic_trailing_slash() {
    let mut router = DefaultRouter::new();

    router.add_route(path("/items/", MockHandler::new("with slash")).with_name("items"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert!(routes[0].path.ends_with('/'));
}

// Test: Routes without trailing slash (inspired by DRF's TestTrailingSlashRemoved)
#[tokio::test]
async fn test_router_basic_no_trailing_slash() {
    let mut router = DefaultRouter::new();

    router.add_route(path("/items", MockHandler::new("no slash")).with_name("items"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert!(!routes[0].path.ends_with('/'));
}

// Test: Custom route names (inspired by DRF's TestNameableRoot)
#[tokio::test]
async fn test_router_basic_custom_names() {
    let mut router = DefaultRouter::new();

    router.add_route(path("/custom/", MockHandler::new("custom")).with_name("custom-route-name"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].name.as_deref(), Some("custom-route-name"));
}

// Test: Multiple routes with same prefix but different paths
#[tokio::test]
async fn test_router_basic_same_prefix() {
    let mut router = DefaultRouter::new();

    router.add_route(path("/api/items/", MockHandler::new("items")).with_name("items"));
    router.add_route(path("/api/users/", MockHandler::new("users")).with_name("users"));

    let routes = router.get_routes();
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0].path, "/api/items/");
    assert_eq!(routes[1].path, "/api/users/");
}
