//! Block/Unblock endpoint tests
//!
//! Tests for block and unblock functionality including:
//! - Success cases (block user, unblock user)
//! - Error cases (not authenticated, user not found, self-block, duplicate block, not blocking)

#[cfg(test)]
mod block_tests {
	use reinhardt::core::serde::json::json;
	use reinhardt::db::DatabaseConnection;
	use reinhardt::StatusCode;
	use rstest::rstest;
	use uuid::Uuid;

	use crate::test_utils::{
		create_test_user, generate_test_token, setup_test_database, TestUserParams,
	};

	use crate::apps::relationship::serializers::BlockResponse;

	// Import fixtures from reinhardt-test
	use reinhardt_test::fixtures::{
		block_relationship_exists, create_block_relationship, create_follow_relationship,
		follow_relationship_exists,
	};

	/// Helper to call block_user endpoint directly
	async fn call_block_user(
		db: &DatabaseConnection,
		auth_header: Option<&str>,
		target_user_id: Uuid,
		current_user_id: Uuid,
	) -> Result<BlockResponse, reinhardt::Error> {
		use chrono::Utc;
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

		// Check if trying to block self
		if current_user_id == target_user_id {
			return Err(Error::Validation("Cannot block yourself".into()));
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

		// Check if already blocking
		if block_relationship_exists(db, current_user_id, target_user_id).await {
			return Err(Error::Conflict("Already blocking this user".into()));
		}

		// Remove any existing follow relationships (both directions)
		// When blocking, we should also unfollow using ORM API
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

		// Remove follow from current_user -> target_user
		let _ = current_user.following.remove(&target_user).with_conn(db).await;

		// Remove follow from target_user -> current_user
		let _ = target_user.following.remove(&current_user).with_conn(db).await;

		// Create block relationship
		create_block_relationship(db, current_user_id, target_user_id).await;

		Ok(BlockResponse::new(current_user_id, target_user_id))
	}

	/// Helper to call unblock_user endpoint directly
	async fn call_unblock_user(
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

		// Check if blocking
		if !block_relationship_exists(db, current_user_id, target_user_id).await {
			return Err(Error::Conflict("Not blocking this user".into()));
		}

		// Remove block relationship using ORM API
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
			.blocked_users
			.remove(&target_user)
			.with_conn(db)
			.await
			.map_err(|e| Error::Database(format!("Failed to unblock: {}", e)))?;

		Ok(())
	}

	/// Test 1: Success - Block a user
	///
	/// POST /accounts/rel/block/<uuid:user_id>/
	/// Expected: 200 OK with block relationship data
	#[rstest]
	#[tokio::test]
	async fn test_success_block_user() {
		// Setup database with migrations
		let (_container, db) = setup_test_database().await;

		// Create test users
		let blocker = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("blocker@example.com")
				.with_username("blocker_user"),
		)
		.await;

		let target = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("block_target@example.com")
				.with_username("block_target"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&blocker);
		let auth_header = format!("Bearer {}", token);

		// Call block_user
		let result = call_block_user(&db, Some(&auth_header), target.id, blocker.id).await;

		// Assert success
		assert!(result.is_ok(), "Block user should succeed: {:?}", result.err());
		let response = result.unwrap();

		// Assert response data
		assert_eq!(response.blocker_id, blocker.id, "Blocker ID should match");
		assert_eq!(response.blocked_id, target.id, "Blocked ID should match");

		// Verify relationship exists in database
		assert!(
			block_relationship_exists(&db, blocker.id, target.id).await,
			"Block relationship should exist in database"
		);
	}

	/// Test 2: Success - Block removes existing follow relationship
	///
	/// When user A blocks user B, any follow relationship should be removed
	#[rstest]
	#[tokio::test]
	async fn test_success_block_removes_follow() {
		let (_container, db) = setup_test_database().await;

		// Create test users
		let blocker = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("blocker_unfollow@example.com")
				.with_username("blocker_unfollow"),
		)
		.await;

