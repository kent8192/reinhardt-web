//! DM WebSocket handler implementation
//!
//! This module implements the WebSocket handler for direct messaging (DM) functionality.
//! It handles real-time message delivery, room management, and message persistence.

use async_trait::async_trait;
use reinhardt_websockets::integration::pages::PagesAuthenticator;
use reinhardt_websockets::room::RoomManager;
use reinhardt_websockets::{
	ConsumerContext, Message, WebSocketConsumer, WebSocketError, WebSocketResult,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[allow(unused_imports)]
use crate::apps::dm::models::{DMMessage, DMRoom};

/// Payload for DM messages sent over WebSocket
#[derive(Debug, Serialize, Deserialize)]
pub struct DMMessagePayload {
	/// ID of the DM room
	pub room_id: String,
	/// Message content
	pub content: String,
}

/// Simple in-memory message structure for Phase 1
/// TODO: Replace with DMMessage model when database integration is ready
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SimpleMessage {
	room_id: String,
	sender_id: String,
	content: String,
	timestamp: chrono::DateTime<chrono::Utc>,
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
	authenticator: Arc<PagesAuthenticator>,
	/// In-memory message storage (Phase 1)
	/// TODO: Replace with database connection for persistent storage
	messages: Arc<RwLock<Vec<SimpleMessage>>>,
}

impl DMHandler {
	/// Create a new DMHandler
	pub fn new() -> Self {
		Self {
			room_manager: Arc::new(RwLock::new(RoomManager::new())),
			authenticator: Arc::new(PagesAuthenticator::new()),
			messages: Arc::new(RwLock::new(Vec::new())),
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

		// Get user ID from metadata
		let user_id = context
			.metadata
			.get("user_id")
			.cloned()
			.unwrap_or_else(|| "unknown".to_string());

		// Join room with user ID as client ID
		room.join(user_id, context.connection.clone())
			.await
			.map_err(|e| WebSocketError::Internal(e.to_string()))?;

		Ok(())
	}

	/// Broadcast message to all members in a room
	async fn broadcast_to_room(&self, room_id: &str, message: Message) -> WebSocketResult<()> {
		let manager = self.room_manager.read().await;
		if let Some(room) = manager.get_room(room_id).await {
			room.broadcast(message)
				.await
				.map_err(|e| WebSocketError::Internal(e.to_string()))?;
		}
		Ok(())
	}

	/// Persist message to database
	async fn persist_message(
		&self,
		room_id: &str,
		sender_id: &str,
		content: &str,
	) -> WebSocketResult<()> {
		// Phase 1: In-memory persistence
		// TODO: Replace with database persistence when DB integration is ready
		let message = SimpleMessage {
			room_id: room_id.to_string(),
			sender_id: sender_id.to_string(),
			content: content.to_string(),
			timestamp: chrono::Utc::now(),
		};

		let mut messages = self.messages.write().await;
		messages.push(message);
		Ok(())
	}
}

impl Default for DMHandler {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl WebSocketConsumer for DMHandler {
	async fn on_connect(&self, context: &mut ConsumerContext) -> WebSocketResult<()> {
		// Extract Cookie header from WebSocket handshake
		// TODO: Extract cookies from WebSocket connection headers
		// For now, this is a placeholder implementation
		let cookies = ""; // Placeholder

		// Authenticate user using PagesAuthenticator
		let user = self
			.authenticator
			.authenticate_from_cookies(cookies)
			.await
			.map_err(|e| WebSocketError::Internal(e.to_string()))?;

		// Store user ID in context metadata for later use
		context
			.metadata
			.insert("user_id".to_string(), user.id().to_string());

		// Extract room ID from connection path
		let room_id = self.extract_room_id(context)?;

		// Join the DM room
		self.join_room(&room_id, context).await?;

		// User connected (logging removed to avoid additional dependencies)
		// User {} connected to DM room {}

		Ok(())
	}

	async fn on_message(
		&self,
		context: &mut ConsumerContext,
		message: Message,
	) -> WebSocketResult<()> {
		match message {
			Message::Text { data } => {
				// Parse JSON message payload
				let payload: DMMessagePayload = serde_json::from_str(&data)
					.map_err(|e| WebSocketError::Internal(e.to_string()))?;

				// Get authenticated user ID from context
				let user_id = self.get_user_id(context)?;

				// Persist message to database
				self.persist_message(&payload.room_id, &user_id, &payload.content)
					.await?;

				// Broadcast message to all room members
				self.broadcast_to_room(&payload.room_id, Message::text(data))
					.await?;

				// Message sent (logging removed)
				// User {} sent message to room {}: {}
			}
			Message::Binary { .. } => {
				// Binary messages not supported for DM
				// Received binary message in DM handler, ignoring
			}
			Message::Close { .. } => {
				// Close messages are handled by on_disconnect
			}
			Message::Ping | Message::Pong => {
				// Ping/Pong messages are handled automatically by the WebSocket layer
			}
		}

		Ok(())
	}

	async fn on_disconnect(&self, context: &mut ConsumerContext) -> WebSocketResult<()> {
		// Get user ID if available
		if let Some(user_id) = context.metadata.get("user_id") {
			// Extract room ID from connection path
			if let Ok(room_id) = self.extract_room_id(context) {
				// Remove connection from room
				let manager = self.room_manager.read().await;
				if let Some(room) = manager.get_room(&room_id).await {
					room.leave(user_id)
						.await
						.map_err(|e| WebSocketError::Internal(e.to_string()))?;
				}

				// User disconnected (logging removed)
				// User {} disconnected from DM room {}
			}
		}

		Ok(())
	}
}
