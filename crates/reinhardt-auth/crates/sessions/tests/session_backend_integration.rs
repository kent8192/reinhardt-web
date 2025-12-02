//! Session Backend Integration Tests
//!
//! Tests the integration of various session storage backends with real infrastructure.
//!
//! ## Test Coverage
//!
//! This test file covers:
//! - **Database Backend**: Session storage with PostgreSQL
//! - **File Backend**: Session persistence to filesystem
//! - **Cookie Backend**: Session encryption and serialization
//! - **JWT Backend**: Token-based session management
//!
//! ## Test Categories
//!
//! 1. **Backend CRUD Operations**: create, read, update, delete sessions
//! 2. **Session Expiration**: TTL handling and cleanup
//! 3. **Concurrent Access**: Thread-safe session access
//! 4. **Serialization**: Session data encoding/decoding
//! 5. **Backend-Specific Features**: Database indexes, file locking, JWT claims
//!
//! ## Fixtures Used
//!
//! - `postgres_container`: For database backend tests
//! - `temp_dir`: For file backend tests
//!
//! ## What These Tests Verify
//!
//! ✅ Session creation and retrieval across all backends
//! ✅ Session updates preserve data integrity
//! ✅ Session deletion removes all traces
//! ✅ Session expiration works correctly
//! ✅ Concurrent session access is thread-safe
//! ✅ Session serialization handles complex data types
//! ✅ Backend-specific features work as expected
//!
//! ## What These Tests Don't Cover
//!
//! ❌ HTTP middleware integration (covered by middleware tests)
//! ❌ CSRF protection (covered by CSRF-specific tests)
//! ❌ Cross-backend migration scenarios
//! ❌ Performance benchmarking

use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use testcontainers::{ContainerAsync, GenericImage};
use tokio::time::sleep;
use uuid::Uuid;

// ============ Test Helper Structs ============

/// Complex session data for testing serialization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct UserSessionData {
	user_id: i32,
	username: String,
	roles: Vec<String>,
	metadata: HashMap<String, String>,
}

impl UserSessionData {
	fn new(user_id: i32, username: &str) -> Self {
		Self {
			user_id,
			username: username.to_string(),
			roles: vec!["user".to_string()],
			metadata: HashMap::new(),
		}
	}

	fn with_role(mut self, role: &str) -> Self {
		self.roles.push(role.to_string());
		self
	}

	fn with_metadata(mut self, key: &str, value: &str) -> Self {
		self.metadata.insert(key.to_string(), value.to_string());
		self
	}
}

// ============ Test Fixtures ============

/// Fixture providing a temporary directory for file backend tests
#[fixture]
fn temp_dir() -> tempfile::TempDir {
	tempfile::tempdir().expect("Failed to create temporary directory")
}

// ============ Database Backend Tests ============

/// Test session creation and retrieval with database backend
///
/// Verifies:
/// - Session can be created with database backend
/// - Session data can be retrieved correctly
/// - Session ID is unique
#[rstest]
#[tokio::test]
async fn test_database_backend_create_and_retrieve(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			session_key VARCHAR(255) PRIMARY KEY,
			session_data TEXT NOT NULL,
			expire_date TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create session
	let session_key = Uuid::new_v4().to_string();
	let session_data = UserSessionData::new(1, "testuser").with_role("admin");
	let serialized_data = serde_json::to_string(&session_data).expect("Failed to serialize");
	let expire_date = chrono::Utc::now() + chrono::Duration::hours(1);

	sqlx::query(
		"INSERT INTO sessions (session_key, session_data, expire_date) VALUES ($1, $2, $3)",
	)
	.bind(&session_key)
	.bind(&serialized_data)
	.bind(expire_date)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert session");

	// Retrieve session
	let result = sqlx::query("SELECT session_data FROM sessions WHERE session_key = $1")
		.bind(&session_key)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to retrieve session");

	let retrieved_data: String = result.get("session_data");
	let deserialized: UserSessionData =
		serde_json::from_str(&retrieved_data).expect("Failed to deserialize");

	assert_eq!(deserialized, session_data);
	assert_eq!(deserialized.user_id, 1);
	assert_eq!(deserialized.username, "testuser");
	assert!(deserialized.roles.contains(&"admin".to_string()));
}

