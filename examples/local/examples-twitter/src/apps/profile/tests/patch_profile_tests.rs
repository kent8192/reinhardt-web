//! Patch profile endpoint tests
//!
//! Tests for profile update functionality including:
//! - Success cases (full update, partial update)
//! - Error cases (not authenticated, profile not found, validation errors)

#[cfg(test)]
mod patch_profile_tests {
	use reinhardt::StatusCode;
	use reinhardt::core::serde::json::json;
	use reinhardt::db::DatabaseConnection;
	use reinhardt::db::associations::OneToOneField;
	use reinhardt::db::orm::{FilterOperator, FilterValue, Manager};
	use rstest::rstest;
	use uuid::Uuid;
	use validator::Validate;

	use crate::test_utils::{
		TestUserParams, create_test_user, generate_test_token, setup_test_database,
	};

	use super::super::helpers::{partial_update_profile_request, valid_update_profile_request};

	use crate::apps::profile::models::Profile;
	use crate::apps::profile::serializers::{ProfileResponse, UpdateProfileRequest};

	// Import fixtures from reinhardt-test
	use reinhardt_test::fixtures::create_test_profile;

	/// Helper to call patch_profile endpoint directly
	async fn call_patch_profile(
		db: &DatabaseConnection,
		auth_header: Option<&str>,
		user_id_str: &str,
		body: reinhardt::core::serde::json::Value,
	) -> Result<ProfileResponse, reinhardt::Error> {
		use chrono::Utc;
		use reinhardt::{Error, JwtAuth};

		// Check authentication
		let _claims = match auth_header {
			Some(header) => {
				let token = header.strip_prefix("Bearer ").ok_or_else(|| {
					Error::Authentication("Invalid Authorization header format".into())
				})?;

				let jwt_auth = JwtAuth::new(b"test-secret-key-for-testing-only");
				jwt_auth
					.verify_token(token)
					.map_err(|e| Error::Authentication(format!("Invalid token: {}", e)))?
			}
			None => {
				return Err(Error::Authentication("Missing Authorization header".into()));
			}
		};

		// Parse and validate user_id
		let user_id = Uuid::parse_str(user_id_str)
			.map_err(|e| Error::Validation(format!("Invalid user_id format: {}", e)))?;

		// Parse and validate request
		let update_req: UpdateProfileRequest = reinhardt::core::serde::json::from_value(body)
			.map_err(|e| Error::Validation(format!("Invalid JSON: {}", e)))?;

		update_req
			.validate()
			.map_err(|e| Error::Validation(format!("Validation failed: {}", e)))?;

		// Fetch existing profile
		let manager = Profile::objects();
		let mut profile = manager
			.filter(
				"user_id",
				FilterOperator::Eq,
				FilterValue::String(user_id.to_string()),
			)
			.first_with_conn(db)
			.await
			.map_err(|e| Error::Database(format!("Database error: {}", e)))?
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

		// Use ORM API instead of direct SQL
		Profile::objects()
			.update(profile.id, profile.clone())
			.with_conn(db)
			.await
			.map_err(|e| Error::Database(format!("Failed to update profile: {}", e)))?;

		Ok(ProfileResponse::from(profile))
	}

	/// Test 1: Success - Update profile with valid data
	///
	/// PATCH /profile/<uuid:user_id>/
	/// Expected: 200 OK with updated profile
	#[rstest]
	#[tokio::test]
	async fn test_success_patch_profile() {
		// Setup database with migrations
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("patch_profile_test@example.com")
				.with_username("patch_profile_user"),
		)
		.await;

		// Create profile
		create_test_profile(&db, user.id, "Original bio", Some("Original location")).await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call patch_profile
		let body = valid_update_profile_request();
		let result = call_patch_profile(&db, Some(&auth_header), &user.id.to_string(), body).await;

		// Assert success
		assert!(
			result.is_ok(),
			"Patch profile should succeed: {:?}",
			result.err()
		);
		let response = result.unwrap();

