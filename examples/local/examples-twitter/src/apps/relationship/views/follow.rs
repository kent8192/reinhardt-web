//! Follow/Unfollow view handlers
//!
//! Handles follow and unfollow operations

use crate::apps::auth::models::User;
use crate::apps::relationship::serializers::FollowResponse;
use reinhardt::db::orm::Model;
use reinhardt::db::DatabaseConnection;
use reinhardt::{delete, post};
use reinhardt::{Error, Path, Response, StatusCode, ViewResult};
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
	#[inject] current_user: Arc<User>,
) -> ViewResult<Response> {
	// Check if target user exists
	let target_user = User::objects()
		.filter(User::field_id().eq(followed_id))
		.get(&db)
		.await?
		.ok_or_else(|| Error::NotFound("User not found".into()))?;

	// Check if already following
	let is_following = current_user.following.contains(&db, target_user.id).await?;
	if is_following {
		return Err(Error::Conflict("Already following this user".into()));
	}

	// Add to following
	current_user.following.add(&db, target_user.id).await?;

	let response_data = FollowResponse::new(current_user.id, target_user.id);
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
	#[inject] current_user: Arc<User>,
) -> ViewResult<Response> {
	// Check if target user exists
	let target_user = User::objects()
		.filter(User::field_id().eq(followed_id))
		.get(&db)
		.await?
		.ok_or_else(|| Error::NotFound("User not found".into()))?;

	// Check if following
	let is_following = current_user.following.contains(&db, target_user.id).await?;
	if !is_following {
		return Err(Error::Conflict("Not following this user".into()));
	}

	// Remove from following
	current_user.following.remove(&db, target_user.id).await?;

	Response::ok()
		.with_json(&serde_json::json!({"message": "Successfully unfollowed"}))
		.map_err(Into::into)
}
