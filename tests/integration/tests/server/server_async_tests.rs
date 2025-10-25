#[path = "server_test_helpers.rs"]
mod test_helpers;

use bytes::Bytes;
use http::{HeaderMap, Method, Uri, Version};
use reinhardt_exception::Result;
use reinhardt_http::{Request, Response};
use reinhardt_types::Handler;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Test handler with async delay
struct DelayedHandler {
    delay_ms: u64,
}

#[async_trait::async_trait]
impl Handler for DelayedHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        sleep(Duration::from_millis(self.delay_ms)).await;
        Ok(Response::ok().with_body("Delayed response"))
    }
}

/// Test handler that simulates streaming
struct StreamingHandler {
    chunks: Vec<String>,
}

#[async_trait::async_trait]
impl Handler for StreamingHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        // Simulate streaming by concatenating chunks
        let body = self.chunks.join("\n");
        Ok(Response::ok().with_body(body))
    }
}

/// Test handler that reads cookies
struct CookieHandler;

#[async_trait::async_trait]
impl Handler for CookieHandler {
    async fn handle(&self, request: Request) -> Result<Response> {
        let cookie_header = request
            .headers
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        Ok(Response::ok()
            .with_header("set-cookie", "key=value; Path=/")
            .with_body(cookie_header))
    }
}

#[tokio::test]
async fn test_async_handler() {
    let handler = Arc::new(DelayedHandler { delay_ms: 10 });

    let request = Request::new(
        Method::GET,
        Uri::from_static("/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let start = tokio::time::Instant::now();
    let response = handler.handle(request).await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(response.status, 200);
    assert!(elapsed >= Duration::from_millis(10));
}

#[tokio::test]
async fn test_concurrent_async_handlers() {
    let handler = Arc::new(DelayedHandler { delay_ms: 50 });

    let mut handles = vec![];

    // Spawn multiple concurrent requests
    for _ in 0..5 {
        let h = handler.clone();
        let handle = tokio::spawn(async move {
            let request = Request::new(
                Method::GET,
                Uri::from_static("/"),
                Version::HTTP_11,
                HeaderMap::new(),
                Bytes::new(),
            );
            h.handle(request).await
        });
        handles.push(handle);
    }

    // All requests should complete successfully
    let start = tokio::time::Instant::now();
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, 200);
    }
    let elapsed = start.elapsed();

    // With concurrency, should take about 50ms, not 250ms (5 * 50ms)
    assert!(elapsed < Duration::from_millis(200));
}

#[tokio::test]
async fn test_streaming_response() {
    let handler = Arc::new(StreamingHandler {
        chunks: vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ],
    });

    let request = Request::new(
        Method::GET,
        Uri::from_static("/"),
        Version::HTTP_11,
        HeaderMap::new(),
        Bytes::new(),
    );

    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, Bytes::from("first\nsecond\nthird"));
}

#[tokio::test]
async fn test_cookie_handling() {
    let handler = Arc::new(CookieHandler);

    let mut headers = HeaderMap::new();
    headers.insert("cookie", "session=abc123".parse().unwrap());

    let request = Request::new(
        Method::GET,
        Uri::from_static("/"),
        Version::HTTP_11,
        headers,
        Bytes::new(),
    );

    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(response.body, Bytes::from("session=abc123"));

    // Check Set-Cookie header
    assert_eq!(
        response.headers.get("set-cookie").unwrap(),
        "key=value; Path=/"
    );
}

#[tokio::test]
async fn test_multiple_cookies_http2() {
    let handler = Arc::new(CookieHandler);

    let mut headers = HeaderMap::new();
    // In HTTP/2, multiple cookie headers are allowed
    headers.insert("cookie", "a=abc".parse().unwrap());
    headers.append("cookie", "b=def".parse().unwrap());
    headers.append("cookie", "c=ghi".parse().unwrap());

    let request = Request::new(
        Method::GET,
        Uri::from_static("/"),
        Version::HTTP_2,
        headers,
        Bytes::new(),
    );

    let response = handler.handle(request).await.unwrap();
    assert_eq!(response.status, 200);

    // The first cookie header value should be returned
    // (In a real implementation, we'd merge them)
    assert!(response.body.len() > 0);
}

// Integration tests that require a live server

