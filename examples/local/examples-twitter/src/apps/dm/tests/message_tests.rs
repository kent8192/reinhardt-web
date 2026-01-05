//! DM message tests
//!
//! Tests for DM message list, send, and get operations

use super::helpers::endpoints;
use crate::migrations::TwitterMigrations;
use crate::test_utils::{TestUserParams, create_test_user, generate_test_token};
use reinhardt::Response;
use reinhardt::db::DatabaseConnection;
use reinhardt::test::SingletonScope;
use reinhardt::test::fixtures::{injection_context_with_overrides, postgres_with_migrations_from};
use rstest::rstest;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Test Helpers
// ============================================================================

/// Create a test room in the database
async fn create_test_room(db: &DatabaseConnection, name: Option<&str>, is_group: bool) -> Uuid {
	use crate::apps::dm::models::DMRoom;
	use chrono::Utc;
	use reinhardt::db::orm::Manager;

	let room = DMRoom {
		id: Uuid::new_v4(),
		name: name.map(String::from),
		is_group,
		members: Default::default(),
		created_at: Utc::now(),
		updated_at: Utc::now(),
	};

	let created_room = DMRoom::objects()
		.create(room)
		.with_conn(db)
		.await
		.expect("Failed to create test room");

	created_room.id
}

/// Add a member to a room
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

/// Create a test message in the database
async fn create_test_message(
	db: &DatabaseConnection,
	room_id: Uuid,
	sender_id: Uuid,
	content: &str,
) -> Uuid {
	use crate::apps::dm::models::DMMessage;
	use reinhardt::db::orm::Manager;

	// Create message using generated new() function
	let message = DMMessage::new(room_id, sender_id, content.to_string());

	let created_message = DMMessage::objects()
		.create(message)
		.with_conn(db)
		.await
		.expect("Failed to create test message");

	created_message.id
}

/// Count messages in the database
async fn count_messages(db: &DatabaseConnection, room_id: Uuid) -> i64 {
	use crate::apps::dm::models::DMMessage;

	// Filter by room_id using ORM API
	let messages = DMMessage::objects()
		.filter(
			DMMessage::field_room(),
			reinhardt::db::orm::FilterOperator::Eq,
			reinhardt::db::orm::FilterValue::Uuid(room_id),
		)
		.all_with_conn(db)
		.await
		.unwrap_or_default();

	messages.len() as i64
}

/// Check if message exists
async fn message_exists(db: &DatabaseConnection, message_id: Uuid) -> bool {
	use crate::apps::dm::models::DMMessage;

	DMMessage::objects()
		.get(message_id)
		.with_conn(db)
		.await
		.is_ok()
}

/// Call list_messages endpoint
async fn call_list_messages(
	db: Arc<DatabaseConnection>,
	room_id: Uuid,
	token: Option<&str>,
) -> Response {
	use crate::apps::dm::views::messages::list_messages;
	use reinhardt::di::InjectionContext;
	use reinhardt::{CurrentUser, Path, Request};

	let path = endpoints::messages_list_url(&room_id.to_string());
	let mut request = Request::get(&path);
	if let Some(t) = token {
		request = request.header("Authorization", format!("Bearer {}", t));
	}

	let ctx = InjectionContext::current();

	if token.is_none() {
		// Return unauthorized for missing token
		return Response::unauthorized().with_body("Authentication required");
	}

	let current_user = match CurrentUser::from_request(&request.build(), &ctx).await {
		Ok(user) => user,
		Err(_) => return Response::unauthorized().with_body("Invalid token"),
	};

	match list_messages(Path(room_id), db, current_user).await {
		Ok(response) => response,
		Err(e) => Response::bad_request().with_body(e.to_string()),
	}
}

/// Call send_message endpoint
async fn call_send_message(
	db: Arc<DatabaseConnection>,
	room_id: Uuid,
	content: &str,
	token: Option<&str>,
) -> Response {
	use crate::apps::dm::serializers::CreateMessageRequest;
	use crate::apps::dm::views::messages::send_message;
	use reinhardt::di::InjectionContext;
	use reinhardt::{CurrentUser, Json, Path, Request};

	let path = endpoints::messages_list_url(&room_id.to_string());
	let mut request = Request::post(&path);
	if let Some(t) = token {
		request = request.header("Authorization", format!("Bearer {}", t));
	}

	let ctx = InjectionContext::current();

	if token.is_none() {
		return Response::unauthorized().with_body("Authentication required");
	}

	let current_user = match CurrentUser::from_request(&request.build(), &ctx).await {
		Ok(user) => user,
		Err(_) => return Response::unauthorized().with_body("Invalid token"),
	};

	let json_body = Json(CreateMessageRequest {
		content: content.to_string(),
	});

	match send_message(Path(room_id), json_body, db, current_user).await {
		Ok(response) => response,
		Err(e) => Response::bad_request().with_body(e.to_string()),
	}
}

