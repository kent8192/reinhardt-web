#[cfg(feature = "websocket")]
use bytes::Bytes;
#[cfg(feature = "websocket")]
use futures_util::{SinkExt, StreamExt};
#[cfg(feature = "websocket")]
use reinhardt_server::{ShutdownCoordinator, WebSocketHandler, WebSocketServer};
#[cfg(feature = "websocket")]
use rstest::*;
#[cfg(feature = "websocket")]
use std::sync::Arc;
#[cfg(feature = "websocket")]
use std::time::Duration;
#[cfg(feature = "websocket")]
use tokio::net::TcpListener;
#[cfg(feature = "websocket")]
use tokio_tungstenite::tungstenite::Message;

// ============================================================================
// Test Handlers
// ============================================================================

#[cfg(feature = "websocket")]
/// Ping/Pong handler - responds to ping frames
#[derive(Clone)]
struct PingPongHandler;

#[cfg(feature = "websocket")]
#[async_trait::async_trait]
impl WebSocketHandler for PingPongHandler {
	async fn handle_message(&self, message: String) -> Result<String, String> {
		Ok(message)
	}
}

#[cfg(feature = "websocket")]
/// Fragment handler - handles fragmented messages
#[derive(Clone)]
struct FragmentHandler;

#[cfg(feature = "websocket")]
#[async_trait::async_trait]
impl WebSocketHandler for FragmentHandler {
	async fn handle_message(&self, message: String) -> Result<String, String> {
		Ok(format!("Received: {}", message))
	}
}

#[cfg(feature = "websocket")]
/// Slow handler - simulates backpressure with artificial delays
#[derive(Clone)]
struct SlowHandler {
	delay: Duration,
}

#[cfg(feature = "websocket")]
#[async_trait::async_trait]
impl WebSocketHandler for SlowHandler {
	async fn handle_message(&self, message: String) -> Result<String, String> {
		tokio::time::sleep(self.delay).await;
		Ok(format!("Slow response: {}", message))
	}
}

#[cfg(feature = "websocket")]
/// Timeout handler - simulates connection timeout
#[derive(Clone)]
struct TimeoutHandler {
	response_delay: Duration,
}

#[cfg(feature = "websocket")]
#[async_trait::async_trait]
impl WebSocketHandler for TimeoutHandler {
	async fn handle_message(&self, message: String) -> Result<String, String> {
		tokio::time::sleep(self.response_delay).await;
		Ok(format!("Response: {}", message))
	}
}

// ============================================================================
// Test Fixtures
// ============================================================================

#[cfg(feature = "websocket")]
/// WebSocket server fixture for testing
#[fixture]
async fn websocket_server() -> (
	Arc<ShutdownCoordinator>,
	String,
	tokio::task::JoinHandle<()>,
) {
	let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
	let listener = TcpListener::bind(addr).await.unwrap();
	let actual_addr = listener.local_addr().unwrap();
	let url = format!("ws://{}", actual_addr);
	drop(listener);

	let coordinator = Arc::new(ShutdownCoordinator::new(Duration::from_secs(5)));
	let handler = Arc::new(PingPongHandler);

	let server_coordinator = (*coordinator).clone();
	let task = tokio::spawn(async move {
		let server = WebSocketServer::from_arc(handler);
		let _ = server
			.listen_with_shutdown(actual_addr, server_coordinator)
			.await;
	});

	tokio::time::sleep(Duration::from_millis(100)).await;

	(coordinator, url, task)
}

#[cfg(feature = "websocket")]
/// WebSocket client fixture
#[fixture]
async fn websocket_client(
	#[future] websocket_server: (
		Arc<ShutdownCoordinator>,
		String,
		tokio::task::JoinHandle<()>,
	),
) -> (
	tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
	Arc<ShutdownCoordinator>,
	tokio::task::JoinHandle<()>,
) {
	let (coordinator, url, task) = websocket_server.await;
	let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
		.await
		.expect("Failed to connect WebSocket");
	(ws_stream, coordinator, task)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(feature = "websocket")]