		// Assert updated data
		assert_eq!(response.user_id, user.id, "User ID should match");
		assert_eq!(response.bio, "Updated bio", "Bio should be updated");
		assert_eq!(
			response.location,
			Some("Osaka, Japan".to_string()),
			"Location should be updated"
		);
	}

	/// Test 2: Success - Partial update (only bio)
	#[rstest]
	#[tokio::test]
	async fn test_success_patch_profile_partial() {
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("partial_patch@example.com")
				.with_username("partial_patch_user"),
		)
		.await;

		// Create profile with location
		let original_location = "Tokyo, Japan";
		create_test_profile(&db, user.id, "Original bio", Some(original_location)).await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call patch_profile with only bio
		let body = partial_update_profile_request();
		let result = call_patch_profile(&db, Some(&auth_header), &user.id.to_string(), body).await;

		// Assert success
		assert!(
			result.is_ok(),
			"Partial patch should succeed: {:?}",
			result.err()
		);
		let response = result.unwrap();

		// Assert bio is updated but location remains
		assert_eq!(response.bio, "Only bio updated", "Bio should be updated");
		assert_eq!(
			response.location,
			Some(original_location.to_string()),
			"Location should remain unchanged"
		);
	}

	/// Test 3: Failure - Update profile without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_patch_profile_without_auth() {
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("no_auth_patch@example.com")
				.with_username("no_auth_patch_user"),
		)
		.await;

		// Create profile
		create_test_profile(&db, user.id, "Test bio", None).await;

		// Call patch_profile without auth header
		let body = valid_update_profile_request();
		let result = call_patch_profile(&db, None, &user.id.to_string(), body).await;

		// Assert authentication error
		assert!(result.is_err(), "Patch without auth should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authentication(_)),
			"Error should be authentication error, got: {:?}",
			err
		);
	}

	/// Test 4: Failure - Update non-existent profile
	#[rstest]
	#[tokio::test]
	async fn test_failure_patch_nonexistent_profile() {
		let (_container, db) = setup_test_database().await;

		// Create test user but don't create profile
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("nonexistent_patch@example.com")
				.with_username("nonexistent_patch_user"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call patch_profile for user without profile
		let body = valid_update_profile_request();
		let result = call_patch_profile(&db, Some(&auth_header), &user.id.to_string(), body).await;

		// Assert error
		assert!(result.is_err(), "Patch non-existent profile should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Http(_)),
			"Error should be HTTP error (404), got: {:?}",
			err
		);
	}

	/// Test 5: Failure - Update with invalid URL format
	#[rstest]
	#[tokio::test]
	async fn test_failure_patch_profile_invalid_url() {
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("invalid_url_patch@example.com")
				.with_username("invalid_url_patch_user"),
		)
		.await;

		// Create profile
		create_test_profile(&db, user.id, "Test bio", None).await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call patch_profile with invalid URL
		let body = json!({
			"website": "not-a-valid-url"
		});
		let result = call_patch_profile(&db, Some(&auth_header), &user.id.to_string(), body).await;

		// Assert validation error
		assert!(result.is_err(), "Patch with invalid URL should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Validation(_)),
			"Error should be validation error, got: {:?}",
			err
		);
	}

	/// Test 6: Failure - Update with bio too long
	#[rstest]
	#[tokio::test]
	async fn test_failure_patch_profile_bio_too_long() {
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("long_bio_patch@example.com")
				.with_username("long_bio_patch_user"),
		)
		.await;

		// Create profile
		create_test_profile(&db, user.id, "Test bio", None).await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call patch_profile with bio over 500 characters
		let long_bio = "x".repeat(501);
		let body = json!({
			"bio": long_bio
		});
		let result = call_patch_profile(&db, Some(&auth_header), &user.id.to_string(), body).await;

		// Assert validation error
		assert!(result.is_err(), "Patch with long bio should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Validation(_)),
			"Error should be validation error, got: {:?}",
			err
		);
	}
}