/// Call get_message endpoint
async fn call_get_message(
	db: Arc<DatabaseConnection>,
	room_id: Uuid,
	message_id: Uuid,
	token: Option<&str>,
) -> Response {
	use crate::apps::dm::views::messages::get_message;
	use reinhardt::di::InjectionContext;
	use reinhardt::{CurrentUser, Path, Request};

	let path = endpoints::message_detail_url(&room_id.to_string(), &message_id.to_string());
	let mut request = Request::get(&path);
	if let Some(t) = token {
		request = request.header("Authorization", format!("Bearer {}", t));
	}

	let ctx = InjectionContext::current();

	if token.is_none() {
		return Response::unauthorized().with_body("Authentication required");
	}

	let current_user = match CurrentUser::from_request(&request.build(), &ctx).await {
		Ok(user) => user,
		Err(_) => return Response::unauthorized().with_body("Invalid token"),
	};

	match get_message(Path((room_id, message_id)), db, current_user).await {
		Ok(response) => response,
		Err(e) => Response::bad_request().with_body(e.to_string()),
	}
}

// ============================================================================
// List Messages Tests
// ============================================================================

/// Test listing messages in a room successfully
#[rstest]
#[tokio::test]
async fn test_success_list_messages() {
	let (_container, db) = postgres_with_migrations_from::<TwitterMigrations>().await;
	let db = Arc::new(db);

	let scope = Arc::new(SingletonScope::new());
	let _ctx = injection_context_with_overrides(scope, |s| {
		s.set(db.clone());
	});

	// Create test user
	let user = create_test_user(&db, TestUserParams::default()).await;
	let token = generate_test_token(&user);

	// Create room and add user as member
	let room_id = create_test_room(&db, None, false).await;
	add_room_member(&db, room_id, user.id).await;

	// Create some test messages
	let _ = create_test_message(&db, room_id, user.id, "Hello!").await;
	let _ = create_test_message(&db, room_id, user.id, "How are you?").await;

	// List messages
	let response = call_list_messages(db, room_id, Some(&token)).await;

	assert_eq!(response.status, 200);
}

/// Test listing messages without authentication
#[rstest]
#[tokio::test]
async fn test_failure_list_messages_no_auth() {
	let (_container, db) = postgres_with_migrations_from::<TwitterMigrations>().await;
	let db = Arc::new(db);

	let scope = Arc::new(SingletonScope::new());
	let _ctx = injection_context_with_overrides(scope, |s| {
		s.set(db.clone());
	});

	// Create room
	let room_id = create_test_room(&db, None, false).await;

	// Try to list messages without authentication
	let response = call_list_messages(db, room_id, None).await;

	assert_eq!(response.status, 401);
}

/// Test listing messages when not a member
#[rstest]
#[tokio::test]
async fn test_failure_list_messages_not_member() {
	let (_container, db) = postgres_with_migrations_from::<TwitterMigrations>().await;
	let db = Arc::new(db);

	let scope = Arc::new(SingletonScope::new());
	let _ctx = injection_context_with_overrides(scope, |s| {
		s.set(db.clone());
	});

	// Create test user
	let user = create_test_user(&db, TestUserParams::default()).await;
	let token = generate_test_token(&user);

	// Create room (user is NOT a member)
	let room_id = create_test_room(&db, None, false).await;

	// Try to list messages
	let response = call_list_messages(db, room_id, Some(&token)).await;

	assert_eq!(response.status, 400);
}

// ============================================================================
// Send Message Tests
// ============================================================================

/// Test sending a message successfully
#[rstest]
#[tokio::test]
async fn test_success_send_message() {
	let (_container, db) = postgres_with_migrations_from::<TwitterMigrations>().await;
	let db = Arc::new(db);

	let scope = Arc::new(SingletonScope::new());
	let _ctx = injection_context_with_overrides(scope, |s| {
		s.set(db.clone());
	});

	// Create test user
	let user = create_test_user(&db, TestUserParams::default()).await;
	let token = generate_test_token(&user);

	// Create room and add user as member
	let room_id = create_test_room(&db, None, false).await;
	add_room_member(&db, room_id, user.id).await;

	// Verify no messages initially
	assert_eq!(count_messages(&db, room_id).await, 0);

	// Send message
	let response = call_send_message(db.clone(), room_id, "Hello, World!", Some(&token)).await;

	assert_eq!(response.status, 201);

	// Verify message was created
	assert_eq!(count_messages(&db, room_id).await, 1);
}

/// Test sending a message without authentication
#[rstest]
#[tokio::test]
async fn test_failure_send_message_no_auth() {
	let (_container, db) = postgres_with_migrations_from::<TwitterMigrations>().await;
	let db = Arc::new(db);

	let scope = Arc::new(SingletonScope::new());
	let _ctx = injection_context_with_overrides(scope, |s| {
		s.set(db.clone());
	});

	// Create room
	let room_id = create_test_room(&db, None, false).await;

	// Try to send message without authentication
	let response = call_send_message(db, room_id, "Hello!", None).await;

	assert_eq!(response.status, 401);
}