/// Test 1: Ping/Pong frames handling
#[rstest]
#[tokio::test]
async fn test_ping_pong_frames(
	#[future] websocket_client: (
		tokio_tungstenite::WebSocketStream<
			tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
		>,
		Arc<ShutdownCoordinator>,
		tokio::task::JoinHandle<()>,
	),
) {
	let (mut ws_stream, coordinator, task) = websocket_client.await;

	// Send Ping frame
	ws_stream
		.send(Message::Ping(Bytes::from(vec![1, 2, 3, 4])))
		.await
		.expect("Failed to send Ping");

	// Receive Pong frame
	let response = tokio::time::timeout(Duration::from_secs(2), ws_stream.next())
		.await
		.expect("Timeout waiting for Pong")
		.expect("Stream ended unexpectedly")
		.expect("WebSocket error");

	assert!(response.is_pong());
	assert_eq!(response.into_data(), Bytes::from(vec![1, 2, 3, 4]));

	// Cleanup
	coordinator.shutdown();
	task.abort();
}

#[cfg(feature = "websocket")]
/// Test 2: Close handshake
#[rstest]
#[tokio::test]
async fn test_close_handshake(
	#[future] websocket_client: (
		tokio_tungstenite::WebSocketStream<
			tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
		>,
		Arc<ShutdownCoordinator>,
		tokio::task::JoinHandle<()>,
	),
) {
	let (mut ws_stream, coordinator, task) = websocket_client.await;

	// Send Close frame
	ws_stream
		.send(Message::Close(None))
		.await
		.expect("Failed to send Close");

	// Server should respond with Close frame
	let response = tokio::time::timeout(Duration::from_secs(2), ws_stream.next())
		.await
		.expect("Timeout waiting for Close response")
		.expect("Stream ended unexpectedly")
		.expect("WebSocket error");

	assert!(response.is_close());

	// Cleanup
	coordinator.shutdown();
	task.abort();
}

#[cfg(feature = "websocket")]
/// Test 3: Message fragmentation handling
#[rstest]
#[tokio::test]
async fn test_message_fragmentation() {
	// Setup server with FragmentHandler
	let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
	let listener = TcpListener::bind(addr).await.unwrap();
	let actual_addr = listener.local_addr().unwrap();
	let url = format!("ws://{}", actual_addr);
	drop(listener);

	let coordinator = Arc::new(ShutdownCoordinator::new(Duration::from_secs(5)));
	let handler = Arc::new(FragmentHandler);

	let server_coordinator = (*coordinator).clone();
	let task = tokio::spawn(async move {
		let server = WebSocketServer::from_arc(handler);
		let _ = server
			.listen_with_shutdown(actual_addr, server_coordinator)
			.await;
	});

	tokio::time::sleep(Duration::from_millis(100)).await;

	// Connect client
	let (mut ws_stream, _) = tokio_tungstenite::connect_async(&url)
		.await
		.expect("Failed to connect WebSocket");

	// Send a large message that may be fragmented
	let large_message = "A".repeat(10000);
	ws_stream
		.send(Message::Text(large_message.clone().into()))
		.await
		.expect("Failed to send large message");

	// Receive response
	let response = tokio::time::timeout(Duration::from_secs(2), ws_stream.next())
		.await
		.expect("Timeout waiting for response")
		.expect("Stream ended unexpectedly")
		.expect("WebSocket error");

	assert!(response.is_text());
	let response_text = response.to_text().unwrap();
	assert!(response_text.contains("Received:"));

	// Cleanup
	coordinator.shutdown();
	task.abort();
}