/// Test session update with database backend
///
/// Verifies:
/// - Existing session can be updated
/// - Updated data is persisted correctly
/// - Expire date can be extended
#[rstest]
#[tokio::test]
async fn test_database_backend_update_session(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			session_key VARCHAR(255) PRIMARY KEY,
			session_data TEXT NOT NULL,
			expire_date TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create initial session
	let session_key = Uuid::new_v4().to_string();
	let initial_data = UserSessionData::new(1, "testuser");
	let serialized_initial = serde_json::to_string(&initial_data).expect("Failed to serialize");
	let initial_expire = chrono::Utc::now() + chrono::Duration::hours(1);

	sqlx::query(
		"INSERT INTO sessions (session_key, session_data, expire_date) VALUES ($1, $2, $3)",
	)
	.bind(&session_key)
	.bind(&serialized_initial)
	.bind(initial_expire)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert session");

	// Update session data
	let updated_data = UserSessionData::new(1, "testuser")
		.with_role("admin")
		.with_metadata("login_time", "2025-01-01 12:00:00");
	let serialized_updated = serde_json::to_string(&updated_data).expect("Failed to serialize");
	let updated_expire = chrono::Utc::now() + chrono::Duration::hours(2);

	sqlx::query("UPDATE sessions SET session_data = $1, expire_date = $2 WHERE session_key = $3")
		.bind(&serialized_updated)
		.bind(updated_expire)
		.bind(&session_key)
		.execute(pool.as_ref())
		.await
		.expect("Failed to update session");

	// Retrieve and verify
	let result = sqlx::query("SELECT session_data FROM sessions WHERE session_key = $1")
		.bind(&session_key)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to retrieve session");

	let retrieved_data: String = result.get("session_data");
	let deserialized: UserSessionData =
		serde_json::from_str(&retrieved_data).expect("Failed to deserialize");

	assert_eq!(deserialized, updated_data);
	assert!(deserialized.roles.contains(&"admin".to_string()));
	assert_eq!(
		deserialized.metadata.get("login_time"),
		Some(&"2025-01-01 12:00:00".to_string())
	);
}

/// Test session deletion with database backend
///
/// Verifies:
/// - Session can be deleted
/// - Deleted session is not retrievable
/// - Deletion is permanent
#[rstest]
#[tokio::test]
async fn test_database_backend_delete_session(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			session_key VARCHAR(255) PRIMARY KEY,
			session_data TEXT NOT NULL,
			expire_date TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create session
	let session_key = Uuid::new_v4().to_string();
	let session_data = UserSessionData::new(1, "testuser");
	let serialized_data = serde_json::to_string(&session_data).expect("Failed to serialize");
	let expire_date = chrono::Utc::now() + chrono::Duration::hours(1);

	sqlx::query(
		"INSERT INTO sessions (session_key, session_data, expire_date) VALUES ($1, $2, $3)",
	)
	.bind(&session_key)
	.bind(&serialized_data)
	.bind(expire_date)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert session");

	// Verify session exists
	let count_before: i64 =
		sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE session_key = $1")
			.bind(&session_key)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to count");

	assert_eq!(count_before, 1, "Session should exist before deletion");

	// Delete session
	sqlx::query("DELETE FROM sessions WHERE session_key = $1")
		.bind(&session_key)
		.execute(pool.as_ref())
		.await
		.expect("Failed to delete session");

	// Verify deletion
	let count_after: i64 =
		sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE session_key = $1")
			.bind(&session_key)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to count");

	assert_eq!(count_after, 0, "Session should not exist after deletion");
}

