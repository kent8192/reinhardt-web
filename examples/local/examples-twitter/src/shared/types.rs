//! Shared types used by both client and server
//!
//! These types are serializable and can be sent between the WASM client
//! and the Rust server via server functions.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// OpenAPI schema generation (server-side only)
#[cfg(not(target_arch = "wasm32"))]
use reinhardt::rest::openapi::{Schema, ToSchema};

// ============================================================================
// User Types
// ============================================================================

/// User information (shared between client and server)
#[cfg_attr(not(target_arch = "wasm32"), derive(Schema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
	pub id: Uuid,
	pub username: String,
	pub email: String,
	pub is_active: bool,
}

/// Conversion from server-side User model to shared UserInfo
#[cfg(not(target_arch = "wasm32"))]
impl From<crate::apps::auth::models::User> for UserInfo {
	fn from(user: crate::apps::auth::models::User) -> Self {
		UserInfo {
			id: user.id(),
			username: user.username().to_string(),
			email: user.email().to_string(),
			is_active: user.is_active(),
		}
	}
}

// ============================================================================
// Authentication Types
// ============================================================================

/// Login request
#[cfg_attr(not(target_arch = "wasm32"), derive(Schema))]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoginRequest {
	#[validate(email(message = "Invalid email address"))]
	pub email: String,

	#[validate(length(min = 1, message = "Password is required"))]
	pub password: String,
}

/// Register request
#[cfg_attr(not(target_arch = "wasm32"), derive(Schema))]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RegisterRequest {
	#[validate(length(
		min = 3,
		max = 150,
		message = "Username must be between 3 and 150 characters"
	))]
	pub username: String,

	#[validate(email(message = "Invalid email address"))]
	pub email: String,

	#[validate(length(min = 8, message = "Password must be at least 8 characters"))]
	pub password: String,

	#[validate(length(
		min = 8,
		message = "Password confirmation must be at least 8 characters"
	))]
	pub password_confirmation: String,
}

impl RegisterRequest {
	/// Validate that password and password_confirmation match
	pub fn validate_passwords_match(&self) -> Result<(), String> {
		if self.password != self.password_confirmation {
			return Err("Passwords do not match".to_string());
		}
		Ok(())
	}
}

// ============================================================================
// Profile Types
// ============================================================================

/// Profile response
#[cfg_attr(not(target_arch = "wasm32"), derive(Schema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResponse {
	pub user_id: Uuid,
	pub bio: Option<String>,
	pub avatar_url: Option<String>,
	pub location: Option<String>,
	pub website: Option<String>,
}

/// Conversion from server-side Profile model to shared ProfileResponse
#[cfg(not(target_arch = "wasm32"))]
impl From<crate::apps::profile::models::Profile> for ProfileResponse {
	fn from(profile: crate::apps::profile::models::Profile) -> Self {
		ProfileResponse {
			user_id: profile.user_id(),
			bio: Some(profile.bio().to_string()),
			avatar_url: Some(profile.avatar_url().to_string()),
			location: profile.location().clone(),
			website: profile.website().clone(),
		}
	}
}

/// Update profile request
#[cfg_attr(not(target_arch = "wasm32"), derive(Schema))]
#[derive(Debug, Clone, Serialize, Deserialize, Validate, Default)]
pub struct UpdateProfileRequest {
	#[validate(length(max = 500, message = "Bio must be less than 500 characters"))]
	pub bio: Option<String>,

	#[validate(url(message = "Invalid avatar URL"))]
	pub avatar_url: Option<String>,

	#[validate(length(max = 100, message = "Location must be less than 100 characters"))]
	pub location: Option<String>,

	#[validate(url(message = "Invalid website URL"))]
	pub website: Option<String>,
}

// ============================================================================
// Tweet Types
// ============================================================================

/// Tweet information
#[cfg_attr(not(target_arch = "wasm32"), derive(Schema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweetInfo {
	pub id: Uuid,
	pub user_id: Uuid,
	pub username: String, // Also includes username
	pub content: String,
	pub like_count: i32,
	pub retweet_count: i32,
	pub created_at: String, // ISO 8601 format
}

impl TweetInfo {
	/// Create a new TweetInfo instance
	pub fn new(
		id: Uuid,
		user_id: Uuid,
		username: String,
		content: String,
		like_count: i32,
		retweet_count: i32,
		created_at: String,
	) -> Self {
		Self {
			id,
			user_id,
			username,
			content,
			like_count,
			retweet_count,
			created_at,
		}
	}
}

/// Conversion from server-side Tweet model to shared TweetInfo
#[cfg(not(target_arch = "wasm32"))]
impl From<crate::apps::tweet::models::Tweet> for TweetInfo {
	fn from(tweet: crate::apps::tweet::models::Tweet) -> Self {
		TweetInfo {
			id: tweet.id(),
			user_id: *tweet.user_id(),
			username: String::new(), // Will be set by server_fn
			content: tweet.content().to_string(),
			like_count: tweet.like_count(),
			retweet_count: tweet.retweet_count(),
			created_at: tweet.created_at().to_rfc3339(),
		}
	}
}

/// Create tweet request
#[cfg_attr(not(target_arch = "wasm32"), derive(Schema))]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateTweetRequest {
	#[validate(length(
		min = 1,
		max = 280,
		message = "Tweet must be between 1 and 280 characters"
	))]
	pub content: String,
}
