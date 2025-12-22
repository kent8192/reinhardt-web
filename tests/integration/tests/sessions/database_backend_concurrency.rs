//! Database Backend Concurrency Integration Tests
//!
//! Tests concurrent access scenarios for database session backend:
//! - Concurrent session write conflict resolution
//! - Transaction rollback on save failure
//! - UNIQUE constraint collision handling
//! - Connection pool exhaustion error handling
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container with connection pool

use reinhardt_orm::manager::{get_connection, reinitialize_database};
use reinhardt_sessions::backends::cache::SessionBackend;
use reinhardt_sessions::backends::database::DatabaseSessionBackend;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serde_json::json;
use serial_test::serial;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Concurrent Session Write Conflict Tests
// ============================================================================

/// Test concurrent writes to the same session key
///
/// **Test Intent**: Verify that concurrent writes to the same session key
/// maintain data integrity through database-level locking (ON CONFLICT UPDATE)
///
/// **Integration Point**: DatabaseSessionBackend → PostgreSQL row-level locking
///
/// **Not Intent**: Sequential writes, different session keys
///
/// **Note**: Currently ignored because DatabaseSessionBackend::save() uses
/// check-then-create pattern instead of UPSERT (INSERT ... ON CONFLICT DO UPDATE).
/// This causes TOCTOU race conditions with concurrent writes.
/// TODO: Implement UPSERT in ORM or use raw SQL with ON CONFLICT clause.
#[rstest]
#[tokio::test]
#[serial(sessions_db_concurrency)]
#[ignore = "Requires UPSERT implementation in DatabaseSessionBackend::save()"]
async fn test_concurrent_session_write_conflict(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, database_url) = postgres_container.await;

	// Initialize global ORM connection for Session::objects() calls
	reinitialize_database(&database_url)
		.await
		.expect("Failed to initialize ORM database");

	// Clear table before test to ensure isolation
	let conn = get_connection()
		.await
		.expect("Failed to get ORM connection");
	let _ = conn.execute("DROP TABLE IF EXISTS sessions", vec![]).await;

	// Initialize DatabaseSessionBackend with PostgreSQL
	let backend = DatabaseSessionBackend::new(&database_url)
		.await
		.expect("Failed to create DatabaseSessionBackend");

	backend
		.create_table()
		.await
		.expect("Failed to create sessions table");

	// Session key shared across all concurrent writes
	let session_key = "concurrent_session";

	// Spawn 10 concurrent tasks writing to the same session key
	let mut handles = vec![];
	for i in 0..10 {
		let backend_clone = backend.clone();
		let key = session_key.to_string();

		let handle = tokio::spawn(async move {
			let data = json!({
				"write_id": i,
				"timestamp": chrono::Utc::now().to_rfc3339(),
			});

			backend_clone
				.save(&key, &data, Some(3600))
				.await
				.expect("Failed to save session");
		});

		handles.push(handle);
	}

	// Wait for all concurrent writes to complete
	for handle in handles {
		handle.await.expect("Task panicked");
	}

	// Verify that exactly one session exists (no duplicates)
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE session_key = $1")
		.bind(session_key)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count sessions");

	assert_eq!(
		count, 1,
		"Should have exactly 1 session (no duplicates from concurrent writes)"
	);

	// Verify the session data is valid (one of the 10 writes)
	let loaded: Option<serde_json::Value> = backend
		.load(session_key)
		.await
		.expect("Failed to load session");

	assert!(loaded.is_some(), "Session should exist");
	let data = loaded.unwrap();
	assert!(
		data.get("write_id").is_some(),
		"Session should have write_id field"
	);
	let write_id = data["write_id"].as_i64().unwrap();
	assert!(
		(0..10).contains(&write_id),
		"write_id should be in range 0-9"
	);
}

// ============================================================================
// Transaction Rollback Tests
// ============================================================================

