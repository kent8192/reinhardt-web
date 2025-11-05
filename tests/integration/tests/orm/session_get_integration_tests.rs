//! Session::get() Database Integration Tests
//!
//! Tests the Session::get() implementation using real PostgreSQL databases to validate:
//! - Field metadata-based query generation
//! - Database row to model object mapping
//! - Identity map caching
//! - Type-safe deserialization
//!
//! Run with: cargo test --test session_get_integration_tests

use reinhardt_macros::Model;
use reinhardt_orm::{DatabaseConnection, query_types::DbBackend, session::Session};
use rstest::*;
use serde::{Deserialize, Serialize};
use sqlx::{AnyPool, Row};
use std::sync::Arc;
use testcontainers::{GenericImage, ImageExt, core::WaitFor, runners::AsyncRunner};

/// Test model using derive(Model) macro
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Model)]
#[model(app_label = "test_app", table_name = "test_users")]
struct TestUser {
	#[field(primary_key = true)]
	id: Option<i32>,

	#[field(max_length = 100, null = false)]
	username: String,

	#[field(max_length = 255)]
	email: String,

	#[field(null = true)]
	age: Option<i32>,

	#[field(default = "true")]
	is_active: bool,
}

/// rstest fixture providing a PostgreSQL container, pool, and Session
///
/// The container is automatically cleaned up when the test ends.
#[fixture]
async fn postgres_fixture() -> (
	testcontainers::ContainerAsync<GenericImage>,
	Arc<AnyPool>,
	Session,
) {
	// Install sqlx drivers for AnyPool
	sqlx::any::install_default_drivers();

	// Start PostgreSQL container
	let postgres = GenericImage::new("postgres", "16-alpine")
		.with_wait_for(WaitFor::message_on_stderr(
			"database system is ready to accept connections",
		))
		.with_env_var("POSTGRES_PASSWORD", "test")
		.with_env_var("POSTGRES_DB", "session_test_db")
		.start()
		.await
		.expect("Failed to start PostgreSQL container");

	let port = postgres
		.get_host_port_ipv4(5432)
		.await
		.expect("Failed to get PostgreSQL port");

	let database_url = format!(
		"postgres://postgres:test@localhost:{}/session_test_db",
		port
	);

	// Create connection pool
	let pool = Arc::new(
		AnyPool::connect(&database_url)
			.await
			.expect("Failed to connect to database"),
	);

	// Create session with PostgreSQL backend
	let session = Session::new(pool.clone(), DbBackend::Postgres)
		.await
		.expect("Failed to create session");

	// Create test_users table
	let conn = DatabaseConnection::connect(&database_url)
		.await
		.expect("Failed to create DatabaseConnection");

	conn.execute(
		"CREATE TABLE IF NOT EXISTS test_users (
            id SERIAL PRIMARY KEY,
            username VARCHAR(100) NOT NULL UNIQUE,
            email VARCHAR(255) NOT NULL,
            age INTEGER,
            is_active BOOLEAN NOT NULL DEFAULT TRUE
        )",
	)
	.await
	.expect("Failed to create test_users table");

	(postgres, pool, session)
}

/// Helper to insert test user data directly via AnyPool
async fn insert_test_user(
	pool: &AnyPool,
	username: &str,
	email: &str,
	age: Option<i32>,
	is_active: bool,
) -> i32 {
	let age_str = age
		.map(|a| a.to_string())
		.unwrap_or_else(|| "NULL".to_string());
	let query = format!(
		"INSERT INTO test_users (username, email, age, is_active) VALUES ('{}', '{}', {}, {}) RETURNING id",
		username, email, age_str, is_active
	);

	let row = sqlx::query(&query)
		.fetch_one(pool)
		.await
		.expect("Failed to insert test user");

	row.try_get::<i32, _>("id")
		.expect("Failed to get inserted ID")
}

#[rstest]
#[tokio::test]
async fn test_session_get_basic(
	#[future] postgres_fixture: (
		testcontainers::ContainerAsync<GenericImage>,
		Arc<AnyPool>,
		Session,
	),
) {
	let (_container, pool, mut session) = postgres_fixture.await;

	// Insert test data
	let user_id = insert_test_user(&pool, "alice", "alice@example.com", Some(25), true).await;

	// Get user via Session::get()
	let result = session.get::<TestUser>(user_id).await;
	assert!(result.is_ok(), "Session::get() should succeed");

	let user = result.unwrap();
	assert!(user.is_some(), "User should be found");

	let user = user.unwrap();
	assert_eq!(user.id, Some(user_id));
	assert_eq!(user.username, "alice");
	assert_eq!(user.email, "alice@example.com");
	assert_eq!(user.age, Some(25));
	assert_eq!(user.is_active, true);
}

#[rstest]
#[tokio::test]
async fn test_session_get_not_found(
	#[future] postgres_fixture: (
		testcontainers::ContainerAsync<GenericImage>,
		Arc<AnyPool>,
		Session,
	),
) {
	let (_container, _pool, mut session) = postgres_fixture.await;

	// Try to get non-existent user
	let result = session.get::<TestUser>(999).await;
	assert!(
		result.is_ok(),
		"Session::get() should not error for non-existent ID"
	);

	let user = result.unwrap();
	assert!(user.is_none(), "Non-existent user should return None");
}

