//! Security Middleware Integration Tests
//!
//! Tests the integration of SecurityMiddleware with HTTP request/response handling
//! Based on Django's middleware/test_security.py

use hyper::{Method, StatusCode};
use reinhardt_apps::{Handler, Middleware, Request, Response, Result};
use reinhardt_integration_tests::security_test_helpers::*;
use reinhardt_middleware::SecurityConfig;
use reinhardt_middleware::SecurityMiddleware;
use std::sync::Arc;

// Mock handler that just returns OK
struct MockHandler;

#[async_trait::async_trait]
impl Handler for MockHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        Ok(create_test_response())
    }
}

// Helper to process request through SecurityMiddleware
async fn process_with_security(config: SecurityConfig, request: Request) -> Result<Response> {
    let middleware = SecurityMiddleware::with_config(config);
    let handler = Arc::new(MockHandler);
    middleware.call(request, handler).await
}

#[tokio::test]
async fn test_hsts_on_secure_connection() {
    // Test: With HSTS enabled, the middleware adds Strict-Transport-Security header on HTTPS
    let config = SecurityConfig {
        hsts_enabled: true,
        hsts_seconds: 3600,
        ..Default::default()
    };

    let request = create_secure_request("GET", "/test");
    let response = process_with_security(config, request).await.unwrap();

    assert_has_header(&response, "strict-transport-security");
    assert_header_equals(&response, "strict-transport-security", "max-age=3600");
}

#[tokio::test]
async fn test_hsts_not_on_insecure_connection() {
    // Test: HSTS header is not added to insecure (HTTP) connections
    let config = SecurityConfig {
        hsts_enabled: true,
        hsts_seconds: 3600,
        ..Default::default()
    };

    let request = create_insecure_request("GET", "/test");
    let response = process_with_security(config, request).await.unwrap();

    assert_no_header(&response, "strict-transport-security");
}

#[tokio::test]
async fn test_hsts_with_includesubdomains() {
    // Test: HSTS with includeSubDomains directive
    let config = SecurityConfig {
        hsts_enabled: true,
        hsts_seconds: 600,
        hsts_include_subdomains: true,
        ..Default::default()
    };

    let request = create_secure_request("GET", "/test");
    let response = process_with_security(config, request).await.unwrap();

    assert_header_equals(
        &response,
        "strict-transport-security",
        "max-age=600; includeSubDomains",
    );
}

#[tokio::test]
async fn test_hsts_with_preload() {
    // Test: HSTS with preload directive
    let config = SecurityConfig {
        hsts_enabled: true,
        hsts_seconds: 10886400,
        hsts_preload: true,
        ..Default::default()
    };

    let request = create_secure_request("GET", "/test");
    let response = process_with_security(config, request).await.unwrap();

    assert_header_contains(&response, "strict-transport-security", "preload");
}

#[tokio::test]
async fn test_security_integration_hsts_full() {
    // Test: HSTS with both includeSubDomains and preload
    let config = SecurityConfig {
        hsts_enabled: true,
        hsts_seconds: 10886400,
        hsts_include_subdomains: true,
        hsts_preload: true,
        ..Default::default()
    };

    let request = create_secure_request("GET", "/test");
    let response = process_with_security(config, request).await.unwrap();

    let header = get_header(&response, "strict-transport-security").unwrap();
    assert!(header.contains("includeSubDomains"));
    assert!(header.contains("preload"));
    assert!(header.contains("max-age=10886400"));
}

#[tokio::test]
async fn test_hsts_disabled() {
    // Test: When HSTS is disabled, no header is added
    let config = SecurityConfig {
        hsts_enabled: false,
        ..Default::default()
    };

    let request = create_secure_request("GET", "/test");
    let response = process_with_security(config, request).await.unwrap();

    assert_no_header(&response, "strict-transport-security");
}

#[tokio::test]
async fn test_content_type_nosniff_on() {
    // Test: X-Content-Type-Options: nosniff is added when enabled
    let config = SecurityConfig {
        content_type_nosniff: true,
        ..Default::default()
    };

    let request = create_test_request("GET", "/test", false);
    let response = process_with_security(config, request).await.unwrap();

    assert_has_header(&response, "x-content-type-options");
    assert_header_equals(&response, "x-content-type-options", "nosniff");
}

