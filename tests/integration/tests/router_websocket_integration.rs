// Router and WebSocket integration tests
// Inspired by Django Channels' WebSocket routing and FastAPI's WebSocket support

use async_trait::async_trait;
use reinhardt_websockets::{
    Message, RoomManager, WebSocketConnection, WebSocketHandler, WebSocketResult,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

// Mock echo handler
#[derive(Clone)]
struct EchoHandler;

#[async_trait]
impl WebSocketHandler for EchoHandler {
    async fn on_connect(&self, _connection: Arc<WebSocketConnection>) -> WebSocketResult<()> {
        Ok(())
    }

    async fn on_message(
        &self,
        connection: Arc<WebSocketConnection>,
        message: Message,
    ) -> WebSocketResult<()> {
        // Echo back the message
        connection.send(message).await
    }

    async fn on_close(&self, _connection: Arc<WebSocketConnection>) -> WebSocketResult<()> {
        Ok(())
    }

    async fn on_error(
        &self,
        _connection: Arc<WebSocketConnection>,
        _error: String,
    ) -> WebSocketResult<()> {
        Ok(())
    }
}

// Broadcast handler
struct BroadcastHandler {
    room_manager: Arc<RoomManager>,
    room: String,
}

impl BroadcastHandler {
    fn new(room_manager: Arc<RoomManager>, room: String) -> Self {
        Self { room_manager, room }
    }
}

#[async_trait]
impl WebSocketHandler for BroadcastHandler {
    async fn on_connect(&self, connection: Arc<WebSocketConnection>) -> WebSocketResult<()> {
        self.room_manager
            .join_room(self.room.clone(), connection)
            .await;
        Ok(())
    }

    async fn on_message(
        &self,
        _connection: Arc<WebSocketConnection>,
        message: Message,
    ) -> WebSocketResult<()> {
        // Broadcast to all in room
        self.room_manager
            .broadcast_to_room(&self.room, message)
            .await
    }

    async fn on_close(&self, connection: Arc<WebSocketConnection>) -> WebSocketResult<()> {
        self.room_manager
            .leave_room(&self.room, connection.id())
            .await;
        Ok(())
    }

    async fn on_error(
        &self,
        _connection: Arc<WebSocketConnection>,
        _error: String,
    ) -> WebSocketResult<()> {
        Ok(())
    }
}

// Chat room handler with user tracking
struct ChatRoomHandler {
    room_manager: Arc<RoomManager>,
    connection_count: Arc<AtomicUsize>,
}

impl ChatRoomHandler {
    fn new(room_manager: Arc<RoomManager>) -> Self {
        Self {
            room_manager,
            connection_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn get_connection_count(&self) -> usize {
        self.connection_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl WebSocketHandler for ChatRoomHandler {
    async fn on_connect(&self, connection: Arc<WebSocketConnection>) -> WebSocketResult<()> {
        self.connection_count.fetch_add(1, Ordering::SeqCst);
        self.room_manager
            .join_room("chat".to_string(), connection.clone())
            .await;

        // Send join message to room
        let join_msg = Message::text(format!("{} joined the room", connection.id()));
        self.room_manager
            .broadcast_to_room("chat", join_msg)
            .await?;

        Ok(())
    }

    async fn on_message(
        &self,
        connection: Arc<WebSocketConnection>,
        message: Message,
    ) -> WebSocketResult<()> {
        // Broadcast message with sender ID
        if let Message::Text { data } = message {
            let formatted = Message::text(format!("{}: {}", connection.id(), data));
            self.room_manager
                .broadcast_to_room("chat", formatted)
                .await?;
        }
        Ok(())
    }

    async fn on_close(&self, connection: Arc<WebSocketConnection>) -> WebSocketResult<()> {
        self.connection_count.fetch_sub(1, Ordering::SeqCst);
        self.room_manager.leave_room("chat", connection.id()).await;

        // Send leave message
        let leave_msg = Message::text(format!("{} left the room", connection.id()));
        self.room_manager
            .broadcast_to_room("chat", leave_msg)
            .await?;

        Ok(())
    }

    async fn on_error(
        &self,
        _connection: Arc<WebSocketConnection>,
        _error: String,
    ) -> WebSocketResult<()> {
        Ok(())
    }
}

// Test 1: Basic WebSocket connection
#[tokio::test]
async fn test_basic_websocket_connection() {
    let (tx, _rx) = mpsc::unbounded_channel();
    let connection = Arc::new(WebSocketConnection::new("user1".to_string(), tx));

    assert_eq!(connection.id(), "user1");
    assert!(!connection.is_closed().await);
}

// Test 2: WebSocket echo handler
#[tokio::test]
async fn test_websocket_echo_handler() {
    let handler = EchoHandler;
    let (tx, mut rx) = mpsc::unbounded_channel();
    let connection = Arc::new(WebSocketConnection::new("user1".to_string(), tx));

    // Test connection
    handler.on_connect(connection.clone()).await.unwrap();

    // Test echo
    let msg = Message::text("Hello".to_string());
    handler
        .on_message(connection.clone(), msg.clone())
        .await
        .unwrap();

    let received = rx.recv().await.unwrap();
    match received {
        Message::Text { data } => assert_eq!(data, "Hello"),
        _ => panic!("Expected text message"),
    }

    // Test close
    handler.on_close(connection).await.unwrap();
}

// Test 3: WebSocket connection lifecycle
#[tokio::test]
async fn test_websocket_connection_lifecycle() {
    let handler = EchoHandler;
    let (tx, _rx) = mpsc::unbounded_channel();
    let connection = Arc::new(WebSocketConnection::new("user1".to_string(), tx));

    // Connect
    assert!(handler.on_connect(connection.clone()).await.is_ok());

    // Send message
    let msg = Message::text("test".to_string());
    assert!(handler.on_message(connection.clone(), msg).await.is_ok());

    // Close
    assert!(handler.on_close(connection.clone()).await.is_ok());
}

// Test 4: WebSocket broadcast handler
#[tokio::test]
async fn test_websocket_broadcast_handler() {
    let room_manager = Arc::new(RoomManager::new());
    let handler = BroadcastHandler::new(room_manager.clone(), "room1".to_string());

    let (tx1, mut rx1) = mpsc::unbounded_channel();
    let (tx2, mut rx2) = mpsc::unbounded_channel();

    let conn1 = Arc::new(WebSocketConnection::new("user1".to_string(), tx1));
    let conn2 = Arc::new(WebSocketConnection::new("user2".to_string(), tx2));

    // Connect both users
    handler.on_connect(conn1.clone()).await.unwrap();
    handler.on_connect(conn2.clone()).await.unwrap();

    // Send message from user1
    let msg = Message::text("Hello everyone!".to_string());
    handler.on_message(conn1, msg).await.unwrap();

    // Both should receive
    let received1 = rx1.recv().await.unwrap();
    let received2 = rx2.recv().await.unwrap();

    match (received1, received2) {
        (Message::Text { data: d1 }, Message::Text { data: d2 }) => {
            assert_eq!(d1, "Hello everyone!");
            assert_eq!(d2, "Hello everyone!");
        }
        _ => panic!("Expected text messages"),
    }
}

// Test 5: Chat room WebSocket handler
#[tokio::test]
async fn test_chat_room_websocket_handler() {
    let room_manager = Arc::new(RoomManager::new());
    let handler = ChatRoomHandler::new(room_manager.clone());

    let (tx1, mut rx1) = mpsc::unbounded_channel();
    let (tx2, mut rx2) = mpsc::unbounded_channel();

    let conn1 = Arc::new(WebSocketConnection::new("alice".to_string(), tx1));
    let conn2 = Arc::new(WebSocketConnection::new("bob".to_string(), tx2));

    // Alice connects
    handler.on_connect(conn1.clone()).await.unwrap();
    assert_eq!(handler.get_connection_count(), 1);

    // Clear join message
    let _ = rx1.recv().await;

    // Bob connects
    handler.on_connect(conn2.clone()).await.unwrap();
    assert_eq!(handler.get_connection_count(), 2);

    // Both should receive bob's join message
    let _ = rx1.recv().await;
    let _ = rx2.recv().await;

    // Alice sends a message
    let msg = Message::text("Hi Bob!".to_string());
    handler.on_message(conn1.clone(), msg).await.unwrap();

    // Both should receive the formatted message
    let received1 = rx1.recv().await.unwrap();
    let received2 = rx2.recv().await.unwrap();

    match (received1, received2) {
        (Message::Text { data: d1 }, Message::Text { data: d2 }) => {
            assert!(d1.contains("alice:"));
            assert!(d1.contains("Hi Bob!"));
            assert_eq!(d1, d2);
        }
        _ => panic!("Expected text messages"),
    }

    // Alice disconnects
    handler.on_close(conn1).await.unwrap();
    assert_eq!(handler.get_connection_count(), 1);
}

// Test 6: Multiple WebSocket rooms
#[tokio::test]
async fn test_multiple_websocket_rooms() {
    let room_manager = Arc::new(RoomManager::new());

    let (tx1, _rx1) = mpsc::unbounded_channel();
    let (tx2, _rx2) = mpsc::unbounded_channel();

    let conn1 = Arc::new(WebSocketConnection::new("user1".to_string(), tx1));
    let conn2 = Arc::new(WebSocketConnection::new("user2".to_string(), tx2));

    room_manager.join_room("room1".to_string(), conn1).await;
    room_manager.join_room("room2".to_string(), conn2).await;

    assert_eq!(room_manager.get_room_size("room1").await, 1);
    assert_eq!(room_manager.get_room_size("room2").await, 1);

    let rooms = room_manager.get_all_rooms().await;
    assert_eq!(rooms.len(), 2);
    assert!(rooms.contains(&"room1".to_string()));
    assert!(rooms.contains(&"room2".to_string()));
}

// Test 7: WebSocket with URL parameters (simulated)
#[tokio::test]
async fn test_websocket_with_url_parameters() {
    let room_manager = Arc::new(RoomManager::new());

    // Simulate room IDs from URL params
    let room_id_1 = "lobby";
    let room_id_2 = "vip";

    let handler1 = BroadcastHandler::new(room_manager.clone(), room_id_1.to_string());
    let handler2 = BroadcastHandler::new(room_manager.clone(), room_id_2.to_string());

    let (tx1, _rx1) = mpsc::unbounded_channel();
    let (tx2, _rx2) = mpsc::unbounded_channel();

    let conn1 = Arc::new(WebSocketConnection::new("user1".to_string(), tx1));
    let conn2 = Arc::new(WebSocketConnection::new("user2".to_string(), tx2));

    handler1.on_connect(conn1).await.unwrap();
    handler2.on_connect(conn2).await.unwrap();

    assert_eq!(room_manager.get_room_size("lobby").await, 1);
    assert_eq!(room_manager.get_room_size("vip").await, 1);
}

// Test 8: WebSocket message types
#[tokio::test]
async fn test_websocket_message_types() {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let connection = Arc::new(WebSocketConnection::new("user1".to_string(), tx));

    // Text message
    connection.send_text("Hello".to_string()).await.unwrap();
    match rx.recv().await.unwrap() {
        Message::Text { data } => assert_eq!(data, "Hello"),
        _ => panic!("Expected text message"),
    }

    // Binary message
    connection.send_binary(vec![1, 2, 3]).await.unwrap();
    match rx.recv().await.unwrap() {
        Message::Binary { data } => assert_eq!(data, vec![1, 2, 3]),
        _ => panic!("Expected binary message"),
    }

    // JSON message
    #[derive(serde::Serialize)]
    struct TestData {
        value: i32,
    }
    connection.send_json(&TestData { value: 42 }).await.unwrap();
    match rx.recv().await.unwrap() {
        Message::Text { data } => assert!(data.contains("42")),
        _ => panic!("Expected text message"),
    }
}

// Test 9: WebSocket room broadcasting
#[tokio::test]
async fn test_websocket_room_broadcasting() {
    let room_manager = Arc::new(RoomManager::new());

    let (tx1, mut rx1) = mpsc::unbounded_channel();
    let (tx2, mut rx2) = mpsc::unbounded_channel();
    let (tx3, mut rx3) = mpsc::unbounded_channel();

    let conn1 = Arc::new(WebSocketConnection::new("user1".to_string(), tx1));
    let conn2 = Arc::new(WebSocketConnection::new("user2".to_string(), tx2));
    let conn3 = Arc::new(WebSocketConnection::new("user3".to_string(), tx3));

    // Join room
    room_manager.join_room("room1".to_string(), conn1).await;
    room_manager.join_room("room1".to_string(), conn2).await;
    room_manager.join_room("room1".to_string(), conn3).await;

    // Broadcast to room
    let msg = Message::text("Broadcast message".to_string());
    room_manager.broadcast_to_room("room1", msg).await.unwrap();

    // All should receive
    let received1 = rx1.recv().await.unwrap();
    let received2 = rx2.recv().await.unwrap();
    let received3 = rx3.recv().await.unwrap();

    for msg in [received1, received2, received3] {
        match msg {
            Message::Text { data } => assert_eq!(data, "Broadcast message"),
            _ => panic!("Expected text message"),
        }
    }
}

// Test 10: WebSocket handler with error handling
#[tokio::test]
async fn test_websocket_error_handling() {
    let handler = EchoHandler;
    let (tx, _rx) = mpsc::unbounded_channel();
    let connection = Arc::new(WebSocketConnection::new("user1".to_string(), tx));

    // Test error handler
    let result = handler
        .on_error(connection.clone(), "Test error".to_string())
        .await;
    assert!(result.is_ok());

    // Test connection still works after error
    let msg = Message::text("test".to_string());
    assert!(handler.on_message(connection, msg).await.is_ok());
}

// Test 11: WebSocket close handling
#[tokio::test]
async fn test_websocket_close_handling() {
    let (tx, _rx) = mpsc::unbounded_channel();
    let connection = Arc::new(WebSocketConnection::new("user1".to_string(), tx));

    assert!(!connection.is_closed().await);

    // Close may fail if connection already closed internally, but that's ok
    let _ = connection.close().await;

    assert!(connection.is_closed().await);

    // Sending after close should fail
    let result = connection.send_text("test".to_string()).await;
    assert!(result.is_err());
}

// Test 12: WebSocket concurrent connections
#[tokio::test]
async fn test_websocket_concurrent_connections() {
    let room_manager = Arc::new(RoomManager::new());

    // Create 5 concurrent connections
    let mut handles = vec![];
    for i in 0..5 {
        let rm = room_manager.clone();
        let handle = tokio::spawn(async move {
            let (tx, _rx) = mpsc::unbounded_channel();
            let conn = Arc::new(WebSocketConnection::new(format!("user{}", i), tx));
            rm.join_room("concurrent".to_string(), conn).await;
        });
        handles.push(handle);
    }

    // Wait for all to join
    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(room_manager.get_room_size("concurrent").await, 5);
}
