//! Integration tests for Server implementation

use async_trait::async_trait;
use reinhardt_apps::{Handler, Request, Response, Result, Router, Server};
use std::sync::Arc;

/// Simple handler for testing
struct TestHandler {
    message: String,
}

#[async_trait]
impl Handler for TestHandler {
    async fn handle(&self, _request: Request) -> Result<Response> {
        Ok(Response::ok().with_body(self.message.clone()))
    }
}

/// Handler that echoes request method
struct EchoMethodHandler;

#[async_trait]
impl Handler for EchoMethodHandler {
    async fn handle(&self, request: Request) -> Result<Response> {
        Ok(Response::ok().with_body(format!("Method: {}", request.method)))
    }
}

/// Handler that returns request headers
struct HeadersHandler;

#[async_trait]
impl Handler for HeadersHandler {
    async fn handle(&self, request: Request) -> Result<Response> {
        let user_agent = request
            .headers
            .get(hyper::header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("Unknown");

        Ok(Response::ok().with_body(format!("User-Agent: {}", user_agent)))
    }
}

#[tokio::test]
async fn test_server_basic_request() {
    let router = Arc::new(Router::new().get(
        "/test",
        Arc::new(TestHandler {
            message: "Server works!".to_string(),
        }),
    ));

    // Spawn server in background
    let addr = "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let server_addr = listener.local_addr().unwrap();

    let router_clone = router.clone();
    tokio::spawn(async move {
        let server = Server::new(router_clone);
        let _ = server.listen(server_addr).await;
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Make HTTP request
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/test", server_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "Server works!");
}

#[tokio::test]
async fn test_server_multiple_requests() {
    let router = Arc::new(
        Router::new()
            .get(
                "/hello",
                Arc::new(TestHandler {
                    message: "Hello!".to_string(),
                }),
            )
            .get(
                "/goodbye",
                Arc::new(TestHandler {
                    message: "Goodbye!".to_string(),
                }),
            ),
    );

    let addr = "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let server_addr = listener.local_addr().unwrap();

    let router_clone = router.clone();
    tokio::spawn(async move {
        let server = Server::new(router_clone);
        let _ = server.listen(server_addr).await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();

    let response = client
        .get(format!("http://{}/hello", server_addr))
        .send()
        .await
        .unwrap();
    assert_eq!(response.text().await.unwrap(), "Hello!");

    let response = client
        .get(format!("http://{}/goodbye", server_addr))
        .send()
        .await
        .unwrap();
    assert_eq!(response.text().await.unwrap(), "Goodbye!");
}

#[tokio::test]
async fn test_server_post_request() {
    struct PostHandler;

    #[async_trait]
    impl Handler for PostHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            let body_str = String::from_utf8(request.body.to_vec()).unwrap_or_default();
            Ok(Response::ok().with_body(format!("Received: {}", body_str)))
        }
    }

    let router = Arc::new(Router::new().post("/submit", Arc::new(PostHandler)));

    let addr = "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let server_addr = listener.local_addr().unwrap();

    let router_clone = router.clone();
    tokio::spawn(async move {
        let server = Server::new(router_clone);
        let _ = server.listen(server_addr).await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{}/submit", server_addr))
        .body("test data")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "Received: test data");
}

#[tokio::test]
async fn test_server_json_request_response() {
    struct JsonEchoHandler;

    #[async_trait]
    impl Handler for JsonEchoHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            let json: serde_json::Value = request.json()?;
            Response::ok().with_json(&json)
        }
    }

    let router = Arc::new(Router::new().post("/echo", Arc::new(JsonEchoHandler)));

    let addr = "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let server_addr = listener.local_addr().unwrap();

    let router_clone = router.clone();
    tokio::spawn(async move {
        let server = Server::new(router_clone);
        let _ = server.listen(server_addr).await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let test_data = serde_json::json!({
        "name": "Alice",
        "age": 30
    });

    let response = client
        .post(format!("http://{}/echo", server_addr))
        .json(&test_data)
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let response_json: serde_json::Value = response.json().await.unwrap();
    assert_eq!(response_json["name"], "Alice");
    assert_eq!(response_json["age"], 30);
}

#[tokio::test]
async fn test_server_404_response() {
    let router = Arc::new(Router::new().get(
        "/exists",
        Arc::new(TestHandler {
            message: "I exist!".to_string(),
        }),
    ));

    let addr = "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let server_addr = listener.local_addr().unwrap();

    let router_clone = router.clone();
    tokio::spawn(async move {
        let server = Server::new(router_clone);
        let _ = server.listen(server_addr).await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/notfound", server_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_server_concurrent_requests() {
    let router = Arc::new(Router::new().get("/test", Arc::new(EchoMethodHandler)));

    let addr = "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let server_addr = listener.local_addr().unwrap();

    let router_clone = router.clone();
    tokio::spawn(async move {
        let server = Server::new(router_clone);
        let _ = server.listen(server_addr).await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Make multiple concurrent requests
    let mut handles = vec![];
    for _ in 0..10 {
        let addr = server_addr;
        let handle = tokio::spawn(async move {
            let client = reqwest::Client::new();
            client
                .get(format!("http://{}/test", addr))
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert_eq!(result, "Method: GET");
    }
}

#[tokio::test]
async fn test_server_custom_headers() {
    let router = Arc::new(Router::new().get("/headers", Arc::new(HeadersHandler)));

    let addr = "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let server_addr = listener.local_addr().unwrap();

    let router_clone = router.clone();
    tokio::spawn(async move {
        let server = Server::new(router_clone);
        let _ = server.listen(server_addr).await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/headers", server_addr))
        .header("User-Agent", "TestAgent/1.0")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "User-Agent: TestAgent/1.0");
}

#[tokio::test]
async fn test_server_path_parameters() {
    struct PathParamHandler;

    #[async_trait]
    impl Handler for PathParamHandler {
        async fn handle(&self, request: Request) -> Result<Response> {
            let id = request.path_params.get("id").unwrap();
            Ok(Response::ok().with_body(format!("ID: {}", id)))
        }
    }

    let router = Arc::new(Router::new().get("/users/:id", Arc::new(PathParamHandler)));

    let addr = "127.0.0.1:0".parse::<std::net::SocketAddr>().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let server_addr = listener.local_addr().unwrap();

    let router_clone = router.clone();
    tokio::spawn(async move {
        let server = Server::new(router_clone);
        let _ = server.listen(server_addr).await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/users/123", server_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(response.text().await.unwrap(), "ID: 123");
}