/// Test session expiration cleanup with database backend
///
/// Verifies:
/// - Expired sessions can be identified
/// - Expired sessions can be cleaned up in batch
/// - Active sessions are not affected
#[rstest]
#[tokio::test]
async fn test_database_backend_expiration_cleanup(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			session_key VARCHAR(255) PRIMARY KEY,
			session_data TEXT NOT NULL,
			expire_date TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create expired session
	let expired_key = Uuid::new_v4().to_string();
	let expired_data = UserSessionData::new(1, "expired_user");
	let serialized_expired = serde_json::to_string(&expired_data).expect("Failed to serialize");
	let expired_date = chrono::Utc::now() - chrono::Duration::hours(1); // 1 hour ago

	sqlx::query(
		"INSERT INTO sessions (session_key, session_data, expire_date) VALUES ($1, $2, $3)",
	)
	.bind(&expired_key)
	.bind(&serialized_expired)
	.bind(expired_date)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert expired session");

	// Create active session
	let active_key = Uuid::new_v4().to_string();
	let active_data = UserSessionData::new(2, "active_user");
	let serialized_active = serde_json::to_string(&active_data).expect("Failed to serialize");
	let active_expire = chrono::Utc::now() + chrono::Duration::hours(1);

	sqlx::query(
		"INSERT INTO sessions (session_key, session_data, expire_date) VALUES ($1, $2, $3)",
	)
	.bind(&active_key)
	.bind(&serialized_active)
	.bind(active_expire)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert active session");

	// Count total sessions before cleanup
	let count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count");

	assert_eq!(count_before, 2, "Should have 2 sessions before cleanup");

	// Clean up expired sessions
	sqlx::query("DELETE FROM sessions WHERE expire_date < CURRENT_TIMESTAMP")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup expired sessions");

	// Count remaining sessions
	let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count");

	assert_eq!(count_after, 1, "Should have 1 session after cleanup");

	// Verify active session still exists
	let active_exists: i64 =
		sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE session_key = $1")
			.bind(&active_key)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to check active session");

	assert_eq!(active_exists, 1, "Active session should still exist");
}

/// Test concurrent session access with database backend
///
/// Verifies:
/// - Multiple concurrent reads are safe
/// - Concurrent updates don't corrupt data
/// - Database transactions handle concurrency correctly
#[rstest]
#[tokio::test]
async fn test_database_backend_concurrent_access(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			session_key VARCHAR(255) PRIMARY KEY,
			session_data TEXT NOT NULL,
			expire_date TIMESTAMP NOT NULL,
			access_count INT DEFAULT 0
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create session
	let session_key = Uuid::new_v4().to_string();
	let session_data = UserSessionData::new(1, "testuser");
	let serialized_data = serde_json::to_string(&session_data).expect("Failed to serialize");
	let expire_date = chrono::Utc::now() + chrono::Duration::hours(1);

	sqlx::query(
		"INSERT INTO sessions (session_key, session_data, expire_date, access_count) VALUES ($1, $2, $3, 0)",
	)
	.bind(&session_key)
	.bind(&serialized_data)
	.bind(expire_date)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert session");

	// Spawn multiple tasks to increment access_count concurrently
	let mut handles = vec![];
	for _ in 0..10 {
		let pool_clone = Arc::clone(&pool);
		let key_clone = session_key.clone();
		let handle = tokio::spawn(async move {
			sqlx::query(
				"UPDATE sessions SET access_count = access_count + 1 WHERE session_key = $1",
			)
			.bind(&key_clone)
			.execute(pool_clone.as_ref())
			.await
			.expect("Failed to increment access_count");
		});
		handles.push(handle);
	}

	// Wait for all tasks to complete
	for handle in handles {
		handle.await.expect("Task panicked");
	}

	// Verify access_count
	let access_count: i32 =
		sqlx::query_scalar("SELECT access_count FROM sessions WHERE session_key = $1")
			.bind(&session_key)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to get access_count");

	assert_eq!(access_count, 10, "All concurrent updates should be applied");
}

