//! Profile server functions
//!
//! Server functions for user profile management.
//! These are accessible from both WASM (client stubs) and server (handlers).

use crate::shared::types::{ProfileResponse, UpdateProfileRequest};

// Server-only imports
#[cfg(not(target_arch = "wasm32"))]
use {
	crate::apps::profile::models::Profile,
	reinhardt::DatabaseConnection,
	reinhardt::db::orm::{FilterOperator, FilterValue, Model},
	reinhardt::middleware::session::SessionData,
	reinhardt::pages::server_fn::{ServerFnError, server_fn},
	uuid::Uuid,
	validator::Validate,
};

// WASM-only imports
#[cfg(target_arch = "wasm32")]
use {
	reinhardt::pages::server_fn::{ServerFnError, server_fn},
	uuid::Uuid,
};

/// Fetch user profile
#[cfg(not(target_arch = "wasm32"))]
#[server_fn(use_inject = true)]
pub async fn fetch_profile(
	user_id: Uuid,
	#[inject] _db: DatabaseConnection,
) -> std::result::Result<ProfileResponse, ServerFnError> {
	let profile = Profile::objects()
		.filter(
			Profile::field_user_id(),
			FilterOperator::Eq,
			FilterValue::String(user_id.to_string()),
		)
		.first()
		.await
		.map_err(|e| ServerFnError::server(500, format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(404, "Profile not found"))?;

	Ok(ProfileResponse::from(profile))
}

/// Fetch profile - WASM client stub
#[cfg(target_arch = "wasm32")]
#[server_fn]
pub async fn fetch_profile(user_id: Uuid) -> std::result::Result<ProfileResponse, ServerFnError> {
	unreachable!("This function body should be replaced by the server_fn macro")
}

/// Update user profile
#[cfg(not(target_arch = "wasm32"))]
#[server_fn(use_inject = true)]
pub async fn update_profile(
	request: UpdateProfileRequest,
	#[inject] db: DatabaseConnection,
	#[inject] session: SessionData,
) -> std::result::Result<ProfileResponse, ServerFnError> {
	// Validate request
	request
		.validate()
		.map_err(|e| ServerFnError::server(400, format!("Validation failed: {}", e)))?;

	let user_id = session
		.get::<Uuid>("user_id")
		.ok_or_else(|| ServerFnError::server(401, "Not authenticated"))?;

	// Find existing profile
	let mut profile = Profile::objects()
		.filter(
			Profile::field_user_id(),
			FilterOperator::Eq,
			FilterValue::String(user_id.to_string()),
		)
		.first()
		.await
		.map_err(|e| ServerFnError::server(500, format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(404, "Profile not found"))?;

	// Update fields
	if let Some(bio) = request.bio {
		profile.set_bio(bio);
	}
	if let Some(avatar_url) = request.avatar_url {
		profile.set_avatar_url(avatar_url);
	}
	if let Some(location) = request.location {
		profile.set_location(Some(location));
	}
	if let Some(website) = request.website {
		profile.set_website(Some(website));
	}

	// Save to database
	Profile::objects()
		.update_with_conn(&db, &profile)
		.await
		.map_err(|e| ServerFnError::server(500, format!("Database error: {}", e)))?;

	Ok(ProfileResponse::from(profile))
}

/// Update profile - WASM client stub
#[cfg(target_arch = "wasm32")]
#[server_fn]
pub async fn update_profile(
	request: UpdateProfileRequest,
) -> std::result::Result<ProfileResponse, ServerFnError> {
	unreachable!("This function body should be replaced by the server_fn macro")
}

/// Form-compatible wrapper for update_profile
///
/// This wrapper accepts individual field arguments from form! macro's server_fn integration
/// and converts them to UpdateProfileRequest. Returns `Result<(), ServerFnError>`
/// as expected by form! macro's submit() method.
///
/// The argument order matches form! macro's field order: avatar_url, bio, location, website
#[cfg(not(target_arch = "wasm32"))]
#[server_fn(use_inject = true)]
pub async fn update_profile_form(
	avatar_url: String,
	bio: String,
	location: String,
	website: String,
	#[inject] db: DatabaseConnection,
	#[inject] session: SessionData,
) -> std::result::Result<(), ServerFnError> {
	let request = UpdateProfileRequest {
		avatar_url: if avatar_url.is_empty() {
			None
		} else {
			Some(avatar_url)
		},
		bio: if bio.is_empty() { None } else { Some(bio) },
		location: if location.is_empty() {
			None
		} else {
			Some(location)
		},
		website: if website.is_empty() {
			None
		} else {
			Some(website)
		},
	};

	// Call the main update_profile logic (inline to avoid async recursion issues)
	request
		.validate()
		.map_err(|e| ServerFnError::server(400, format!("Validation failed: {}", e)))?;

	let user_id = session
		.get::<Uuid>("user_id")
		.ok_or_else(|| ServerFnError::server(401, "Not authenticated"))?;

	let mut profile = Profile::objects()
		.filter(
			Profile::field_user_id(),
			FilterOperator::Eq,
			FilterValue::String(user_id.to_string()),
		)
		.first()
		.await
		.map_err(|e| ServerFnError::server(500, format!("Database error: {}", e)))?
		.ok_or_else(|| ServerFnError::server(404, "Profile not found"))?;

	if let Some(bio) = request.bio {
		profile.set_bio(bio);
	}
	if let Some(avatar_url) = request.avatar_url {
		profile.set_avatar_url(avatar_url);
	}
	if let Some(location) = request.location {
		profile.set_location(Some(location));
	}
	if let Some(website) = request.website {
		profile.set_website(Some(website));
	}

	Profile::objects()
		.update_with_conn(&db, &profile)
		.await
		.map_err(|e| ServerFnError::server(500, format!("Database error: {}", e)))?;

	Ok(())
}

/// Form-compatible update profile - WASM client stub
#[cfg(target_arch = "wasm32")]
#[server_fn]
pub async fn update_profile_form(
	avatar_url: String,
	bio: String,
	location: String,
	website: String,
) -> std::result::Result<(), ServerFnError> {
	unreachable!("This function body should be replaced by the server_fn macro")
}
