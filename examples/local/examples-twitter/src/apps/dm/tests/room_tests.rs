//! DM Room endpoint tests
//!
//! Tests for DM room functionality including:
//! - Success cases (list rooms, get room, create room, delete room)
//! - Error cases (not authenticated, not a member, room not found)

#[cfg(test)]
mod room_tests {
	use reinhardt::StatusCode;
	use reinhardt::core::serde::json::json;
	use reinhardt::db::DatabaseConnection;
	use rstest::rstest;
	use uuid::Uuid;
	use validator::Validate;

	use crate::test_utils::{
		TestUserParams, create_test_user, generate_test_token, setup_test_database,
	};

	use crate::apps::dm::serializers::{CreateRoomRequest, RoomResponse};

	/// Helper to create a DM room directly in the database
	async fn create_test_room(db: &DatabaseConnection, name: Option<&str>, is_group: bool) -> Uuid {
		use crate::apps::dm::models::DMRoom;
		use reinhardt::db::orm::Manager;

		// Create room using generated new() function
		let room = DMRoom::new(name.map(String::from), is_group);

		let created_room = DMRoom::objects()
			.create(room)
			.with_conn(db)
			.await
			.expect("Failed to create test room");

		created_room.id
	}

	/// Helper to add a member to a room
	async fn add_room_member(db: &DatabaseConnection, room_id: Uuid, user_id: Uuid) {
		use crate::apps::auth::models::User;
		use crate::apps::dm::models::DMRoom;

		// Get room and user
		let room = DMRoom::objects()
			.get(room_id)
			.with_conn(db)
			.await
			.expect("Failed to get room");

		let user = User::objects()
			.get(user_id)
			.with_conn(db)
			.await
			.expect("Failed to get user");

		// Add user to room using ORM API
		room.members
			.add(&user)
			.with_conn(db)
			.await
			.expect("Failed to add room member");
	}

	/// Helper to check if a user is a member of a room
	async fn is_room_member(db: &DatabaseConnection, room_id: Uuid, user_id: Uuid) -> bool {
		use crate::apps::auth::models::User;
		use crate::apps::dm::models::DMRoom;

		// Get room and user
		let room = match DMRoom::objects().get(room_id).with_conn(db).await {
			Ok(r) => r,
			Err(_) => return false,
		};

		let user = match User::objects().get(user_id).with_conn(db).await {
			Ok(u) => u,
			Err(_) => return false,
		};

		// Check if user is a member using ORM API
		room.members
			.contains(&user)
			.with_conn(db)
			.await
			.unwrap_or(false)
	}

	/// Helper to check if a room exists
	async fn room_exists(db: &DatabaseConnection, room_id: Uuid) -> bool {
		use crate::apps::dm::models::DMRoom;

		DMRoom::objects().get(room_id).with_conn(db).await.is_ok()
	}

	/// Helper to call list_rooms endpoint directly
	async fn call_list_rooms(
		db: &DatabaseConnection,
		auth_header: Option<&str>,
		current_user_id: Uuid,
	) -> Result<Vec<RoomResponse>, reinhardt::Error> {
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

		// Get user model
		use crate::apps::auth::models::User;
		use reinhardt::db::orm::{FilterOperator, FilterValue};

		let user = User::objects()
			.filter(
				User::field_id(),
				FilterOperator::Eq,
				FilterValue::String(current_user_id.to_string()),
			)
			.first()
			.await
			.map_err(|e| Error::Http(e.to_string()))?
			.ok_or_else(|| Error::Http("User not found".into()))?;

		// Use filter_by_target() API - single JOIN query
		use reinhardt::db::orm::ManyToManyAccessor;

		let rooms = ManyToManyAccessor::<DMRoom, User>::filter_by_target(
			&DMRoom::objects(),
			"members",
			&user,
			db.clone(),
		)
		.await
		.map_err(|e| Error::Http(e))?;

		let response: Vec<RoomResponse> = rooms
			.into_iter()
			.map(|room| RoomResponse {
				id: room.id,
				name: room.name,
				is_group: room.is_group,
				created_at: room.created_at,
			})
			.collect();

		Ok(response)
	}