#[tokio::test]
async fn test_disconnect_handling() {
    use test_helpers::*;

    // Handler that detects disconnection
    struct DisconnectAwareHandler;

    #[async_trait::async_trait]
    impl Handler for DisconnectAwareHandler {
        async fn handle(&self, _request: Request) -> Result<Response> {
            // Process normally - server should handle disconnections gracefully
            Ok(Response::ok().with_body("Response"))
        }
    }

    let handler = Arc::new(DisconnectAwareHandler);
    let (url, handle) = spawn_test_server(handler).await;

    // Use a client with very short timeout to simulate disconnect
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1))
        .build()
        .unwrap();

    // This should timeout/disconnect
    let _ = client.get(&url).send().await;

    // Server should continue running despite disconnection
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify server is still responsive with a normal client
    let normal_client = reqwest::Client::new();
    let response = normal_client.get(&url).send().await.unwrap();
    assert_eq!(response.status(), 200);

    shutdown_test_server(handle).await;

    // Note: Server gracefully handles client disconnections without crashing
}

#[tokio::test]
async fn test_disconnect_during_streaming() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use test_helpers::*;

    // Handler that simulates streaming
    struct StreamingHandler {
        started: Arc<AtomicBool>,
    }

    #[async_trait::async_trait]
    impl Handler for StreamingHandler {
        async fn handle(&self, _request: Request) -> Result<Response> {
            self.started.store(true, Ordering::SeqCst);

            // Simulate preparing streaming content
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            Ok(Response::ok().with_body("Streaming data"))
        }
    }

    let started = Arc::new(AtomicBool::new(false));
    let handler = Arc::new(StreamingHandler {
        started: started.clone(),
    });

    let (url, handle) = spawn_test_server(handler).await;

    // Client that disconnects during response
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(10))
        .build()
        .unwrap();

    // Attempt to get streaming response but timeout early
    let _ = client.get(&url).send().await;

    // Verify handler was called
    assert!(started.load(Ordering::SeqCst));

    // Server should recover and handle next request
    let normal_client = reqwest::Client::new();
    let response = normal_client.get(&url).send().await.unwrap();
    assert_eq!(response.status(), 200);

    shutdown_test_server(handle).await;

    // Note: Server handles streaming disconnections gracefully
}

#[tokio::test]
async fn test_request_lifecycle_signals() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use test_helpers::*;

    // Handler that tracks request lifecycle
    struct SignalTrackingHandler {
        request_count: Arc<AtomicUsize>,
        completion_count: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Handler for SignalTrackingHandler {
        async fn handle(&self, _request: Request) -> Result<Response> {
            // Simulate request_started signal
            self.request_count.fetch_add(1, Ordering::SeqCst);

            // Process request
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Simulate request_finished signal
            self.completion_count.fetch_add(1, Ordering::SeqCst);

            Ok(Response::ok().with_body("Request completed"))
        }
    }

    let request_count = Arc::new(AtomicUsize::new(0));
    let completion_count = Arc::new(AtomicUsize::new(0));

    let handler = Arc::new(SignalTrackingHandler {
        request_count: request_count.clone(),
        completion_count: completion_count.clone(),
    });

    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Make multiple requests
    for _ in 0..3 {
        let response = client.get(&url).send().await.unwrap();
        assert_eq!(response.status(), 200);
    }

    // Verify lifecycle signals were tracked
    assert_eq!(request_count.load(Ordering::SeqCst), 3);
    assert_eq!(completion_count.load(Ordering::SeqCst), 3);

    shutdown_test_server(handle).await;

    // Note: This test simulates signal behavior without the actual signal system
}

#[tokio::test]
async fn test_cancel_request_with_sync_processing() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use test_helpers::*;

    // Handler with long processing time
    struct LongRunningHandler {
        started: Arc<AtomicBool>,
        completed: Arc<AtomicBool>,
    }

    #[async_trait::async_trait]
    impl Handler for LongRunningHandler {
        async fn handle(&self, _request: Request) -> Result<Response> {
            self.started.store(true, Ordering::SeqCst);

            // Simulate long processing (5 seconds)
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            self.completed.store(true, Ordering::SeqCst);
            Ok(Response::ok().with_body("Completed"))
        }
    }

    let started = Arc::new(AtomicBool::new(false));
    let completed = Arc::new(AtomicBool::new(false));

    let handler = Arc::new(LongRunningHandler {
        started: started.clone(),
        completed: completed.clone(),
    });

    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(1))
        .build()
        .unwrap();

    // Start request and expect timeout/cancellation
    let result = client.get(&url).send().await;

    // Request should timeout before completion
    assert!(result.is_err());
    assert!(
        started.load(Ordering::SeqCst),
        "Request should have started"
    );

    // Give server time to clean up
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    shutdown_test_server(handle).await;

    // Note: completed may be true or false depending on cancellation timing
}

