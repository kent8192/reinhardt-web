//! Profile serializers
//!
//! Serializers for profile CRUD operations

use crate::apps::profile::models::Profile;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Request data for creating a profile (POST)
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreateProfileRequest {
	/// Biography (up to 500 characters)
	#[validate(length(max = 500, message = "Bio must be at most 500 characters"))]
	pub bio: Option<String>,

	/// Avatar URL
	#[validate(length(max = 255, message = "Avatar URL must be at most 255 characters"))]
	#[validate(url(message = "Avatar URL must be a valid URL"))]
	pub avatar_url: Option<String>,

	/// Location
	#[validate(length(max = 255, message = "Location must be at most 255 characters"))]
	pub location: Option<String>,

	/// Website URL
	#[validate(length(max = 255, message = "Website must be at most 255 characters"))]
	#[validate(url(message = "Website must be a valid URL"))]
	pub website: Option<String>,
}

/// Request data for updating a profile (PATCH)
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateProfileRequest {
	/// Biography (up to 500 characters)
	#[validate(length(max = 500, message = "Bio must be at most 500 characters"))]
	pub bio: Option<String>,

	/// Avatar URL
	#[validate(length(max = 255, message = "Avatar URL must be at most 255 characters"))]
	#[validate(url(message = "Avatar URL must be a valid URL"))]
	pub avatar_url: Option<String>,

	/// Location
	#[validate(length(max = 255, message = "Location must be at most 255 characters"))]
	pub location: Option<String>,

	/// Website URL
	#[validate(length(max = 255, message = "Website must be at most 255 characters"))]
	#[validate(url(message = "Website must be a valid URL"))]
	pub website: Option<String>,
}

/// Response data for profile retrieval
#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileResponse {
	pub id: Uuid,
	pub user_id: Uuid,
	pub bio: String,
	pub avatar_url: Option<String>,
	pub location: Option<String>,
	pub website: Option<String>,
}

impl From<Profile> for ProfileResponse {
	fn from(profile: Profile) -> Self {
		ProfileResponse {
			id: profile.id,
			user_id: profile.user_id,
			bio: profile.bio,
			avatar_url: profile.avatar_url,
			location: profile.location,
			website: profile.website,
		}
	}
}