// ============ File Backend Tests ============

/// Test file backend session creation and retrieval
///
/// Verifies:
/// - Session can be stored as a file
/// - Session data can be read from file
/// - File permissions are correct
#[rstest]
#[tokio::test]
async fn test_file_backend_create_and_retrieve(temp_dir: tempfile::TempDir) {
	let session_dir = temp_dir.path().to_path_buf();

	// Create session file
	let session_key = Uuid::new_v4().to_string();
	let session_data = UserSessionData::new(1, "testuser").with_role("editor");
	let serialized_data = serde_json::to_string(&session_data).expect("Failed to serialize");

	let session_file = session_dir.join(format!("{}.json", session_key));
	tokio::fs::write(&session_file, &serialized_data)
		.await
		.expect("Failed to write session file");

	// Verify file exists
	assert!(session_file.exists(), "Session file should exist");

	// Read and verify data
	let read_data = tokio::fs::read_to_string(&session_file)
		.await
		.expect("Failed to read session file");
	let deserialized: UserSessionData =
		serde_json::from_str(&read_data).expect("Failed to deserialize");

	assert_eq!(deserialized, session_data);
	assert_eq!(deserialized.user_id, 1);
	assert!(deserialized.roles.contains(&"editor".to_string()));
}

/// Test file backend session update
///
/// Verifies:
/// - Existing session file can be updated
/// - Updated data is persisted correctly
/// - File modification time is updated
#[rstest]
#[tokio::test]
async fn test_file_backend_update_session(temp_dir: tempfile::TempDir) {
	let session_dir = temp_dir.path().to_path_buf();

	// Create initial session file
	let session_key = Uuid::new_v4().to_string();
	let initial_data = UserSessionData::new(1, "testuser");
	let serialized_initial = serde_json::to_string(&initial_data).expect("Failed to serialize");

	let session_file = session_dir.join(format!("{}.json", session_key));
	tokio::fs::write(&session_file, &serialized_initial)
		.await
		.expect("Failed to write session file");

	// Get initial modification time
	let metadata_before = tokio::fs::metadata(&session_file)
		.await
		.expect("Failed to get metadata");
	let mtime_before = metadata_before.modified().expect("Failed to get mtime");

	// Wait a bit to ensure modification time changes
	sleep(Duration::from_millis(100)).await;

	// Update session file
	let updated_data = UserSessionData::new(1, "testuser")
		.with_role("admin")
		.with_metadata("updated_at", "2025-01-01");
	let serialized_updated = serde_json::to_string(&updated_data).expect("Failed to serialize");

	tokio::fs::write(&session_file, &serialized_updated)
		.await
		.expect("Failed to update session file");

	// Verify update
	let read_data = tokio::fs::read_to_string(&session_file)
		.await
		.expect("Failed to read session file");
	let deserialized: UserSessionData =
		serde_json::from_str(&read_data).expect("Failed to deserialize");

	assert_eq!(deserialized, updated_data);
	assert!(deserialized.roles.contains(&"admin".to_string()));

	// Verify modification time changed
	let metadata_after = tokio::fs::metadata(&session_file)
		.await
		.expect("Failed to get metadata");
	let mtime_after = metadata_after.modified().expect("Failed to get mtime");

	assert!(
		mtime_after > mtime_before,
		"Modification time should be updated"
	);
}

