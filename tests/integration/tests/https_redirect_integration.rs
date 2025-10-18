//! HTTPS Redirect Integration Tests
//!
//! Tests HTTP to HTTPS redirect functionality
//! Based on Django's middleware/test_security.py SSL redirect tests

use reinhardt_integration_tests::security_test_helpers::*;

use hyper::{Method, StatusCode, Uri};
use reinhardt_apps::{Handler, Middleware, Request, Response, Result};
use reinhardt_middleware::{HttpsRedirectConfig, HttpsRedirectMiddleware};

use std::sync::Arc;

// Mock handler
struct MockHandler;

#[async_trait::async_trait]
impl Handler for MockHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        Ok(create_test_response())
    }
}

// Helper to process request through HttpsRedirectMiddleware
async fn process_with_redirect(config: HttpsRedirectConfig, request: Request) -> Result<Response> {
    let middleware = HttpsRedirectMiddleware::with_config(config);
    let handler = Arc::new(MockHandler);
    middleware.call(request, handler).await
}

#[tokio::test]
async fn test_https_redirect_enabled() {
    // Test: HTTP requests are redirected to HTTPS
    let config = HttpsRedirectConfig {
        enabled: true,
        ..Default::default()
    };

    let request = create_insecure_request("GET", "/some/url");
    let response = process_with_redirect(config, request).await.unwrap();

    assert_status(&response, StatusCode::MOVED_PERMANENTLY);
    assert_has_header(&response, "location");
    let location = get_header(&response, "location").unwrap();
    assert!(location.starts_with("https://"));
}

#[tokio::test]
async fn test_https_redirect_preserves_query_string() {
    // Test: Query strings are preserved in redirect
    let config = HttpsRedirectConfig {
        enabled: true,
        ..Default::default()
    };

    let request = create_insecure_request("GET", "/test?foo=bar&baz=qux");
    let response = process_with_redirect(config, request).await.unwrap();

    let location = get_header(&response, "location").unwrap();
    assert!(location.contains("foo=bar"));
    assert!(location.contains("baz=qux"));
}

#[tokio::test]
async fn test_https_redirect_preserves_path() {
    // Test: Path is preserved in redirect
    let config = HttpsRedirectConfig {
        enabled: true,
        ..Default::default()
    };

    let request = create_insecure_request("GET", "/api/v1/users/123");
    let response = process_with_redirect(config, request).await.unwrap();

    let location = get_header(&response, "location").unwrap();
    assert!(location.contains("/api/v1/users/123"));
}

#[tokio::test]
async fn test_no_redirect_on_https() {
    // Test: HTTPS requests are not redirected
    let config = HttpsRedirectConfig {
        enabled: true,
        ..Default::default()
    };

    let request = create_secure_request("GET", "/test");
    let response = process_with_redirect(config, request).await.unwrap();

    assert_status(&response, StatusCode::OK);
    assert_no_header(&response, "location");
}

#[tokio::test]
async fn test_redirect_disabled() {
    // Test: When disabled, no redirect occurs
    let config = HttpsRedirectConfig {
        enabled: false,
        ..Default::default()
    };

    let request = create_insecure_request("GET", "/test");
    let response = process_with_redirect(config, request).await.unwrap();

    assert_status(&response, StatusCode::OK);
    assert_no_header(&response, "location");
}

#[tokio::test]
async fn test_redirect_with_custom_host() {
    // Test: Redirect to custom host if specified
    let config = HttpsRedirectConfig {
        enabled: true,
        redirect_host: Some("secure.example.com".to_string()),
        ..Default::default()
    };

    let request = create_insecure_request("GET", "/test");
    let response = process_with_redirect(config, request).await.unwrap();

    let location = get_header(&response, "location").unwrap();
    assert!(location.contains("secure.example.com"));
}

#[tokio::test]
async fn test_redirect_exempt_paths() {
    // Test: Exempt paths are not redirected
    let config = HttpsRedirectConfig {
        enabled: true,
        exempt_paths: vec!["/health".to_string(), "/metrics".to_string()],
        ..Default::default()
    };

    // Test exempt path
    let request1 = create_insecure_request("GET", "/health");
    let response1 = process_with_redirect(config.clone(), request1)
        .await
        .unwrap();
    assert_status(&response1, StatusCode::OK);

    // Test non-exempt path
    let request2 = create_insecure_request("GET", "/api/test");
    let response2 = process_with_redirect(config, request2).await.unwrap();
    assert_status(&response2, StatusCode::MOVED_PERMANENTLY);
}

#[tokio::test]
async fn test_redirect_all_methods() {
    // Test: All HTTP methods are redirected
    let config = HttpsRedirectConfig {
        enabled: true,
        ..Default::default()
    };

    for method in &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"] {
        let request = create_insecure_request(method, "/test");
        let response = process_with_redirect(config.clone(), request)
            .await
            .unwrap();
        assert_status(&response, StatusCode::MOVED_PERMANENTLY);
    }
}

#[tokio::test]
async fn test_redirect_status_code() {
    // Test: Redirect uses 301 (Moved Permanently) by default
    let config = HttpsRedirectConfig {
        enabled: true,
        ..Default::default()
    };

    let request = create_insecure_request("GET", "/test");
    let response = process_with_redirect(config, request).await.unwrap();

    assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
}

#[tokio::test]
async fn test_redirect_preserves_fragment() {
    // Test: URL fragments are preserved (though typically handled client-side)
    let config = HttpsRedirectConfig {
        enabled: true,
        ..Default::default()
    };

    let request = create_insecure_request("GET", "/page#section");
    let response = process_with_redirect(config, request).await.unwrap();

    // Note: Fragments are typically not sent to server, but test the path
    let location = get_header(&response, "location").unwrap();
    assert!(location.contains("/page"));
}
