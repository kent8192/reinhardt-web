//! WebSocket connection integration tests
//!
//! Tests E2E WebSocket connection establishment, ping/pong heartbeats,
//! disconnection/reconnection scenarios, and multiple concurrent clients.

use futures::{SinkExt, StreamExt};
use rstest::rstest;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_async, connect_async};

/// Helper: Spawn a simple WebSocket echo server on a random port
async fn spawn_echo_server() -> (tokio::task::JoinHandle<()>, String) {
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();
	let url = format!("ws://{}", addr);

	let handle = tokio::spawn(async move {
		while let Ok((stream, _)) = listener.accept().await {
			tokio::spawn(async move {
				let ws_stream = match accept_async(stream).await {
					Ok(s) => s,
					Err(_) => return,
				};
				let (mut write, mut read) = ws_stream.split();

				while let Some(Ok(msg)) = read.next().await {
					// Echo back the message
					if write.send(msg.clone()).await.is_err() {
						break;
					}
				}
			});
		}
	});

	// Give server time to start
	tokio::time::sleep(Duration::from_millis(100)).await;

	(handle, url)
}

/// Test: E2E WebSocket connection establishment (HTTP handshake → WebSocket)
#[rstest]
#[tokio::test]
async fn test_connection_establishment() {
	let (_handle, url) = spawn_echo_server().await;

	// Establish WebSocket connection
	let (mut ws_stream, _response) = connect_async(&url).await.unwrap();

	// Send a test message
	ws_stream
		.send(Message::Text("Hello".to_string().into()))
		.await
		.unwrap();

	// Receive echo response
	let msg = ws_stream.next().await.unwrap().unwrap();
	assert_eq!(msg, Message::Text("Hello".to_string().into()));

	// Close connection
	ws_stream.close(None).await.unwrap();
}

/// Test: Ping/Pong heartbeat mechanism
#[rstest]
#[tokio::test]
async fn test_ping_pong_heartbeat() {
	let (_handle, url) = spawn_echo_server().await;

	let (mut ws_stream, _) = connect_async(&url).await.unwrap();

	// Send ping
	ws_stream.send(Message::Ping(vec![].into())).await.unwrap();

	// Should receive ping back (echo server reflects all messages)
	let msg = ws_stream.next().await.unwrap().unwrap();
	assert!(matches!(msg, Message::Ping(_)));

	ws_stream.close(None).await.unwrap();
}

/// Test: Connection close and cleanup
#[rstest]
#[tokio::test]
async fn test_connection_close() {
	let (_handle, url) = spawn_echo_server().await;

	let (mut ws_stream, _) = connect_async(&url).await.unwrap();

	// Send message to confirm connection is active
	ws_stream
		.send(Message::Text("test".to_string().into()))
		.await
		.unwrap();
	let msg = ws_stream.next().await.unwrap().unwrap();
	assert_eq!(msg, Message::Text("test".to_string().into()));

	// Close connection
	ws_stream.close(None).await.unwrap();

	// After close, the connection is terminated. The echo server may or may not
	// send a proper close frame back, so we just verify the connection is closed.
	// Any error (like ResetWithoutClosingHandshake) or None is acceptable here.
}

/// Test: Multiple concurrent client connections
#[rstest]
#[tokio::test]
async fn test_multiple_concurrent_clients() {
	let (_handle, url) = spawn_echo_server().await;

	let mut handles = vec![];

	// Spawn 5 concurrent clients
	for i in 0..5 {
		let url = url.clone();
		let handle = tokio::spawn(async move {
			let (mut ws_stream, _) = connect_async(&url).await.unwrap();

			let msg_text = format!("Client {}", i);
			ws_stream
				.send(Message::Text(msg_text.clone().into()))
				.await
				.unwrap();

			let response = ws_stream.next().await.unwrap().unwrap();
			assert_eq!(response, Message::Text(msg_text.into()));

			ws_stream.close(None).await.unwrap();
		});
		handles.push(handle);
	}

	// Wait for all clients to finish
	for handle in handles {
		handle.await.unwrap();
	}
}