#[rstest]
#[tokio::test]
async fn test_session_get_with_null_field(
	#[future] postgres_fixture: (
		testcontainers::ContainerAsync<GenericImage>,
		Arc<AnyPool>,
		Session,
	),
) {
	let (_container, pool, mut session) = postgres_fixture.await;

	// Insert user with NULL age
	let user_id = insert_test_user(&pool, "bob", "bob@example.com", None, false).await;

	// Get user via Session::get()
	let result = session.get::<TestUser>(user_id).await;
	assert!(result.is_ok(), "Session::get() should succeed");

	let user = result.unwrap();
	assert!(user.is_some(), "User should be found");

	let user = user.unwrap();
	assert_eq!(user.id, Some(user_id));
	assert_eq!(user.username, "bob");
	assert_eq!(user.email, "bob@example.com");
	assert_eq!(user.age, None, "NULL age should map to None");
	assert_eq!(user.is_active, false);
}

#[rstest]
#[tokio::test]
async fn test_session_get_identity_map_caching(
	#[future] postgres_fixture: (
		testcontainers::ContainerAsync<GenericImage>,
		Arc<AnyPool>,
		Session,
	),
) {
	let (_container, pool, mut session) = postgres_fixture.await;

	// Insert test data
	let user_id = insert_test_user(&pool, "charlie", "charlie@example.com", Some(30), true).await;

	// First get - from database
	let result1 = session.get::<TestUser>(user_id).await;
	assert!(result1.is_ok(), "First get should succeed");
	let user1 = result1.unwrap().unwrap();

	// Verify identity map count increased
	assert_eq!(
		session.identity_count(),
		1,
		"Identity map should have 1 entry"
	);

	// Second get - from identity map (should be faster, no DB query)
	let result2 = session.get::<TestUser>(user_id).await;
	assert!(result2.is_ok(), "Second get should succeed");
	let user2 = result2.unwrap().unwrap();

	// Identity map count should remain 1 (no new entry)
	assert_eq!(
		session.identity_count(),
		1,
		"Identity map should still have 1 entry"
	);

	// Both results should be identical
	assert_eq!(user1, user2, "Both gets should return identical data");
}

#[rstest]
#[tokio::test]
async fn test_session_get_multiple_users(
	#[future] postgres_fixture: (
		testcontainers::ContainerAsync<GenericImage>,
		Arc<AnyPool>,
		Session,
	),
) {
	let (_container, pool, mut session) = postgres_fixture.await;

	// Insert multiple users
	let user1_id = insert_test_user(&pool, "user1", "user1@example.com", Some(20), true).await;
	let user2_id = insert_test_user(&pool, "user2", "user2@example.com", Some(25), false).await;
	let user3_id = insert_test_user(&pool, "user3", "user3@example.com", None, true).await;

	// Get all users
	let user1 = session.get::<TestUser>(user1_id).await.unwrap().unwrap();
	let user2 = session.get::<TestUser>(user2_id).await.unwrap().unwrap();
	let user3 = session.get::<TestUser>(user3_id).await.unwrap().unwrap();

	// Verify identity map has 3 entries
	assert_eq!(
		session.identity_count(),
		3,
		"Identity map should have 3 entries"
	);

	// Verify data
	assert_eq!(user1.username, "user1");
	assert_eq!(user1.age, Some(20));
	assert_eq!(user1.is_active, true);

	assert_eq!(user2.username, "user2");
	assert_eq!(user2.age, Some(25));
	assert_eq!(user2.is_active, false);

	assert_eq!(user3.username, "user3");
	assert_eq!(user3.age, None);
	assert_eq!(user3.is_active, true);
}

#[rstest]
#[tokio::test]
async fn test_session_get_after_database_update(
	#[future] postgres_fixture: (
		testcontainers::ContainerAsync<GenericImage>,
		Arc<AnyPool>,
		Session,
	),
) {
	let (_container, pool, mut session) = postgres_fixture.await;

	// Insert test data
	let user_id = insert_test_user(&pool, "david", "david@example.com", Some(35), true).await;

	// First get - loads into identity map
	let user1 = session.get::<TestUser>(user_id).await.unwrap().unwrap();
	assert_eq!(user1.username, "david");
	assert_eq!(user1.age, Some(35));

	// Update database directly (bypassing session)
	sqlx::query(&format!(
		"UPDATE test_users SET age = 40 WHERE id = {}",
		user_id
	))
	.execute(&*pool)
	.await
	.expect("Failed to update user");

	// Second get - returns cached version from identity map
	// (Database update is not reflected because of identity map caching)
	let user2 = session.get::<TestUser>(user_id).await.unwrap().unwrap();
	assert_eq!(
		user2.age,
		Some(35),
		"Identity map should return cached value, not updated database value"
	);
}
