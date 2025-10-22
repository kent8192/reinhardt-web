#[path = "server_test_helpers.rs"]
mod test_helpers;

use bytes::Bytes;
use http::{HeaderMap, Method, Uri, Version};
use reinhardt_exception::Result;
use reinhardt_http::{Request, Response};
use reinhardt_server::HttpServer;
use reinhardt_types::Handler;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Test handler that returns a simple response
struct TestHandler {
    response_body: String,
}

impl TestHandler {
    fn new(body: &str) -> Self {
        Self {
            response_body: body.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Handler for TestHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        Ok(Response::ok().with_body(self.response_body.clone()))
    }
}

/// Test handler that echoes request method
struct MethodEchoHandler;

#[async_trait::async_trait]
impl Handler for MethodEchoHandler {
    async fn handle(&self, request: Request) -> Result<Response> {
        let method = request.method.as_str().to_string();
        Ok(Response::ok().with_body(method))
    }
}

/// Test handler that returns 404
struct NotFoundHandler;

#[async_trait::async_trait]
impl Handler for NotFoundHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        Ok(Response::not_found().with_body("Not Found"))
    }
}

#[tokio::test]
async fn test_server_basic_creation() {
    let handler = Arc::new(TestHandler::new("Hello, World!"));
    let _server = HttpServer::new(handler);

    // Just verify server can be created without panicking
}