#[tokio::test]
async fn test_content_type_nosniff_off() {
    // Test: X-Content-Type-Options header is not added when disabled
    let config = SecurityConfig {
        content_type_nosniff: false,
        ..Default::default()
    };

    let request = create_test_request("GET", "/test", false);
    let response = process_with_security(config, request).await.unwrap();

    assert_no_header(&response, "x-content-type-options");
}

#[tokio::test]
async fn test_ssl_redirect_on() {
    // Test: SSL redirect enabled redirects HTTP to HTTPS
    let config = SecurityConfig {
        ssl_redirect: true,
        ..Default::default()
    };

    let request = create_insecure_request("GET", "/some/url?query=string");
    let response = process_with_security(config, request).await.unwrap();

    assert_status(&response, StatusCode::MOVED_PERMANENTLY);
    assert_has_header(&response, "location");
    // Note: Actual redirect URL depends on implementation
}

#[tokio::test]
async fn test_no_redirect_on_secure() {
    // Test: SSL redirect does not redirect HTTPS requests
    let config = SecurityConfig {
        ssl_redirect: true,
        ..Default::default()
    };

    let request = create_secure_request("GET", "/some/url");
    let response = process_with_security(config, request).await.unwrap();

    assert_status(&response, StatusCode::OK);
    assert_no_header(&response, "location");
}

#[tokio::test]
async fn test_ssl_redirect_off() {
    // Test: SSL redirect disabled does not redirect
    let config = SecurityConfig {
        ssl_redirect: false,
        ..Default::default()
    };

    let request = create_insecure_request("GET", "/some/url");
    let response = process_with_security(config, request).await.unwrap();

    assert_status(&response, StatusCode::OK);
    assert_no_header(&response, "location");
}

#[tokio::test]
async fn test_referrer_policy_on() {
    // Test: Referrer-Policy header is added when set
    let config = SecurityConfig {
        referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
        ..Default::default()
    };

    let request = create_test_request("GET", "/test", false);
    let response = process_with_security(config, request).await.unwrap();

    assert_has_header(&response, "referrer-policy");
    assert_header_equals(
        &response,
        "referrer-policy",
        "strict-origin-when-cross-origin",
    );
}

#[tokio::test]
async fn test_referrer_policy_off() {
    // Test: Referrer-Policy header is not added when None
    let config = SecurityConfig {
        referrer_policy: None,
        ..Default::default()
    };

    let request = create_test_request("GET", "/test", false);
    let response = process_with_security(config, request).await.unwrap();

    assert_no_header(&response, "referrer-policy");
}

#[tokio::test]
async fn test_coop_on() {
    // Test: Cross-Origin-Opener-Policy header is added when set
    let config = SecurityConfig {
        cross_origin_opener_policy: Some("same-origin".to_string()),
        ..Default::default()
    };

    let request = create_test_request("GET", "/test", false);
    let response = process_with_security(config, request).await.unwrap();

    assert_has_header(&response, "cross-origin-opener-policy");
    assert_header_equals(&response, "cross-origin-opener-policy", "same-origin");
}

#[tokio::test]
async fn test_coop_off() {
    // Test: Cross-Origin-Opener-Policy header is not added when None
    let config = SecurityConfig {
        cross_origin_opener_policy: None,
        ..Default::default()
    };

    let request = create_test_request("GET", "/test", false);
    let response = process_with_security(config, request).await.unwrap();

    assert_no_header(&response, "cross-origin-opener-policy");
}

#[tokio::test]
async fn test_multiple_security_headers() {
    // Test: Multiple security headers can be enabled simultaneously
    let config = SecurityConfig {
        hsts_enabled: true,
        hsts_seconds: 31536000,
        content_type_nosniff: true,
        referrer_policy: Some("no-referrer".to_string()),
        cross_origin_opener_policy: Some("same-origin".to_string()),
        ssl_redirect: false,
        ..Default::default()
    };

    let request = create_secure_request("GET", "/test");
    let response = process_with_security(config, request).await.unwrap();

    assert_has_header(&response, "strict-transport-security");
    assert_has_header(&response, "x-content-type-options");
    assert_has_header(&response, "referrer-policy");
    assert_has_header(&response, "cross-origin-opener-policy");
}

#[tokio::test]
async fn test_security_headers_on_different_methods() {
    // Test: Security headers are added for all HTTP methods
    let config = SecurityConfig {
        content_type_nosniff: true,
        ..Default::default()
    };

    for method in &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"] {
        let request = create_test_request(method, "/test", false);
        let response = process_with_security(config.clone(), request)
            .await
            .unwrap();
        assert_has_header(&response, "x-content-type-options");
    }
}
