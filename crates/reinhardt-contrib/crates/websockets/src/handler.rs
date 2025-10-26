//! WebSocket handler and room management

use crate::connection::{Message, WebSocketConnection, WebSocketResult};
use crate::room::{Room, RoomError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Manages WebSocket rooms and client connections with backward compatibility
///
/// # Deprecation Notice
///
/// This type is deprecated and will be removed in a future version.
/// Please use [`Room`] directly instead.
///
/// ## Migration Guide
///
/// ### Before (using RoomManager):
/// ```ignore
/// let manager = RoomManager::new();
/// manager.join_room("chat".to_string(), connection).await;
/// manager.broadcast_to_room("chat", message).await?;
/// ```
///
/// ### After (using Room directly):
/// ```ignore
/// let room = Room::new("chat".to_string());
/// room.join("user1".to_string(), connection).await?;
/// room.broadcast(message).await?;
/// ```
///
/// For managing multiple rooms, maintain a `HashMap<String, Arc<Room>>` in your application.
#[deprecated(
    since = "0.2.0",
    note = "Use Room directly instead. See migration guide in documentation."
)]
pub struct RoomManager {
    rooms: Arc<RwLock<HashMap<String, Arc<Room>>>>,
}

impl RoomManager {
    /// Create a new RoomManager
    ///
    /// # Deprecated
    /// This method is deprecated. Use [`Room::new`] instead.
    #[deprecated(since = "0.2.0", note = "Use Room::new instead")]
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a connection to a room
    ///
    /// # Deprecated
    /// This method is deprecated. Use [`Room::join`] instead.
    #[deprecated(since = "0.2.0", note = "Use Room::join instead")]
    pub async fn join_room(&self, room_name: String, connection: Arc<WebSocketConnection>) {
        let room = self.get_or_create_room(&room_name).await;
        let client_id = connection.id().to_string();
        let _ = room.join(client_id, connection).await;
    }

    /// Remove a connection from a room
    ///
    /// # Deprecated
    /// This method is deprecated. Use [`Room::leave`] instead.
    #[deprecated(since = "0.2.0", note = "Use Room::leave instead")]
    pub async fn leave_room(&self, room_name: &str, client_id: &str) {
        if let Some(room) = self.get_room(room_name).await {
            let _ = room.leave(client_id).await;
            if room.is_empty().await {
                let _ = self.delete_room(room_name).await;
            }
        }
    }

    /// Broadcast a message to all connections in a room
    ///
    /// # Deprecated
    /// This method is deprecated. Use [`Room::broadcast`] instead.
    #[deprecated(since = "0.2.0", note = "Use Room::broadcast instead")]
    pub async fn broadcast_to_room(
        &self,
        room_name: &str,
        message: Message,
    ) -> WebSocketResult<()> {
        if let Some(room) = self.get_room(room_name).await {
            room.broadcast(message).await.map_err(|e| match e {
                RoomError::WebSocket(ws_err) => ws_err,
                _ => crate::connection::WebSocketError::Send(e.to_string()),
            })?;
        }
        Ok(())
    }

    /// Broadcast a message to all rooms
    ///
    /// # Deprecated
    /// This method is deprecated. Iterate over your rooms manually instead.
    #[deprecated(since = "0.2.0", note = "Iterate over your rooms manually instead")]
    pub async fn broadcast_to_all(&self, message: Message) -> WebSocketResult<()> {
        let rooms = self.rooms.read().await;
        for room in rooms.values() {
            room.broadcast(message.clone()).await.map_err(|e| match e {
                RoomError::WebSocket(ws_err) => ws_err,
                _ => crate::connection::WebSocketError::Send(e.to_string()),
            })?;
        }
        Ok(())
    }

    /// Get the number of connections in a room
    ///
    /// # Deprecated
    /// This method is deprecated. Use [`Room::client_count`] instead.
    #[deprecated(since = "0.2.0", note = "Use Room::client_count instead")]
    pub async fn get_room_size(&self, room_name: &str) -> usize {
        if let Some(room) = self.get_room(room_name).await {
            room.client_count().await
        } else {
            0
        }
    }

    /// Get all room names
    ///
    /// # Deprecated
    /// This method is deprecated. Manage rooms in your application instead.
    #[deprecated(since = "0.2.0", note = "Manage rooms in your application instead")]
    pub async fn get_all_rooms(&self) -> Vec<String> {
        let rooms = self.rooms.read().await;
        rooms.keys().cloned().collect()
    }

    async fn get_room(&self, id: &str) -> Option<Arc<Room>> {
        let rooms = self.rooms.read().await;
        rooms.get(id).cloned()
    }

    async fn get_or_create_room(&self, id: &str) -> Arc<Room> {
        if let Some(room) = self.get_room(id).await {
            return room;
        }

        let mut rooms = self.rooms.write().await;
        let room = Arc::new(Room::new(id.to_string()));
        rooms.insert(id.to_string(), room.clone());
        room
    }

    async fn delete_room(&self, id: &str) -> Result<(), RoomError> {
        let mut rooms = self.rooms.write().await;
        rooms
            .remove(id)
            .ok_or_else(|| RoomError::RoomNotFound(id.to_string()))?;
        Ok(())
    }
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket handler trait
pub trait WebSocketHandler: Send + Sync {
    /// Handle incoming message
    fn on_message(
        &self,
        message: Message,
    ) -> impl std::future::Future<Output = WebSocketResult<()>> + Send;

    /// Handle connection open
    fn on_connect(&self) -> impl std::future::Future<Output = WebSocketResult<()>> + Send;

    /// Handle connection close
    fn on_disconnect(&self) -> impl std::future::Future<Output = WebSocketResult<()>> + Send;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_room_manager_basic() {
        let manager = RoomManager::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let conn = Arc::new(WebSocketConnection::new("test1".to_string(), tx));

        manager.join_room("chat".to_string(), conn.clone()).await;
        assert_eq!(manager.get_room_size("chat").await, 1);

        manager.leave_room("chat", "test1").await;
        assert_eq!(manager.get_room_size("chat").await, 0);
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_room_manager_broadcast() {
        let manager = RoomManager::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();

        let conn1 = Arc::new(WebSocketConnection::new("user1".to_string(), tx1));
        let conn2 = Arc::new(WebSocketConnection::new("user2".to_string(), tx2));

        manager.join_room("chat".to_string(), conn1).await;
        manager.join_room("chat".to_string(), conn2).await;

        let msg = Message::text("Hello everyone".to_string());
        manager.broadcast_to_room("chat", msg).await.unwrap();

        assert!(matches!(rx1.try_recv(), Ok(Message::Text { .. })));
        assert!(matches!(rx2.try_recv(), Ok(Message::Text { .. })));
    }
}