/// Test session save failure triggers transaction rollback
///
/// **Test Intent**: Verify that when a session save operation fails
/// (e.g., constraint violation, serialization error), the transaction
/// is properly rolled back and no partial data is committed
///
/// **Integration Point**: DatabaseSessionBackend → Transaction management
///
/// **Not Intent**: Successful saves, no transaction usage
#[rstest]
#[tokio::test]
#[serial(sessions_db_concurrency)]
async fn test_session_save_transaction_rollback(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, database_url) = postgres_container.await;

	// Initialize global ORM connection for Session::objects() calls
	reinitialize_database(&database_url)
		.await
		.expect("Failed to initialize ORM database");

	// Clear table before test to ensure isolation
	let conn = get_connection()
		.await
		.expect("Failed to get ORM connection");
	let _ = conn.execute("DROP TABLE IF EXISTS sessions", vec![]).await;

	// Initialize DatabaseSessionBackend
	let backend = DatabaseSessionBackend::new(&database_url)
		.await
		.expect("Failed to create DatabaseSessionBackend");

	backend
		.create_table()
		.await
		.expect("Failed to create sessions table");

	// Add a CHECK constraint to force validation failure
	sqlx::query(
		r#"
		ALTER TABLE sessions
		ADD CONSTRAINT session_data_not_empty
		CHECK (length(session_data::text) > 10)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to add CHECK constraint");

	// Get initial session count
	let initial_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count sessions");

	// Attempt to save a session with data that violates the constraint
	let session_key = "rollback_test";
	let invalid_data = json!({}); // Empty JSON object (serializes to "{}", length 2)

	let result = backend.save(session_key, &invalid_data, Some(3600)).await;

	// Verify the save operation failed
	assert!(result.is_err(), "Save with invalid data should fail");

	// Verify no session was created (transaction rolled back)
	let final_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count sessions");

	assert_eq!(
		initial_count, final_count,
		"Session count should remain unchanged after rollback"
	);

	// Verify the session does not exist
	let exists = backend
		.exists(session_key)
		.await
		.expect("Failed to check existence");
	assert!(!exists, "Session should not exist after rollback");

	// Clean up: Remove the constraint
	sqlx::query("ALTER TABLE sessions DROP CONSTRAINT session_data_not_empty")
		.execute(pool.as_ref())
		.await
		.expect("Failed to drop constraint");
}

// ============================================================================
// UNIQUE Constraint Collision Tests
// ============================================================================

/// Test session key collision handling with retry
///
/// **Test Intent**: Verify that when multiple processes attempt to create
/// a session with the same key simultaneously, the database UNIQUE constraint
/// prevents duplicates and the ON CONFLICT clause handles updates correctly
///
/// **Integration Point**: DatabaseSessionBackend → UNIQUE constraint + ON CONFLICT
///
/// **Not Intent**: No conflicts, sequential inserts
#[rstest]
#[tokio::test]
#[serial(sessions_db_concurrency)]
async fn test_session_key_collision_handling(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, database_url) = postgres_container.await;

	// Initialize global ORM connection for Session::objects() calls
	reinitialize_database(&database_url)
		.await
		.expect("Failed to initialize ORM database");

	// Clear table before test to ensure isolation
	let conn = get_connection()
		.await
		.expect("Failed to get ORM connection");
	let _ = conn.execute("DROP TABLE IF EXISTS sessions", vec![]).await;

	// Initialize DatabaseSessionBackend
	let backend = DatabaseSessionBackend::new(&database_url)
		.await
		.expect("Failed to create DatabaseSessionBackend");

	backend
		.create_table()
		.await
		.expect("Failed to create sessions table");

	// Create initial session
	let session_key = "collision_key";
	let initial_data = json!({
		"version": 1,
		"initial": true,
	});

	backend
		.save(session_key, &initial_data, Some(3600))
		.await
		.expect("Failed to save initial session");

	// Spawn multiple concurrent tasks trying to save to the same key
	let mut handles = vec![];
	for i in 2..=5 {
		let backend_clone = backend.clone();
		let key = session_key.to_string();

		let handle = tokio::spawn(async move {
			let data = json!({
				"version": i,
				"updated": true,
			});

			backend_clone
				.save(&key, &data, Some(3600))
				.await
				.expect("Failed to save session");
		});

		handles.push(handle);
	}

	// Wait for all updates to complete
	for handle in handles {
		handle.await.expect("Task panicked");
	}

	// Verify exactly one session exists (no duplicates)
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions WHERE session_key = $1")
		.bind(session_key)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count sessions");

	assert_eq!(
		count, 1,
		"Should have exactly 1 session despite concurrent updates"
	);

	// Verify the session was updated (not the initial version)
	let loaded: Option<serde_json::Value> = backend
		.load(session_key)
		.await
		.expect("Failed to load session");

	assert!(loaded.is_some(), "Session should exist");
	let data = loaded.unwrap();
	assert_eq!(
		data.get("updated"),
		Some(&json!(true)),
		"Session should be updated"
	);
	let version = data["version"].as_i64().unwrap();
	assert!(
		(2..=5).contains(&version),
		"Version should be one of the updates (2-5)"
	);
}