#[cfg(feature = "websocket")]
/// Test 4: Backpressure handling
#[rstest]
#[tokio::test]
async fn test_backpressure() {
	// Setup server with SlowHandler
	let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
	let listener = TcpListener::bind(addr).await.unwrap();
	let actual_addr = listener.local_addr().unwrap();
	let url = format!("ws://{}", actual_addr);
	drop(listener);

	let coordinator = Arc::new(ShutdownCoordinator::new(Duration::from_secs(10)));
	let handler = Arc::new(SlowHandler {
		delay: Duration::from_millis(100),
	});

	let server_coordinator = (*coordinator).clone();
	let task = tokio::spawn(async move {
		let server = WebSocketServer::from_arc(handler);
		let _ = server
			.listen_with_shutdown(actual_addr, server_coordinator)
			.await;
	});

	tokio::time::sleep(Duration::from_millis(100)).await;

	// Connect client
	let (mut ws_stream, _) = tokio_tungstenite::connect_async(&url)
		.await
		.expect("Failed to connect WebSocket");

	// Send multiple messages quickly
	for i in 0..10 {
		ws_stream
			.send(Message::Text(format!("Message {}", i).into()))
			.await
			.expect("Failed to send message");
	}

	// Receive all responses (server should handle backpressure)
	let mut received_count = 0;
	for _ in 0..10 {
		let response = tokio::time::timeout(Duration::from_secs(3), ws_stream.next())
			.await
			.expect("Timeout waiting for response")
			.expect("Stream ended unexpectedly")
			.expect("WebSocket error");

		if response.is_text() {
			received_count += 1;
			assert!(response.to_text().unwrap().contains("Slow response:"));
		}
	}

	assert_eq!(received_count, 10);

	// Cleanup
	coordinator.shutdown();
	task.abort();
}

#[cfg(feature = "websocket")]
/// Test 5: Connection timeout handling
#[rstest]
#[tokio::test]
async fn test_connection_timeout() {
	// Setup server with TimeoutHandler
	let addr: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
	let listener = TcpListener::bind(addr).await.unwrap();
	let actual_addr = listener.local_addr().unwrap();
	let url = format!("ws://{}", actual_addr);
	drop(listener);

	let coordinator = Arc::new(ShutdownCoordinator::new(Duration::from_secs(5)));
	let handler = Arc::new(TimeoutHandler {
		response_delay: Duration::from_millis(50),
	});

	let server_coordinator = (*coordinator).clone();
	let task = tokio::spawn(async move {
		let server = WebSocketServer::from_arc(handler);
		let _ = server
			.listen_with_shutdown(actual_addr, server_coordinator)
			.await;
	});

	tokio::time::sleep(Duration::from_millis(100)).await;

	// Connect client
	let (mut ws_stream, _) = tokio_tungstenite::connect_async(&url)
		.await
		.expect("Failed to connect WebSocket");

	// Send message with timeout constraint
	ws_stream
		.send(Message::Text("Test timeout".into()))
		.await
		.expect("Failed to send message");

	// Receive response within timeout
	let response = tokio::time::timeout(Duration::from_secs(1), ws_stream.next())
		.await
		.expect("Timeout waiting for response")
		.expect("Stream ended unexpectedly")
		.expect("WebSocket error");

	assert!(response.is_text());
	assert_eq!(response.to_text().unwrap(), "Response: Test timeout");

	// Test connection timeout by not sending any messages
	// Connection should remain alive until explicit close or coordinator shutdown
	tokio::time::sleep(Duration::from_millis(500)).await;

	// Send another message to verify connection is still alive
	ws_stream
		.send(Message::Text("Still alive".into()))
		.await
		.expect("Failed to send message after idle period");

	let response = tokio::time::timeout(Duration::from_secs(1), ws_stream.next())
		.await
		.expect("Timeout waiting for response")
		.expect("Stream ended unexpectedly")
		.expect("WebSocket error");

	assert!(response.is_text());
	assert_eq!(response.to_text().unwrap(), "Response: Still alive");

	// Cleanup
	coordinator.shutdown();
	task.abort();
}
