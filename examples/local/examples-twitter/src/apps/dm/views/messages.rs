//! DM Message views

use crate::apps::auth::models::User;
use crate::apps::dm::models::{DMMessage, DMRoom};
use crate::apps::dm::serializers::{CreateMessageRequest, MessageResponse};
use reinhardt::db::orm::Model;
use reinhardt::db::DatabaseConnection;
use reinhardt::{delete, get, patch, post, Error, Json, Path, Response, StatusCode, ViewResult};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

/// List all messages in a room
#[get("/rooms/{<uuid:room_id>}/messages", name = "dm_messages_list", use_inject = true)]
pub async fn list_messages(
	Path(room_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: Arc<User>,
) -> ViewResult<Response> {
	// Verify room exists and user is a member
	let room = DMRoom::objects()
		.filter(DMRoom::field_id().eq(room_id))
		.get(&db)
		.await?
		.ok_or_else(|| Error::NotFound("Room not found".into()))?;

	let is_member = room.members.contains(&db, current_user.id).await?;
	if !is_member {
		return Err(Error::Forbidden("You are not a member of this room".into()));
	}

	// Get messages for this room
	let messages = DMMessage::objects()
		.filter(DMMessage::field_room_id().eq(room_id))
		.order_by("-created_at")
		.all(&db)
		.await?;

	let response: Vec<MessageResponse> = messages.into_iter().map(MessageResponse::from).collect();

	Response::ok().with_json(&response).map_err(Into::into)
}

/// Send a message to a room
#[post("/rooms/{<uuid:room_id>}/messages", name = "dm_messages_send", use_inject = true)]
pub async fn send_message(
	Path(room_id): Path<Uuid>,
	Json(request): Json<CreateMessageRequest>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: Arc<User>,
) -> ViewResult<Response> {
	request.validate()?;

	// Verify room exists and user is a member
	let room = DMRoom::objects()
		.filter(DMRoom::field_id().eq(room_id))
		.get(&db)
		.await?
		.ok_or_else(|| Error::NotFound("Room not found".into()))?;

	let is_member = room.members.contains(&db, current_user.id).await?;
	if !is_member {
		return Err(Error::Forbidden("You are not a member of this room".into()));
	}

	// Create the message
	let message = DMMessage::new(room_id, current_user.id, request.content);

	// Save the message
	message.save(&db).await?;

	let response = MessageResponse::from(message);
	Response::new(StatusCode::CREATED)
		.with_json(&response)
		.map_err(Into::into)
}

/// Get a specific message
#[get("/rooms/{<uuid:room_id>}/messages/{<uuid:message_id>}", name = "dm_messages_get", use_inject = true)]
pub async fn get_message(
	Path((room_id, message_id)): Path<(Uuid, Uuid)>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: Arc<User>,
) -> ViewResult<Response> {
	// Verify room exists and user is a member
	let room = DMRoom::objects()
		.filter(DMRoom::field_id().eq(room_id))
		.get(&db)
		.await?
		.ok_or_else(|| Error::NotFound("Room not found".into()))?;

	let is_member = room.members.contains(&db, current_user.id).await?;
	if !is_member {
		return Err(Error::Forbidden("You are not a member of this room".into()));
	}

	// Get the message
	let message = DMMessage::objects()
		.filter(DMMessage::field_id().eq(message_id))
		.filter(DMMessage::field_room_id().eq(room_id))
		.get(&db)
		.await?
		.ok_or_else(|| Error::NotFound("Message not found".into()))?;

	let response = MessageResponse::from(message);
	Response::ok().with_json(&response).map_err(Into::into)
}

/// Mark a message as read
#[patch("/rooms/{<uuid:room_id>}/messages/{<uuid:message_id>}/read", name = "dm_messages_mark_read", use_inject = true)]
pub async fn mark_as_read(
	Path((room_id, message_id)): Path<(Uuid, Uuid)>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: Arc<User>,
) -> ViewResult<Response> {
	// Verify room exists and user is a member
	let room = DMRoom::objects()
		.filter(DMRoom::field_id().eq(room_id))
		.get(&db)
		.await?
		.ok_or_else(|| Error::NotFound("Room not found".into()))?;

	let is_member = room.members.contains(&db, current_user.id).await?;
	if !is_member {
		return Err(Error::Forbidden("You are not a member of this room".into()));
	}

	// Get and update the message
	let mut message = DMMessage::objects()
		.filter(DMMessage::field_id().eq(message_id))
		.filter(DMMessage::field_room_id().eq(room_id))
		.get(&db)
		.await?
		.ok_or_else(|| Error::NotFound("Message not found".into()))?;

	message.is_read = true;
	message.save(&db).await?;

	let response = MessageResponse::from(message);
	Response::ok().with_json(&response).map_err(Into::into)
}

/// Delete a message (only the sender can delete)
#[delete("/rooms/{<uuid:room_id>}/messages/{<uuid:message_id>}", name = "dm_messages_delete", use_inject = true)]
pub async fn delete_message(
	Path((room_id, message_id)): Path<(Uuid, Uuid)>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: Arc<User>,
) -> ViewResult<Response> {
	// Get the message
	let message = DMMessage::objects()
		.filter(DMMessage::field_id().eq(message_id))
		.filter(DMMessage::field_room_id().eq(room_id))
		.get(&db)
		.await?
		.ok_or_else(|| Error::NotFound("Message not found".into()))?;

	// Only the sender can delete the message
	if message.sender_id != current_user.id {
		return Err(Error::Forbidden("Only the sender can delete this message".into()));
	}

	message.delete(&db).await?;

	Ok(Response::new(StatusCode::NO_CONTENT))
}
