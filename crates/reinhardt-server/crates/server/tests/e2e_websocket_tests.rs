#![cfg(feature = "websocket")]

use futures_util::{SinkExt, StreamExt};
use reinhardt_server::{WebSocketHandler, WebSocketServer};
use serde::{Deserialize, Serialize};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Echo handler - simply echoes messages back
struct EchoHandler;

#[async_trait::async_trait]
impl WebSocketHandler for EchoHandler {
    async fn handle_message(&self, message: String) -> Result<String, String> {
        Ok(format!("Echo: {}", message))
    }
}

/// Chat room handler - broadcasts messages to all connected clients
struct ChatRoomHandler {
    messages: Arc<Mutex<Vec<String>>>,
}

impl ChatRoomHandler {
    fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait::async_trait]
impl WebSocketHandler for ChatRoomHandler {
    async fn handle_message(&self, message: String) -> Result<String, String> {
        let mut messages = self.messages.lock().unwrap();
        messages.push(message.clone());
        Ok(format!("Message received: {}", message))
    }

    async fn on_connect(&self) {
        println!("Client connected to chat room");
    }

    async fn on_disconnect(&self) {
        println!("Client disconnected from chat room");
    }
}

/// Calculator handler - performs simple arithmetic
#[derive(Deserialize)]
struct CalculatorRequest {
    operation: String,
    a: f64,
    b: f64,
}

#[derive(Serialize, Deserialize)]
struct CalculatorResponse {
    result: f64,
}

struct CalculatorHandler;

#[async_trait::async_trait]
impl WebSocketHandler for CalculatorHandler {
    async fn handle_message(&self, message: String) -> Result<String, String> {
        let req: CalculatorRequest =
            serde_json::from_str(&message).map_err(|e| format!("Invalid JSON: {}", e))?;

        let result = match req.operation.as_str() {
            "add" => req.a + req.b,
            "subtract" => req.a - req.b,
            "multiply" => req.a * req.b,
            "divide" => {
                if req.b == 0.0 {
                    return Err("Division by zero".to_string());
                }
                req.a / req.b
            }
            _ => return Err(format!("Unknown operation: {}", req.operation)),
        };

        let response = CalculatorResponse { result };
        serde_json::to_string(&response).map_err(|e| format!("Serialization error: {}", e))
    }
}

/// Connection counter handler
struct ConnectionCounterHandler {
    connection_count: Arc<AtomicUsize>,
}

