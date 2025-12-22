//! Database session backend capacity tests
//!
//! Tests for handling large session data and verifying database storage limits.

use reinhardt_orm::manager::{get_connection, reinitialize_database};
use reinhardt_sessions::backends::cache::SessionBackend;
use reinhardt_sessions::backends::database::DatabaseSessionBackend;
use rstest::*;
use serde_json::json;
use serial_test::serial;

/// Fixture providing a test database session backend
#[fixture]
async fn backend() -> DatabaseSessionBackend {
	// Use shared cache mode for SQLite in-memory database
	// This allows multiple connections to share the same in-memory database
	let database_url = "sqlite:file::memory:?cache=shared";

	// Initialize global ORM connection for Session::objects() calls
	reinitialize_database(database_url)
		.await
		.expect("Failed to initialize ORM database");

	// Clear table before each test to ensure isolation
	// DROP TABLE ensures clean state even if previous test failed
	let conn = get_connection()
		.await
		.expect("Failed to get ORM connection");
	let _ = conn.execute("DROP TABLE IF EXISTS sessions", vec![]).await;

	let backend = DatabaseSessionBackend::new(database_url)
		.await
		.expect("Failed to create test backend");
	backend
		.create_table()
		.await
		.expect("Failed to create table");
	backend
}

/// Test storing and retrieving large session data (>1MB)
#[rstest]
#[tokio::test]
#[serial(sessions_db_capacity)]
async fn test_large_session_data_storage(#[future] backend: DatabaseSessionBackend) {
	let backend = backend.await;
	let session_key = "large_session_test";

	// Generate large JSON data (>1MB)
	// Create an array with many objects to exceed 1MB
	let mut large_items = Vec::new();
	for i in 0..10_000 {
		large_items.push(json!({
			"id": i,
			"name": format!("User_{}", i),
			"email": format!("user{}@example.com", i),
			"description": "A".repeat(100), // 100 bytes per item
			"metadata": {
				"created_at": "2025-01-01T00:00:00Z",
				"updated_at": "2025-01-01T00:00:00Z",
				"tags": ["tag1", "tag2", "tag3"],
			}
		}));
	}

	let large_data = json!({
		"items": large_items,
		"total_count": 10_000,
	});

	// Verify data size is >1MB
	let json_string = serde_json::to_string(&large_data).expect("Failed to serialize JSON");
	assert!(
		json_string.len() > 1_000_000,
		"Test data should be larger than 1MB, got {} bytes",
		json_string.len()
	);

	// Save large session data
	backend
		.save(session_key, &large_data, Some(3600))
		.await
		.expect("Failed to save large session data");

	// Load and verify large session data
	let loaded: Option<serde_json::Value> = backend
		.load(session_key)
		.await
		.expect("Failed to load large session data");

	assert!(loaded.is_some(), "Large session data should be loaded");

	let loaded_data = loaded.unwrap();
	assert_eq!(
		loaded_data["total_count"], 10_000,
		"Total count should match"
	);

	let loaded_items = loaded_data["items"]
		.as_array()
		.expect("Items should be an array");
	assert_eq!(loaded_items.len(), 10_000, "Should load all 10,000 items");

	// Verify first and last items to ensure data integrity
	assert_eq!(loaded_items[0]["id"], 0);
	assert_eq!(loaded_items[0]["name"], "User_0");
	assert_eq!(loaded_items[9_999]["id"], 9_999);
	assert_eq!(loaded_items[9_999]["name"], "User_9999");

	// Clean up
	backend
		.delete(session_key)
		.await
		.expect("Failed to delete session");
}
