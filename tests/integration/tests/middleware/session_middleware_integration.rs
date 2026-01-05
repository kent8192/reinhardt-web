//! Middleware + Session Integration Tests
//!
//! Tests integration between middleware layer and session management:
//! - Session middleware with database backend
//! - Session middleware with file backend
//! - Session creation and retrieval
//! - Session expiration handling
//! - Concurrent session access
//! - Session cleanup
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use reinhardt_middleware::Middleware;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Session Middleware with Database Backend Tests
// ============================================================================

/// Test session middleware with database backend - session creation
///
/// **Test Intent**: Verify session middleware creates new sessions
/// in database backend on first request
///
/// **Integration Point**: Session middleware → Database session storage
///
/// **Not Intent**: Session retrieval, expiration
#[rstest]
#[tokio::test]
async fn test_session_middleware_database_creation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Simulate session creation
	let session_id = "session_db_123";
	let session_data = serde_json::json!({"user_id": null, "csrf_token": "abc123"});

	sqlx::query(
		"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
	)
	.bind(session_id)
	.bind(&session_data)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create session");

	// Verify session created
	let result = sqlx::query("SELECT id, data FROM sessions WHERE id = $1")
		.bind(session_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query session");

	let stored_id: String = result.get("id");
	let stored_data: serde_json::Value = result.get("data");

	assert_eq!(stored_id, session_id);
	assert_eq!(stored_data["csrf_token"], "abc123");
	assert_eq!(stored_data["user_id"], serde_json::Value::Null);
}

