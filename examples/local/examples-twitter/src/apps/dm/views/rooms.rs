//! DM Room views

use crate::apps::auth::models::User;
use crate::apps::dm::models::DMRoom;
use crate::apps::dm::serializers::{CreateRoomRequest, RoomResponse};
use reinhardt::db::orm::{FilterOperator, FilterValue, ManyToManyAccessor, Model};
use reinhardt::db::DatabaseConnection;
use reinhardt::{delete, get, post, CurrentUser, Json, Path, Response, StatusCode, ViewResult};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

/// List all rooms for the current user
#[get("/rooms", name = "rooms_list", use_inject = true)]
pub async fn list_rooms(
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get current user ID
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	// TODO: Implement JOIN-based query to avoid N+1 problem
	// Current implementation loads all rooms then filters in memory
	// For production, use direct SQL JOIN on dm_dmroom_members table
	let all_rooms = DMRoom::objects().all().all().await?;

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
#[get("/rooms/{<uuid:room_id>}", name = "rooms_get", use_inject = true)]
pub async fn get_room(
	Path(room_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get current user ID
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	// Get room using Manager API
	let room = DMRoom::objects()
		.filter(
			DMRoom::field_id(),
			FilterOperator::Eq,
			FilterValue::String(room_id.to_string()),
		)
		.first()
		.await?
		.ok_or_else(|| "Room not found".to_string())?;

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
#[post("/rooms", name = "rooms_create", use_inject = true)]
pub async fn create_room(
	Json(request): Json<CreateRoomRequest>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	request.validate()?;

	// Get creator ID from current user
	let creator_id = current_user.id().map_err(|e| e.to_string())?;

	// Create the room using generated new() function
	// new() auto-generates id, timestamps, and ManyToManyField instance
	let is_group = request.member_ids.len() > 1; // is_group if more than 1 additional member
	let room = DMRoom::new(request.name, is_group);

	// Save the room using Manager
	let created = DMRoom::objects().create_with_conn(&db, &room).await?;

	// Add members using ManyToManyAccessor
	let accessor = ManyToManyAccessor::<DMRoom, User>::new(&created, "members", (*db).clone());

	// Add the creator as a member
	let creator = User::objects()
		.filter(
			User::field_id(),
			FilterOperator::Eq,
			FilterValue::String(creator_id.to_string()),
		)
		.first()
		.await?
		.ok_or_else(|| "Creator not found".to_string())?;
	accessor.add(&creator).await.map_err(|e| e.to_string())?;

	// Add other members from request (bulk fetch to avoid N+1 queries)
	if !request.member_ids.is_empty() {
		// Bulk fetch all members in a single query using is_in
		let member_ids_vec: Vec<Uuid> = request.member_ids.clone();
		let members_query = User::objects().filter(
			User::field_id(),
			FilterOperator::In,
			FilterValue::Array(
				member_ids_vec
					.iter()
					.map(|id| id.to_string())
					.collect(),
			),
		);

		let members = members_query.all().await.map_err(|e| e.to_string())?;

		// Verify all requested members exist
		if members.len() != member_ids_vec.len() {
			return Err("Some member IDs are invalid".into());
		}

		// Add all members
		for user in members {
			accessor.add(&user).await.map_err(|e| e.to_string())?;
		}
	}

	let response = RoomResponse::from(created);
	Response::new(StatusCode::CREATED)
		.with_json(&response)
		.map_err(Into::into)
}

/// Delete a room (only if user is a member)
#[delete("/rooms/{<uuid:room_id>}", name = "rooms_delete", use_inject = true)]
pub async fn delete_room(
	Path(room_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get current user ID
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	// Get room using Manager API
	let room = DMRoom::objects()
		.filter(
			DMRoom::field_id(),
			FilterOperator::Eq,
			FilterValue::String(room_id.to_string()),
		)
		.first()
		.await?
		.ok_or_else(|| "Room not found".to_string())?;

	// Verify user is a member
	let accessor = ManyToManyAccessor::<DMRoom, User>::new(&room, "members", (*db).clone());
	let members = accessor.all().await.map_err(|e| e.to_string())?;
	if !members.iter().any(|m| m.id == user_id) {
		return Err("Not a member of this room".into());
	}

	// Delete using Manager
	DMRoom::objects().delete_with_conn(&db, room_id).await?;

	Ok(Response::new(StatusCode::NO_CONTENT))
}
