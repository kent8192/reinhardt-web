//! Relationship serializers for Follow and Block operations
//!
//! Response serializers for relationship endpoints

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Query parameters for pagination
#[derive(Debug, Deserialize, Default)]
pub struct PaginationParams {
	#[serde(default = "default_page")]
	pub page: usize,
	#[serde(default = "default_limit")]
	pub limit: usize,
}

fn default_page() -> usize {
	1
}

fn default_limit() -> usize {
	20
}

/// Response for successful follow operation
#[derive(Debug, Serialize, Deserialize)]
pub struct FollowResponse {
	pub follower_id: Uuid,
	pub followed_id: Uuid,
	pub followed_at: DateTime<Utc>,
}

impl FollowResponse {
	/// Create a new follow response
	pub fn new(follower_id: Uuid, followed_id: Uuid) -> Self {
		Self {
			follower_id,
			followed_id,
			followed_at: Utc::now(),
		}
	}
}

/// Response for successful block operation
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockResponse {
	pub blocker_id: Uuid,
	pub blocked_id: Uuid,
	pub blocked_at: DateTime<Utc>,
}

impl BlockResponse {
	/// Create a new block response
	pub fn new(blocker_id: Uuid, blocked_id: Uuid) -> Self {
		Self {
			blocker_id,
			blocked_id,
			blocked_at: Utc::now(),
		}
	}
}

/// Simple user info for list responses
#[derive(Debug, Serialize, Deserialize)]
pub struct UserSummary {
	pub id: Uuid,
	pub username: String,
	pub email: String,
}

impl From<crate::apps::auth::models::User> for UserSummary {
	fn from(user: crate::apps::auth::models::User) -> Self {
		Self {
			id: user.id,
			username: user.username,
			email: user.email,
		}
	}
}

/// Paginated list response for followers
#[derive(Debug, Serialize, Deserialize)]
pub struct FollowerListResponse {
	pub count: usize,
	pub next: Option<String>,
	pub previous: Option<String>,
	pub results: Vec<UserSummary>,
}

/// Paginated list response for followings
#[derive(Debug, Serialize, Deserialize)]
pub struct FollowingListResponse {
	pub count: usize,
	pub next: Option<String>,
	pub previous: Option<String>,
	pub results: Vec<UserSummary>,
}

/// Paginated list response for blocked users
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockingListResponse {
	pub count: usize,
	pub next: Option<String>,
	pub previous: Option<String>,
	pub results: Vec<UserSummary>,
}
