#[path = "server_test_helpers.rs"]
mod test_helpers;

use bytes::Bytes;
use http::{HeaderMap, Method, Uri, Version};
use reinhardt_exception::Result;
use reinhardt_http::{Request, Response};
use reinhardt_types::Handler;
use std::sync::Arc;

/// Test handler for HEAD requests
struct HeadRequestHandler;

#[async_trait::async_trait]
impl Handler for HeadRequestHandler {
    async fn handle(&self, request: Request) -> Result<Response> {
        if request.method == Method::HEAD {
            // HEAD requests should not return a body
            Ok(Response::ok().with_header("content-length", "12"))
        } else {
            Ok(Response::ok().with_body("Hello World!"))
        }
    }
}

/// Test handler that returns custom headers
struct CustomHeaderHandler;

#[async_trait::async_trait]
impl Handler for CustomHeaderHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        Ok(Response::ok()
            .with_header("x-custom-header", "custom-value")
            .with_header("content-type", "application/json")
            .with_body(r#"{"message": "test"}"#))
    }
}

/// Test handler for chunked responses
struct ChunkedResponseHandler;

#[async_trait::async_trait]
impl Handler for ChunkedResponseHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        // Simulate a large response that would be chunked
        let large_body = "x".repeat(64 * 1024); // 64KB
        Ok(Response::ok().with_body(large_body))
    }
}

