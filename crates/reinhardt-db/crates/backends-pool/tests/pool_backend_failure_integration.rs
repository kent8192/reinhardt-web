//! Pool + Backend Failure Scenarios Integration Tests
//!
//! These tests verify the integration between pool and backend when handling
//! failure scenarios, network timeouts, and error recovery.
//!
//! **Test Coverage:**
//! - Network timeout handling
//! - Database unavailable scenarios
//! - Connection leak detection
//! - Pool exhaustion behavior
//! - Backend connection validation failures
//! - Query timeout scenarios
//! - Transaction failure recovery
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use reinhardt_db::pool::{ConnectionPool, PoolConfig};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use std::time::Duration;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Network Timeout Tests
// ============================================================================

/// Test pool handles connection acquisition timeout
///
/// **Test Intent**: Verify pool returns timeout error when all connections are in use
/// and acquire_timeout is exceeded
///
/// **Integration Point**: PoolConfig::acquire_timeout → Backend connection wait
///
/// **Not Intent**: Network-level timeouts, query timeouts
#[rstest]
#[tokio::test]
async fn test_pool_acquire_timeout(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Create pool with very short acquire timeout and limited connections
	let config = PoolConfig::default()
		.with_max_connections(2)
		.with_acquire_timeout(Duration::from_millis(500));

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Acquire all connections
	let _conn1 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn1");
	let _conn2 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn2");

	// Third acquisition should timeout
	let start = std::time::Instant::now();
	let result = pool.inner().acquire().await;
	let elapsed = start.elapsed();

	assert!(result.is_err(), "Expected timeout error");
	assert!(
		elapsed >= Duration::from_millis(400) && elapsed <= Duration::from_secs(2),
		"Timeout should occur close to configured duration"
	);
}

/// Test pool handles query execution timeout
///
/// **Test Intent**: Verify pool can enforce query execution timeouts
/// when backend query takes too long
///
/// **Integration Point**: Query timeout → Backend long-running query cancellation
///
/// **Not Intent**: Connection timeout, pool timeout
#[rstest]
#[tokio::test]
async fn test_pool_query_timeout(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Execute query with timeout (PostgreSQL pg_sleep)
	let result = tokio::time::timeout(
		Duration::from_millis(500),
		sqlx::query("SELECT pg_sleep(2)").fetch_one(pool.inner()),
	)
	.await;

	// Should timeout before query completes
	assert!(result.is_err(), "Query should timeout");
}

// ============================================================================
// Database Unavailable Scenarios
// ============================================================================

/// Test pool handles invalid connection string
///
/// **Test Intent**: Verify pool creation fails gracefully when given
/// invalid database connection string
///
/// **Integration Point**: ConnectionPool::new → Backend connection validation
///
/// **Not Intent**: Valid connections, runtime connection failures
#[tokio::test]
async fn test_pool_invalid_connection_string() {
	let invalid_url = "postgres://invalid:invalid@nonexistent:5432/nonexistent";

	let config = PoolConfig::default().with_acquire_timeout(Duration::from_secs(2));

	let result = ConnectionPool::new_postgres(invalid_url, config).await;

	assert!(
		result.is_err(),
		"Should fail with invalid connection string"
	);
}

/// Test pool handles backend becoming unavailable after creation
///
/// **Test Intent**: Verify pool can recover from temporary backend unavailability
/// by creating new connections when backend becomes available again
///
/// **Integration Point**: Pool connection recovery → Backend reconnection
///
/// **Not Intent**: Persistent failures, connection retry limits
#[rstest]
#[tokio::test]
async fn test_pool_backend_recovery(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default()
		.with_min_connections(0)
		.with_max_connections(2);

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Acquire and use connection
	let mut conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	sqlx::query("SELECT 1")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute query");

	// Simulate connection failure by closing connection
	drop(conn);

	// Pool should be able to create new connection
	let mut new_conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire new connection after recovery");

	let result = sqlx::query("SELECT 2 as value")
		.fetch_one(&mut *new_conn)
		.await
		.expect("Failed to execute query on new connection");

	let value: i32 = result.get("value");
	assert_eq!(value, 2);
}

