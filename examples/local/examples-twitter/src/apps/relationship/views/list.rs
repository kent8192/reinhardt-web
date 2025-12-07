//! List view handlers for followers, followings, and blocked users
//!
//! Handles paginated list endpoints for relationships

use crate::apps::auth::models::User;
use crate::apps::relationship::serializers::{
	BlockingListResponse, FollowerListResponse, FollowingListResponse, PaginationParams,
	UserSummary,
};
use reinhardt::db::orm::{ManyToManyAccessor, Model};
use reinhardt::db::DatabaseConnection;
use reinhardt::get;
use reinhardt::{CurrentUser, Error, Query, Response, StatusCode, ViewResult};
use std::sync::Arc;

/// List followers of the authenticated user
///
/// GET /accounts/rel/followers/
/// Success response: 200 OK with paginated follower list
/// Error responses:
/// - 401 Unauthorized: Not authenticated
#[get("/accounts/rel/followers/", name = "relationship_followers", use_inject = true)]
pub async fn fetch_followers(
	Query(params): Query<PaginationParams>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get authenticated user from CurrentUser
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	// Load user from database
	let user = User::objects()
		.filter_by(User::field_id().eq(user_id))
		.get_with_db(&db)
		.await?;

	// Extract pagination params
	let page = params.page;
	let limit = params.limit;

	// Query followers from database using ManyToManyAccessor
	// Note: "followers" is the reverse relationship of "following"
	let accessor = ManyToManyAccessor::<User, User>::new(&user, "followers", (*db).clone());
	let all_followers = accessor.all().await.map_err(|e| e.to_string())?;

	// Apply pagination
	let total_count = all_followers.len();
	let start = (page - 1) * limit;
	let followers: Vec<_> = all_followers.into_iter().skip(start).take(limit).collect();

	// Build response
	let response_data = FollowerListResponse {
		count: total_count,
		next: if start + limit < total_count {
			Some(format!("?page={}", page + 1))
		} else {
			None
		},
		previous: if page > 1 {
			Some(format!("?page={}", page - 1))
		} else {
			None
		},
		results: followers.into_iter().map(UserSummary::from).collect(),
	};

	let json = serde_json::to_string(&response_data)
		.map_err(|e| Error::Serialization(format!("JSON serialization failed: {}", e)))?;
	Ok(Response::new(StatusCode::OK).with_body(json))
}

/// List users the authenticated user is following
/// GET /accounts/rel/followings/
/// Success response: 200 OK with paginated following list
/// Error responses:
/// - 401 Unauthorized: Not authenticated
#[get("/accounts/rel/followings/", name = "relationship_followings", use_inject = true)]
pub async fn fetch_followings(
	Query(params): Query<PaginationParams>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get authenticated user from CurrentUser
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	// Load user from database
	let user = User::objects()
		.filter_by(User::field_id().eq(user_id))
		.get_with_db(&db)
		.await?;

	// Extract pagination params
	let page = params.page;
	let limit = params.limit;

	// Query followings from database using ManyToManyAccessor
	let accessor = ManyToManyAccessor::<User, User>::new(&user, "following", (*db).clone());
	let all_followings = accessor.all().await.map_err(|e| e.to_string())?;

	// Apply pagination
	let total_count = all_followings.len();
	let start = (page - 1) * limit;
	let followings: Vec<_> = all_followings.into_iter().skip(start).take(limit).collect();

	// Build response
	let response_data = FollowingListResponse {
		count: total_count,
		next: if start + limit < total_count {
			Some(format!("?page={}", page + 1))
		} else {
			None
		},
		previous: if page > 1 {
			Some(format!("?page={}", page - 1))
		} else {
			None
		},
		results: followings.into_iter().map(UserSummary::from).collect(),
	};

	let json = serde_json::to_string(&response_data)
		.map_err(|e| Error::Serialization(format!("JSON serialization failed: {}", e)))?;
	Ok(Response::new(StatusCode::OK).with_body(json))
}

/// List blocked users
/// GET /accounts/rel/blocking/
/// Success response: 200 OK with paginated blocking list
/// Error responses:
/// - 401 Unauthorized: Not authenticated
#[get("/accounts/rel/blocking/", name = "relationship_blocking", use_inject = true)]
pub async fn fetch_blockings(
	Query(params): Query<PaginationParams>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get authenticated user from CurrentUser
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	// Load user from database
	let user = User::objects()
		.filter_by(User::field_id().eq(user_id))
		.get_with_db(&db)
		.await?;

	// Extract pagination params
	let page = params.page;
	let limit = params.limit;

	// Query blockings from database using ManyToManyAccessor
	let accessor = ManyToManyAccessor::<User, User>::new(&user, "blocked_users", (*db).clone());
	let all_blockings = accessor.all().await.map_err(|e| e.to_string())?;

	// Apply pagination
	let total_count = all_blockings.len();
	let start = (page - 1) * limit;
	let blockings: Vec<_> = all_blockings.into_iter().skip(start).take(limit).collect();

	// Build response
	let response_data = BlockingListResponse {
		count: total_count,
		next: if start + limit < total_count {
			Some(format!("?page={}", page + 1))
		} else {
			None
		},
		previous: if page > 1 {
			Some(format!("?page={}", page - 1))
		} else {
			None
		},
		results: blockings.into_iter().map(UserSummary::from).collect(),
	};

	let json = serde_json::to_string(&response_data)
		.map_err(|e| Error::Serialization(format!("JSON serialization failed: {}", e)))?;
	Ok(Response::new(StatusCode::OK).with_body(json))
}
