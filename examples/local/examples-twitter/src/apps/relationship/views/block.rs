//! Block/Unblock view handlers
//!
//! Handles block and unblock operations

use crate::apps::auth::models::User;
use crate::apps::relationship::serializers::BlockResponse;
use reinhardt::db::orm::Model;
use reinhardt::db::DatabaseConnection;
use reinhardt::{delete, post};
use reinhardt::{Error, Path, Response, StatusCode, ViewResult};
use std::sync::Arc;
use uuid::Uuid;

/// Block a user
///
/// POST /accounts/rel/block/<uuid:user_id>/
/// Success response: 200 OK with block relationship data
/// Error responses:
/// - 401 Unauthorized: Not authenticated
/// - 404 Not Found: User not found
/// - 409 Conflict: Already blocking this user
#[post("/accounts/rel/block/{<uuid:user_id>}/", name = "relationship_block", use_inject = true)]
pub async fn block_user(
	Path(blocked_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: Arc<User>,
) -> ViewResult<Response> {
	// Check if target user exists
	let target_user = User::objects()
		.filter(User::field_id().eq(blocked_id))
		.get(&db)
		.await?
		.ok_or_else(|| Error::NotFound("User not found".into()))?;

	// Check if already blocking
	let is_blocking = current_user.blocked_users.contains(&db, target_user.id).await?;
	if is_blocking {
		return Err(Error::Conflict("Already blocking this user".into()));
	}

	// Add to blocked_users
	current_user.blocked_users.add(&db, target_user.id).await?;

	// Also unfollow if following
	let is_following = current_user.following.contains(&db, target_user.id).await?;
	if is_following {
		current_user.following.remove(&db, target_user.id).await?;
	}

	let response_data = BlockResponse::new(current_user.id, target_user.id);
	Response::ok().with_json(&response_data).map_err(Into::into)
}

/// Unblock a user
///
/// DELETE /accounts/rel/block/<uuid:user_id>/
/// Success response: 200 OK
/// Error responses:
/// - 401 Unauthorized: Not authenticated
/// - 404 Not Found: User not found or not blocking
/// - 409 Conflict: Not blocking this user
#[delete("/accounts/rel/block/{<uuid:user_id>}/", name = "relationship_unblock", use_inject = true)]
pub async fn unblock_user(
	Path(blocked_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: Arc<User>,
) -> ViewResult<Response> {
	// Check if target user exists
	let target_user = User::objects()
		.filter(User::field_id().eq(blocked_id))
		.get(&db)
		.await?
		.ok_or_else(|| Error::NotFound("User not found".into()))?;

	// Check if blocking
	let is_blocking = current_user.blocked_users.contains(&db, target_user.id).await?;
	if !is_blocking {
		return Err(Error::Conflict("Not blocking this user".into()));
	}

	// Remove from blocked_users
	current_user.blocked_users.remove(&db, target_user.id).await?;

	Response::ok()
		.with_json(&serde_json::json!({"message": "Successfully unblocked"}))
		.map_err(Into::into)
}