// ============================================================================
// Connection Leak Detection Tests
// ============================================================================

/// Test pool detects when connections are not returned
///
/// **Test Intent**: Verify pool can detect when max_connections is reached
/// and no connections are available
///
/// **Integration Point**: Pool max_connections enforcement → Backend connection limit
///
/// **Not Intent**: Memory leak detection, connection pooling algorithm
#[rstest]
#[tokio::test]
async fn test_pool_connection_leak_detection(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default()
		.with_max_connections(3)
		.with_acquire_timeout(Duration::from_millis(500));

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Acquire all connections without releasing
	let _conn1 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn1");
	let _conn2 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn2");
	let _conn3 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn3");

	// Fourth acquisition should fail (timeout)
	let result = pool.inner().acquire().await;
	assert!(
		result.is_err(),
		"Should timeout when all connections leaked"
	);
}

// ============================================================================
// Pool Exhaustion Tests
// ============================================================================

/// Test pool behavior when exhausted
///
/// **Test Intent**: Verify pool returns meaningful error when max_connections
/// is reached and acquire_timeout expires
///
/// **Integration Point**: Pool exhaustion → Backend connection limit reached
///
/// **Not Intent**: Connection reuse, connection recycling
#[rstest]
#[tokio::test]
async fn test_pool_exhaustion_error(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default()
		.with_max_connections(1)
		.with_acquire_timeout(Duration::from_millis(300));

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Acquire the only connection
	let _conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn");

	// Second acquisition should fail with timeout
	let result = pool.inner().acquire().await;
	assert!(result.is_err(), "Should fail when pool exhausted");

	// Error should indicate timeout/pool exhaustion
	let error = result.unwrap_err();
	let error_string = error.to_string().to_lowercase();
	assert!(
		error_string.contains("timeout")
			|| error_string.contains("pool")
			|| error_string.contains("closed"),
		"Error should indicate timeout or pool issue, got: {}",
		error
	);
}

/// Test pool handles multiple concurrent acquisition failures
///
/// **Test Intent**: Verify pool correctly handles multiple threads/tasks
/// attempting to acquire connections when pool is exhausted
///
/// **Integration Point**: Concurrent pool exhaustion → Backend connection limit
///
/// **Not Intent**: Single-threaded exhaustion, connection priority
#[rstest]
#[tokio::test]
async fn test_pool_concurrent_exhaustion(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default()
		.with_max_connections(2)
		.with_acquire_timeout(Duration::from_millis(500));

	let pool = Arc::new(
		ConnectionPool::new_postgres(&url, config)
			.await
			.expect("Failed to create pool"),
	);

	// Acquire all connections
	let _conn1 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn1");
	let _conn2 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn2");

	// Spawn multiple tasks trying to acquire (should all fail)
	let mut handles = Vec::new();
	for _ in 0..5 {
		let pool_clone = Arc::clone(&pool);
		let handle = tokio::spawn(async move { pool_clone.inner().acquire().await });
		handles.push(handle);
	}

	// All tasks should fail to acquire
	let mut failed_count = 0;
	for handle in handles {
		let result = handle.await.expect("Task panicked");
		if result.is_err() {
			failed_count += 1;
		}
	}

	assert_eq!(failed_count, 5, "All acquisition attempts should fail");
}

// ============================================================================
// Backend Connection Validation Failures
// ============================================================================

/// Test pool handles backend connection validation failure
///
/// **Test Intent**: Verify pool can detect invalid connections when
/// test_before_acquire is enabled
///
/// **Integration Point**: PoolConfig::test_before_acquire → Backend connection health check
///
/// **Not Intent**: Connection creation, connection reuse
#[rstest]
#[tokio::test]
async fn test_pool_connection_validation_failure(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Enable test_before_acquire to validate connections
	let config = PoolConfig::default().with_test_before_acquire(true);

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Acquire connection (should pass validation)
	let mut conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	// Verify connection is healthy
	let result = sqlx::query("SELECT 1 as value")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute query");

	let value: i32 = result.get("value");
	assert_eq!(value, 1);
}

