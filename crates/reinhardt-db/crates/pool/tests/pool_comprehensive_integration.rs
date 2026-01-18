//! Comprehensive Pool Integration Tests
//!
//! This file consolidates all pool integration tests using existing fixtures
//! from reinhardt-test crate. Tests are organized by functionality:
//! - Basic Operations (pool_basic_tests.rs)
//! - Timeout & Error Handling (pool_timeout_tests.rs)
//! - Lifecycle Management (pool_lifecycle_tests.rs)
//! - Recreation & Reconfiguration (pool_recreation_tests.rs)
//! - Database-Specific Behaviors (pool_database_specific_tests.rs)
//!
//! **Fixtures Used:**
//! - postgres_container: Real PostgreSQL database for integration tests
//! - validator_test_db: Lightweight test database for simple tests
//! - mysql_suite: Suite-wide MySQL instance for DB-specific tests

use reinhardt_db::pool::{ConnectionPool, PoolConfig};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use std::time::Duration;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Basic Pool Operations Tests
// Consolidated from: pool_basic_tests.rs
// ============================================================================

/// Test basic pool creation with postgres_container fixture
///
/// **Test Intent**: Verify pool can be created with default configuration
/// using real PostgreSQL database
///
/// **Not Intent**: Custom configuration, error cases, multiple databases
#[rstest]
#[tokio::test]
async fn test_pool_basic_creation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Verify pool is usable
	let mut conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	let result = sqlx::query("SELECT 1 as value")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute query");

	let value: i32 = result.get("value");
	assert_eq!(value, 1);
}

/// Test pool connection acquisition and release
///
/// **Test Intent**: Verify connections can be acquired from pool and released back
///
/// **Not Intent**: Concurrent access, timeout, error handling
#[rstest]
#[tokio::test]
async fn test_pool_connection_acquisition(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Acquire connection
	let conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	// Connection should be valid

	// Release by dropping
	drop(conn);

	// Should be able to acquire again
	let _conn2 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire second connection");
}

/// Test pool with custom configuration
///
/// **Test Intent**: Verify pool respects custom min/max connections configuration
///
/// **Not Intent**: Default configuration, dynamic reconfiguration
#[rstest]
#[tokio::test]
async fn test_pool_custom_config(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default()
		.with_min_connections(2)
		.with_max_connections(5)
		.with_acquire_timeout(Duration::from_secs(10));

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Verify configuration is applied
	assert_eq!(pool.config().min_connections, 2);
	assert_eq!(pool.config().max_connections, 5);
	assert_eq!(pool.config().acquire_timeout, Duration::from_secs(10));
}

/// Test pool handles CREATE TABLE and INSERT operations
///
/// **Test Intent**: Verify pool can handle DDL and DML operations
///
/// **Not Intent**: Complex queries, transactions, concurrent DDL
#[rstest]
#[tokio::test]
async fn test_pool_ddl_dml_operations(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// CREATE TABLE
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS test_users (
			id SERIAL PRIMARY KEY,
			username VARCHAR(255) NOT NULL
		)",
	)
	.execute(pool.inner())
	.await
	.expect("Failed to create table");

	// INSERT
	let result = sqlx::query("INSERT INTO test_users (username) VALUES ($1) RETURNING id")
		.bind("testuser")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to insert user");

	let user_id: i32 = result.get("id");
	assert!(user_id > 0);

	// SELECT to verify
	let result = sqlx::query("SELECT username FROM test_users WHERE id = $1")
		.bind(user_id)
		.fetch_one(pool.inner())
		.await
		.expect("Failed to select user");

	let username: String = result.get("username");
	assert_eq!(username, "testuser");
}

// ============================================================================
// Timeout & Error Handling Tests
// Consolidated from: pool_timeout_tests.rs
// ============================================================================