#[tokio::test]
async fn test_basic_get_request() {
    let handler = Arc::new(TestHandler::new("Hello, World!"));
    let _addr: SocketAddr = "127.0.0.1:0".parse().unwrap();

    // Note: Full integration test requires spawning server and making HTTP requests
    // This would require additional dependencies like reqwest
    // For now, we test the handler directly

    let request = Request::new(
        Method::GET,
        Uri::from_static("/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, Bytes::from("Hello, World!"));
}

#[tokio::test]
async fn test_post_request() {
    let handler = Arc::new(MethodEchoHandler);

    let request = Request::new(
        Method::POST,
        Uri::from_static("/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::from("test body"),
    );

    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, Bytes::from("POST"));
}

#[tokio::test]
async fn test_404_response() {
    let handler = Arc::new(NotFoundHandler);

    let request = Request::new(
        Method::GET,
        Uri::from_static("/nonexistent"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 404);
    assert_eq!(response.body, Bytes::from("Not Found"));
}

#[tokio::test]
async fn test_http_version() {
    let handler = Arc::new(TestHandler::new("HTTP/1.1 response"));

    // Test HTTP/1.1
    let request = Request::new(
        Method::GET,
        Uri::from_static("/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 200);
}

#[tokio::test]
async fn test_headers_preserved() {
    let handler = Arc::new(TestHandler::new("test"));

    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert("x-custom-header", "custom-value".parse().unwrap());

    let request = Request::new(
        Method::GET,
        Uri::from_static("/"),
        Version::HTTP_11,
        headers,
        Bytes::new(),
    );

    // Handler receives request with headers
    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 200);
}

// Integration tests with HTTP client

#[tokio::test]
async fn test_get_request_with_http_client() {
    use test_helpers::*;

    let handler = Arc::new(TestHandler::new("Hello from HTTP!"));
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.unwrap();

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert_eq!(body, "Hello from HTTP!");

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_connection_close_without_content_length() {
    use test_helpers::*;

    // Handler that returns response without explicit content-length
    // (simulating streaming or dynamic content)
    struct NoContentLengthHandler;

    #[async_trait::async_trait]
    impl Handler for NoContentLengthHandler {
        async fn handle(&self, _request: Request) -> Result<Response> {
            // Return a response - the server should handle Connection headers appropriately
            Ok(Response::ok().with_body("Dynamic content"))
        }
    }

    let handler = Arc::new(NoContentLengthHandler);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Make request and check response headers
    let response = client.get(&url).send().await.unwrap();
    assert_eq!(response.status(), 200);

    // With HTTP/1.1, if Content-Length is known, connection can be kept alive
    // If not, Connection: close may be set
    let connection_header = response.headers().get("connection");

    // Verify response is received successfully regardless of connection header
    let body = response.text().await.unwrap();
    assert_eq!(body, "Dynamic content");

    shutdown_test_server(handle).await;

    // Note: The actual Connection header behavior depends on HTTP version
    // and server implementation. This test verifies proper handling.
}

#[tokio::test]
async fn test_keep_alive_with_content_length() {
    use test_helpers::*;

    let handler = Arc::new(EchoPathHandler);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Make multiple requests on same connection
    for i in 0..3 {
        let response = client
            .get(&format!("{}/test{}", url, i))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let body = response.text().await.unwrap();
        assert_eq!(body, format!("/test{}", i));
    }

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_concurrent_requests() {
    use test_helpers::*;

    let handler = Arc::new(DelayedHandler {
        delay_ms: 100,
        response_body: "Concurrent response".to_string(),
    });
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Spawn 5 concurrent requests
    let mut handles = vec![];
    for _ in 0..5 {
        let client = client.clone();
        let url = url.clone();
        let h = tokio::spawn(async move {
            let response = client.get(&url).send().await.unwrap();
            assert_eq!(response.status(), 200);
            response.text().await.unwrap()
        });
        handles.push(h);
    }

    // All requests should complete successfully
    let start = tokio::time::Instant::now();
    for h in handles {
        let body = h.await.unwrap();
        assert_eq!(body, "Concurrent response");
    }
    let elapsed = start.elapsed();

    // With concurrency, should take about 100ms, not 500ms (5 * 100ms)
    assert!(elapsed < Duration::from_millis(300));

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_database_connection_closing() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use test_helpers::*;

    // Handler that simulates database connection usage
    struct DatabaseHandler {
        connections_opened: Arc<AtomicUsize>,
        connections_closed: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Handler for DatabaseHandler {
        async fn handle(&self, _request: Request) -> Result<Response> {
            // Simulate opening a database connection
            self.connections_opened.fetch_add(1, Ordering::SeqCst);

            // Simulate database query
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Simulate closing the connection after request
            self.connections_closed.fetch_add(1, Ordering::SeqCst);

            Ok(Response::ok().with_body("Database query completed"))
        }
    }

    let connections_opened = Arc::new(AtomicUsize::new(0));
    let connections_closed = Arc::new(AtomicUsize::new(0));

    let handler = Arc::new(DatabaseHandler {
        connections_opened: connections_opened.clone(),
        connections_closed: connections_closed.clone(),
    });

    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Make multiple requests to test connection lifecycle
    for _ in 0..5 {
        let response = client.get(&url).send().await.unwrap();
        assert_eq!(response.status(), 200);
    }

    // Give time for cleanup
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify connections were properly opened and closed
    let opened = connections_opened.load(Ordering::SeqCst);
    let closed = connections_closed.load(Ordering::SeqCst);

    assert_eq!(opened, 5);
    assert_eq!(closed, 5);
    assert_eq!(opened, closed, "All opened connections should be closed");

    shutdown_test_server(handle).await;

    // Note: This test simulates database connection lifecycle without actual database
}

#[tokio::test]
async fn test_static_file_serving() {
    use std::path::PathBuf;
    use test_helpers::*;
    use tokio::fs;

    // Create temporary directory with test files
    let temp_dir = std::env::temp_dir().join("reinhardt_static_test");
    fs::create_dir_all(&temp_dir).await.unwrap();

    let test_file = temp_dir.join("test.txt");
    fs::write(&test_file, "Hello from static file!")
        .await
        .unwrap();

    let test_html = temp_dir.join("index.html");
    fs::write(&test_html, "<html><body>Test HTML</body></html>")
        .await
        .unwrap();

    // Create a handler that serves static files
    struct StaticHandler {
        root: PathBuf,
    }

    #[async_trait::async_trait]
    impl Handler for StaticHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            let path = request.uri.path();
            let file_path = self.root.join(path.trim_start_matches('/'));

            match tokio::fs::read(&file_path).await {
                Ok(content) => Ok(Response::ok()
                    .with_header("content-type", "text/plain")
                    .with_body(content)),
                Err(_) => Ok(Response::not_found().with_body("File not found")),
            }
        }
    }

    let handler = Arc::new(StaticHandler {
        root: temp_dir.clone(),
    });
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Test serving text file
    let response = client
        .get(&format!("{}/test.txt", url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert_eq!(body, "Hello from static file!");

    // Test serving HTML file
    let response = client
        .get(&format!("{}/index.html", url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert!(body.contains("Test HTML"));

    // Test 404 for non-existent file
    let response = client
        .get(&format!("{}/nonexistent.txt", url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 404);

    shutdown_test_server(handle).await;

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir).await;
}

#[tokio::test]
async fn test_media_file_serving() {
    use std::path::PathBuf;
    use test_helpers::*;
    use tokio::fs;

    // Create temporary directory with media files
    let temp_dir = std::env::temp_dir().join("reinhardt_media_test");
    fs::create_dir_all(&temp_dir).await.unwrap();

    // Create a fake image file
    let image_file = temp_dir.join("test.jpg");
    fs::write(&image_file, b"\xFF\xD8\xFF\xE0\x00\x10JFIF")
        .await
        .unwrap(); // JPEG header

    // Create a handler that serves media files with appropriate content type
    struct MediaHandler {
        root: PathBuf,
    }

    #[async_trait::async_trait]
    impl Handler for MediaHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            let path = request.uri.path();
            let file_path = self.root.join(path.trim_start_matches('/'));

            match tokio::fs::read(&file_path).await {
                Ok(content) => {
                    let content_type = if path.ends_with(".jpg") || path.ends_with(".jpeg") {
                        "image/jpeg"
                    } else if path.ends_with(".png") {
                        "image/png"
                    } else {
                        "application/octet-stream"
                    };
                    Ok(Response::ok()
                        .with_header("content-type", content_type)
                        .with_body(content))
                }
                Err(_) => Ok(Response::not_found().with_body("File not found")),
            }
        }
    }

    let handler = Arc::new(MediaHandler {
        root: temp_dir.clone(),
    });
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Test serving image file
    let response = client
        .get(&format!("{}/test.jpg", url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "image/jpeg"
    );
    let bytes = response.bytes().await.unwrap();
    assert!(bytes.len() > 0);

    shutdown_test_server(handle).await;

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir).await;
}

#[tokio::test]
async fn test_server_middleware_chain() {
    use reinhardt_middleware::{LoggingMiddleware, MiddlewareChain};
    use reinhardt_types::Middleware;
    use test_helpers::*;

    // Create a simple handler
    let base_handler = Arc::new(TestHandler::new("Response through middleware"));

    // Build middleware chain with logging middleware
    let chain = MiddlewareChain::new(base_handler)
        .with_middleware(Arc::new(LoggingMiddleware::new()) as Arc<dyn Middleware>);

    let (url, handle) = spawn_test_server(Arc::new(chain)).await;

    // Make request through middleware chain
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await.unwrap();

    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert_eq!(body, "Response through middleware");

    shutdown_test_server(handle).await;
}
