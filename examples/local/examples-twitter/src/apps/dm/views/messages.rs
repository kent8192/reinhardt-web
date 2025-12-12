//! DM Message views

use crate::apps::auth::models::User;
use crate::apps::dm::models::{DMMessage, DMRoom};
use crate::apps::dm::serializers::{CreateMessageRequest, MessageResponse};
use reinhardt::db::orm::Model;
use reinhardt::db::DatabaseConnection;
use reinhardt::{get, post, CurrentUser, Json, Path, Response, StatusCode, ViewResult};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

/// List all messages in a room
#[get("/rooms/{<uuid:room_id>}/messages", name = "messages_list", use_inject = true)]
pub async fn list_messages(
	Path(room_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get current user ID
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	// Verify room exists
	let room = DMRoom::objects()
		.filter_by(DMRoom::field_id().eq(room_id))
		.get_with_db(&db)
		.await?;

	// Check membership using generated accessor method
	let accessor = room.members_accessor((*db).clone());
	let members = accessor.all().await.map_err(|e| e.to_string())?;
	let is_member = members.iter().any(|m| m.id == user_id);
	if !is_member {
		return Err("Not a member of this room".into());
	}

	// Get messages for this room using filter_by with the room's foreign key
	let messages = DMMessage::objects()
		.filter_by(DMMessage::field_room().eq(room_id))
		.all_with_db(&db)
		.await?;

	let response: Vec<MessageResponse> = messages.into_iter().map(MessageResponse::from).collect();

	Response::ok().with_json(&response).map_err(Into::into)
}

/// Send a message to a room
#[post("/rooms/{<uuid:room_id>}/messages", name = "messages_send", use_inject = true)]
pub async fn send_message(
	Path(room_id): Path<Uuid>,
	Json(request): Json<CreateMessageRequest>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	request.validate()?;

	// Get sender ID from current user
	let sender_id = current_user.id().map_err(|e| e.to_string())?;

	// Verify room exists
	let room = DMRoom::objects()
		.filter_by(DMRoom::field_id().eq(room_id))
		.get_with_db(&db)
		.await?;

	// Check membership using generated accessor method
	let accessor = room.members_accessor((*db).clone());
	let members = accessor.all().await.map_err(|e| e.to_string())?;
	let is_member = members.iter().any(|m| m.id == sender_id);
	if !is_member {
		return Err("Not a member of this room".into());
	}

	// Create the message using generated new() function
	// new() auto-generates id, timestamps, and ForeignKeyField instances
	let mut message = DMMessage::new(request.content);

	// Manually set FK IDs (not included in constructor)
	message.room_id = room_id;
	message.sender_id = sender_id;

	// Save the message using Manager
	let manager = DMMessage::objects();
	let created = manager.create_with_conn(&db, &message).await?;

	let response = MessageResponse::from(created);
	Response::new(StatusCode::CREATED)
		.with_json(&response)
		.map_err(Into::into)
}

/// Get a specific message
#[get("/rooms/{<uuid:room_id>}/messages/{<uuid:message_id>}", name = "messages_get", use_inject = true)]
pub async fn get_message(
	Path((room_id, message_id)): Path<(Uuid, Uuid)>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get current user ID
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	// Verify room exists
	let room = DMRoom::objects()
		.filter_by(DMRoom::field_id().eq(room_id))
		.get_with_db(&db)
		.await?;

	// Check membership using generated accessor method
	let accessor = room.members_accessor((*db).clone());
	let members = accessor.all().await.map_err(|e| e.to_string())?;
	let is_member = members.iter().any(|m| m.id == user_id);
	if !is_member {
		return Err("Not a member of this room".into());
	}

	// Get the message
	let message = DMMessage::objects()
		.filter_by(DMMessage::field_id().eq(message_id))
		.get_with_db(&db)
		.await?;

	let response = MessageResponse::from(message);
	Response::ok().with_json(&response).map_err(Into::into)
}
