//! Block/Unblock view handlers
//!
//! Handles block and unblock operations

use crate::apps::auth::models::User;
use crate::apps::relationship::serializers::BlockResponse;
use reinhardt::db::orm::{ManyToManyAccessor, Model};
use reinhardt::db::DatabaseConnection;
use reinhardt::{delete, post, CurrentUser};
use reinhardt::{Path, Response, ViewResult};
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
#[post("/accounts/rel/block/{<uuid:user_id>}/", name = "block", use_inject = true)]
pub async fn block_user(
	Path(blocked_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get blocker ID from current user
	let blocker_id = current_user.id().map_err(|e| e.to_string())?;

	// Load blocker user
	let blocker = User::objects()
		.filter_by(User::field_id().eq(blocker_id))
		.get_with_db(&db)
		.await?;

	// Check if target user exists
	let target_user = User::objects()
		.filter_by(User::field_id().eq(blocked_id))
		.get_with_db(&db)
		.await?;

	// Add block relationship using ManyToManyAccessor
	let accessor = ManyToManyAccessor::<User, User>::new(&blocker, "blocked_users", (*db).clone());
	accessor.add(&target_user).await.map_err(|e| e.to_string())?;

	let response_data = BlockResponse::new(blocker.id, target_user.id);
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
#[delete("/accounts/rel/block/{<uuid:user_id>}/", name = "unblock", use_inject = true)]
pub async fn unblock_user(
	Path(blocked_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get blocker ID from current user
	let blocker_id = current_user.id().map_err(|e| e.to_string())?;

	// Load blocker user
	let blocker = User::objects()
		.filter_by(User::field_id().eq(blocker_id))
		.get_with_db(&db)
		.await?;

	// Check if target user exists
	let target_user = User::objects()
		.filter_by(User::field_id().eq(blocked_id))
		.get_with_db(&db)
		.await?;

	// Remove block relationship using ManyToManyAccessor
	let accessor = ManyToManyAccessor::<User, User>::new(&blocker, "blocked_users", (*db).clone());
	accessor.remove(&target_user).await.map_err(|e| e.to_string())?;

	Response::ok()
		.with_json(&serde_json::json!({"message": "Successfully unblocked"}))
		.map_err(Into::into)
}