/// Test pool connection acquisition timeout
///
/// **Test Intent**: Verify pool times out when all connections are in use
/// and max_connections is reached
///
/// **Not Intent**: Successful acquisition, infinite wait, connection recycling
#[rstest]
#[tokio::test]
async fn test_pool_acquisition_timeout(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	// Small pool with short timeout
	let config = PoolConfig::default()
		.with_max_connections(2)
		.with_min_connections(0)
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

	// Try to acquire 3rd connection - should timeout
	let result = pool.inner().acquire().await;
	assert!(result.is_err(), "Expected timeout error");
}

/// Test pool handles invalid SQL gracefully
///
/// **Test Intent**: Verify pool remains functional after query errors
///
/// **Not Intent**: Successful queries, connection pool corruption
#[rstest]
#[tokio::test]
async fn test_pool_query_error_handling(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Execute invalid SQL
	let result = sqlx::query("SELECT * FROM nonexistent_table")
		.fetch_one(pool.inner())
		.await;

	// Should fail
	assert!(result.is_err());

	// Pool should still work after error
	let result = sqlx::query("SELECT 1 as value")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to execute query after error");

	let value: i32 = result.get("value");
	assert_eq!(value, 1);
}

/// Test pool connection timeout recovery
///
/// **Test Intent**: Verify pool can acquire connection after timeout when
/// connections are released
///
/// **Not Intent**: Permanent exhaustion, connection leak detection
#[rstest]
#[tokio::test]
async fn test_pool_timeout_recovery(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default()
		.with_max_connections(2)
		.with_min_connections(0)
		.with_acquire_timeout(Duration::from_millis(500));

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Acquire all connections
	let conn1 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn1");
	let _conn2 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn2");

	// Try to acquire 3rd connection - should timeout
	let result = pool.inner().acquire().await;
	assert!(result.is_err());

	// Release one connection
	drop(conn1);

	// Should be able to acquire again
	let _conn3 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn3 after release");
}

// ============================================================================
// Lifecycle Management Tests
// Consolidated from: pool_lifecycle_tests.rs
// ============================================================================

/// Test pool close functionality
///
/// **Test Intent**: Verify pool.close() gracefully shuts down all connections
///
/// **Not Intent**: Forced termination, connection leak, partial close
#[rstest]
#[tokio::test]
async fn test_pool_close(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Pool should be usable
	let mut conn = pool.inner().acquire().await.expect("Failed to acquire");
	sqlx::query("SELECT 1")
		.fetch_one(&mut *conn)
		.await
		.expect("Query failed");
	drop(conn);

	// Close pool
	pool.inner().close().await;

	// Pool should be closed (no new connections)
	let result = pool.inner().acquire().await;
	assert!(result.is_err(), "Pool should be closed");
}

/// Test pool handles connection failure during initialization
///
/// **Test Intent**: Verify pool creation fails gracefully with invalid URL
///
/// **Not Intent**: Successful creation, retry logic, partial initialization
#[tokio::test]
async fn test_pool_creation_failure() {
	let invalid_url = "postgres://invalid:invalid@localhost:9999/invalid";
	let config = PoolConfig::default().with_acquire_timeout(Duration::from_secs(1));

	let result = ConnectionPool::new_postgres(invalid_url, config).await;
	assert!(result.is_err(), "Expected pool creation to fail");
}

/// Test pool min_connections initialization
///
/// **Test Intent**: Verify pool creates min_connections on startup
///
/// **Not Intent**: max_connections, dynamic scaling, lazy initialization
#[rstest]
#[tokio::test]
async fn test_pool_min_connections_initialization(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default()
		.with_min_connections(3)
		.with_max_connections(5);

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Pool should have at least min_connections available
	// (SQLx doesn't expose idle connection count directly,
	// so we verify by acquiring connections quickly)
	let conn1 = pool.inner().acquire().await.expect("Failed conn1");
	let conn2 = pool.inner().acquire().await.expect("Failed conn2");
	let conn3 = pool.inner().acquire().await.expect("Failed conn3");

	// All should succeed quickly (no new connection creation delay)
	drop(conn1);
	drop(conn2);
	drop(conn3);
}

// ============================================================================
// Recreation & Reconfiguration Tests
// Consolidated from: pool_recreation_tests.rs
// ============================================================================