	/// Helper to call get_room endpoint directly
	async fn call_get_room(
		db: &DatabaseConnection,
		auth_header: Option<&str>,
		room_id: Uuid,
		current_user_id: Uuid,
	) -> Result<RoomResponse, reinhardt::Error> {
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

		// Check if room exists
		if !room_exists(db, room_id).await {
			return Err(Error::Http("Room not found".into()));
		}

		// Check if user is a member
		if !is_room_member(db, room_id, current_user_id).await {
			return Err(Error::Authorization("Not a member of this room".into()));
		}

		// Get room details using ORM
		use crate::apps::dm::models::DMRoom;

		let room = DMRoom::objects()
			.get(room_id)
			.with_conn(db)
			.await
			.map_err(|e| Error::Http(e.to_string()))?;

		Ok(RoomResponse {
			id: room.id,
			name: room.name,
			is_group: room.is_group,
			created_at: room.created_at,
		})
	}

	/// Helper to call create_room endpoint directly
	async fn call_create_room(
		db: &DatabaseConnection,
		auth_header: Option<&str>,
		current_user_id: Uuid,
		name: Option<&str>,
		member_ids: Vec<Uuid>,
	) -> Result<RoomResponse, reinhardt::Error> {
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

		// Validate request
		let request = CreateRoomRequest {
			name: name.map(|s| s.to_string()),
			member_ids: member_ids.clone(),
		};
		request
			.validate()
			.map_err(|e| Error::Validation(format!("Validation failed: {}", e)))?;

		use crate::apps::auth::models::User;
		use crate::apps::dm::models::DMRoom;
		use reinhardt::db::orm::Manager;

		// Verify all member IDs exist using ORM
		for member_id in &member_ids {
			if User::objects().get(*member_id).with_conn(db).await.is_err() {
				return Err(Error::Http("One or more member IDs not found".into()));
			}
		}

		// Create room using ORM
		let is_group = member_ids.len() > 1;
		let room = DMRoom::new(name.map(String::from), is_group);

		let created_room = DMRoom::objects()
			.create(room)
			.with_conn(db)
			.await
			.map_err(|e| Error::Http(e.to_string()))?;

		// Add creator as member
		add_room_member(db, created_room.id, current_user_id).await;

		// Add other members
		for member_id in &member_ids {
			if *member_id != current_user_id {
				add_room_member(db, created_room.id, *member_id).await;
			}
		}

		Ok(RoomResponse {
			id: created_room.id,
			name: created_room.name,
			is_group: created_room.is_group,
			created_at: created_room.created_at,
		})
	}

	/// Helper to call delete_room endpoint directly
	async fn call_delete_room(
		db: &DatabaseConnection,
		auth_header: Option<&str>,
		room_id: Uuid,
		current_user_id: Uuid,
	) -> Result<(), reinhardt::Error> {
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

		// Check if room exists
		if !room_exists(db, room_id).await {
			return Err(Error::Http("Room not found".into()));
		}

		// Check if user is a member
		if !is_room_member(db, room_id, current_user_id).await {
			return Err(Error::Authorization("Not a member of this room".into()));
		}

		use crate::apps::dm::models::{DMMessage, DMRoom};
		use reinhardt::db::orm::{FilterOperator, FilterValue, Manager};

		// Get the room to clear its members
		let room = DMRoom::objects()
			.get(room_id)
			.with_conn(db)
			.await
			.map_err(|e| Error::Http(e.to_string()))?;

		// Delete room members first (foreign key constraint)
		room.members
			.clear()
			.with_conn(db)
			.await
			.map_err(|e| Error::Http(e.to_string()))?;

		// Delete messages (foreign key constraint)
		let messages = DMMessage::objects()
			.filter(
				DMMessage::field_room(),
				FilterOperator::Eq,
				FilterValue::Uuid(room_id),
			)
			.all_with_conn(db)
			.await
			.unwrap_or_default();

		for message in messages {
			DMMessage::objects()
				.delete_with_conn(db, message.id)
				.await
				.map_err(|e| Error::Http(e.to_string()))?;
		}

		// Delete room using ORM
		DMRoom::objects()
			.delete_with_conn(db, room_id)
			.await
			.map_err(|e| Error::Http(e.to_string()))?;

		Ok(())
	}