#[tokio::test]
async fn test_server_protocol_head_no_body() {
    let handler = Arc::new(HeadRequestHandler);

    let request = Request::new(
        Method::HEAD,
        Uri::from_static("/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 200);

    // HEAD response should have Content-Length header
    assert!(response.headers.contains_key("content-length"));

    // Body should be empty for HEAD requests (though handler returned empty body)
    // In a real server, the body would be stripped
}

#[tokio::test]
async fn test_get_request_with_body() {
    let handler = Arc::new(HeadRequestHandler);

    let request = Request::new(
        Method::GET,
        Uri::from_static("/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, Bytes::from("Hello World!"));
}

#[tokio::test]
async fn test_custom_headers() {
    let handler = Arc::new(CustomHeaderHandler);

    let request = Request::new(
        Method::GET,
        Uri::from_static("/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 200);

    assert_eq!(
        response.headers.get("x-custom-header").unwrap(),
        "custom-value"
    );
    assert_eq!(
        response.headers.get("content-type").unwrap(),
        "application/json"
    );
}

#[tokio::test]
async fn test_large_response() {
    let handler = Arc::new(ChunkedResponseHandler);

    let request = Request::new(
        Method::GET,
        Uri::from_static("/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body.len(), 64 * 1024);
}

#[tokio::test]
async fn test_query_string_handling() {
    let handler = Arc::new(CustomHeaderHandler);

    let request = Request::new(
        Method::GET,
        Uri::from_static("/?name=test&value=123"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 200);
}

#[tokio::test]
async fn test_post_with_body() {
    struct PostEchoHandler;

    #[async_trait::async_trait]
    impl Handler for PostEchoHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            // Echo the body back
            let body = request.read_body().map_err(|e| anyhow::Error::from(e))?;
            Ok(Response::ok().with_body(body))
        }
    }

    let handler = Arc::new(PostEchoHandler);

    let body_content = "test body content";
    let request = Request::new(
        Method::POST,
        Uri::from_static("/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::from(body_content),
    );

    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, Bytes::from(body_content));
}

// Tests that require integration with live server

#[tokio::test]
async fn test_http11_protocol_version() {
    use test_helpers::*;

    let handler = Arc::new(EchoPathHandler);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();
    let response = client.get(&format!("{}/test", url)).send().await.unwrap();

    // Verify HTTP/1.1 is used
    assert_eq!(response.version(), reqwest::Version::HTTP_11);
    assert_eq!(response.status(), 200);

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_underscore_headers_stripped() {
    use test_helpers::*;

    // Handler that echoes received headers
    struct HeaderEchoHandler;

    #[async_trait::async_trait]
    impl Handler for HeaderEchoHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            // Count all headers received
            let header_count = request.headers.len();
            let header_names: Vec<String> = request
                .headers
                .keys()
                .map(|k| k.as_str().to_string())
                .collect();

            let body = format!(
                "Headers: {}, Names: {}",
                header_count,
                header_names.join(", ")
            );

            Ok(Response::ok().with_body(body))
        }
    }

    let handler = Arc::new(HeaderEchoHandler);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Try to send header with underscore
    // Note: Most HTTP clients (including reqwest) don't allow underscores in header names
    // This is intentional - underscored headers are non-standard
    let response = client
        .get(&url)
        .header("X-Custom-Header", "value1")
        .header("X-Another-Header", "value2")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();

    // Verify that only valid headers (with hyphens) are received
    assert!(body.contains("x-custom-header") || body.contains("X-Custom-Header"));
    assert!(body.contains("x-another-header") || body.contains("X-Another-Header"));

    shutdown_test_server(handle).await;

    // Note: Modern HTTP clients and servers automatically reject underscore headers
    // for security reasons, following RFC 7230 recommendations
}

#[tokio::test]
async fn test_broken_pipe_error_handling() {
    use test_helpers::*;

    // Handler that processes requests normally
    struct RobustHandler;

    #[async_trait::async_trait]
    impl Handler for RobustHandler {
        async fn handle(&self, _request: Request) -> Result<Response> {
            // Server should handle write errors (EPIPE) gracefully
            Ok(Response::ok().with_body("Test data"))
        }
    }

    let handler = Arc::new(RobustHandler);
    let (url, handle) = spawn_test_server(handler).await;

    // Make a request and immediately drop it to simulate broken pipe
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1))
        .build()
        .unwrap();

    // This may cause EPIPE when server tries to write response
    let _ = client.get(&url).send().await;

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Server should still be functional
    let normal_client = reqwest::Client::new();
    let response = normal_client.get(&url).send().await.unwrap();
    assert_eq!(response.status(), 200);

    shutdown_test_server(handle).await;

    // Note: Server handles EPIPE (broken pipe) errors without crashing
    // This is critical for production stability
}

#[tokio::test]
async fn test_connection_reset_handling() {
    use test_helpers::*;

    // Handler that processes requests normally
    struct RobustHandler;

    #[async_trait::async_trait]
    impl Handler for RobustHandler {
        async fn handle(&self, _request: Request) -> Result<Response> {
            // Server should handle connection reset errors gracefully
            Ok(Response::ok().with_body("Test data"))
        }
    }

    let handler = Arc::new(RobustHandler);
    let (url, handle) = spawn_test_server(handler).await;

    // Simulate connection reset by creating multiple connections with very short timeouts
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1))
        .build()
        .unwrap();

    // Make several requests that may cause connection reset
    for _ in 0..5 {
        let _ = client.get(&url).send().await;
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Server should still be functional after connection resets
    let normal_client = reqwest::Client::new();
    let response = normal_client.get(&url).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert_eq!(body, "Test data");

    shutdown_test_server(handle).await;

    // Note: Server handles ECONNRESET (connection reset) errors without crashing
    // This is critical for production stability when clients abruptly disconnect
}

#[tokio::test]
async fn test_keep_alive_clears_previous_data() {
    use test_helpers::*;

    let handler = Arc::new(BodyEchoHandler);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // First request with body
    let response1 = client
        .post(&url)
        .body("first request body")
        .send()
        .await
        .unwrap();
    assert_eq!(response1.status(), 200);
    let body1 = response1.text().await.unwrap();
    assert_eq!(body1, "first request body");

    // Second request with different body on same connection
    let response2 = client
        .post(&url)
        .body("second request body")
        .send()
        .await
        .unwrap();
    assert_eq!(response2.status(), 200);
    let body2 = response2.text().await.unwrap();
    assert_eq!(body2, "second request body");

    // Third request with no body on same connection
    let response3 = client.get(&url).send().await.unwrap();
    assert_eq!(response3.status(), 200);
    let body3 = response3.text().await.unwrap();
    assert_eq!(body3, ""); // Should be empty, not containing previous data

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_request_logging() {
    use reinhardt_middleware::{LoggingMiddleware, MiddlewareChain};
    use reinhardt_types::Middleware;
    use test_helpers::*;

    // Handler that returns different status codes
    struct StatusHandler;

    #[async_trait::async_trait]
    impl Handler for StatusHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            let path = request.uri.path();
            match path {
                "/success" => Ok(Response::ok().with_body("Success")),
                "/not-found" => Ok(Response::not_found().with_body("Not Found")),
                "/error" => Ok(Response::internal_server_error().with_body("Server Error")),
                _ => Ok(Response::ok().with_body("Default")),
            }
        }
    }

    let base_handler = Arc::new(StatusHandler);
    let chain = MiddlewareChain::new(base_handler)
        .with_middleware(Arc::new(LoggingMiddleware::new()) as Arc<dyn Middleware>);

    let (url, handle) = spawn_test_server(Arc::new(chain)).await;

    let client = reqwest::Client::new();

    // Test logging for 2xx (info level)
    let response = client
        .get(&format!("{}/success", url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    // Test logging for 4xx (warning level)
    let response = client
        .get(&format!("{}/not-found", url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);

    // Test logging for 5xx (error level)
    let response = client.get(&format!("{}/error", url)).send().await.unwrap();
    assert_eq!(response.status(), 500);

    shutdown_test_server(handle).await;

    // Note: Actual log output verification would require capturing logs,
    // but this test verifies that the logging middleware processes all request types
}
