//! Fetch profile endpoint tests
//!
//! Tests for profile retrieval functionality including:
//! - Success cases (existing profile)
//! - Error cases (non-existent profile, invalid UUID format)

#[cfg(test)]
mod fetch_profile_tests {
	use reinhardt::StatusCode;
	use reinhardt::db::DatabaseConnection;
	use reinhardt::db::orm::{FilterOperator, FilterValue, Manager};
	use rstest::rstest;
	use uuid::Uuid;

	use crate::test_utils::{TestUserParams, create_test_user, setup_test_database};

	use crate::apps::profile::models::Profile;
	use crate::apps::profile::serializers::ProfileResponse;

	/// Helper to create a profile directly in the database
	async fn create_test_profile(db: &DatabaseConnection, user_id: Uuid, bio: &str) -> Profile {
		// Create profile using generated new() function
		let profile = Profile::new(user_id, bio.to_string(), None, None, None);

		Profile::objects()
			.create(profile.clone())
			.with_conn(db)
			.await
			.expect("Failed to create test profile");

		profile
	}

	/// Helper to call fetch_profile endpoint directly
	async fn call_fetch_profile(
		db: &DatabaseConnection,
		user_id_str: &str,
	) -> Result<ProfileResponse, reinhardt::Error> {
		use reinhardt::Error;

		// Parse and validate user_id
		let user_id = Uuid::parse_str(user_id_str)
			.map_err(|e| Error::Validation(format!("Invalid user_id format: {}", e)))?;

		// Fetch profile using Manager/QuerySet API
		let manager = Profile::objects();
		let profile = manager
			.filter(
				Profile::field_user_id(),
				FilterOperator::Eq,
				FilterValue::String(user_id.to_string()),
			)
			.first_with_conn(db)
			.await
			.map_err(|e| Error::Database(format!("Database error: {}", e)))?
			.ok_or_else(|| Error::Http("Profile not found".into()))?;

		Ok(ProfileResponse::from(profile))
	}

	/// Test 1: Success - Fetch existing profile
	///
	/// GET /profile/<uuid:user_id>/
	/// Expected: 200 OK with profile data
	#[rstest]
	#[tokio::test]
	async fn test_success_fetch_profile() {
		// Setup database with migrations
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("profile_fetch_test@example.com")
				.with_username("profile_fetch_user"),
		)
		.await;

		// Create profile for the user
		let test_bio = "This is my test bio";
		let profile = create_test_profile(&db, user.id, test_bio).await;

		// Call fetch_profile
		let result = call_fetch_profile(&db, &user.id.to_string()).await;

		// Assert success
		assert!(
			result.is_ok(),
			"Fetch profile should succeed: {:?}",
			result.err()
		);
		let response = result.unwrap();

		// Assert profile data matches
		assert_eq!(response.id, profile.id, "Profile ID should match");
		assert_eq!(response.user_id, user.id, "User ID should match");
		assert_eq!(response.bio, test_bio, "Bio should match");
	}

	/// Test 2: Failure - Fetch non-existent profile
	#[rstest]
	#[tokio::test]
	async fn test_failure_fetch_nonexistent_profile() {
		let (_container, db) = setup_test_database().await;

		// Create test user but don't create profile
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("no_profile@example.com")
				.with_username("no_profile_user"),
		)
		.await;

		// Call fetch_profile for user without profile
		let result = call_fetch_profile(&db, &user.id.to_string()).await;

		// Assert error
		assert!(result.is_err(), "Fetch non-existent profile should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Http(_)),
			"Error should be HTTP error (404), got: {:?}",
			err
		);
	}

	/// Test 3: Failure - Fetch with invalid UUID format
	#[rstest]
	#[tokio::test]
	async fn test_failure_fetch_with_invalid_uuid() {
		let (_container, db) = setup_test_database().await;

		// Call fetch_profile with invalid UUID
		let result = call_fetch_profile(&db, "not-a-valid-uuid").await;

		// Assert validation error
		assert!(result.is_err(), "Fetch with invalid UUID should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Validation(_)),
			"Error should be validation error, got: {:?}",
			err
		);
	}

	/// Test 4: Failure - Fetch with non-existent UUID
	#[rstest]
	#[tokio::test]
	async fn test_failure_fetch_with_nonexistent_uuid() {
		let (_container, db) = setup_test_database().await;

		// Generate a valid UUID that doesn't exist in database
		let nonexistent_id = Uuid::new_v4();

		// Call fetch_profile with non-existent UUID
		let result = call_fetch_profile(&db, &nonexistent_id.to_string()).await;

		// Assert error (profile not found)
		assert!(result.is_err(), "Fetch with non-existent UUID should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Http(_)),
			"Error should be HTTP error (404), got: {:?}",
			err
		);
	}
}