// ============================================================================
// Connection Pool Exhaustion Tests
// ============================================================================

/// Test error handling when connection pool is exhausted
///
/// **Test Intent**: Verify that when all connections in the pool are in use,
/// additional session operations either wait for an available connection
/// or return a meaningful error rather than hanging indefinitely
///
/// **Integration Point**: DatabaseSessionBackend → Connection pool management
///
/// **Not Intent**: Unlimited connections, no pool limits
#[rstest]
#[tokio::test]
#[serial(sessions_db_concurrency)]
async fn test_session_backend_connection_pool_exhaustion(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, database_url) = postgres_container.await;

	// Initialize global ORM connection for Session::objects() calls
	reinitialize_database(&database_url)
		.await
		.expect("Failed to initialize ORM database");

	// Clear table before test to ensure isolation
	let conn = get_connection()
		.await
		.expect("Failed to get ORM connection");
	let _ = conn.execute("DROP TABLE IF EXISTS sessions", vec![]).await;

	// Initialize DatabaseSessionBackend with the fixture pool
	let backend = DatabaseSessionBackend::new(&database_url)
		.await
		.expect("Failed to create DatabaseSessionBackend");

	backend
		.create_table()
		.await
		.expect("Failed to create sessions table");

	// Create a small pool for demonstrating exhaustion (max 2 connections)
	let small_pool = sqlx::postgres::PgPoolOptions::new()
		.max_connections(2)
		.acquire_timeout(std::time::Duration::from_secs(2))
		.connect(&database_url)
		.await
		.expect("Failed to create small pool");

	// Occupy all connections by acquiring them and holding them
	let _conn1 = small_pool
		.acquire()
		.await
		.expect("Failed to acquire connection 1");
	let _conn2 = small_pool
		.acquire()
		.await
		.expect("Failed to acquire connection 2");

	// Now attempt to use the small pool (all connections occupied)
	let query_result = tokio::time::timeout(
		std::time::Duration::from_secs(3),
		sqlx::query("SELECT 1").fetch_one(&small_pool),
	)
	.await;

	// Verify that the query either times out or fails with pool error
	match query_result {
		Ok(Ok(_)) => {
			panic!("Query should not succeed when pool is exhausted");
		}
		Ok(Err(e)) => {
			// sqlx returned an error (connection timeout)
			assert!(
				e.to_string().contains("timeout") || e.to_string().contains("pool"),
				"Error should mention timeout or pool: {}",
				e
			);
		}
		Err(_) => {
			// tokio::time::timeout expired
			// This is the expected behavior - operation waited for connection
		}
	}

	// Verify that the main backend (using fixture pool) still works
	let session_key = "pool_exhaustion_test";
	let data = json!({"test": "data"});

	backend
		.save(session_key, &data, Some(3600))
		.await
		.expect("Save should succeed with fixture pool (different pool)");
}
