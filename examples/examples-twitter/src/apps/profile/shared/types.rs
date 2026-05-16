//! Shared types for profile application
//!
//! These types are serializable and can be sent between the WASM client
//! and the Rust server via server functions.

use reinhardt::dto;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Profile response
#[dto]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResponse {
	pub user_id: Uuid,
	pub bio: Option<String>,
	pub avatar_url: Option<String>,
	pub location: Option<String>,
	pub website: Option<String>,
}

/// Conversion from server-side Profile model to shared ProfileResponse
#[cfg(native)]
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
#[dto]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