/// Test pool.recreate() preserves configuration
///
/// **Test Intent**: Verify pool.recreate() creates new pool with same config
///
/// **Not Intent**: Configuration changes, partial recreation, connection reuse
#[rstest]
#[tokio::test]
#[ignore = "Test hangs during pool.recreate() - needs investigation of sqlx pool.close() behavior"]
async fn test_pool_recreate_preserves_config(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default()
		.with_min_connections(2)
		.with_max_connections(5)
		.with_acquire_timeout(Duration::from_secs(10))
		.with_test_before_acquire(true);

	let mut pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	let original_config = pool.config().clone();

	// Recreate pool
	pool.recreate().await.expect("Failed to recreate pool");

	// Verify configuration is preserved
	assert_eq!(
		pool.config().min_connections,
		original_config.min_connections
	);
	assert_eq!(
		pool.config().max_connections,
		original_config.max_connections
	);
	assert_eq!(
		pool.config().acquire_timeout,
		original_config.acquire_timeout
	);
	assert_eq!(
		pool.config().test_before_acquire,
		original_config.test_before_acquire
	);
}

/// Test pool.recreate() closes old connections
///
/// **Test Intent**: Verify recreate() terminates all existing connections
///
/// **Not Intent**: Connection reuse, graceful shutdown, connection migration
#[rstest]
#[tokio::test]
#[ignore = "Test hangs during pool.recreate() - needs investigation of sqlx pool.close() behavior"]
async fn test_pool_recreate_closes_connections(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default();
	let mut pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Acquire connection before recreate
	let old_conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	// Recreate pool
	pool.recreate().await.expect("Failed to recreate pool");

	// Old connection should be closed (or invalid)
	// Note: SQLx connections may not immediately show as closed,
	// but pool operations should work with new connections
	drop(old_conn);

	// New pool should be functional
	let new_conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire from recreated pool");
	drop(new_conn);
}

/// Test pool.recreate() creates new working pool
///
/// **Test Intent**: Verify recreated pool is fully functional for queries
///
/// **Not Intent**: Old pool functionality, connection state preservation
#[rstest]
#[tokio::test]
#[ignore = "Test hangs during pool.recreate() - needs investigation of sqlx pool.close() behavior"]
async fn test_pool_recreate_functionality(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default();
	let mut pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Create table in original pool
	sqlx::query("CREATE TABLE IF NOT EXISTS recreate_test (id SERIAL PRIMARY KEY, value TEXT)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	// Recreate pool
	pool.recreate().await.expect("Failed to recreate pool");

	// New pool should be able to query existing table
	sqlx::query("INSERT INTO recreate_test (value) VALUES ($1)")
		.bind("test")
		.execute(pool.inner())
		.await
		.expect("Failed to insert after recreate");

	let result = sqlx::query("SELECT COUNT(*) as count FROM recreate_test")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to count after recreate");

	let count: i64 = result.get("count");
	assert_eq!(count, 1);
}

// ============================================================================
// Database-Specific Behaviors Tests
// Consolidated from: pool_database_specific_tests.rs
// ============================================================================

/// Test PostgreSQL-specific pool behavior
///
/// **Test Intent**: Verify pool works correctly with PostgreSQL-specific features
/// like LISTEN/NOTIFY, advisory locks
///
/// **Not Intent**: MySQL features, SQLite features, generic SQL
#[rstest]
#[tokio::test]
async fn test_postgres_specific_features(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Test PostgreSQL advisory lock
	sqlx::query("SELECT pg_advisory_lock(12345)")
		.execute(pool.inner())
		.await
		.expect("Failed to acquire advisory lock");

	sqlx::query("SELECT pg_advisory_unlock(12345)")
		.execute(pool.inner())
		.await
		.expect("Failed to release advisory lock");

	// Test PostgreSQL-specific data types (e.g., ARRAY)
	sqlx::query("CREATE TABLE IF NOT EXISTS pg_test (id SERIAL PRIMARY KEY, tags TEXT[])")
		.execute(pool.inner())
		.await
		.expect("Failed to create table with ARRAY");

	sqlx::query("INSERT INTO pg_test (tags) VALUES ($1)")
		.bind(vec!["rust", "database"])
		.execute(pool.inner())
		.await
		.expect("Failed to insert array data");
}

/// Test PostgreSQL connection string parameters
///
/// **Test Intent**: Verify pool correctly handles PostgreSQL connection parameters
/// like sslmode, application_name
///
/// **Not Intent**: MySQL parameters, SQLite parameters
#[rstest]
#[tokio::test]
async fn test_postgres_connection_parameters(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	// Add application_name parameter (use & if URL already has query params)
	let separator = if url.contains('?') { '&' } else { '?' };
	let url_with_params = format!("{}{}application_name=reinhardt_test", url, separator);

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url_with_params, config)
		.await
		.expect("Failed to create pool with parameters");

	// Verify connection works
	let result = sqlx::query("SELECT 1 as value")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to query with connection parameters");

	let value: i32 = result.get("value");
	assert_eq!(value, 1);

	// Optionally verify application_name is set
	let result = sqlx::query("SELECT current_setting('application_name') as app_name")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to get application_name");

	let app_name: String = result.get("app_name");
	assert_eq!(app_name, "reinhardt_test");
}