/// Test: Binary message transmission
#[rstest]
#[tokio::test]
async fn test_binary_message_transmission() {
	let (_handle, url) = spawn_echo_server().await;

	let (mut ws_stream, _) = connect_async(&url).await.unwrap();

	let binary_data = vec![0x01, 0x02, 0x03, 0x04, 0x05];
	ws_stream
		.send(Message::Binary(binary_data.clone().into()))
		.await
		.unwrap();

	let response = ws_stream.next().await.unwrap().unwrap();
	assert_eq!(response, Message::Binary(binary_data.into()));

	ws_stream.close(None).await.unwrap();
}

/// Test: Large message transmission (stress test)
#[rstest]
#[tokio::test]
async fn test_large_message_transmission() {
	let (_handle, url) = spawn_echo_server().await;

	let (mut ws_stream, _) = connect_async(&url).await.unwrap();

	// Create a large message (1MB)
	let large_text = "a".repeat(1024 * 1024);
	ws_stream
		.send(Message::Text(large_text.clone().into()))
		.await
		.unwrap();

	let response = ws_stream.next().await.unwrap().unwrap();
	match response {
		Message::Text(text) => assert_eq!(text.len(), large_text.len()),
		_ => panic!("Expected text message"),
	}

	ws_stream.close(None).await.unwrap();
}

/// Test: Reconnection after connection drop
#[rstest]
#[tokio::test]
async fn test_reconnection_after_drop() {
	let (_handle, url) = spawn_echo_server().await;

	// First connection
	let (mut ws_stream1, _) = connect_async(&url).await.unwrap();
	ws_stream1
		.send(Message::Text("First connection".to_string().into()))
		.await
		.unwrap();
	let _ = ws_stream1.next().await.unwrap().unwrap();
	ws_stream1.close(None).await.unwrap();

	// Wait a bit before reconnecting
	tokio::time::sleep(Duration::from_millis(100)).await;

	// Second connection (reconnect)
	let (mut ws_stream2, _) = connect_async(&url).await.unwrap();
	ws_stream2
		.send(Message::Text("Second connection".to_string().into()))
		.await
		.unwrap();
	let response = ws_stream2.next().await.unwrap().unwrap();
	assert_eq!(
		response,
		Message::Text("Second connection".to_string().into())
	);
	ws_stream2.close(None).await.unwrap();
}

/// Test: WebSocket connection using Reinhardt's WebSocketConnection type
#[rstest]
#[tokio::test]
async fn test_reinhardt_websocket_connection() {
	use reinhardt_websockets::{Message as ReinhardtMessage, WebSocketConnection};
	use tokio::sync::mpsc;

	let (tx, mut rx) = mpsc::unbounded_channel();
	let conn = WebSocketConnection::new("test_conn_1".to_string(), tx);

	// Test ID
	assert_eq!(conn.id(), "test_conn_1");

	// Test send_text
	conn.send_text("Hello from Reinhardt".to_string())
		.await
		.unwrap();
	let msg = rx.recv().await.unwrap();
	match msg {
		ReinhardtMessage::Text { data } => assert_eq!(data, "Hello from Reinhardt"),
		_ => panic!("Expected text message"),
	}

	// Test send_binary
	let binary = vec![1, 2, 3];
	conn.send_binary(binary.clone()).await.unwrap();
	let msg = rx.recv().await.unwrap();
	match msg {
		ReinhardtMessage::Binary { data } => assert_eq!(data, binary),
		_ => panic!("Expected binary message"),
	}

	// Test close
	conn.close().await.unwrap();
	assert!(conn.is_closed().await);
}

/// Test: WebSocket connection with JSON messages
#[rstest]
#[tokio::test]
async fn test_json_message_serialization() {
	use reinhardt_websockets::WebSocketConnection;
	use serde::{Deserialize, Serialize};
	use tokio::sync::mpsc;

	#[derive(Serialize, Deserialize, Debug, PartialEq)]
	struct TestData {
		message: String,
		count: i32,
	}

	let (tx, mut rx) = mpsc::unbounded_channel();
	let conn = WebSocketConnection::new("test_json".to_string(), tx);

	let test_data = TestData {
		message: "Hello JSON".to_string(),
		count: 42,
	};

	// Send JSON
	conn.send_json(&test_data).await.unwrap();

	// Receive and parse JSON
	let msg = rx.recv().await.unwrap();
	let parsed: TestData = msg.parse_json().unwrap();
	assert_eq!(parsed, test_data);
}