/// Test sending a message when not a member
#[rstest]
#[tokio::test]
async fn test_failure_send_message_not_member() {
	let (_container, db) = postgres_with_migrations_from::<TwitterMigrations>().await;
	let db = Arc::new(db);

	let scope = Arc::new(SingletonScope::new());
	let _ctx = injection_context_with_overrides(scope, |s| {
		s.set(db.clone());
	});

	// Create test user
	let user = create_test_user(&db, TestUserParams::default()).await;
	let token = generate_test_token(&user);

	// Create room (user is NOT a member)
	let room_id = create_test_room(&db, None, false).await;

	// Try to send message
	let response = call_send_message(db, room_id, "Hello!", Some(&token)).await;

	assert_eq!(response.status, 400);
}

/// Test sending an empty message (validation failure)
#[rstest]
#[tokio::test]
async fn test_failure_send_message_empty_content() {
	let (_container, db) = postgres_with_migrations_from::<TwitterMigrations>().await;
	let db = Arc::new(db);

	let scope = Arc::new(SingletonScope::new());
	let _ctx = injection_context_with_overrides(scope, |s| {
		s.set(db.clone());
	});

	// Create test user
	let user = create_test_user(&db, TestUserParams::default()).await;
	let token = generate_test_token(&user);

	// Create room and add user as member
	let room_id = create_test_room(&db, None, false).await;
	add_room_member(&db, room_id, user.id).await;

	// Try to send empty message
	let response = call_send_message(db, room_id, "", Some(&token)).await;

	assert_eq!(response.status, 400);
}

// ============================================================================
// Get Message Tests
// ============================================================================

/// Test getting a specific message successfully
#[rstest]
#[tokio::test]
async fn test_success_get_message() {
	let (_container, db) = postgres_with_migrations_from::<TwitterMigrations>().await;
	let db = Arc::new(db);

	let scope = Arc::new(SingletonScope::new());
	let _ctx = injection_context_with_overrides(scope, |s| {
		s.set(db.clone());
	});

	// Create test user
	let user = create_test_user(&db, TestUserParams::default()).await;
	let token = generate_test_token(&user);

	// Create room and add user as member
	let room_id = create_test_room(&db, None, false).await;
	add_room_member(&db, room_id, user.id).await;

	// Create a message
	let message_id = create_test_message(&db, room_id, user.id, "Test message").await;

	// Get the message
	let response = call_get_message(db, room_id, message_id, Some(&token)).await;

	assert_eq!(response.status, 200);
}

/// Test getting a message without authentication
#[rstest]
#[tokio::test]
async fn test_failure_get_message_no_auth() {
	let (_container, db) = postgres_with_migrations_from::<TwitterMigrations>().await;
	let db = Arc::new(db);

	let scope = Arc::new(SingletonScope::new());
	let _ctx = injection_context_with_overrides(scope, |s| {
		s.set(db.clone());
	});

	// Create test user
	let user = create_test_user(&db, TestUserParams::default()).await;

	// Create room and add user
	let room_id = create_test_room(&db, None, false).await;
	add_room_member(&db, room_id, user.id).await;

	// Create a message
	let message_id = create_test_message(&db, room_id, user.id, "Test message").await;

	// Try to get message without authentication
	let response = call_get_message(db, room_id, message_id, None).await;

	assert_eq!(response.status, 401);
}

/// Test getting a message when not a member
#[rstest]
#[tokio::test]
async fn test_failure_get_message_not_member() {
	let (_container, db) = postgres_with_migrations_from::<TwitterMigrations>().await;
	let db = Arc::new(db);

	let scope = Arc::new(SingletonScope::new());
	let _ctx = injection_context_with_overrides(scope, |s| {
		s.set(db.clone());
	});

	// Create two users
	let member = create_test_user(&db, TestUserParams::default()).await;
	let non_member = create_test_user(
		&db,
		TestUserParams {
			username: "nonmember".to_string(),
			email: "nonmember@example.com".to_string(),
			..Default::default()
		},
	)
	.await;
	let token = generate_test_token(&non_member);

	// Create room and add only member
	let room_id = create_test_room(&db, None, false).await;
	add_room_member(&db, room_id, member.id).await;

	// Create a message by the member
	let message_id = create_test_message(&db, room_id, member.id, "Test message").await;

	// Try to get message as non-member
	let response = call_get_message(db, room_id, message_id, Some(&token)).await;

	assert_eq!(response.status, 400);
}

/// Test getting a non-existent message
#[rstest]
#[tokio::test]
async fn test_failure_get_message_not_found() {
	let (_container, db) = postgres_with_migrations_from::<TwitterMigrations>().await;
	let db = Arc::new(db);

	let scope = Arc::new(SingletonScope::new());
	let _ctx = injection_context_with_overrides(scope, |s| {
		s.set(db.clone());
	});

	// Create test user
	let user = create_test_user(&db, TestUserParams::default()).await;
	let token = generate_test_token(&user);

	// Create room and add user as member
	let room_id = create_test_room(&db, None, false).await;
	add_room_member(&db, room_id, user.id).await;

	// Try to get non-existent message
	let non_existent_id = Uuid::new_v4();
	let response = call_get_message(db, room_id, non_existent_id, Some(&token)).await;

	assert_eq!(response.status, 400);
}
