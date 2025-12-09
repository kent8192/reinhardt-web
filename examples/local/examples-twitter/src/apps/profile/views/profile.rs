//! Profile view handlers
//!
//! Handles profile CRUD endpoints.
//! Uses reinhardt ORM (Model::objects()) for database operations.

use crate::apps::auth::models::User;
use crate::apps::profile::models::Profile;
use crate::apps::profile::serializers::{
	CreateProfileRequest, ProfileResponse, UpdateProfileRequest,
};
use chrono::Utc;
use reinhardt::prelude::*;
use reinhardt::db::orm::{FilterOperator, FilterValue};
use reinhardt::{Error, Json, Path};
use uuid::Uuid;
use validator::Validate;

/// Fetch a profile by user ID
///
/// GET /profile/<uuid: user_id>/
/// Success response: 200 OK with profile data
/// Error responses:
/// - 404 Not Found: Profile not found
#[get("/{<uuid:user_id>}/", name = "fetch", use_inject = true)]
pub async fn fetch_profile(
	Path(user_id): Path<Uuid>,
	#[inject] _db: DatabaseConnection,
) -> ViewResult<Response> {
	// Fetch profile using Manager API
	let profile_manager = Profile::objects();
	let profile = profile_manager
		.filter(
			"user_id",
			FilterOperator::Eq,
			FilterValue::String(user_id.to_string()),
		)
		.first()
		.await?
		.ok_or_else(|| Error::Http("Profile not found".into()))?;

	let response_data = ProfileResponse::from(profile);
	Response::ok()
		.with_json(&response_data)
		.map_err(Into::into)
}

/// Create a new profile for a user
/// POST /profile/<uuid: user_id>/
/// Request body:
/// ```json
/// {
///   "bio": "My bio",
///   "avatar_url": "https://example.com/avatar.jpg",
///   "location": "Tokyo",
///   "website": "https://example.com"
/// }
/// ```
/// Success response: 201 Created with created profile
/// Error responses:
/// - 401 Unauthorized: Not authenticated
/// - 404 Not Found: User not found
/// - 422 Unprocessable Entity: Validation errors
#[post("/{<uuid:user_id>}/", name = "create", use_inject = true)]
pub async fn create_profile(
	Path(user_id): Path<Uuid>,
	Json(create_req): Json<CreateProfileRequest>,
	#[inject] db: DatabaseConnection,
) -> ViewResult<Response> {
	// Validate request (automatic JSON parsing by Json extractor)
	create_req
		.validate()
		.map_err(|e| Error::Validation(format!("Validation failed: {}", e)))?;

	// Verify user exists using Manager API
	let user_manager = User::objects();
	user_manager
		.filter(
			"id",
			FilterOperator::Eq,
			FilterValue::String(user_id.to_string()),
		)
		.first()
		.await?
		.ok_or_else(|| Error::Http("User not found".into()))?;

	// Create new profile using generated new() function
	// new() auto-generates id, timestamps, and OneToOneField instance
	let mut profile = Profile::new(
		create_req.bio.unwrap_or_default(),
		create_req.avatar_url,
		create_req.location,
		create_req.website,
	);

	// Manually set user_id (not included in constructor)
	profile.user_id = user_id;

	// Create profile using Manager API
	let profile_manager = Profile::objects();
	let created = profile_manager.create_with_conn(&db, &profile).await?;

	let response_data = ProfileResponse::from(created);
	Response::ok()
		.with_json(&response_data)
		.map_err(Into::into)
}

/// Update an existing profile
/// PATCH /profile/<uuid: user_id>/
/// Request body (all fields optional):
/// ```json
/// {
///   "bio": "Updated bio",
///   "avatar_url": "https://example.com/new-avatar.jpg",
///   "location": "Osaka",
///   "website": "https://newsite.com"
/// }
/// ```
/// Success response: 200 OK with updated profile
/// Error responses:
/// - 404 Not Found: Profile not found
/// - 422 Unprocessable Entity: Validation errors
#[patch("/{<uuid:user_id>}/", name = "patch", use_inject = true)]
pub async fn patch_profile(
	Path(user_id): Path<Uuid>,
	Json(update_req): Json<UpdateProfileRequest>,
	#[inject] db: DatabaseConnection,
) -> ViewResult<Response> {
	// Validate request (automatic JSON parsing by Json extractor)
	update_req
		.validate()
		.map_err(|e| Error::Validation(format!("Validation failed: {}", e)))?;

	// Fetch existing profile using Manager API
	let profile_manager = Profile::objects();
	let mut profile = profile_manager
		.filter(
			"user_id",
			FilterOperator::Eq,
			FilterValue::String(user_id.to_string()),
		)
		.first()
		.await?
		.ok_or_else(|| Error::Http("Profile not found".into()))?;

	// Apply updates
	if let Some(bio) = update_req.bio {
		profile.bio = bio;
	}
	if let Some(avatar_url) = update_req.avatar_url {
		profile.avatar_url = Some(avatar_url);
	}
	if let Some(location) = update_req.location {
		profile.location = Some(location);
	}
	if let Some(website) = update_req.website {
		profile.website = Some(website);
	}
	profile.updated_at = Utc::now();

	// Update profile using Manager API
	let updated = profile_manager.update_with_conn(&db, &profile).await?;

	let response_data = ProfileResponse::from(updated);
	Response::ok()
		.with_json(&response_data)
		.map_err(Into::into)
}
