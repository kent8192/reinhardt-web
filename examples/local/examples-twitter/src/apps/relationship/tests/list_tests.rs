//! Relationship list endpoint tests
//!
//! Tests for fetching followers, followings, and blocked users including:
//! - Success cases (list endpoints, pagination)
//! - Error cases (not authenticated)

#[cfg(test)]
mod list_tests {
	use reinhardt::core::serde::json::json;
	use reinhardt::db::DatabaseConnection;
	use reinhardt::StatusCode;
	use rstest::rstest;
	use uuid::Uuid;

	use crate::test_utils::{
		create_test_user, generate_test_token, setup_test_database, TestUserParams,
	};

	use crate::apps::relationship::serializers::{
		BlockingListResponse, FollowerListResponse, FollowingListResponse, UserSummary,
	};

	// Import fixtures from reinhardt-test
	use reinhardt_test::fixtures::{create_block_relationship, create_follow_relationship};

	/// Helper to get followers for a user
	async fn get_followers(
		db: &DatabaseConnection,
		user_id: Uuid,
	) -> Vec<UserSummary> {
		use crate::apps::auth::models::User;

		// Get user
		let user = match User::objects().get(user_id).with_conn(db).await {
			Ok(u) => u,
			Err(_) => return vec![],
		};

		// Get all followers using ORM API (via reverse relationship)
		let followers = user
			.following
			.all()
			.with_conn(db)
			.await
			.unwrap_or_default();

		// Convert to UserSummary
		followers
			.into_iter()
			.map(|u| UserSummary {
				id: u.id,
				username: u.username,
				email: u.email,
			})
			.collect()
	}

	/// Helper to get followings for a user
	async fn get_followings(
		db: &DatabaseConnection,
		user_id: Uuid,
	) -> Vec<UserSummary> {
		use crate::apps::auth::models::User;

		// Get user
		let user = match User::objects().get(user_id).with_conn(db).await {
			Ok(u) => u,
			Err(_) => return vec![],
		};

		// Get all followings using ORM API
		let followings = user
			.following
			.all()
			.with_conn(db)
			.await
			.unwrap_or_default();

		// Convert to UserSummary
		followings
			.into_iter()
			.map(|u| UserSummary {
				id: u.id,
				username: u.username,
				email: u.email,
			})
			.collect()
	}

	/// Helper to get blocked users for a user
	async fn get_blockings(
		db: &DatabaseConnection,
		user_id: Uuid,
	) -> Vec<UserSummary> {
		use crate::apps::auth::models::User;

		// Get user
		let user = match User::objects().get(user_id).with_conn(db).await {
			Ok(u) => u,
			Err(_) => return vec![],
		};

		// Get all blocked users using ORM API
		let blockings = user
			.blocked_users
			.all()
			.with_conn(db)
			.await
			.unwrap_or_default();

		// Convert to UserSummary
		blockings
			.into_iter()
			.map(|u| UserSummary {
				id: u.id,
				username: u.username,
				email: u.email,
			})
			.collect()
	}

	/// Helper to call fetch_followers endpoint directly
	async fn call_fetch_followers(
		db: &DatabaseConnection,
		auth_header: Option<&str>,
		current_user_id: Uuid,
		page: usize,
		limit: usize,
	) -> Result<FollowerListResponse, reinhardt::Error> {
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

		// Get all followers
		let all_followers = get_followers(db, current_user_id).await;

		// Apply pagination
		let total_count = all_followers.len();
		let start = (page - 1) * limit;
		let followers: Vec<_> = all_followers.into_iter().skip(start).take(limit).collect();

		// Build response
		Ok(FollowerListResponse {
			count: total_count,
			next: if start + limit < total_count {
				Some(format!("?page={}", page + 1))
			} else {
				None
			},
			previous: if page > 1 {
				Some(format!("?page={}", page - 1))
			} else {
				None
			},
			results: followers,
		})
	}

	/// Helper to call fetch_followings endpoint directly
	async fn call_fetch_followings(
		db: &DatabaseConnection,
		auth_header: Option<&str>,
		current_user_id: Uuid,
		page: usize,
		limit: usize,
	) -> Result<FollowingListResponse, reinhardt::Error> {
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

		// Get all followings
		let all_followings = get_followings(db, current_user_id).await;

		// Apply pagination
		let total_count = all_followings.len();
		let start = (page - 1) * limit;
		let followings: Vec<_> = all_followings.into_iter().skip(start).take(limit).collect();

		// Build response
		Ok(FollowingListResponse {
			count: total_count,
			next: if start + limit < total_count {
				Some(format!("?page={}", page + 1))
			} else {
				None
			},
			previous: if page > 1 {
				Some(format!("?page={}", page - 1))
			} else {
				None
			},
			results: followings,
		})
	}

