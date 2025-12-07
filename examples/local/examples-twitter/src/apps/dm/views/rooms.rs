//! DM Room views

use crate::apps::auth::models::User;
use crate::apps::dm::models::DMRoom;
use crate::apps::dm::serializers::{CreateRoomRequest, RoomResponse};
use reinhardt::db::associations::ManyToManyField;
use reinhardt::db::orm::{ManyToManyAccessor, Manager, Model};
use reinhardt::db::DatabaseConnection;
use reinhardt::{delete, get, post, CurrentUser, Json, Path, Response, StatusCode, ViewResult};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

/// List all rooms for the current user
#[get("/rooms", name = "dm_rooms_list", use_inject = true)]
pub async fn list_rooms(
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get current user ID
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	// Get all rooms and filter by membership
	let all_rooms = DMRoom::objects()
		.all()
		.all_with_db(&db)
		.await?;

	// Filter rooms where user is a member
	let mut user_rooms = Vec::new();
	for room in all_rooms {
		let accessor = ManyToManyAccessor::<DMRoom, User>::new(&room, "members", (*db).clone());
		let members = accessor.all().await.map_err(|e| e.to_string())?;
		if members.iter().any(|m| m.id == user_id) {
			user_rooms.push(room);
		}
	}

	let response: Vec<RoomResponse> = user_rooms.into_iter().map(RoomResponse::from).collect();

	Response::ok().with_json(&response).map_err(Into::into)
}

/// Get a specific room by ID
#[get("/rooms/{<uuid:room_id>}", name = "dm_rooms_get", use_inject = true)]
pub async fn get_room(
	Path(room_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get current user ID
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	let room = DMRoom::objects()
		.filter_by(DMRoom::field_id().eq(room_id))
		.get_with_db(&db)
		.await?;

	// Verify user is a member
	let accessor = ManyToManyAccessor::<DMRoom, User>::new(&room, "members", (*db).clone());
	let members = accessor.all().await.map_err(|e| e.to_string())?;
	if !members.iter().any(|m| m.id == user_id) {
		return Err("Not a member of this room".into());
	}

	let response = RoomResponse::from(room);
	Response::ok().with_json(&response).map_err(Into::into)
}

/// Create a new room (1-on-1 or group)
#[post("/rooms", name = "dm_rooms_create", use_inject = true)]
pub async fn create_room(
	Json(request): Json<CreateRoomRequest>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	request.validate()?;

	// Get creator ID from current user
	let creator_id = current_user.id().map_err(|e| e.to_string())?;

	// Create the room using struct initialization
	let room = DMRoom {
		id: Uuid::new_v4(),
		name: request.name,
		is_group: request.member_ids.len() > 1, // is_group if more than 1 additional member
		members: ManyToManyField::new(),
		created_at: chrono::Utc::now(),
		updated_at: chrono::Utc::now(),
	};

	// Save the room using Manager
	let manager = Manager::<DMRoom>::new();
	let created = manager.create_with_conn(&db, &room).await?;

	// Add members using ManyToManyAccessor
	let accessor = ManyToManyAccessor::<DMRoom, User>::new(&created, "members", (*db).clone());

	// Add the creator as a member
	let creator = User::objects()
		.filter_by(User::field_id().eq(creator_id))
		.get_with_db(&db)
		.await?;
	accessor.add(&creator).await.map_err(|e| e.to_string())?;

	// Add other members from request
	for member_id in &request.member_ids {
		let user = User::objects()
			.filter_by(User::field_id().eq(*member_id))
			.get_with_db(&db)
			.await?;
		accessor.add(&user).await.map_err(|e| e.to_string())?;
	}

	let response = RoomResponse::from(created);
	Response::new(StatusCode::CREATED)
		.with_json(&response)
		.map_err(Into::into)
}

/// Delete a room (only if user is a member)
#[delete("/rooms/{<uuid:room_id>}", name = "dm_rooms_delete", use_inject = true)]
pub async fn delete_room(
	Path(room_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get current user ID
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	let room = DMRoom::objects()
		.filter_by(DMRoom::field_id().eq(room_id))
		.get_with_db(&db)
		.await?;

	// Verify user is a member
	let accessor = ManyToManyAccessor::<DMRoom, User>::new(&room, "members", (*db).clone());
	let members = accessor.all().await.map_err(|e| e.to_string())?;
	if !members.iter().any(|m| m.id == user_id) {
		return Err("Not a member of this room".into());
	}

	// Delete using Manager
	let manager = Manager::<DMRoom>::new();
	manager.delete_with_conn(&db, room_id).await?;

	Ok(Response::new(StatusCode::NO_CONTENT))
}