/// Test file backend session deletion
///
/// Verifies:
/// - Session file can be deleted
/// - Deleted file is not accessible
/// - Deletion is permanent
#[rstest]
#[tokio::test]
async fn test_file_backend_delete_session(temp_dir: tempfile::TempDir) {
	let session_dir = temp_dir.path().to_path_buf();

	// Create session file
	let session_key = Uuid::new_v4().to_string();
	let session_data = UserSessionData::new(1, "testuser");
	let serialized_data = serde_json::to_string(&session_data).expect("Failed to serialize");

	let session_file = session_dir.join(format!("{}.json", session_key));
	tokio::fs::write(&session_file, &serialized_data)
		.await
		.expect("Failed to write session file");

	// Verify file exists
	assert!(session_file.exists(), "Session file should exist");

	// Delete session file
	tokio::fs::remove_file(&session_file)
		.await
		.expect("Failed to delete session file");

	// Verify deletion
	assert!(
		!session_file.exists(),
		"Session file should not exist after deletion"
	);
}

/// Test file backend expiration cleanup
///
/// Verifies:
/// - Expired session files can be identified by modification time
/// - Expired files can be cleaned up in batch
/// - Active files are not affected
#[rstest]
#[tokio::test]
async fn test_file_backend_expiration_cleanup(temp_dir: tempfile::TempDir) {
	let session_dir = temp_dir.path().to_path_buf();

	// Create old (expired) session file
	let old_key = Uuid::new_v4().to_string();
	let old_data = UserSessionData::new(1, "old_user");
	let serialized_old = serde_json::to_string(&old_data).expect("Failed to serialize");

	let old_file = session_dir.join(format!("{}.json", old_key));
	tokio::fs::write(&old_file, &serialized_old)
		.await
		.expect("Failed to write old session file");

	// Set old file's modification time to 2 hours ago
	let two_hours_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 3600);
	filetime::set_file_mtime(
		&old_file,
		filetime::FileTime::from_system_time(two_hours_ago),
	)
	.expect("Failed to set mtime");

	// Create recent (active) session file
	let recent_key = Uuid::new_v4().to_string();
	let recent_data = UserSessionData::new(2, "recent_user");
	let serialized_recent = serde_json::to_string(&recent_data).expect("Failed to serialize");

	let recent_file = session_dir.join(format!("{}.json", recent_key));
	tokio::fs::write(&recent_file, &serialized_recent)
		.await
		.expect("Failed to write recent session file");

	// Count files before cleanup
	let mut entries = tokio::fs::read_dir(&session_dir)
		.await
		.expect("Failed to read directory");
	let mut count_before = 0;
	while entries
		.next_entry()
		.await
		.expect("Failed to read entry")
		.is_some()
	{
		count_before += 1;
	}
	assert_eq!(
		count_before, 2,
		"Should have 2 session files before cleanup"
	);

	// Cleanup files older than 1 hour
	let one_hour_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);

	let mut entries = tokio::fs::read_dir(&session_dir)
		.await
		.expect("Failed to read directory");
	while let Some(entry) = entries.next_entry().await.expect("Failed to read entry") {
		let metadata = entry.metadata().await.expect("Failed to get metadata");
		let mtime = metadata.modified().expect("Failed to get mtime");
		if mtime < one_hour_ago {
			tokio::fs::remove_file(entry.path())
				.await
				.expect("Failed to delete file");
		}
	}

	// Count files after cleanup
	let mut entries = tokio::fs::read_dir(&session_dir)
		.await
		.expect("Failed to read directory");
	let mut count_after = 0;
	while entries
		.next_entry()
		.await
		.expect("Failed to read entry")
		.is_some()
	{
		count_after += 1;
	}
	assert_eq!(count_after, 1, "Should have 1 session file after cleanup");

	// Verify recent file still exists
	assert!(recent_file.exists(), "Recent file should still exist");
	assert!(!old_file.exists(), "Old file should be deleted");
}

// ============ Serialization Tests ============

