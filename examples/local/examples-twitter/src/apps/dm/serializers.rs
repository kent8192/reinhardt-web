//! Serializers module for dm app (RESTful)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::apps::dm::models::{DMMessage, DMRoom};

/// Request to create a new DM room
#[derive(Debug, Deserialize, Validate)]
pub struct CreateRoomRequest {
	/// Optional name for the room (for group chats)
	#[validate(length(max = 255))]
	pub name: Option<String>,
	/// List of user IDs to add to the room
	#[validate(length(min = 1, message = "At least one member is required"))]
	pub member_ids: Vec<Uuid>,
}

/// Response for a DM room
#[derive(Debug, Serialize)]
pub struct RoomResponse {
	pub id: Uuid,
	pub name: Option<String>,
	pub is_group: bool,
	pub created_at: DateTime<Utc>,
}

impl From<DMRoom> for RoomResponse {
	fn from(room: DMRoom) -> Self {
		Self {
			id: room.id,
			name: room.name,
			is_group: room.is_group,
			created_at: room.created_at,
		}
	}
}

/// Request to create a new message
#[derive(Debug, Deserialize, Validate)]
pub struct CreateMessageRequest {
	/// Message content
	#[validate(length(
		min = 1,
		max = 1000,
		message = "Content must be between 1 and 1000 characters"
	))]
	pub content: String,
}

/// Response for a DM message
#[derive(Debug, Serialize)]
pub struct MessageResponse {
	pub id: Uuid,
	pub room_id: Uuid,
	pub sender_id: Uuid,
	pub content: String,
	pub is_read: bool,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl From<DMMessage> for MessageResponse {
	fn from(message: DMMessage) -> Self {
		Self {
			id: message.id,
			// Use auto-generated _id fields from ForeignKeyField
			room_id: message.room_id,
			sender_id: message.sender_id,
			content: message.content,
			is_read: message.is_read,
			created_at: message.created_at,
			updated_at: message.updated_at,
		}
	}
}