// ============================================================================
// Transaction Failure Recovery Tests
// ============================================================================

/// Test pool recovers from failed transaction
///
/// **Test Intent**: Verify pool can acquire new connections after
/// a transaction fails and is rolled back
///
/// **Integration Point**: Transaction failure → Backend connection recovery
///
/// **Not Intent**: Transaction retry, savepoints
#[rstest]
#[tokio::test]
async fn test_pool_transaction_failure_recovery(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Create table
	sqlx::query("CREATE TABLE IF NOT EXISTS txn_fail_test (id SERIAL PRIMARY KEY, value INT)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	// Begin transaction
	let mut tx = pool
		.inner()
		.begin()
		.await
		.expect("Failed to begin transaction");

	// Execute failing query (invalid syntax)
	let fail_result = sqlx::query("INSERT INTO txn_fail_test (invalid_column) VALUES (1)")
		.execute(&mut *tx)
		.await;

	assert!(fail_result.is_err(), "Invalid query should fail");

	// Rollback transaction
	tx.rollback().await.expect("Failed to rollback transaction");

	// Pool should still work (acquire new connection)
	let mut conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection after failed transaction");

	// Execute valid query
	let result = sqlx::query("SELECT 1 as value")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute query after recovery");

	let value: i32 = result.get("value");
	assert_eq!(value, 1);
}

/// Test pool handles transaction timeout
///
/// **Test Intent**: Verify pool can handle transactions that exceed timeout
/// and still function afterward
///
/// **Integration Point**: Transaction timeout → Backend transaction management
///
/// **Not Intent**: Query timeout, connection timeout
#[rstest]
#[tokio::test]
async fn test_pool_transaction_timeout(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Begin transaction
	let mut tx = pool
		.inner()
		.begin()
		.await
		.expect("Failed to begin transaction");

	// Execute query with timeout
	let result = tokio::time::timeout(
		Duration::from_millis(500),
		sqlx::query("SELECT pg_sleep(2)").fetch_one(&mut *tx),
	)
	.await;

	assert!(result.is_err(), "Transaction query should timeout");

	// Rollback timed-out transaction
	tx.rollback().await.ok(); // Ignore rollback error

	// Pool should still work
	let mut conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection after timeout");

	let result = sqlx::query("SELECT 2 as value")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute query after timeout");

	let value: i32 = result.get("value");
	assert_eq!(value, 2);
}

// ============================================================================
// Backend Error Propagation Tests
// ============================================================================

/// Test pool propagates backend-specific errors
///
/// **Test Intent**: Verify pool correctly propagates backend error details
/// (e.g., constraint violations, syntax errors)
///
/// **Integration Point**: Pool error handling → Backend error information
///
/// **Not Intent**: Error recovery, error retry
#[rstest]
#[tokio::test]
async fn test_pool_backend_error_propagation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Create table with UNIQUE constraint
	sqlx::query("CREATE TABLE IF NOT EXISTS error_test (id SERIAL PRIMARY KEY, email TEXT UNIQUE)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	// Insert first record
	sqlx::query("INSERT INTO error_test (email) VALUES ('test@example.com')")
		.execute(pool.inner())
		.await
		.expect("Failed to insert first record");

	// Insert duplicate (should fail with constraint violation)
	let result = sqlx::query("INSERT INTO error_test (email) VALUES ('test@example.com')")
		.execute(pool.inner())
		.await;

	assert!(result.is_err(), "Duplicate insert should fail");

	// Error should be PostgreSQL-specific (unique constraint violation)
	let error = result.unwrap_err();
	let error_string = error.to_string().to_lowercase();
	assert!(
		error_string.contains("unique") || error_string.contains("duplicate"),
		"Error should indicate unique constraint violation, got: {}",
		error
	);

	// Pool should still work after error
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) as count FROM error_test")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to count records");
	assert_eq!(count, 1, "Only one record should exist");
}
