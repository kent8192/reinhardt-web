//! List view handlers for followers, followings, and blocked users
//!
//! Handles paginated list endpoints for relationships

use crate::apps::auth::models::User;
use crate::apps::relationship::serializers::{
	BlockingListResponse, FollowerListResponse, FollowingListResponse, PaginationParams,
	UserSummary,
};
use reinhardt::db::orm::{FilterOperator, FilterValue, ManyToManyAccessor, Model};
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
#[get("/accounts/rel/followers/", name = "followers", use_inject = true)]
pub async fn fetch_followers(
	Query(params): Query<PaginationParams>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get authenticated user from CurrentUser
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	// Load user from database using Manager API
	let user = User::objects()
		.filter(
			User::field_id(),
			FilterOperator::Eq,
			FilterValue::String(user_id.to_string()),
		)
		.first()
		.await?
		.ok_or_else(|| "User not found".to_string())?;

	// Extract pagination params
	let page = params.page;
	let limit = params.limit;
	let offset = (page - 1) * limit;

	// Database-level pagination using LIMIT/OFFSET
	let accessor = ManyToManyAccessor::<User, User>::new(&user, "following", (*db).clone());

	// Get total count (separate query)
	let total_count = accessor.count().await.map_err(|e| e.to_string())?;

	// Get paginated results
	let followers = accessor
		.limit(limit)
		.offset(offset)
		.all()
		.await
		.map_err(|e| e.to_string())?;

	// Build response
	let response_data = FollowerListResponse {
		count: total_count,
		next: if offset + limit < total_count {
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
#[get("/accounts/rel/followings/", name = "followings", use_inject = true)]
pub async fn fetch_followings(
	Query(params): Query<PaginationParams>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get authenticated user from CurrentUser
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	// Load user from database using Manager API
	let user = User::objects()
		.filter(
			User::field_id(),
			FilterOperator::Eq,
			FilterValue::String(user_id.to_string()),
		)
		.first()
		.await?
		.ok_or_else(|| "User not found".to_string())?;

	// Extract pagination params
	let page = params.page;
	let limit = params.limit;
	let offset = (page - 1) * limit;

	// Database-level pagination using LIMIT/OFFSET
	let accessor = ManyToManyAccessor::<User, User>::new(&user, "following", (*db).clone());

	// Get total count (separate query)
	let total_count = accessor.count().await.map_err(|e| e.to_string())?;

	// Get paginated results
	let followings = accessor
		.limit(limit)
		.offset(offset)
		.all()
		.await
		.map_err(|e| e.to_string())?;

	// Build response
	let response_data = FollowingListResponse {
		count: total_count,
		next: if offset + limit < total_count {
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
#[get("/accounts/rel/blocking/", name = "blocking", use_inject = true)]
pub async fn fetch_blockings(
	Query(params): Query<PaginationParams>,
	#[inject] db: Arc<DatabaseConnection>,
	#[inject] current_user: CurrentUser<User>,
) -> ViewResult<Response> {
	// Get authenticated user from CurrentUser
	let user_id = current_user.id().map_err(|e| e.to_string())?;

	// Load user from database using Manager API
	let user = User::objects()
		.filter(
			User::field_id(),
			FilterOperator::Eq,
			FilterValue::String(user_id.to_string()),
		)
		.first()
		.await?
		.ok_or_else(|| "User not found".to_string())?;

	// Extract pagination params
	let page = params.page;
	let limit = params.limit;
	let offset = (page - 1) * limit;

	// Database-level pagination using LIMIT/OFFSET
	let accessor = ManyToManyAccessor::<User, User>::new(&user, "blocked_users", (*db).clone());

	// Get total count (separate query)
	let total_count = accessor.count().await.map_err(|e| e.to_string())?;

	// Get paginated results
	let blockings = accessor
		.limit(limit)
		.offset(offset)
		.all()
		.await
		.map_err(|e| e.to_string())?;

	// Build response
	let response_data = BlockingListResponse {
		count: total_count,
		next: if offset + limit < total_count {
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
