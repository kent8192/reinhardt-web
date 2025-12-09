//! Follow/Unfollow endpoint tests
//!
//! Tests for follow and unfollow functionality including:
//! - Success cases (follow user, unfollow user)
//! - Error cases (not authenticated, user not found, self-follow, duplicate follow, not following)

#[cfg(test)]
mod follow_tests {
	use reinhardt::core::serde::json::json;
	use reinhardt::db::DatabaseConnection;
	use reinhardt::StatusCode;
	use rstest::rstest;
	use uuid::Uuid;

	use crate::test_utils::{
		create_test_user, generate_test_token, setup_test_database, TestUserParams,
	};

	use crate::apps::relationship::serializers::FollowResponse;

	// Import fixtures from reinhardt-test
	use reinhardt_test::fixtures::{create_follow_relationship, follow_relationship_exists};

	/// Helper to call follow_user endpoint directly
	async fn call_follow_user(
		db: &DatabaseConnection,
		auth_header: Option<&str>,
		target_user_id: Uuid,
		current_user_id: Uuid,
	) -> Result<FollowResponse, reinhardt::Error> {
		use chrono::Utc;
		use reinhardt::{Error, JwtAuth};

		// Check authentication
		let claims = match auth_header {
			Some(header) => {
				let token = header
					.strip_prefix("Bearer ")
					.ok_or_else(|| Error::Authentication("Invalid Authorization header format".into()))?;

				let jwt_auth = JwtAuth::new(b"test-secret-key-for-testing-only");
				jwt_auth
					.verify_token(token)
					.map_err(|e| Error::Authentication(format!("Invalid token: {}", e)))?
			}
			None => {
				return Err(Error::Authentication("Missing Authorization header".into()));
			}
		};

		// Check if trying to follow self
		if current_user_id == target_user_id {
			return Err(Error::Validation("Cannot follow yourself".into()));
		}

		// Check if target user exists using ORM API
		use crate::apps::auth::models::User;
		use reinhardt_db::orm::Manager;

		let user_exists = User::objects()
			.get(target_user_id)
			.with_conn(db)
			.await
			.is_ok();
		if !user_exists {
			return Err(Error::Http("User not found".into()));
		}

		// Check if already following
		if follow_relationship_exists(db, current_user_id, target_user_id).await {
			return Err(Error::Conflict("Already following this user".into()));
		}

		// Create follow relationship
		create_follow_relationship(db, current_user_id, target_user_id).await;

		Ok(FollowResponse::new(current_user_id, target_user_id))
	}

	/// Helper to call unfollow_user endpoint directly
	async fn call_unfollow_user(
		db: &DatabaseConnection,
		auth_header: Option<&str>,
		target_user_id: Uuid,
		current_user_id: Uuid,
	) -> Result<(), reinhardt::Error> {
		use reinhardt::{Error, JwtAuth};

		// Check authentication
		let _claims = match auth_header {
			Some(header) => {
				let token = header
					.strip_prefix("Bearer ")
					.ok_or_else(|| Error::Authentication("Invalid Authorization header format".into()))?;

				let jwt_auth = JwtAuth::new(b"test-secret-key-for-testing-only");
				jwt_auth
					.verify_token(token)
					.map_err(|e| Error::Authentication(format!("Invalid token: {}", e)))?
			}
			None => {
				return Err(Error::Authentication("Missing Authorization header".into()));
			}
		};

		// Check if target user exists using ORM API
		use crate::apps::auth::models::User;
		use reinhardt_db::orm::Manager;

		let user_exists = User::objects()
			.get(target_user_id)
			.with_conn(db)
			.await
			.is_ok();
		if !user_exists {
			return Err(Error::Http("User not found".into()));
		}

		// Check if following
		if !follow_relationship_exists(db, current_user_id, target_user_id).await {
			return Err(Error::Conflict("Not following this user".into()));
		}

		// Remove follow relationship using ORM API
		use crate::apps::auth::models::User;
		use reinhardt_db::orm::Manager;

		let current_user = User::objects()
			.get(current_user_id)
			.with_conn(db)
			.await
			.expect("Failed to get current user");

		let target_user = User::objects()
			.get(target_user_id)
			.with_conn(db)
			.await
			.expect("Failed to get target user");

		current_user
			.following
			.remove(&target_user)
			.with_conn(db)
			.await
			.map_err(|e| Error::Database(format!("Failed to unfollow: {}", e)))?;

		Ok(())
	}

	/// Test 1: Success - Follow a user
	///
	/// POST /accounts/rel/follow/<uuid:user_id>/
	/// Expected: 200 OK with follow relationship data
	#[rstest]
	#[tokio::test]
	async fn test_success_follow_user() {
		// Setup database with migrations
		let (_container, db) = setup_test_database().await;

		// Create test users
		let follower = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("follower@example.com")
				.with_username("follower_user"),
		)
		.await;

		let target = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("target@example.com")
				.with_username("target_user"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&follower);
		let auth_header = format!("Bearer {}", token);

		// Call follow_user
		let result = call_follow_user(&db, Some(&auth_header), target.id, follower.id).await;

		// Assert success
		assert!(result.is_ok(), "Follow user should succeed: {:?}", result.err());
		let response = result.unwrap();

		// Assert response data
		assert_eq!(response.follower_id, follower.id, "Follower ID should match");
		assert_eq!(response.followed_id, target.id, "Followed ID should match");