impl ConnectionCounterHandler {
    fn new() -> Self {
        Self {
            connection_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl WebSocketHandler for ConnectionCounterHandler {
    async fn handle_message(&self, _message: String) -> Result<String, String> {
        let count = self.connection_count.load(Ordering::SeqCst);
        Ok(format!("Active connections: {}", count))
    }

    async fn on_connect(&self) {
        self.connection_count.fetch_add(1, Ordering::SeqCst);
    }

    async fn on_disconnect(&self) {
        self.connection_count.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Helper function to spawn a WebSocket server
async fn spawn_websocket_server(
    handler: Arc<dyn WebSocketHandler>,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("ws://{}", addr);

    let server = WebSocketServer::new(handler);

    let handle = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let handler_clone = server.handler.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            WebSocketServer::handle_connection(stream, handler_clone, peer_addr)
                                .await
                        {
                            eprintln!("WebSocket connection error: {:?}", e);
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

    tokio::time::sleep(Duration::from_millis(100)).await;

    (url, handle)
}

#[tokio::test]
async fn test_e2e_websocket_echo() {
    let handler = Arc::new(EchoHandler);
    let (url, server_handle) = spawn_websocket_server(handler).await;

    // Connect to WebSocket server
    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Send message
    write
        .send(Message::Text("Hello".to_string()))
        .await
        .unwrap();

    // Receive echo
    let msg = timeout(Duration::from_secs(2), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    assert_eq!(msg.to_text().unwrap(), "Echo: Hello");

    server_handle.abort();
}

#[tokio::test]
async fn test_e2e_websocket_multiple_messages() {
    let handler = Arc::new(EchoHandler);
    let (url, server_handle) = spawn_websocket_server(handler).await;

    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Send multiple messages
    for i in 0..5 {
        let msg = format!("Message {}", i);
        write.send(Message::Text(msg.clone())).await.unwrap();

        let response = timeout(Duration::from_secs(2), read.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();

        assert_eq!(response.to_text().unwrap(), format!("Echo: {}", msg));
    }

    server_handle.abort();
}

#[tokio::test]
async fn test_e2e_websocket_calculator() {
    let handler = Arc::new(CalculatorHandler);
    let (url, server_handle) = spawn_websocket_server(handler).await;

    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Test addition
    let request = r#"{"operation":"add","a":5.0,"b":3.0}"#;
    write
        .send(Message::Text(request.to_string()))
        .await
        .unwrap();

    let response = timeout(Duration::from_secs(2), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let result: CalculatorResponse = serde_json::from_str(response.to_text().unwrap()).unwrap();
    assert_eq!(result.result, 8.0);

    // Test multiplication
    let request = r#"{"operation":"multiply","a":4.0,"b":2.5}"#;
    write
        .send(Message::Text(request.to_string()))
        .await
        .unwrap();

    let response = timeout(Duration::from_secs(2), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let result: CalculatorResponse = serde_json::from_str(response.to_text().unwrap()).unwrap();
    assert_eq!(result.result, 10.0);

    server_handle.abort();
}

#[tokio::test]
async fn test_e2e_websocket_calculator_error() {
    let handler = Arc::new(CalculatorHandler);
    let (url, server_handle) = spawn_websocket_server(handler).await;

    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Test division by zero
    let request = r#"{"operation":"divide","a":10.0,"b":0.0}"#;
    write
        .send(Message::Text(request.to_string()))
        .await
        .unwrap();

    let response = timeout(Duration::from_secs(2), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    assert!(response.to_text().unwrap().contains("Division by zero"));

    server_handle.abort();
}

#[tokio::test]
async fn test_e2e_websocket_chat_room() {
    let handler = Arc::new(ChatRoomHandler::new());
    let (url, server_handle) = spawn_websocket_server(handler).await;

    // Connect first client
    let (ws_stream1, _) = connect_async(&url).await.unwrap();
    let (mut write1, mut read1) = ws_stream1.split();

    // Send message from first client
    write1
        .send(Message::Text("Hello from client 1".to_string()))
        .await
        .unwrap();

    let response = timeout(Duration::from_secs(2), read1.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    assert!(response.to_text().unwrap().contains("Message received"));

    server_handle.abort();
}

#[tokio::test]
async fn test_e2e_websocket_connection_lifecycle() {
    let handler = Arc::new(ConnectionCounterHandler::new());
    let handler_clone = handler.clone();
    let (url, server_handle) = spawn_websocket_server(handler).await;

    // Initial count should be 0
    assert_eq!(handler_clone.connection_count.load(Ordering::SeqCst), 0);

    // Connect first client
    let (ws_stream1, _) = connect_async(&url).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(handler_clone.connection_count.load(Ordering::SeqCst), 1);

    // Connect second client
    let (ws_stream2, _) = connect_async(&url).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(handler_clone.connection_count.load(Ordering::SeqCst), 2);

    // Disconnect first client
    drop(ws_stream1);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(handler_clone.connection_count.load(Ordering::SeqCst), 1);

    // Disconnect second client
    drop(ws_stream2);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(handler_clone.connection_count.load(Ordering::SeqCst), 0);

    server_handle.abort();
}

#[tokio::test]
async fn test_e2e_websocket_concurrent_connections() {
    let handler = Arc::new(EchoHandler);
    let (url, server_handle) = spawn_websocket_server(handler).await;

    // Create 10 concurrent connections
    let mut handles = vec![];
    for i in 0..10 {
        let url = url.clone();
        let handle = tokio::spawn(async move {
            let (ws_stream, _) = connect_async(&url).await.unwrap();
            let (mut write, mut read) = ws_stream.split();

            let msg = format!("Message from connection {}", i);
            write.send(Message::Text(msg.clone())).await.unwrap();

            let response = timeout(Duration::from_secs(2), read.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();

            assert_eq!(response.to_text().unwrap(), format!("Echo: {}", msg));
        });
        handles.push(handle);
    }

    // Wait for all connections to complete
    for handle in handles {
        handle.await.unwrap();
    }

    server_handle.abort();
}

#[tokio::test]
async fn test_e2e_websocket_binary_message() {
    let handler = Arc::new(EchoHandler);
    let (url, server_handle) = spawn_websocket_server(handler).await;

    let (ws_stream, _) = connect_async(&url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Send binary message
    let binary_data = vec![1, 2, 3, 4, 5];
    write
        .send(Message::Binary(binary_data.clone()))
        .await
        .unwrap();

    // WebSocket server should echo binary messages
    let response = timeout(Duration::from_secs(2), read.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    assert_eq!(response.into_data(), binary_data);

    server_handle.abort();
}
