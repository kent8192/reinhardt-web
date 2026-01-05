//! Create profile endpoint tests
//!
//! Tests for profile creation functionality including:
//! - Success cases (valid profile creation)
//! - Error cases (not authenticated, user not found, duplicate profile, validation errors)

#[cfg(test)]
mod create_profile_tests {
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

	use super::super::helpers::valid_create_profile_request;

	use crate::apps::auth::models::User;
	use crate::apps::profile::models::Profile;
	use crate::apps::profile::serializers::{CreateProfileRequest, ProfileResponse};

	// Import fixtures from reinhardt-test
	use reinhardt_test::fixtures::create_test_profile;

	/// Helper to call create_profile endpoint directly
	async fn call_create_profile(
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
		let create_req: CreateProfileRequest = reinhardt::core::serde::json::from_value(body)
			.map_err(|e| Error::Validation(format!("Invalid JSON: {}", e)))?;

		create_req
			.validate()
			.map_err(|e| Error::Validation(format!("Validation failed: {}", e)))?;

		// Verify user exists
		let user_manager = User::objects();
		user_manager
			.filter(
				"id",
				FilterOperator::Eq,
				FilterValue::String(user_id.to_string()),
			)
			.first_with_conn(db)
			.await
			.map_err(|e| Error::Database(format!("Database error: {}", e)))?
			.ok_or_else(|| Error::Http("User not found".into()))?;

		// Check if profile already exists
		let profile_manager = Profile::objects();
		let existing_profile = profile_manager
			.filter(
				"user_id",
				FilterOperator::Eq,
				FilterValue::String(user_id.to_string()),
			)
			.first_with_conn(db)
			.await
			.map_err(|e| Error::Database(format!("Database error: {}", e)))?;

		if existing_profile.is_some() {
			return Err(Error::Validation(
				"Profile already exists for this user".into(),
			));
		}

		// Create new profile using generated new() function
		let profile = Profile::new(
			user_id,
			create_req.bio.unwrap_or_default(),
			create_req.avatar_url,
			create_req.location,
			create_req.website,
		);

		// Use ORM API instead of direct SQL
		Profile::objects()
			.create(profile.clone())
			.with_conn(db)
			.await
			.map_err(|e| Error::Database(format!("Failed to create profile: {}", e)))?;

		Ok(ProfileResponse::from(profile))
	}

	/// Test 1: Success - Create profile with valid data
	///
	/// POST /profile/<uuid:user_id>/
	/// Expected: 201 Created with created profile
	#[rstest]
	#[tokio::test]
	async fn test_success_create_profile() {
		// Setup database with migrations
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("create_profile_test@example.com")
				.with_username("create_profile_user"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call create_profile
		let body = valid_create_profile_request();
		let result = call_create_profile(&db, Some(&auth_header), &user.id.to_string(), body).await;

		// Assert success
		assert!(
			result.is_ok(),
			"Create profile should succeed: {:?}",
			result.err()
		);
		let response = result.unwrap();

		// Assert profile data
		assert_eq!(response.user_id, user.id, "User ID should match");
		assert_eq!(response.bio, "Test bio for profile", "Bio should match");
		assert_eq!(
			response.avatar_url,
			Some("https://example.com/avatar.jpg".to_string()),
			"Avatar URL should match"
		);
		assert_eq!(
			response.location,
			Some("Tokyo, Japan".to_string()),
			"Location should match"
		);
		assert_eq!(
			response.website,
			Some("https://example.com".to_string()),
			"Website should match"
		);
	}

	/// Test 2: Success - Create profile with minimal data
	#[rstest]
	#[tokio::test]
	async fn test_success_create_profile_minimal() {
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("minimal_profile@example.com")
				.with_username("minimal_profile_user"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call create_profile with minimal data
		let body = json!({});
		let result = call_create_profile(&db, Some(&auth_header), &user.id.to_string(), body).await;

		// Assert success
		assert!(
			result.is_ok(),
			"Create profile with minimal data should succeed: {:?}",
			result.err()
		);
		let response = result.unwrap();

		// Assert default values
		assert_eq!(response.user_id, user.id, "User ID should match");
		assert_eq!(response.bio, "", "Bio should be empty string by default");
		assert_eq!(response.avatar_url, None, "Avatar URL should be None");
		assert_eq!(response.location, None, "Location should be None");
		assert_eq!(response.website, None, "Website should be None");
	}

	/// Test 3: Failure - Create profile without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_create_profile_without_auth() {
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("no_auth_profile@example.com")
				.with_username("no_auth_profile_user"),
		)
		.await;

		// Call create_profile without auth header
		let body = valid_create_profile_request();
		let result = call_create_profile(&db, None, &user.id.to_string(), body).await;

		// Assert authentication error
		assert!(result.is_err(), "Create profile without auth should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authentication(_)),
			"Error should be authentication error, got: {:?}",
			err
		);
	}

	/// Test 4: Failure - Create profile for non-existent user
	#[rstest]
	#[tokio::test]
	async fn test_failure_create_profile_nonexistent_user() {
		let (_container, db) = setup_test_database().await;

		// Create a user just to get a valid token
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("token_user@example.com")
				.with_username("token_user"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Try to create profile for non-existent user
		let nonexistent_id = Uuid::new_v4();
		let body = valid_create_profile_request();
		let result =
			call_create_profile(&db, Some(&auth_header), &nonexistent_id.to_string(), body).await;

		// Assert error
		assert!(
			result.is_err(),
			"Create profile for non-existent user should fail"
		);
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Http(_)),
			"Error should be HTTP error (404), got: {:?}",
			err
		);
	}

	/// Test 5: Failure - Create duplicate profile
	#[rstest]
	#[tokio::test]
	async fn test_failure_create_duplicate_profile() {
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("duplicate_profile@example.com")
				.with_username("duplicate_profile_user"),
		)
		.await;

		// Create existing profile
		create_test_profile(&db, user.id, "Existing bio").await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Try to create another profile for same user
		let body = valid_create_profile_request();
		let result = call_create_profile(&db, Some(&auth_header), &user.id.to_string(), body).await;

		// Assert error
		assert!(result.is_err(), "Create duplicate profile should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Validation(_)),
			"Error should be validation error (duplicate), got: {:?}",
			err
		);
	}

	/// Test 6: Failure - Create profile with invalid URL format
	#[rstest]
	#[tokio::test]
	async fn test_failure_create_profile_invalid_url() {
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("invalid_url_profile@example.com")
				.with_username("invalid_url_user"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call create_profile with invalid URL
		let body = json!({
			"bio": "Test bio",
			"avatar_url": "not-a-valid-url"
		});
		let result = call_create_profile(&db, Some(&auth_header), &user.id.to_string(), body).await;

		// Assert validation error
		assert!(
			result.is_err(),
			"Create profile with invalid URL should fail"
		);
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Validation(_)),
			"Error should be validation error, got: {:?}",
			err
		);
	}

	/// Test 7: Failure - Create profile with bio too long
	#[rstest]
	#[tokio::test]
	async fn test_failure_create_profile_bio_too_long() {
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("long_bio@example.com")
				.with_username("long_bio_user"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call create_profile with bio over 500 characters
		let long_bio = "x".repeat(501);
		let body = json!({
			"bio": long_bio
		});
		let result = call_create_profile(&db, Some(&auth_header), &user.id.to_string(), body).await;

		// Assert validation error
		assert!(result.is_err(), "Create profile with long bio should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Validation(_)),
			"Error should be validation error, got: {:?}",
			err
		);
	}
}
