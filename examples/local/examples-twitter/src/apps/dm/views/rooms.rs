//! DM Room views

use crate::apps::auth::models::User;
use crate::apps::dm::models::DMRoom;
use crate::apps::dm::serializers::{CreateRoomRequest, RoomResponse};
use reinhardt::db::orm::Model;
use reinhardt::db::DatabaseConnection;
use reinhardt::{delete, get, post, Error, Json, Path, Response, StatusCode, ViewResult};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

/// List all rooms for the current user
#[get("/rooms", name = "dm_rooms_list", use_inject = true)]
pub async fn list_rooms(
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: Arc<User>,
) -> ViewResult<Response> {
	// Get rooms where current user is a member
	let rooms = DMRoom::objects()
		.filter(DMRoom::field_members().contains(current_user.id))
		.all(&db)
		.await?;

	let response: Vec<RoomResponse> = rooms.into_iter().map(RoomResponse::from).collect();

	Response::ok().with_json(&response).map_err(Into::into)
}

/// Get a specific room by ID
#[get("/rooms/{<uuid:room_id>}", name = "dm_rooms_get", use_inject = true)]
pub async fn get_room(
	Path(room_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: Arc<User>,
) -> ViewResult<Response> {
	let room = DMRoom::objects()
		.filter(DMRoom::field_id().eq(room_id))
		.get(&db)
		.await?
		.ok_or_else(|| Error::NotFound("Room not found".into()))?;

	// Verify user is a member
	let is_member = room.members.contains(&db, current_user.id).await?;

	if !is_member {
		return Err(Error::Forbidden("You are not a member of this room".into()));
	}

	let response = RoomResponse::from(room);
	Response::ok().with_json(&response).map_err(Into::into)
}

/// Create a new room (1-on-1 or group)
#[post("/rooms", name = "dm_rooms_create", use_inject = true)]
pub async fn create_room(
	Json(request): Json<CreateRoomRequest>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: Arc<User>,
) -> ViewResult<Response> {
	request.validate()?;

	// Create the room
	let room = DMRoom::new(
		request.name,
		request.member_ids.len() > 1, // is_group if more than 1 additional member
	);

	// Save the room
	room.save(&db).await?;

	// Add current user as member
	room.members.add(&db, current_user.id).await?;

	// Add other members
	for member_id in request.member_ids {
		room.members.add(&db, member_id).await?;
	}

	let response = RoomResponse::from(room);
	Response::new(StatusCode::CREATED)
		.with_json(&response)
		.map_err(Into::into)
}

/// Delete a room (only if user is a member)
#[delete("/rooms/{<uuid:room_id>}", name = "dm_rooms_delete", use_inject = true)]
pub async fn delete_room(
	Path(room_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: Arc<User>,
) -> ViewResult<Response> {
	let room = DMRoom::objects()
		.filter(DMRoom::field_id().eq(room_id))
		.get(&db)
		.await?
		.ok_or_else(|| Error::NotFound("Room not found".into()))?;

	// Verify user is a member
	let is_member = room.members.contains(&db, current_user.id).await?;

	if !is_member {
		return Err(Error::Forbidden("You are not a member of this room".into()));
	}

	room.delete(&db).await?;

	Ok(Response::new(StatusCode::NO_CONTENT))
}
