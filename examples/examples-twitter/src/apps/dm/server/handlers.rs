//! DM WebSocket handler implementation
//!
//! This module implements the WebSocket handler for direct messaging (DM) functionality.
//! It handles real-time message delivery, room management, and message persistence.
use crate::apps::dm::models::DMMessage;
use crate::apps::dm::shared::types::MessageInfo;
use reinhardt::DatabaseConnection;
use reinhardt::core::async_trait;
use reinhardt::db::orm::Model;
use reinhardt::{
	ConsumerContext, InMemorySessionBackend, Message, PagesAuthenticator, RoomManager,
	WebSocketConsumer, WebSocketError, WebSocketResult,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
/// Payload for DM messages sent over WebSocket
#[derive(Debug, Serialize, Deserialize)]
pub struct DMMessagePayload {
	/// Type of message (usually "message")
	#[serde(rename = "type")]
	pub message_type: Option<String>,
	/// ID of the DM room
	pub room_id: String,
	/// Message content
	pub content: String,
}
/// DM WebSocket handler
///
/// Handles WebSocket connections for direct messaging:
/// - Authenticates users via Cookie/session
/// - Manages room membership
/// - Persists messages to database
/// - Broadcasts messages to room members
pub struct DMHandler {
	/// Room manager for tracking active WebSocket connections
	room_manager: Arc<RwLock<RoomManager>>,
	/// Authenticator for Cookie/session-based authentication
	authenticator: Arc<PagesAuthenticator<InMemorySessionBackend>>,
	/// Database connection for message persistence
	db: DatabaseConnection,
}
impl DMHandler {
	/// Create a new DMHandler with database connection
	pub fn new(db: DatabaseConnection) -> Self {
		let session_backend = InMemorySessionBackend::new();
		Self {
			room_manager: Arc::new(RwLock::new(RoomManager::new())),
			authenticator: Arc::new(PagesAuthenticator::new(session_backend)),
			db,
		}
	}
	/// Extract user ID from ConsumerContext metadata
	///
	/// The user ID should be stored in metadata during authentication in `on_connect`
	fn get_user_id(&self, context: &ConsumerContext) -> WebSocketResult<String> {
		context
			.metadata
			.get("user_id")
			.cloned()
			.ok_or_else(|| WebSocketError::Internal("User not authenticated".to_string()))
	}
	/// Extract username from ConsumerContext metadata
	fn get_username(&self, context: &ConsumerContext) -> WebSocketResult<String> {
		context
			.metadata
			.get("username")
			.cloned()
			.ok_or_else(|| WebSocketError::Internal("Username not found".to_string()))
	}
	/// Extract room ID from WebSocket path
	///
	/// Expected path format: `/ws/dm?room_id={room_id}`
	/// The room_id is stored in ConsumerContext metadata during connection.
	fn extract_room_id(&self, context: &ConsumerContext) -> WebSocketResult<String> {
		context
			.metadata
			.get("room_id")
			.cloned()
			.ok_or_else(|| WebSocketError::Internal("room_id not found in metadata".to_string()))
	}
	/// Join a DM room
	///
	/// Adds the current WebSocket connection to the specified room
	async fn join_room(&self, room_id: &str, context: &mut ConsumerContext) -> WebSocketResult<()> {
		let manager = self.room_manager.write().await;
		let room = manager.create_room(room_id.to_string()).await;
		let user_id = context
			.metadata
			.get("user_id")
			.cloned()
			.unwrap_or_else(|| "unknown".to_string());
		room.join(user_id, context.connection.clone())
			.await
			.map_err(|e| WebSocketError::Internal(e.to_string()))?;
		Ok(())
	}
	/// Broadcast message to all members in a room
	async fn broadcast_to_room(&self, room_id: &str, message: Message) -> WebSocketResult<()> {
		let manager = self.room_manager.read().await;
		if let Some(room) = manager.get_room(room_id).await {
			let result = room.broadcast(message).await;
			if result.failure_count() > 0 {
				return Err(WebSocketError::Internal(format!(
					"broadcast failed for {} client(s)",
					result.failure_count()
				)));
			}
		}
		Ok(())
	}
	/// Persist message to database and return the created message info
	async fn persist_message(
		&self,
		room_id: &str,
		sender_id: &str,
		sender_username: &str,
		content: &str,
	) -> WebSocketResult<MessageInfo> {
		let room_uuid = Uuid::parse_str(room_id)
			.map_err(|e| WebSocketError::Internal(format!("Invalid room_id: {}", e)))?;
		let sender_uuid = Uuid::parse_str(sender_id)
			.map_err(|e| WebSocketError::Internal(format!("Invalid sender_id: {}", e)))?;
		let message = DMMessage::new(content.to_string(), room_uuid, sender_uuid);
		DMMessage::objects()
			.create_with_conn(&self.db, &message)
			.await
			.map_err(|e| WebSocketError::Internal(format!("Database error: {}", e)))?;
		Ok(MessageInfo {
			id: message.id(),
			room_id: room_uuid,
			sender_id: sender_uuid,
			sender_username: sender_username.to_string(),
			content: content.to_string(),
			is_read: false,
			created_at: message.created_at().to_rfc3339(),
		})
	}
}
#[async_trait]
impl WebSocketConsumer for DMHandler {
	async fn on_connect(&self, context: &mut ConsumerContext) -> WebSocketResult<()> {
		let cookies = "";
		let user = self
			.authenticator
			.authenticate_from_cookies(cookies)
			.await
			.map_err(|e| WebSocketError::Internal(e.to_string()))?;
		context
			.metadata
			.insert("user_id".to_string(), user.id().to_string());
		context
			.metadata
			.insert("username".to_string(), user.username().to_string());
		let room_id = self.extract_room_id(context)?;
		self.join_room(&room_id, context).await?;
		Ok(())
	}
	async fn on_message(
		&self,
		context: &mut ConsumerContext,
		message: Message,
	) -> WebSocketResult<()> {
		match message {
			Message::Text { data } => {
				let payload: DMMessagePayload = serde_json::from_str(&data)
					.map_err(|e| WebSocketError::Internal(e.to_string()))?;
				let user_id = self.get_user_id(context)?;
				let username = self.get_username(context)?;
				let message_info = self
					.persist_message(&payload.room_id, &user_id, &username, &payload.content)
					.await?;
				let broadcast_data = serde_json::to_string(&message_info)
					.map_err(|e| WebSocketError::Internal(e.to_string()))?;
				self.broadcast_to_room(&payload.room_id, Message::text(broadcast_data))
					.await?;
			}
			Message::Binary { .. } => {}
			Message::Close { .. } => {}
			Message::Ping | Message::Pong => {}
		}
		Ok(())
	}
	async fn on_disconnect(&self, context: &mut ConsumerContext) -> WebSocketResult<()> {
		if let Some(user_id) = context.metadata.get("user_id") {
			if let Ok(room_id) = self.extract_room_id(context) {
				let manager = self.room_manager.read().await;
				if let Some(room) = manager.get_room(&room_id).await {
					room.leave(user_id)
						.await
						.map_err(|e| WebSocketError::Internal(e.to_string()))?;
				}
			}
		}
		Ok(())
	}
}