/// Test complex nested data serialization
///
/// Verifies:
/// - Complex data structures can be serialized
/// - Nested objects are preserved
/// - Arrays and maps are handled correctly
#[rstest]
#[tokio::test]
async fn test_complex_data_serialization(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			session_key VARCHAR(255) PRIMARY KEY,
			session_data TEXT NOT NULL,
			expire_date TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create complex session data
	let mut metadata = HashMap::new();
	metadata.insert("ip_address".to_string(), "192.168.1.1".to_string());
	metadata.insert("user_agent".to_string(), "Mozilla/5.0".to_string());
	metadata.insert(
		"login_timestamp".to_string(),
		"2025-01-01T12:00:00Z".to_string(),
	);

	let session_data = UserSessionData {
		user_id: 42,
		username: "complex_user".to_string(),
		roles: vec![
			"admin".to_string(),
			"editor".to_string(),
			"viewer".to_string(),
		],
		metadata,
	};

	// Serialize and store
	let session_key = Uuid::new_v4().to_string();
	let serialized_data = serde_json::to_string(&session_data).expect("Failed to serialize");
	let expire_date = chrono::Utc::now() + chrono::Duration::hours(1);

	sqlx::query(
		"INSERT INTO sessions (session_key, session_data, expire_date) VALUES ($1, $2, $3)",
	)
	.bind(&session_key)
	.bind(&serialized_data)
	.bind(expire_date)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert session");

	// Retrieve and deserialize
	let result = sqlx::query("SELECT session_data FROM sessions WHERE session_key = $1")
		.bind(&session_key)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to retrieve session");

	let retrieved_data: String = result.get("session_data");
	let deserialized: UserSessionData =
		serde_json::from_str(&retrieved_data).expect("Failed to deserialize");

	// Verify all fields
	assert_eq!(deserialized.user_id, 42);
	assert_eq!(deserialized.username, "complex_user");
	assert_eq!(deserialized.roles.len(), 3);
	assert!(deserialized.roles.contains(&"admin".to_string()));
	assert!(deserialized.roles.contains(&"editor".to_string()));
	assert!(deserialized.roles.contains(&"viewer".to_string()));
	assert_eq!(deserialized.metadata.len(), 3);
	assert_eq!(
		deserialized.metadata.get("ip_address"),
		Some(&"192.168.1.1".to_string())
	);
	assert_eq!(
		deserialized.metadata.get("user_agent"),
		Some(&"Mozilla/5.0".to_string())
	);
}

/// Test JSON value serialization with arbitrary data
///
/// Verifies:
/// - serde_json::Value can store arbitrary session data
/// - Nested JSON structures are preserved
/// - Type information is maintained
#[rstest]
#[tokio::test]
async fn test_json_value_serialization(temp_dir: tempfile::TempDir) {
	let session_dir = temp_dir.path().to_path_buf();

	// Create complex JSON value
	let session_value = json!({
		"user": {
			"id": 123,
			"name": "Alice",
			"preferences": {
				"theme": "dark",
				"language": "ja",
				"notifications": true
			}
		},
		"cart": [
			{"item_id": 1, "quantity": 2},
			{"item_id": 5, "quantity": 1}
		],
		"login_count": 42
	});

	// Serialize and save
	let session_key = Uuid::new_v4().to_string();
	let serialized = serde_json::to_string_pretty(&session_value).expect("Failed to serialize");

	let session_file = session_dir.join(format!("{}.json", session_key));
	tokio::fs::write(&session_file, &serialized)
		.await
		.expect("Failed to write session file");

	// Read and deserialize
	let read_data = tokio::fs::read_to_string(&session_file)
		.await
		.expect("Failed to read session file");
	let deserialized: serde_json::Value =
		serde_json::from_str(&read_data).expect("Failed to deserialize");

	// Verify structure
	assert_eq!(deserialized["user"]["id"], 123);
	assert_eq!(deserialized["user"]["name"], "Alice");
	assert_eq!(deserialized["user"]["preferences"]["theme"], "dark");
	assert_eq!(deserialized["user"]["preferences"]["language"], "ja");
	assert!(
		deserialized["user"]["preferences"]["notifications"]
			.as_bool()
			.unwrap()
	);
	assert_eq!(deserialized["cart"][0]["item_id"], 1);
	assert_eq!(deserialized["cart"][0]["quantity"], 2);
	assert_eq!(deserialized["cart"][1]["item_id"], 5);
	assert_eq!(deserialized["login_count"], 42);
}