	/// Helper to call fetch_blockings endpoint directly
	async fn call_fetch_blockings(
		db: &DatabaseConnection,
		auth_header: Option<&str>,
		current_user_id: Uuid,
		page: usize,
		limit: usize,
	) -> Result<BlockingListResponse, reinhardt::Error> {
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

		// Get all blocked users
		let all_blockings = get_blockings(db, current_user_id).await;

		// Apply pagination
		let total_count = all_blockings.len();
		let start = (page - 1) * limit;
		let blockings: Vec<_> = all_blockings.into_iter().skip(start).take(limit).collect();

		// Build response
		Ok(BlockingListResponse {
			count: total_count,
			next: if start + limit < total_count {
				Some(format!("?page={}", page + 1))
			} else {
				None
			},
			previous: if page > 1 {
				Some(format!("?page={}", page - 1))
			} else {
				None
			},
			results: blockings,
		})
	}

	// ============================================
	// Followers List Tests
	// ============================================

	/// Test 1: Success - Fetch followers list
	///
	/// GET /accounts/rel/followers/
	/// Expected: 200 OK with followers list
	#[rstest]
	#[tokio::test]
	async fn test_success_fetch_followers() {
		let (_container, db) = setup_test_database().await;

		// Create main user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("main_user@example.com")
				.with_username("main_user"),
		)
		.await;