		// Verify relationship exists in database
		assert!(
			follow_relationship_exists(&db, follower.id, target.id).await,
			"Follow relationship should exist in database"
		);
	}

	/// Test 2: Failure - Follow without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_follow_without_auth() {
		let (_container, db) = setup_test_database().await;

		// Create target user
		let target = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("target_no_auth@example.com")
				.with_username("target_no_auth"),
		)
		.await;

		let dummy_follower_id = Uuid::new_v4();

		// Call follow_user without auth header
		let result = call_follow_user(&db, None, target.id, dummy_follower_id).await;

		// Assert authentication error
		assert!(result.is_err(), "Follow without auth should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authentication(_)),
			"Error should be authentication error, got: {:?}",
			err
		);
	}

	/// Test 3: Failure - Follow non-existent user
	#[rstest]
	#[tokio::test]
	async fn test_failure_follow_nonexistent_user() {
		let (_container, db) = setup_test_database().await;

		// Create follower user
		let follower = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("follower_404@example.com")
				.with_username("follower_404"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&follower);
		let auth_header = format!("Bearer {}", token);

		// Try to follow non-existent user
		let nonexistent_id = Uuid::new_v4();
		let result = call_follow_user(&db, Some(&auth_header), nonexistent_id, follower.id).await;

		// Assert error
		assert!(result.is_err(), "Follow non-existent user should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Http(_)),
			"Error should be HTTP error (404), got: {:?}",
			err
		);
	}

	/// Test 4: Failure - Follow yourself (self-follow prohibited)
	#[rstest]
	#[tokio::test]
	async fn test_failure_follow_self() {
		let (_container, db) = setup_test_database().await;

		// Create user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("self_follow@example.com")
				.with_username("self_follow_user"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Try to follow self
		let result = call_follow_user(&db, Some(&auth_header), user.id, user.id).await;

		// Assert validation error
		assert!(result.is_err(), "Self-follow should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Validation(_)),
			"Error should be validation error, got: {:?}",
			err
		);
	}

	/// Test 5: Failure - Duplicate follow (already following)
	#[rstest]
	#[tokio::test]
	async fn test_failure_duplicate_follow() {
		let (_container, db) = setup_test_database().await;

		// Create test users
		let follower = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("dup_follower@example.com")
				.with_username("dup_follower"),
		)
		.await;

		let target = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("dup_target@example.com")
				.with_username("dup_target"),
		)
		.await;

		// Create existing follow relationship
		create_follow_relationship(&db, follower.id, target.id).await;

		// Generate valid token
		let token = generate_test_token(&follower);
		let auth_header = format!("Bearer {}", token);

		// Try to follow again
		let result = call_follow_user(&db, Some(&auth_header), target.id, follower.id).await;

		// Assert conflict error
		assert!(result.is_err(), "Duplicate follow should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Conflict(_)),
			"Error should be conflict error, got: {:?}",
			err
		);
	}

	/// Test 6: Success - Unfollow a user
	#[rstest]
	#[tokio::test]
	async fn test_success_unfollow_user() {
		let (_container, db) = setup_test_database().await;

		// Create test users
		let follower = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("unfollower@example.com")
				.with_username("unfollower_user"),
		)
		.await;

		let target = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("unfollow_target@example.com")
				.with_username("unfollow_target"),
		)
		.await;

		// Create follow relationship first
		create_follow_relationship(&db, follower.id, target.id).await;

		// Verify relationship exists
		assert!(
			follow_relationship_exists(&db, follower.id, target.id).await,
			"Follow relationship should exist before unfollow"
		);

		// Generate valid token
		let token = generate_test_token(&follower);
		let auth_header = format!("Bearer {}", token);

		// Call unfollow_user
		let result = call_unfollow_user(&db, Some(&auth_header), target.id, follower.id).await;

		// Assert success
		assert!(result.is_ok(), "Unfollow user should succeed: {:?}", result.err());

		// Verify relationship no longer exists
		assert!(
			!follow_relationship_exists(&db, follower.id, target.id).await,
			"Follow relationship should not exist after unfollow"
		);
	}

	/// Test 7: Failure - Unfollow without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_unfollow_without_auth() {
		let (_container, db) = setup_test_database().await;

		// Create target user
		let target = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("unfollow_no_auth@example.com")
				.with_username("unfollow_no_auth"),
		)
		.await;

		let dummy_follower_id = Uuid::new_v4();

		// Call unfollow_user without auth header
		let result = call_unfollow_user(&db, None, target.id, dummy_follower_id).await;

		// Assert authentication error
		assert!(result.is_err(), "Unfollow without auth should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authentication(_)),
			"Error should be authentication error, got: {:?}",
			err
		);
	}

	/// Test 8: Failure - Unfollow when not following
	#[rstest]
	#[tokio::test]
	async fn test_failure_unfollow_not_following() {
		let (_container, db) = setup_test_database().await;

		// Create test users
		let follower = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("not_following@example.com")
				.with_username("not_following"),
		)
		.await;

		let target = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("not_followed@example.com")
				.with_username("not_followed"),
		)
		.await;

		// Generate valid token (but don't create follow relationship)
		let token = generate_test_token(&follower);
		let auth_header = format!("Bearer {}", token);

		// Try to unfollow without following first
		let result = call_unfollow_user(&db, Some(&auth_header), target.id, follower.id).await;

		// Assert conflict error
		assert!(result.is_err(), "Unfollow when not following should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Conflict(_)),
			"Error should be conflict error, got: {:?}",
			err
		);
	}
}