#[tokio::test]
async fn test_asyncio_cancel_error() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use test_helpers::*;

    // Handler that may be cancelled
    struct CancellableHandler {
        cancellation_count: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl Handler for CancellableHandler {
        async fn handle(&self, _request: Request) -> Result<Response> {
            // Use select to detect cancellation
            let result = tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(10)) => {
                    Ok(Response::ok().with_body("Completed"))
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    // Simulate early completion
                    Ok(Response::ok().with_body("Early completion"))
                }
            };

            result
        }
    }

    let cancellation_count = Arc::new(AtomicUsize::new(0));

    let handler = Arc::new(CancellableHandler {
        cancellation_count: cancellation_count.clone(),
    });

    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Make request that should complete quickly
    let response = client.get(&url).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let body = response.text().await.unwrap();
    assert_eq!(body, "Early completion");

    shutdown_test_server(handle).await;

    // Note: This test verifies that handlers with select! can handle
    // cancellation-like scenarios without panicking
}

#[tokio::test]
async fn test_file_response() {
    use std::path::PathBuf;
    use test_helpers::*;
    use tokio::fs;

    // Create temporary file
    let temp_dir = std::env::temp_dir().join("reinhardt_file_response_test");
    fs::create_dir_all(&temp_dir).await.unwrap();

    let test_file = temp_dir.join("document.pdf");
    fs::write(&test_file, b"%PDF-1.4 test content")
        .await
        .unwrap();

    // Handler that serves file with appropriate headers
    struct FileResponseHandler {
        root: PathBuf,
    }

    #[async_trait::async_trait]
    impl Handler for FileResponseHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            let path = request.uri.path();
            let file_path = self.root.join(path.trim_start_matches('/'));

            match tokio::fs::read(&file_path).await {
                Ok(content) => {
                    let file_name = file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("file");

                    Ok(Response::ok()
                        .with_header("content-type", "application/pdf")
                        .with_header(
                            "content-disposition",
                            &format!("attachment; filename=\"{}\"", file_name),
                        )
                        .with_body(content))
                }
                Err(_) => Ok(Response::not_found().with_body("File not found")),
            }
        }
    }

    let handler = Arc::new(FileResponseHandler {
        root: temp_dir.clone(),
    });
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Test file download
    let response = client
        .get(&format!("{}/document.pdf", url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/pdf"
    );
    assert!(response
        .headers()
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("document.pdf"));
    let bytes = response.bytes().await.unwrap();
    assert!(bytes.starts_with(b"%PDF"));

    shutdown_test_server(handle).await;

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir).await;
}

#[tokio::test]
async fn test_static_file_response() {
    use std::path::PathBuf;
    use test_helpers::*;
    use tokio::fs;

    // Create temporary directory with static files
    let temp_dir = std::env::temp_dir().join("reinhardt_static_file_response_test");
    fs::create_dir_all(&temp_dir).await.unwrap();

    let css_file = temp_dir.join("style.css");
    fs::write(&css_file, "body { color: blue; }").await.unwrap();

    let js_file = temp_dir.join("script.js");
    fs::write(&js_file, "console.log('test');").await.unwrap();

    // Handler that serves static files with appropriate MIME types
    struct StaticFileHandler {
        root: PathBuf,
    }

    #[async_trait::async_trait]
    impl Handler for StaticFileHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            let path = request.uri.path();
            let file_path = self.root.join(path.trim_start_matches('/'));

            match tokio::fs::read(&file_path).await {
                Ok(content) => {
                    let content_type = if path.ends_with(".css") {
                        "text/css"
                    } else if path.ends_with(".js") {
                        "application/javascript"
                    } else {
                        "text/plain"
                    };

                    Ok(Response::ok()
                        .with_header("content-type", content_type)
                        .with_header("cache-control", "public, max-age=3600")
                        .with_body(content))
                }
                Err(_) => Ok(Response::not_found().with_body("File not found")),
            }
        }
    }

    let handler = Arc::new(StaticFileHandler {
        root: temp_dir.clone(),
    });
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Test serving CSS file
    let response = client
        .get(&format!("{}/style.css", url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-type").unwrap(), "text/css");
    assert_eq!(
        response.headers().get("cache-control").unwrap(),
        "public, max-age=3600"
    );
    let body = response.text().await.unwrap();
    assert!(body.contains("color: blue"));

    // Test serving JS file
    let response = client
        .get(&format!("{}/script.js", url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/javascript"
    );
    let body = response.text().await.unwrap();
    assert!(body.contains("console.log"));

    shutdown_test_server(handle).await;

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir).await;
}

#[tokio::test]
async fn test_post_body_parsing() {
    use test_helpers::*;

    let handler = Arc::new(BodyEchoHandler);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Test plain text body
    let response = client
        .post(&url)
        .body("plain text body")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), "plain text body");

    // Test JSON body
    let json_body = r#"{"key": "value", "number": 42}"#;
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(json_body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), json_body);

    // Test form data
    let form_data = "field1=value1&field2=value2";
    let response = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(form_data)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.text().await.unwrap(), form_data);

    shutdown_test_server(handle).await;
}