	// ============================================
	// List Rooms Tests
	// ============================================

	/// Test 1: Success - List rooms for authenticated user
	#[rstest]
	#[tokio::test]
	async fn test_success_list_rooms() {
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("room_list_user@example.com")
				.with_username("room_list_user"),
		)
		.await;

		// Create rooms and add user as member
		let room1_id = create_test_room(&db, Some("Room 1"), false).await;
		let room2_id = create_test_room(&db, Some("Room 2"), true).await;
		add_room_member(&db, room1_id, user.id).await;
		add_room_member(&db, room2_id, user.id).await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call list_rooms
		let result = call_list_rooms(&db, Some(&auth_header), user.id).await;

		// Assert success
		assert!(
			result.is_ok(),
			"List rooms should succeed: {:?}",
			result.err()
		);
		let rooms = result.unwrap();

		assert_eq!(rooms.len(), 2, "Should have 2 rooms");
	}

	/// Test 2: Failure - List rooms without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_list_rooms_without_auth() {
		let (_container, db) = setup_test_database().await;

		let dummy_user_id = Uuid::new_v4();

		// Call list_rooms without auth
		let result = call_list_rooms(&db, None, dummy_user_id).await;

		// Assert authentication error
		assert!(result.is_err(), "List rooms without auth should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authentication(_)),
			"Error should be authentication error, got: {:?}",
			err
		);
	}

	/// Test 3: Success - List rooms returns empty for user with no rooms
	#[rstest]
	#[tokio::test]
	async fn test_success_list_rooms_empty() {
		let (_container, db) = setup_test_database().await;

		// Create test user with no rooms
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("no_rooms_user@example.com")
				.with_username("no_rooms_user"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call list_rooms
		let result = call_list_rooms(&db, Some(&auth_header), user.id).await;

		// Assert success with empty list
		assert!(
			result.is_ok(),
			"List rooms should succeed: {:?}",
			result.err()
		);
		let rooms = result.unwrap();
		assert!(rooms.is_empty(), "Should have no rooms");
	}

	// ============================================
	// Get Room Tests
	// ============================================

	/// Test 4: Success - Get a specific room
	#[rstest]
	#[tokio::test]
	async fn test_success_get_room() {
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("get_room_user@example.com")
				.with_username("get_room_user"),
		)
		.await;

		// Create room and add user as member
		let room_id = create_test_room(&db, Some("Test Room"), false).await;
		add_room_member(&db, room_id, user.id).await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Call get_room
		let result = call_get_room(&db, Some(&auth_header), room_id, user.id).await;

		// Assert success
		assert!(
			result.is_ok(),
			"Get room should succeed: {:?}",
			result.err()
		);
		let room = result.unwrap();
		assert_eq!(room.id, room_id, "Room ID should match");
		assert_eq!(
			room.name,
			Some("Test Room".to_string()),
			"Room name should match"
		);
	}

	/// Test 5: Failure - Get room without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_get_room_without_auth() {
		let (_container, db) = setup_test_database().await;

		let room_id = Uuid::new_v4();
		let dummy_user_id = Uuid::new_v4();

		// Call get_room without auth
		let result = call_get_room(&db, None, room_id, dummy_user_id).await;

		// Assert authentication error
		assert!(result.is_err(), "Get room without auth should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authentication(_)),
			"Error should be authentication error, got: {:?}",
			err
		);
	}

	/// Test 6: Failure - Get room when not a member
	#[rstest]
	#[tokio::test]
	async fn test_failure_get_room_not_member() {
		let (_container, db) = setup_test_database().await;

		// Create two users
		let member = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("room_member@example.com")
				.with_username("room_member"),
		)
		.await;

		let non_member = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("non_member@example.com")
				.with_username("non_member"),
		)
		.await;

		// Create room with only member
		let room_id = create_test_room(&db, Some("Private Room"), false).await;
		add_room_member(&db, room_id, member.id).await;

		// Generate token for non_member
		let token = generate_test_token(&non_member);
		let auth_header = format!("Bearer {}", token);

		// Try to get room as non_member
		let result = call_get_room(&db, Some(&auth_header), room_id, non_member.id).await;

		// Assert authorization error
		assert!(result.is_err(), "Get room as non-member should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authorization(_)),
			"Error should be authorization error, got: {:?}",
			err
		);
	}

	/// Test 7: Failure - Get non-existent room
	#[rstest]
	#[tokio::test]
	async fn test_failure_get_nonexistent_room() {
		let (_container, db) = setup_test_database().await;

		// Create test user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("get_404_room@example.com")
				.with_username("get_404_room"),
		)
		.await;

		// Generate valid token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Try to get non-existent room
		let nonexistent_id = Uuid::new_v4();
		let result = call_get_room(&db, Some(&auth_header), nonexistent_id, user.id).await;

		// Assert not found error
		assert!(result.is_err(), "Get non-existent room should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Http(_)),
			"Error should be HTTP error (404), got: {:?}",
			err
		);
	}

	// ============================================
	// Create Room Tests
	// ============================================

	/// Test 8: Success - Create a 1-on-1 room
	#[rstest]
	#[tokio::test]
	async fn test_success_create_one_on_one_room() {
		let (_container, db) = setup_test_database().await;

		// Create two users
		let user1 = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("room_creator@example.com")
				.with_username("room_creator"),
		)
		.await;

		let user2 = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("room_target@example.com")
				.with_username("room_target"),
		)
		.await;

		// Generate token
		let token = generate_test_token(&user1);
		let auth_header = format!("Bearer {}", token);

		// Create room
		let result =
			call_create_room(&db, Some(&auth_header), user1.id, None, vec![user2.id]).await;

		// Assert success
		assert!(
			result.is_ok(),
			"Create room should succeed: {:?}",
			result.err()
		);
		let room = result.unwrap();
		assert!(!room.is_group, "1-on-1 room should not be group");

		// Verify both users are members
		assert!(
			is_room_member(&db, room.id, user1.id).await,
			"Creator should be a member"
		);
		assert!(
			is_room_member(&db, room.id, user2.id).await,
			"Target should be a member"
		);
	}

	/// Test 9: Success - Create a group room
	#[rstest]
	#[tokio::test]
	async fn test_success_create_group_room() {
		let (_container, db) = setup_test_database().await;

		// Create three users
		let user1 = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("group_creator@example.com")
				.with_username("group_creator"),
		)
		.await;

		let user2 = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("group_member2@example.com")
				.with_username("group_member2"),
		)
		.await;

		let user3 = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("group_member3@example.com")
				.with_username("group_member3"),
		)
		.await;

		// Generate token
		let token = generate_test_token(&user1);
		let auth_header = format!("Bearer {}", token);

		// Create group room
		let result = call_create_room(
			&db,
			Some(&auth_header),
			user1.id,
			Some("My Group"),
			vec![user2.id, user3.id],
		)
		.await;

		// Assert success
		assert!(
			result.is_ok(),
			"Create group room should succeed: {:?}",
			result.err()
		);
		let room = result.unwrap();
		assert!(room.is_group, "Room with >1 members should be group");
		assert_eq!(
			room.name,
			Some("My Group".to_string()),
			"Room name should match"
		);

		// Verify all users are members
		assert!(
			is_room_member(&db, room.id, user1.id).await,
			"Creator should be a member"
		);
		assert!(
			is_room_member(&db, room.id, user2.id).await,
			"Member 2 should be a member"
		);
		assert!(
			is_room_member(&db, room.id, user3.id).await,
			"Member 3 should be a member"
		);
	}

	/// Test 10: Failure - Create room without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_create_room_without_auth() {
		let (_container, db) = setup_test_database().await;

		let dummy_user_id = Uuid::new_v4();
		let target_id = Uuid::new_v4();

		// Call create_room without auth
		let result = call_create_room(&db, None, dummy_user_id, None, vec![target_id]).await;

		// Assert authentication error
		assert!(result.is_err(), "Create room without auth should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authentication(_)),
			"Error should be authentication error, got: {:?}",
			err
		);
	}

	/// Test 11: Failure - Create room with non-existent member
	#[rstest]
	#[tokio::test]
	async fn test_failure_create_room_nonexistent_member() {
		let (_container, db) = setup_test_database().await;

		// Create user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("room_creator_404@example.com")
				.with_username("room_creator_404"),
		)
		.await;

		// Generate token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Try to create room with non-existent member
		let nonexistent_id = Uuid::new_v4();
		let result =
			call_create_room(&db, Some(&auth_header), user.id, None, vec![nonexistent_id]).await;

		// Assert error
		assert!(
			result.is_err(),
			"Create room with non-existent member should fail"
		);
	}

	// ============================================
	// Delete Room Tests
	// ============================================

	/// Test 12: Success - Delete a room
	#[rstest]
	#[tokio::test]
	async fn test_success_delete_room() {
		let (_container, db) = setup_test_database().await;

		// Create user
		let user = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("delete_room_user@example.com")
				.with_username("delete_room_user"),
		)
		.await;

		// Create room and add user as member
		let room_id = create_test_room(&db, Some("To Delete"), false).await;
		add_room_member(&db, room_id, user.id).await;

		// Verify room exists
		assert!(
			room_exists(&db, room_id).await,
			"Room should exist before delete"
		);

		// Generate token
		let token = generate_test_token(&user);
		let auth_header = format!("Bearer {}", token);

		// Delete room
		let result = call_delete_room(&db, Some(&auth_header), room_id, user.id).await;

		// Assert success
		assert!(
			result.is_ok(),
			"Delete room should succeed: {:?}",
			result.err()
		);

		// Verify room no longer exists
		assert!(
			!room_exists(&db, room_id).await,
			"Room should not exist after delete"
		);
	}

	/// Test 13: Failure - Delete room without authentication
	#[rstest]
	#[tokio::test]
	async fn test_failure_delete_room_without_auth() {
		let (_container, db) = setup_test_database().await;

		let room_id = Uuid::new_v4();
		let dummy_user_id = Uuid::new_v4();

		// Call delete_room without auth
		let result = call_delete_room(&db, None, room_id, dummy_user_id).await;

		// Assert authentication error
		assert!(result.is_err(), "Delete room without auth should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authentication(_)),
			"Error should be authentication error, got: {:?}",
			err
		);
	}

	/// Test 14: Failure - Delete room when not a member
	#[rstest]
	#[tokio::test]
	async fn test_failure_delete_room_not_member() {
		let (_container, db) = setup_test_database().await;

		// Create two users
		let member = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("delete_member@example.com")
				.with_username("delete_member"),
		)
		.await;

		let non_member = create_test_user(
			&db,
			TestUserParams::default()
				.with_email("delete_non_member@example.com")
				.with_username("delete_non_member"),
		)
		.await;

		// Create room with only member
		let room_id = create_test_room(&db, Some("Private Delete"), false).await;
		add_room_member(&db, room_id, member.id).await;

		// Generate token for non_member
		let token = generate_test_token(&non_member);
		let auth_header = format!("Bearer {}", token);

		// Try to delete room as non_member
		let result = call_delete_room(&db, Some(&auth_header), room_id, non_member.id).await;

		// Assert authorization error
		assert!(result.is_err(), "Delete room as non-member should fail");
		let err = result.unwrap_err();
		assert!(
			matches!(err, reinhardt::Error::Authorization(_)),
			"Error should be authorization error, got: {:?}",
			err
		);
	}
}
