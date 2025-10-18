// Test helper functions for integration tests
use reinhardt_http::{Request, Response};
use reinhardt_server::HttpServer;
use reinhardt_types::Handler;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// Spawn a test server on a random port and return the URL and server handle
pub async fn spawn_test_server(handler: Arc<dyn Handler>) -> (String, JoinHandle<()>) {
    // Bind to port 0 to get a random available port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{}", addr);

    // Create server
    let server = HttpServer::new(handler);

    // Spawn server in background task
    let handle = tokio::spawn(async move {
        // Accept connections manually since we need to use our existing listener
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let handler_clone = server.handler.clone();
                    tokio::spawn(async move {
                        if let Err(e) = HttpServer::handle_connection(stream, handler_clone).await {
                            eprintln!("Error handling connection: {:?}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting connection: {:?}", e);
                    break;
                }
            }
        }
    });

    // Give the server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    (url, handle)
}

/// Shutdown a test server
pub async fn shutdown_test_server(handle: JoinHandle<()>) {
    handle.abort();
    // Give it a moment to clean up
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
}

/// Simple test handler that echoes the request path
pub struct EchoPathHandler;

#[async_trait::async_trait]
impl Handler for EchoPathHandler {
    async fn handle(&self, request: Request) -> reinhardt_exception::Result<Response> {
        let path = request.path().to_string();
        Ok(Response::ok().with_body(path))
    }
}

/// Test handler that returns specific status codes based on path
pub struct StatusCodeHandler;

#[async_trait::async_trait]
impl Handler for StatusCodeHandler {
    async fn handle(&self, request: Request) -> reinhardt_exception::Result<Response> {
        match request.path() {
            "/200" => Ok(Response::ok().with_body("OK")),
            "/404" => Ok(Response::not_found().with_body("Not Found")),
            "/500" => Ok(Response::internal_server_error().with_body("Internal Server Error")),
            _ => Ok(Response::ok().with_body("Default")),
        }
    }
}

/// Test handler that echoes the request method
pub struct MethodEchoHandler;

#[async_trait::async_trait]
impl Handler for MethodEchoHandler {
    async fn handle(&self, request: Request) -> reinhardt_exception::Result<Response> {
        let method = request.method.as_str().to_string();
        Ok(Response::ok().with_body(method))
    }
}

/// Test handler with configurable delay
pub struct DelayedHandler {
    pub delay_ms: u64,
    pub response_body: String,
}

#[async_trait::async_trait]
impl Handler for DelayedHandler {
    async fn handle(&self, _request: Request) -> reinhardt_exception::Result<Response> {
        tokio::time::sleep(tokio::time::Duration::from_millis(self.delay_ms)).await;
        Ok(Response::ok().with_body(self.response_body.clone()))
    }
}

/// Test handler that echoes the request body
pub struct BodyEchoHandler;

#[async_trait::async_trait]
impl Handler for BodyEchoHandler {
    async fn handle(&self, request: Request) -> reinhardt_exception::Result<Response> {
        let body = request.read_body()?;
        Ok(Response::ok().with_body(body))
    }
}

/// Test handler that returns a large response
pub struct LargeResponseHandler {
    pub size_kb: usize,
}

#[async_trait::async_trait]
impl Handler for LargeResponseHandler {
    async fn handle(&self, _request: Request) -> reinhardt_exception::Result<Response> {
        let data = "x".repeat(self.size_kb * 1024);
        Ok(Response::ok().with_body(data))
    }
}

/// Test handler that returns different responses based on path
pub struct RouterHandler;

#[async_trait::async_trait]
impl Handler for RouterHandler {
    async fn handle(&self, request: Request) -> reinhardt_exception::Result<Response> {
        let path = request.uri.path();

        match path {
            "/" => Ok(Response::ok().with_body("Home")),
            "/api" => Ok(Response::ok().with_body(r#"{"status": "ok"}"#)),
            "/notfound" => Ok(Response::not_found().with_body("Not Found")),
            _ => Ok(Response::not_found().with_body("Unknown path")),
        }
    }
}