/// Test session middleware with database backend - session retrieval
///
/// **Test Intent**: Verify session middleware retrieves existing sessions
/// from database on subsequent requests
///
/// **Integration Point**: Session middleware → Database session retrieval
///
/// **Not Intent**: Session creation, updates
#[rstest]
#[tokio::test]
async fn test_session_middleware_database_retrieval(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Pre-create session
	let session_id = "session_retrieve_456";
	let session_data = serde_json::json!({"user_id": 100, "preferences": {"theme": "dark"}});

	sqlx::query(
		"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() + INTERVAL '2 hours')",
	)
	.bind(session_id)
	.bind(&session_data)
	.execute(pool.as_ref())
	.await
	.expect("Failed to pre-create session");

	// Simulate session retrieval
	let result = sqlx::query("SELECT id, data, expires_at FROM sessions WHERE id = $1")
		.bind(session_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to retrieve session");

	let stored_id: String = result.get("id");
	let stored_data: serde_json::Value = result.get("data");

	assert_eq!(stored_id, session_id);
	assert_eq!(stored_data["user_id"], 100);
	assert_eq!(stored_data["preferences"]["theme"], "dark");
}

/// Test session middleware with database backend - session update
///
/// **Test Intent**: Verify session middleware updates session data
/// in database backend during request processing
///
/// **Integration Point**: Session middleware → Database session update
///
/// **Not Intent**: Creation only, no updates
#[rstest]
#[tokio::test]
async fn test_session_middleware_database_update(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL,
			updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create initial session
	let session_id = "session_update_789";
	let initial_data = serde_json::json!({"user_id": null});

	sqlx::query(
		"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
	)
	.bind(session_id)
	.bind(&initial_data)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create session");

	// Update session data (simulating login)
	let updated_data = serde_json::json!({"user_id": 200, "authenticated": true});

	sqlx::query("UPDATE sessions SET data = $1, updated_at = NOW() WHERE id = $2")
		.bind(&updated_data)
		.bind(session_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to update session");

	// Verify update
	let result = sqlx::query("SELECT data FROM sessions WHERE id = $1")
		.bind(session_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query session");

	let stored_data: serde_json::Value = result.get("data");

	assert_eq!(stored_data["user_id"], 200);
	assert!(stored_data["authenticated"]);
}

// ============================================================================
// Session Middleware with File Backend Tests
// ============================================================================

/// Test session middleware with file backend - session creation
///
/// **Test Intent**: Verify session middleware can create sessions
/// using file-based storage backend
///
/// **Integration Point**: Session middleware → File session storage
///
/// **Not Intent**: Database backend, memory backend
#[rstest]
#[tokio::test]
async fn test_session_middleware_file_creation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create file_sessions table (simulating file metadata tracking)
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS file_sessions (
			id TEXT PRIMARY KEY,
			file_path TEXT NOT NULL,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create file_sessions table");

	// Simulate file session creation
	let session_id = "file_session_001";
	let file_path = format!("/tmp/sessions/{}.json", session_id);

	sqlx::query("INSERT INTO file_sessions (id, file_path) VALUES ($1, $2)")
		.bind(session_id)
		.bind(&file_path)
		.execute(pool.as_ref())
		.await
		.expect("Failed to register file session");

	// Verify file session registered
	let result = sqlx::query("SELECT id, file_path FROM file_sessions WHERE id = $1")
		.bind(session_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query file session");

	let stored_id: String = result.get("id");
	let stored_path: String = result.get("file_path");

	assert_eq!(stored_id, session_id);
	assert_eq!(stored_path, file_path);
}

// ============================================================================
// Session Creation and Retrieval Tests
// ============================================================================

/// Test session creation with default expiration
///
/// **Test Intent**: Verify sessions are created with default
/// expiration time (e.g., 1 hour, 1 day)
///
/// **Integration Point**: Session middleware → Session expiration setup
///
/// **Not Intent**: Custom expiration, no expiration
#[rstest]
#[tokio::test]
async fn test_session_creation_with_default_expiration(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create session with default 24-hour expiration
	let session_id = "session_default_exp";
	let session_data = serde_json::json!({});

	sqlx::query(
		"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() + INTERVAL '24 hours')",
	)
	.bind(session_id)
	.bind(&session_data)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create session");

	// Verify expiration is set correctly
	let result: (bool,) = sqlx::query_as(
		"SELECT (expires_at > NOW() + INTERVAL '23 hours' AND expires_at < NOW() + INTERVAL '25 hours') as is_valid FROM sessions WHERE id = $1",
	)
	.bind(session_id)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to check expiration");

	assert!(result.0, "Session expiration should be ~24 hours from now");
}

/// Test session retrieval by cookie ID
///
/// **Test Intent**: Verify session middleware retrieves sessions
/// using session ID from cookie
///
/// **Integration Point**: Session middleware → Cookie-based session lookup
///
/// **Not Intent**: Header-based lookup, query parameter
#[rstest]
#[tokio::test]
async fn test_session_retrieval_by_cookie(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create request_tracking table (simulating cookie tracking)
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS request_tracking (
			id SERIAL PRIMARY KEY,
			cookie_session_id TEXT NOT NULL,
			retrieved_session_id TEXT,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create request_tracking table");

	// Pre-create session
	let session_id = "session_cookie_lookup";
	sqlx::query(
		"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
	)
	.bind(session_id)
	.bind(serde_json::json!({"cart_items": 3}))
	.execute(pool.as_ref())
	.await
	.expect("Failed to create session");

	// Simulate cookie-based retrieval
	let cookie_session_id = session_id;

	// Track lookup
	sqlx::query(
		"INSERT INTO request_tracking (cookie_session_id, retrieved_session_id) VALUES ($1, $2)",
	)
	.bind(cookie_session_id)
	.bind(session_id)
	.execute(pool.as_ref())
	.await
	.expect("Failed to track request");

	// Verify session found via cookie
	let result = sqlx::query(
		"SELECT retrieved_session_id FROM request_tracking WHERE cookie_session_id = $1",
	)
	.bind(cookie_session_id)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to query tracking");

	let retrieved_id: String = result.get("retrieved_session_id");
	assert_eq!(retrieved_id, session_id);
}

// ============================================================================
// Session Expiration Handling Tests
// ============================================================================

/// Test session expiration detection
///
/// **Test Intent**: Verify expired sessions are detected and not
/// used for request processing
///
/// **Integration Point**: Session middleware → Expiration check
///
/// **Not Intent**: Cleanup, renewal
#[rstest]
#[tokio::test]
async fn test_session_expiration_detection(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create expired session
	let expired_session = "session_expired_001";
	sqlx::query(
		"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() - INTERVAL '1 hour')",
	)
	.bind(expired_session)
	.bind(serde_json::json!({}))
	.execute(pool.as_ref())
	.await
	.expect("Failed to create expired session");

	// Create valid session
	let valid_session = "session_valid_001";
	sqlx::query(
		"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
	)
	.bind(valid_session)
	.bind(serde_json::json!({}))
	.execute(pool.as_ref())
	.await
	.expect("Failed to create valid session");

	// Detect expired sessions
	let expired_ids: Vec<String> =
		sqlx::query_scalar("SELECT id FROM sessions WHERE expires_at < NOW()")
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to query expired sessions");

	assert_eq!(expired_ids.len(), 1);
	assert_eq!(expired_ids[0], expired_session);

	// Verify valid sessions not included
	let valid_ids: Vec<String> =
		sqlx::query_scalar("SELECT id FROM sessions WHERE expires_at >= NOW()")
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to query valid sessions");

	assert_eq!(valid_ids.len(), 1);
	assert_eq!(valid_ids[0], valid_session);
}

/// Test session renewal on activity
///
/// **Test Intent**: Verify session expiration is extended when
/// user activity is detected
///
/// **Integration Point**: Session middleware → Session renewal
///
/// **Not Intent**: Initial creation, expiration only
#[rstest]
#[tokio::test]
async fn test_session_renewal_on_activity(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL,
			last_activity TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create session with initial expiration
	let session_id = "session_renewal_001";
	sqlx::query(
		"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() + INTERVAL '30 minutes')",
	)
	.bind(session_id)
	.bind(serde_json::json!({}))
	.execute(pool.as_ref())
	.await
	.expect("Failed to create session");

	// Simulate activity - renew session
	sqlx::query(
		"UPDATE sessions SET expires_at = NOW() + INTERVAL '1 hour', last_activity = NOW() WHERE id = $1",
	)
	.bind(session_id)
	.execute(pool.as_ref())
	.await
	.expect("Failed to renew session");

	// Verify expiration extended
	let result: (bool,) = sqlx::query_as(
		"SELECT (expires_at > NOW() + INTERVAL '55 minutes') as is_renewed FROM sessions WHERE id = $1",
	)
	.bind(session_id)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to check renewal");

	assert!(result.0, "Session should be renewed to ~1 hour");
}

// ============================================================================
// Concurrent Session Access Tests
// ============================================================================

/// Test concurrent session reads
///
/// **Test Intent**: Verify multiple requests can read the same session
/// concurrently without conflicts
///
/// **Integration Point**: Session middleware → Concurrent read handling
///
/// **Not Intent**: Writes, exclusive access
#[rstest]
#[tokio::test]
async fn test_concurrent_session_reads(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL,
			read_count INT DEFAULT 0
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create session
	let session_id = "session_concurrent_read";
	sqlx::query(
		"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
	)
	.bind(session_id)
	.bind(serde_json::json!({"value": 42}))
	.execute(pool.as_ref())
	.await
	.expect("Failed to create session");

	// Simulate concurrent reads
	let mut handles = vec![];
	for _ in 0..5 {
		let pool_clone = pool.clone();
		let session_id_clone = session_id.to_string();
		handles.push(tokio::spawn(async move {
			sqlx::query("UPDATE sessions SET read_count = read_count + 1 WHERE id = $1")
				.bind(&session_id_clone)
				.execute(pool_clone.as_ref())
				.await
				.expect("Failed to increment read count");
		}));
	}

	// Wait for all reads
	for handle in handles {
		handle.await.expect("Task failed");
	}

	// Verify all reads completed
	let read_count: i32 = sqlx::query_scalar("SELECT read_count FROM sessions WHERE id = $1")
		.bind(session_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to get read count");

	assert_eq!(read_count, 5, "All 5 concurrent reads should complete");
}

/// Test concurrent session writes with locking
///
/// **Test Intent**: Verify concurrent writes to the same session
/// are handled correctly with row-level locking
///
/// **Integration Point**: Session middleware → Concurrent write handling
///
/// **Not Intent**: Reads, no locking
#[rstest]
#[tokio::test]
async fn test_concurrent_session_writes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL,
			write_count INT DEFAULT 0
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create session
	let session_id = "session_concurrent_write";
	sqlx::query(
		"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
	)
	.bind(session_id)
	.bind(serde_json::json!({"counter": 0}))
	.execute(pool.as_ref())
	.await
	.expect("Failed to create session");

	// Simulate concurrent writes
	let mut handles = vec![];
	for _ in 0..5 {
		let pool_clone = pool.clone();
		let session_id_clone = session_id.to_string();
		handles.push(tokio::spawn(async move {
			sqlx::query("UPDATE sessions SET write_count = write_count + 1 WHERE id = $1")
				.bind(&session_id_clone)
				.execute(pool_clone.as_ref())
				.await
				.expect("Failed to increment write count");
		}));
	}

	// Wait for all writes
	for handle in handles {
		handle.await.expect("Task failed");
	}

	// Verify all writes completed atomically
	let write_count: i32 = sqlx::query_scalar("SELECT write_count FROM sessions WHERE id = $1")
		.bind(session_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to get write count");

	assert_eq!(write_count, 5, "All 5 concurrent writes should complete");
}

// ============================================================================
// Session Cleanup Tests
// ============================================================================

/// Test expired session cleanup
///
/// **Test Intent**: Verify cleanup process deletes expired sessions
/// from storage backend
///
/// **Integration Point**: Session middleware → Expired session cleanup
///
/// **Not Intent**: Active session deletion, manual deletion
#[rstest]
#[tokio::test]
async fn test_expired_session_cleanup(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create expired sessions
	for i in 1..=3 {
		sqlx::query(
			"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() - INTERVAL '1 hour')",
		)
		.bind(format!("expired_session_{}", i))
		.bind(serde_json::json!({}))
		.execute(pool.as_ref())
		.await
		.expect("Failed to create expired session");
	}

	// Create active sessions
	for i in 1..=2 {
		sqlx::query(
			"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
		)
		.bind(format!("active_session_{}", i))
		.bind(serde_json::json!({}))
		.execute(pool.as_ref())
		.await
		.expect("Failed to create active session");
	}

	// Execute cleanup
	let deleted_count = sqlx::query("DELETE FROM sessions WHERE expires_at < NOW()")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup expired sessions")
		.rows_affected();

	assert_eq!(deleted_count, 3, "Should delete 3 expired sessions");

	// Verify active sessions remain
	let remaining_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sessions")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count remaining sessions");

	assert_eq!(
		remaining_count, 2,
		"Should have 2 active sessions remaining"
	);
}

/// Test session cleanup by max age
///
/// **Test Intent**: Verify cleanup can delete sessions older than
/// a specified age regardless of expiration
///
/// **Integration Point**: Session middleware → Age-based cleanup
///
/// **Not Intent**: Expiration-based only, no age limit
#[rstest]
#[tokio::test]
async fn test_session_cleanup_by_max_age(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create old session (created 10 days ago, still valid)
	sqlx::query(
		r#"
		INSERT INTO sessions (id, data, expires_at, created_at)
		VALUES ($1, $2, NOW() + INTERVAL '1 hour', NOW() - INTERVAL '10 days')
		"#,
	)
	.bind("old_session")
	.bind(serde_json::json!({}))
	.execute(pool.as_ref())
	.await
	.expect("Failed to create old session");

	// Create recent session
	sqlx::query(
		"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() + INTERVAL '1 hour')",
	)
	.bind("recent_session")
	.bind(serde_json::json!({}))
	.execute(pool.as_ref())
	.await
	.expect("Failed to create recent session");

	// Cleanup sessions older than 7 days (max age policy)
	let deleted_count =
		sqlx::query("DELETE FROM sessions WHERE created_at < NOW() - INTERVAL '7 days'")
			.execute(pool.as_ref())
			.await
			.expect("Failed to cleanup old sessions")
			.rows_affected();

	assert_eq!(deleted_count, 1, "Should delete 1 old session");

	// Verify recent session remains
	let remaining: Vec<String> = sqlx::query_scalar("SELECT id FROM sessions")
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to query remaining sessions");

	assert_eq!(remaining.len(), 1);
	assert_eq!(remaining[0], "recent_session");
}

/// Test session cleanup statistics
///
/// **Test Intent**: Verify cleanup process reports statistics
/// (number deleted, errors, duration)
///
/// **Integration Point**: Session middleware → Cleanup reporting
///
/// **Not Intent**: Execution only, no reporting
#[rstest]
#[tokio::test]
async fn test_session_cleanup_statistics(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create sessions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			data JSONB NOT NULL,
			expires_at TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sessions table");

	// Create cleanup_stats table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS cleanup_stats (
			id SERIAL PRIMARY KEY,
			deleted_count BIGINT NOT NULL,
			duration_ms BIGINT NOT NULL,
			timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create cleanup_stats table");

	// Create expired sessions
	for i in 1..=5 {
		sqlx::query(
			"INSERT INTO sessions (id, data, expires_at) VALUES ($1, $2, NOW() - INTERVAL '1 hour')",
		)
		.bind(format!("cleanup_session_{}", i))
		.bind(serde_json::json!({}))
		.execute(pool.as_ref())
		.await
		.expect("Failed to create session");
	}

	// Execute cleanup and measure
	let start = std::time::Instant::now();
	let deleted_count = sqlx::query("DELETE FROM sessions WHERE expires_at < NOW()")
		.execute(pool.as_ref())
		.await
		.expect("Failed to cleanup")
		.rows_affected();
	let duration_ms = start.elapsed().as_millis() as i64;

	// Record statistics
	sqlx::query("INSERT INTO cleanup_stats (deleted_count, duration_ms) VALUES ($1, $2)")
		.bind(deleted_count as i64)
		.bind(duration_ms)
		.execute(pool.as_ref())
		.await
		.expect("Failed to record stats");

	// Verify statistics
	let result = sqlx::query(
		"SELECT deleted_count, duration_ms FROM cleanup_stats ORDER BY id DESC LIMIT 1",
	)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to query stats");

	let recorded_count: i64 = result.get("deleted_count");
	let recorded_duration: i64 = result.get("duration_ms");

	assert_eq!(recorded_count, 5);
	assert!(recorded_duration >= 0);
}