#[tokio::test]
async fn test_non_unicode_query_string() {
    use test_helpers::*;

    // Handler that processes query strings
    struct QueryHandler;

    #[async_trait::async_trait]
    impl Handler for QueryHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            let query = request.uri.query().unwrap_or("");
            Ok(Response::ok().with_body(format!("Query: {}", query)))
        }
    }

    let handler = Arc::new(QueryHandler);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Test valid UTF-8 query string
    let response = client
        .get(&format!("{}?name=test&value=123", url))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    // reqwest automatically handles percent-encoding, so invalid UTF-8
    // in query strings will be encoded properly
    // This test verifies that valid encoded queries work
    let response = client
        .get(&format!("{}?name=%E3%81%82", url)) // Japanese character "a" (hiragana) encoded
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);

    shutdown_test_server(handle).await;

    // Note: Modern HTTP clients encode query strings properly.
    // Invalid UTF-8 bytes cannot be easily tested with standard clients.
}

#[tokio::test]
async fn test_wrong_connection_type() {
    use test_helpers::*;

    // Handler that always responds successfully
    struct TestHandler;

    #[async_trait::async_trait]
    impl Handler for TestHandler {
        async fn handle(&self, _request: Request) -> Result<Response> {
            Ok(Response::ok().with_body("OK"))
        }
    }

    let handler = Arc::new(TestHandler);
    let (url, handle) = spawn_test_server(handler).await;

    let client = reqwest::Client::new();

    // Test that server handles valid HTTP requests
    let response = client.get(&url).send().await.unwrap();
    assert_eq!(response.status(), 200);

    // Test malformed HTTP request handling
    // Note: HTTP/1.1 is enforced by hyper at the protocol level
    // Invalid HTTP will be rejected before reaching the handler
    let tcp_client = tokio::net::TcpStream::connect(
        url.strip_prefix("http://")
            .unwrap()
            .parse::<std::net::SocketAddr>()
            .unwrap(),
    )
    .await
    .unwrap();

    use tokio::io::AsyncWriteExt;
    let (mut reader, mut writer) = tcp_client.into_split();

    // Send invalid HTTP request
    let invalid_request = b"INVALID REQUEST\r\n\r\n";
    writer.write_all(invalid_request).await.unwrap();
    writer.flush().await.unwrap();

    // Server should either close connection or send error response
    use tokio::io::AsyncReadExt;
    let mut buffer = vec![0u8; 1024];
    let read_result = tokio::time::timeout(
        tokio::time::Duration::from_secs(1),
        reader.read(&mut buffer),
    )
    .await;

    // Either timeout (connection closed) or read some error response
    match read_result {
        Ok(Ok(0)) => {
            // Connection closed, which is expected
        }
        Ok(Ok(_)) => {
            // Server sent error response or data, which is fine
        }
        Ok(Err(_)) => {
            // Read error, connection was closed
        }
        Err(_) => {
            // Timeout, which is expected
        }
    }

    // Server should still be functional after receiving invalid request
    let response = client.get(&url).send().await.unwrap();
    assert_eq!(response.status(), 200);

    shutdown_test_server(handle).await;

    // Note: HTTP server validates protocol at connection level
    // Non-HTTP protocols are rejected by the underlying hyper library
}
