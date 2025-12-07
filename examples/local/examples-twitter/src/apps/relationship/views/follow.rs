//! Follow/Unfollow view handlers
//!
//! Handles follow and unfollow operations

use crate::apps::auth::models::User;
use crate::apps::relationship::serializers::FollowResponse;
use reinhardt::db::orm::{ManyToManyAccessor, Model};
use reinhardt::db::DatabaseConnection;
use reinhardt::{delete, post, CurrentUser};
use reinhardt::{Path, Response, ViewResult};
use std::sync::Arc;
use uuid::Uuid;

/// Follow a user
///
/// POST /accounts/rel/follow/<uuid:user_id>/
/// Success response: 200 OK with follow relationship data
/// Error responses:
/// - 401 Unauthorized: Not authenticated
/// - 404 Not Found: User not found
/// - 409 Conflict: Already following this user
#[post("/accounts/rel/follow/{<uuid:user_id>}/", name = "relationship_follow", use_inject = true)]
pub async fn follow_user(
	Path(followed_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get follower ID from current user
	let follower_id = current_user.id().map_err(|e| e.to_string())?;

	// Load follower user
	let follower = User::objects()
		.filter_by(User::field_id().eq(follower_id))
		.get_with_db(&db)
		.await?;

	// Check if target user exists
	let target_user = User::objects()
		.filter_by(User::field_id().eq(followed_id))
		.get_with_db(&db)
		.await?;

	// Add follow relationship using ManyToManyAccessor
	let accessor = ManyToManyAccessor::<User, User>::new(&follower, "following", (*db).clone());
	accessor.add(&target_user).await.map_err(|e| e.to_string())?;

	let response_data = FollowResponse::new(follower.id, target_user.id);
	Response::ok().with_json(&response_data).map_err(Into::into)
}

/// Unfollow a user
///
/// DELETE /accounts/rel/follow/<uuid:user_id>/
/// Success response: 200 OK
/// Error responses:
/// - 401 Unauthorized: Not authenticated
/// - 404 Not Found: User not found or not following
/// - 409 Conflict: Not following this user
#[delete("/accounts/rel/follow/{<uuid:user_id>}/", name = "relationship_unfollow", use_inject = true)]
pub async fn unfollow_user(
	Path(followed_id): Path<Uuid>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get follower ID from current user
	let follower_id = current_user.id().map_err(|e| e.to_string())?;

	// Load follower user
	let follower = User::objects()
		.filter_by(User::field_id().eq(follower_id))
		.get_with_db(&db)
		.await?;

	// Check if target user exists
	let target_user = User::objects()
		.filter_by(User::field_id().eq(followed_id))
		.get_with_db(&db)
		.await?;

	// Remove follow relationship using ManyToManyAccessor
	let accessor = ManyToManyAccessor::<User, User>::new(&follower, "following", (*db).clone());
	accessor.remove(&target_user).await.map_err(|e| e.to_string())?;

	Response::ok()
		.with_json(&serde_json::json!({"message": "Successfully unfollowed"}))
		.map_err(Into::into)
}