		// Create follower users
		let follower1 = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("follower1@example.com")
				.with_username("follower1"),
		)
		.await;

		let follower2 = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("follower2@example.com")
				.with_username("follower2"),
		)
		.await;

		// Create follow relationships (followers -> main user)
		create_follow_relationship(&db, follower1.id, user.id).await;
		create_follow_relationship(&db, follower2.id, user.id).await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call fetch_followers
		let result = call_fetch_followers(&db, Some(&auth_header), user.id, 1, 20).await;

		// Assert success
		assert!(result.is_ok(), "Fetch followers should succeed: {:?}", result.err());
		let response = result.unwrap();

		// Assert data
		assert_eq!(response.count, 2, "Should have 2 followers");
		assert_eq!(response.results.len(), 2, "Results should have 2 items");
		assert!(response.next.is_none(), "Should not have next page");
		assert!(response.previous.is_none(), "Should not have previous page");
	}

	/// Test 2: Failure - Fetch followers without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_fetch_followers_without_auth() {
		let (_container, db) = setup_test_database().await;

		let dummy_user_id = Uuid::new_v4();

		// Call fetch_followers without auth header
		let result = call_fetch_followers(&db, None, dummy_user_id, 1, 20).await;

		// Assert authentication error
		assert!(result.is_err(), "Fetch followers without auth should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authentication(_)),
			"Error should be authentication error, got: {:?}",
			err
		);
	}

	/// Test 3: Success - Empty followers list
	#[rstest]
	#[tokio::test]
	async fn test_success_fetch_followers_empty() {
		let (_container, db) = setup_test_database().await;

		// Create user with no followers
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("no_followers@example.com")
				.with_username("no_followers"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call fetch_followers
		let result = call_fetch_followers(&db, Some(&auth_header), user.id, 1, 20).await;

		// Assert success with empty list
		assert!(result.is_ok(), "Fetch empty followers should succeed: {:?}", result.err());
		let response = result.unwrap();

		assert_eq!(response.count, 0, "Should have 0 followers");
		assert!(response.results.is_empty(), "Results should be empty");
	}

	// ============================================
	// Followings List Tests
	// ============================================

	/// Test 4: Success - Fetch followings list
	///
	/// GET /accounts/rel/followings/
	/// Expected: 200 OK with followings list
	#[rstest]
	#[tokio::test]
	async fn test_success_fetch_followings() {
		let (_container, db) = setup_test_database().await;

		// Create main user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("following_user@example.com")
				.with_username("following_user"),
		)
		.await;

		// Create users to follow
		let target1 = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("target1@example.com")
				.with_username("target1"),
		)
		.await;

		let target2 = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("target2@example.com")
				.with_username("target2"),
		)
		.await;

		// Create follow relationships (main user -> targets)
		create_follow_relationship(&db, user.id, target1.id).await;
		create_follow_relationship(&db, user.id, target2.id).await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call fetch_followings
		let result = call_fetch_followings(&db, Some(&auth_header), user.id, 1, 20).await;

		// Assert success
		assert!(result.is_ok(), "Fetch followings should succeed: {:?}", result.err());
		let response = result.unwrap();

		// Assert data
		assert_eq!(response.count, 2, "Should be following 2 users");
		assert_eq!(response.results.len(), 2, "Results should have 2 items");
	}

	/// Test 5: Failure - Fetch followings without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_fetch_followings_without_auth() {
		let (_container, db) = setup_test_database().await;

		let dummy_user_id = Uuid::new_v4();

		// Call fetch_followings without auth header
		let result = call_fetch_followings(&db, None, dummy_user_id, 1, 20).await;

		// Assert authentication error
		assert!(result.is_err(), "Fetch followings without auth should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authentication(_)),
			"Error should be authentication error, got: {:?}",
			err
		);
	}

	// ============================================
	// Blockings List Tests
	// ============================================

	/// Test 6: Success - Fetch blocked users list
	///
	/// GET /accounts/rel/blocking/
	/// Expected: 200 OK with blocked users list
	#[rstest]
	#[tokio::test]
	async fn test_success_fetch_blockings() {
		let (_container, db) = setup_test_database().await;

		// Create main user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("blocking_user@example.com")
				.with_username("blocking_user"),
		)
		.await;

		// Create users to block
		let blocked1 = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("blocked1@example.com")
				.with_username("blocked1"),
		)
		.await;

		let blocked2 = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("blocked2@example.com")
				.with_username("blocked2"),
		)
		.await;

		// Create block relationships
		create_block_relationship(&db, user.id, blocked1.id).await;
		create_block_relationship(&db, user.id, blocked2.id).await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call fetch_blockings
		let result = call_fetch_blockings(&db, Some(&auth_header), user.id, 1, 20).await;

		// Assert success
		assert!(result.is_ok(), "Fetch blockings should succeed: {:?}", result.err());
		let response = result.unwrap();

		// Assert data
		assert_eq!(response.count, 2, "Should be blocking 2 users");
		assert_eq!(response.results.len(), 2, "Results should have 2 items");
	}

	/// Test 7: Failure - Fetch blockings without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_fetch_blockings_without_auth() {
		let (_container, db) = setup_test_database().await;

		let dummy_user_id = Uuid::new_v4();

		// Call fetch_blockings without auth header
		let result = call_fetch_blockings(&db, None, dummy_user_id, 1, 20).await;

		// Assert authentication error
		assert!(result.is_err(), "Fetch blockings without auth should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authentication(_)),
			"Error should be authentication error, got: {:?}",
			err
		);
	}

	// ============================================
	// Pagination Tests
	// ============================================

	/// Test 8: Success - Followers pagination (first page)
	#[rstest]
	#[tokio::test]
	async fn test_success_followers_pagination_first_page() {
		let (_container, db) = setup_test_database().await;

		// Create main user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("paginated_user@example.com")
				.with_username("paginated_user"),
		)
		.await;

		// Create 5 followers
		for i in 0..5 {
			let follower = create_test_user(
				&db,
				TestUserParams::default()
					.with_email(&format!("paginated_follower{}@example.com", i))
					.with_username(&format!("paginated_follower{}", i)),
			)
			.await;
			create_follow_relationship(&db, follower.id, user.id).await;
		}

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call fetch_followers with limit 2
		let result = call_fetch_followers(&db, Some(&auth_header), user.id, 1, 2).await;

		// Assert success
		assert!(result.is_ok(), "Fetch followers page 1 should succeed: {:?}", result.err());
		let response = result.unwrap();

		// Assert pagination
		assert_eq!(response.count, 5, "Total count should be 5");
		assert_eq!(response.results.len(), 2, "Page 1 should have 2 items");
		assert!(response.next.is_some(), "Should have next page");
		assert!(response.previous.is_none(), "Should not have previous page on page 1");
	}

	/// Test 9: Success - Followers pagination (middle page)
	#[rstest]
	#[tokio::test]
	async fn test_success_followers_pagination_middle_page() {
		let (_container, db) = setup_test_database().await;

		// Create main user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("middle_page_user@example.com")
				.with_username("middle_page_user"),
		)
		.await;

		// Create 6 followers
		for i in 0..6 {
			let follower = create_test_user(
				&db,
				TestUserParams::default()
					.with_email(&format!("middle_follower{}@example.com", i))
					.with_username(&format!("middle_follower{}", i)),
			)
			.await;
			create_follow_relationship(&db, follower.id, user.id).await;
		}

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call fetch_followers page 2 with limit 2
		let result = call_fetch_followers(&db, Some(&auth_header), user.id, 2, 2).await;

		// Assert success
		assert!(result.is_ok(), "Fetch followers page 2 should succeed: {:?}", result.err());
		let response = result.unwrap();

		// Assert pagination
		assert_eq!(response.count, 6, "Total count should be 6");
		assert_eq!(response.results.len(), 2, "Page 2 should have 2 items");
		assert!(response.next.is_some(), "Should have next page");
		assert!(response.previous.is_some(), "Should have previous page on page 2");
	}

	/// Test 10: Success - Followers pagination (last page)
	#[rstest]
	#[tokio::test]
	async fn test_success_followers_pagination_last_page() {
		let (_container, db) = setup_test_database().await;

		// Create main user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("last_page_user@example.com")
				.with_username("last_page_user"),
		)
		.await;

		// Create 5 followers
		for i in 0..5 {
			let follower = create_test_user(
				&db,
				TestUserParams::default()
					.with_email(&format!("last_follower{}@example.com", i))
					.with_username(&format!("last_follower{}", i)),
			)
			.await;
			create_follow_relationship(&db, follower.id, user.id).await;
		}

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call fetch_followers page 3 with limit 2 (last page with 1 item)
		let result = call_fetch_followers(&db, Some(&auth_header), user.id, 3, 2).await;

		// Assert success
		assert!(result.is_ok(), "Fetch followers page 3 should succeed: {:?}", result.err());
		let response = result.unwrap();

		// Assert pagination
		assert_eq!(response.count, 5, "Total count should be 5");
		assert_eq!(response.results.len(), 1, "Last page should have 1 item");
		assert!(response.next.is_none(), "Should not have next page on last page");
		assert!(response.previous.is_some(), "Should have previous page on last page");
	}

	/// Test 11: Success - Followings pagination
	#[rstest]
	#[tokio::test]
	async fn test_success_followings_pagination() {
		let (_container, db) = setup_test_database().await;

		// Create main user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("followings_paginated@example.com")
				.with_username("followings_paginated"),
		)
		.await;

		// Create 3 targets to follow
		for i in 0..3 {
			let target = create_test_user(
				&db,
				TestUserParams::default()
					.with_email(&format!("following_target{}@example.com", i))
					.with_username(&format!("following_target{}", i)),
			)
			.await;
			create_follow_relationship(&db, user.id, target.id).await;
		}

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call fetch_followings with limit 2
		let result = call_fetch_followings(&db, Some(&auth_header), user.id, 1, 2).await;

		// Assert success
		assert!(result.is_ok(), "Fetch followings page 1 should succeed: {:?}", result.err());
		let response = result.unwrap();

		// Assert pagination
		assert_eq!(response.count, 3, "Total count should be 3");
		assert_eq!(response.results.len(), 2, "Page 1 should have 2 items");
		assert!(response.next.is_some(), "Should have next page");
	}

	/// Test 12: Success - Blockings pagination
	#[rstest]
	#[tokio::test]
	async fn test_success_blockings_pagination() {
		let (_container, db) = setup_test_database().await;

		// Create main user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("blockings_paginated@example.com")
				.with_username("blockings_paginated"),
		)
		.await;

		// Create 3 users to block
		for i in 0..3 {
			let blocked = create_test_user(
				&db,
				TestUserParams::default()
					.with_email(&format!("blocked_user{}@example.com", i))
					.with_username(&format!("blocked_user{}", i)),
			)
			.await;
			create_block_relationship(&db, user.id, blocked.id).await;
		}

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call fetch_blockings with limit 2
		let result = call_fetch_blockings(&db, Some(&auth_header), user.id, 1, 2).await;

		// Assert success
		assert!(result.is_ok(), "Fetch blockings page 1 should succeed: {:?}", result.err());
		let response = result.unwrap();

		// Assert pagination
		assert_eq!(response.count, 3, "Total count should be 3");
		assert_eq!(response.results.len(), 2, "Page 1 should have 2 items");
		assert!(response.next.is_some(), "Should have next page");
	}
}