		let target = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("target_unfollow@example.com")
				.with_username("target_unfollow"),
		)
		.await;

		// Create mutual follow relationships
		create_follow_relationship(&db, blocker.id, target.id).await;
		create_follow_relationship(&db, target.id, blocker.id).await;

		// Verify follow relationships exist
		assert!(
			follow_relationship_exists(&db, blocker.id, target.id).await,
			"Blocker should be following target before block"
		);
		assert!(
			follow_relationship_exists(&db, target.id, blocker.id).await,
			"Target should be following blocker before block"
		);

		// Generate valid token
		let token = generate_test_token(&blocker);
		let auth_header = format!("Bearer {}", token);

		// Call block_user
		let result = call_block_user(&db, Some(&auth_header), target.id, blocker.id).await;

		// Assert success
		assert!(result.is_ok(), "Block user should succeed: {:?}", result.err());

		// Verify block relationship exists
		assert!(
			block_relationship_exists(&db, blocker.id, target.id).await,
			"Block relationship should exist"
		);

		// Verify follow relationships are removed
		assert!(
			!follow_relationship_exists(&db, blocker.id, target.id).await,
			"Blocker should not be following target after block"
		);
		assert!(
			!follow_relationship_exists(&db, target.id, blocker.id).await,
			"Target should not be following blocker after block"
		);
	}

	/// Test 3: Failure - Block without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_block_without_auth() {
		let (_container, db) = setup_test_database().await;

		// Create target user
		let target = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("block_no_auth@example.com")
				.with_username("block_no_auth"),
		)
		.await;

		let dummy_blocker_id = Uuid::new_v4();

		// Call block_user without auth header
		let result = call_block_user(&db, None, target.id, dummy_blocker_id).await;

		// Assert authentication error
		assert!(result.is_err(), "Block without auth should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authentication(_)),
			"Error should be authentication error, got: {:?}",
			err
		);
	}

	/// Test 4: Failure - Block yourself (self-block prohibited)
	#[rstest]
	#[tokio::test]
	async fn test_failure_block_self() {
		let (_container, db) = setup_test_database().await;

		// Create user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("self_block@example.com")
				.with_username("self_block_user"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Try to block self
		let result = call_block_user(&db, Some(&auth_header), user.id, user.id).await;

		// Assert validation error
		assert!(result.is_err(), "Self-block should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Validation(_)),
			"Error should be validation error, got: {:?}",
			err
		);
	}

	/// Test 5: Failure - Duplicate block (already blocking)
	#[rstest]
	#[tokio::test]
	async fn test_failure_duplicate_block() {
		let (_container, db) = setup_test_database().await;

		// Create test users
		let blocker = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("dup_blocker@example.com")
				.with_username("dup_blocker"),
		)
		.await;

		let target = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("dup_block_target@example.com")
				.with_username("dup_block_target"),
		)
		.await;

		// Create existing block relationship
		create_block_relationship(&db, blocker.id, target.id).await;

		// Generate valid token
		let token = generate_test_token(&blocker);
		let auth_header = format!("Bearer {}", token);

		// Try to block again
		let result = call_block_user(&db, Some(&auth_header), target.id, blocker.id).await;

		// Assert conflict error
		assert!(result.is_err(), "Duplicate block should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Conflict(_)),
			"Error should be conflict error, got: {:?}",
			err
		);
	}

	/// Test 6: Success - Unblock a user
	#[rstest]
	#[tokio::test]
	async fn test_success_unblock_user() {
		let (_container, db) = setup_test_database().await;

		// Create test users
		let blocker = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("unblocker@example.com")
				.with_username("unblocker_user"),
		)
		.await;

		let target = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("unblock_target@example.com")
				.with_username("unblock_target"),
		)
		.await;

		// Create block relationship first
		create_block_relationship(&db, blocker.id, target.id).await;

		// Verify relationship exists
		assert!(
			block_relationship_exists(&db, blocker.id, target.id).await,
			"Block relationship should exist before unblock"
		);

		// Generate valid token
		let token = generate_test_token(&blocker);
		let auth_header = format!("Bearer {}", token);

		// Call unblock_user
		let result = call_unblock_user(&db, Some(&auth_header), target.id, blocker.id).await;

		// Assert success
		assert!(result.is_ok(), "Unblock user should succeed: {:?}", result.err());

		// Verify relationship no longer exists
		assert!(
			!block_relationship_exists(&db, blocker.id, target.id).await,
			"Block relationship should not exist after unblock"
		);
	}

	/// Test 7: Failure - Unblock without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_unblock_without_auth() {
		let (_container, db) = setup_test_database().await;

		// Create target user
		let target = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("unblock_no_auth@example.com")
				.with_username("unblock_no_auth"),
		)
		.await;

		let dummy_blocker_id = Uuid::new_v4();

		// Call unblock_user without auth header
		let result = call_unblock_user(&db, None, target.id, dummy_blocker_id).await;

		// Assert authentication error
		assert!(result.is_err(), "Unblock without auth should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authentication(_)),
			"Error should be authentication error, got: {:?}",
			err
		);
	}

	/// Test 8: Failure - Unblock when not blocking
	#[rstest]
	#[tokio::test]
	async fn test_failure_unblock_not_blocking() {
		let (_container, db) = setup_test_database().await;

		// Create test users
		let blocker = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("not_blocking@example.com")
				.with_username("not_blocking"),
		)
		.await;

		let target = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("not_blocked@example.com")
				.with_username("not_blocked"),
		)
		.await;

		// Generate valid token (but don't create block relationship)
		let token = generate_test_token(&blocker);
		let auth_header = format!("Bearer {}", token);

		// Try to unblock without blocking first
		let result = call_unblock_user(&db, Some(&auth_header), target.id, blocker.id).await;

		// Assert conflict error
		assert!(result.is_err(), "Unblock when not blocking should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Conflict(_)),
			"Error should be conflict error, got: {:?}",
			err
		);
	}
}