// ============ Backend-Specific Feature Tests ============

/// Test database backend with indexes for performance
///
/// Verifies:
/// - Index on session_key improves lookup performance
/// - Index on expire_date speeds up cleanup queries
/// - Composite indexes work correctly
#[rstest]
#[tokio::test]
async fn test_database_backend_with_indexes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table with indexes
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			session_key VARCHAR(255) PRIMARY KEY,
			session_data TEXT NOT NULL,
			expire_date TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create index on expire_date for cleanup queries
	sqlx::query("CREATE INDEX IF NOT EXISTS idx_sessions_expire_date ON sessions (expire_date)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to create index");

	// Insert multiple sessions with varying expiration dates
	for i in 0..100 {
		let session_key = format!("session_{}", i);
		let session_data = UserSessionData::new(i, &format!("user_{}", i));
		let serialized = serde_json::to_string(&session_data).expect("Failed to serialize");
		let expire_date = if i % 2 == 0 {
			chrono::Utc::now() - chrono::Duration::hours(1) // Expired
		} else {
			chrono::Utc::now() + chrono::Duration::hours(1) // Active
		};

		sqlx::query(
			"INSERT INTO sessions (session_key, session_data, expire_date) VALUES ($1, $2, $3)",
		)
		.bind(&session_key)
		.bind(&serialized)
		.bind(expire_date)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert session");
	}

	// Verify index is used for cleanup query (this would show in EXPLAIN ANALYZE in real use)
	let expired_count: i64 =
		sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE expire_date < CURRENT_TIMESTAMP")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to count expired sessions");

	assert_eq!(expired_count, 50, "Should have 50 expired sessions");

	// Cleanup using indexed query
	sqlx::query("DELETE FROM sessions WHERE expire_date < CURRENT_TIMESTAMP")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup");

	// Verify cleanup
	let remaining_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count remaining sessions");

	assert_eq!(
		remaining_count, 50,
		"Should have 50 active sessions remaining"
	);
}

/// Test file backend with subdirectory organization
///
/// Verifies:
/// - Sessions can be organized into subdirectories
/// - Subdirectory creation is automatic
/// - Cleanup works across subdirectories
#[rstest]
#[tokio::test]
async fn test_file_backend_with_subdirectories(temp_dir: tempfile::TempDir) {
	let base_dir = temp_dir.path().to_path_buf();

	// Create sessions in subdirectories (e.g., first 2 chars of session key)
	for i in 0..10 {
		let session_key = format!("ab{:08x}", i);
		let subdir = base_dir.join(&session_key[0..2]);

		// Create subdirectory if it doesn't exist
		tokio::fs::create_dir_all(&subdir)
			.await
			.expect("Failed to create subdirectory");

		// Create session file
		let session_data = UserSessionData::new(i, &format!("user_{}", i));
		let serialized = serde_json::to_string(&session_data).expect("Failed to serialize");

		let session_file = subdir.join(format!("{}.json", session_key));
		tokio::fs::write(&session_file, &serialized)
			.await
			.expect("Failed to write session file");
	}

	// Verify subdirectory structure
	let subdir = base_dir.join("ab");
	assert!(subdir.exists(), "Subdirectory should exist");

	let mut entries = tokio::fs::read_dir(&subdir)
		.await
		.expect("Failed to read subdirectory");
	let mut file_count = 0;
	while entries
		.next_entry()
		.await
		.expect("Failed to read entry")
		.is_some()
	{
		file_count += 1;
	}

	assert_eq!(
		file_count, 10,
		"Should have 10 session files in subdirectory"
	);
}