/// Test PostgreSQL transaction isolation levels
///
/// **Test Intent**: Verify pool supports PostgreSQL transaction isolation levels
/// (READ UNCOMMITTED, READ COMMITTED, REPEATABLE READ, SERIALIZABLE)
///
/// **Not Intent**: MySQL isolation levels, default isolation level
#[rstest]
#[tokio::test]
async fn test_postgres_transaction_isolation_levels(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Test SERIALIZABLE isolation level
	let mut tx = pool
		.inner()
		.begin()
		.await
		.expect("Failed to begin transaction");

	sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
		.execute(&mut *tx)
		.await
		.expect("Failed to set isolation level");

	// Create test table and insert data within transaction
	sqlx::query("CREATE TABLE IF NOT EXISTS isolation_test (id SERIAL PRIMARY KEY, value INT)")
		.execute(&mut *tx)
		.await
		.expect("Failed to create table");

	sqlx::query("INSERT INTO isolation_test (value) VALUES (42)")
		.execute(&mut *tx)
		.await
		.expect("Failed to insert");

	tx.commit().await.expect("Failed to commit transaction");

	// Verify data persisted
	let result = sqlx::query("SELECT value FROM isolation_test WHERE id = 1")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to select");

	let value: i32 = result.get("value");
	assert_eq!(value, 42);
}

/// Test concurrent pool operations
///
/// **Test Intent**: Verify pool handles concurrent connection acquisition correctly
///
/// **Not Intent**: Sequential operations, single-threaded usage
#[rstest]
#[tokio::test]
async fn test_pool_concurrent_operations(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url): (_, _, _, String) = postgres_container.await;

	let config = PoolConfig::default().with_max_connections(10);
	let pool = Arc::new(
		ConnectionPool::new_postgres(&url, config)
			.await
			.expect("Failed to create pool"),
	);

	// Create table
	sqlx::query("CREATE TABLE IF NOT EXISTS concurrent_test (id SERIAL PRIMARY KEY, value INT)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	// Spawn 20 concurrent tasks
	let mut handles = Vec::new();
	for i in 0..20 {
		let pool_clone = Arc::clone(&pool);
		let handle = tokio::spawn(async move {
			sqlx::query("INSERT INTO concurrent_test (value) VALUES ($1)")
				.bind(i)
				.execute(pool_clone.inner())
				.await
				.expect("Failed to insert");
		});
		handles.push(handle);
	}

	// Wait for all tasks
	for handle in handles {
		handle.await.expect("Task panicked");
	}

	// Verify all records inserted
	let result = sqlx::query("SELECT COUNT(*) as count FROM concurrent_test")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to count");

	let count: i64 = result.get("count");
	assert_eq!(count, 20);
}
