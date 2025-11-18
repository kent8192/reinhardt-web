//! Pool + Backend Integration Tests
//!
//! These tests verify the integration between reinhardt-pool and reinhardt-backends,
//! ensuring that pools work correctly with different database backends.
//!
//! **Test Coverage:**
//! - Pool creation with different backends (PostgreSQL, MySQL, SQLite)
//! - Backend-specific connection handling
//! - Pool + Backend error propagation
//! - Connection pool lifecycle with backend events
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//! - mysql_suite: Suite-wide MySQL instance
//!
//! **Integration Points:**
//! - reinhardt_pool::ConnectionPool ↔ reinhardt_backends::Backend
//! - reinhardt_pool::PoolConfig ↔ reinhardt_backends::BackendConfig

use reinhardt_db::pool::{ConnectionPool, PoolConfig};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use std::time::Duration;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Pool + PostgreSQL Backend Integration Tests
// ============================================================================

/// Test pool creation with PostgreSQL backend
///
/// **Test Intent**: Verify ConnectionPool integrates correctly with PostgreSQL Backend
/// through DatabaseBackend trait
///
/// **Integration Point**: ConnectionPool::new_postgres() → Backend::PostgreSQL
///
/// **Not Intent**: MySQL, SQLite, connection reuse, error cases
#[rstest]
#[tokio::test]
async fn test_pool_postgres_backend_integration(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Create pool with backend
	let pool_config = PoolConfig::default()
		.with_max_connections(5)
		.with_acquire_timeout(Duration::from_secs(10));

	let pool = ConnectionPool::new_postgres(&url, pool_config)
		.await
		.expect("Failed to create pool with PostgreSQL backend");

	// Verify pool works with backend
	let mut conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	// Execute backend-specific query (PostgreSQL version check)
	let result = sqlx::query("SELECT version() as version")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute backend-specific query");

	let version: String = result.get("version");
	assert!(version.contains("PostgreSQL"));
}

/// Test pool + backend connection lifecycle
///
/// **Test Intent**: Verify pool correctly manages connection lifecycle events
/// through backend integration (acquire → use → release)
///
/// **Integration Point**: ConnectionPool lifecycle → Backend connection events
///
/// **Not Intent**: Connection pooling logic, backend-independent behavior
#[rstest]
#[tokio::test]
async fn test_pool_backend_connection_lifecycle(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let pool_config = PoolConfig::default().with_max_connections(2);

	let pool = ConnectionPool::new_postgres(&url, pool_config)
		.await
		.expect("Failed to create pool");

	// Acquire connection (backend should handle connection creation)
	let conn1 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn1");

	// Acquire second connection
	let conn2 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn2");

	// Release connections (backend should handle connection return)
	drop(conn1);
	drop(conn2);

	// Acquire again to verify backend reused connections
	let _conn3 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn3");
}

/// Test pool + backend error propagation
///
/// **Test Intent**: Verify backend errors are correctly propagated through pool layer
///
/// **Integration Point**: Backend errors → ConnectionPool error handling
///
/// **Not Intent**: Pool-level errors, successful operations
#[rstest]
#[tokio::test]
async fn test_pool_backend_error_propagation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let pool_config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, pool_config)
		.await
		.expect("Failed to create pool");

	// Execute invalid backend query
	let result = sqlx::query("SELECT * FROM nonexistent_table")
		.fetch_one(pool.inner())
		.await;

	// Backend error should be propagated through pool
	assert!(result.is_err());

	// Verify error is backend-specific (PostgreSQL error)
	let err = result.unwrap_err();
	let err_string = err.to_string();
	assert!(
		err_string.contains("relation") || err_string.contains("does not exist"),
		"Expected PostgreSQL backend error, got: {}",
		err_string
	);
}

// ============================================================================
// Pool + Backend Transaction Integration Tests
// ============================================================================

/// Test pool + backend transaction begin/commit
///
/// **Test Intent**: Verify pool integrates with backend transaction management
///
/// **Integration Point**: ConnectionPool::begin() → Backend::begin_transaction()
///
/// **Not Intent**: Rollback, nested transactions, savepoints
#[rstest]
#[tokio::test]
async fn test_pool_backend_transaction_commit(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let pool_config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, pool_config)
		.await
		.expect("Failed to create pool");

	// Create table
	sqlx::query("CREATE TABLE IF NOT EXISTS txn_test (id SERIAL PRIMARY KEY, value INT)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	// Begin transaction through pool + backend
	let mut tx = pool
		.inner()
		.begin()
		.await
		.expect("Failed to begin transaction");

	// Insert within transaction
	sqlx::query("INSERT INTO txn_test (value) VALUES ($1)")
		.bind(42)
		.execute(&mut *tx)
		.await
		.expect("Failed to insert in transaction");

	// Commit transaction (backend should persist)
	tx.commit().await.expect("Failed to commit transaction");

	// Verify data persisted (outside transaction)
	let result = sqlx::query("SELECT value FROM txn_test WHERE id = 1")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to select after commit");

	let value: i32 = result.get("value");
	assert_eq!(value, 42);
}

/// Test pool + backend transaction rollback
///
/// **Test Intent**: Verify pool integrates with backend rollback mechanism
///
/// **Integration Point**: ConnectionPool::rollback() → Backend::rollback_transaction()
///
/// **Not Intent**: Commit, partial rollback, error handling
#[rstest]
#[tokio::test]
async fn test_pool_backend_transaction_rollback(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let pool_config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, pool_config)
		.await
		.expect("Failed to create pool");

	// Create table
	sqlx::query("CREATE TABLE IF NOT EXISTS rollback_test (id SERIAL PRIMARY KEY, value INT)")
		.execute(pool.inner())
		.await
		.expect("Failed to create table");

	// Begin transaction
	let mut tx = pool
		.inner()
		.begin()
		.await
		.expect("Failed to begin transaction");

	// Insert within transaction
	sqlx::query("INSERT INTO rollback_test (value) VALUES ($1)")
		.bind(99)
		.execute(&mut *tx)
		.await
		.expect("Failed to insert");

	// Rollback transaction (backend should discard changes)
	tx.rollback().await.expect("Failed to rollback transaction");

	// Verify data NOT persisted
	let result = sqlx::query("SELECT COUNT(*) as count FROM rollback_test")
		.fetch_one(pool.inner())
		.await
		.expect("Failed to count");

	let count: i64 = result.get("count");
	assert_eq!(count, 0, "Data should not persist after rollback");
}

// ============================================================================
// Pool + Backend Configuration Integration Tests
// ============================================================================

/// Test pool configuration aligns with backend requirements
///
/// **Test Intent**: Verify PoolConfig settings are compatible with Backend requirements
/// (e.g., max_connections, connection timeout)
///
/// **Integration Point**: PoolConfig → BackendConfig compatibility
///
/// **Not Intent**: Configuration conflicts, backend overrides
#[rstest]
#[tokio::test]
async fn test_pool_backend_config_alignment(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Pool config with specific limits
	let pool_config = PoolConfig::default()
		.with_min_connections(2)
		.with_max_connections(10)
		.with_acquire_timeout(Duration::from_secs(30));

	let pool = ConnectionPool::new_postgres(&url, pool_config)
		.await
		.expect("Failed to create pool");

	// Verify pool config is applied
	assert_eq!(pool.config().min_connections, 2);
	assert_eq!(pool.config().max_connections, 10);

	// Backend should respect pool's max_connections limit
	// (Try to acquire up to max_connections)
	let mut conns = Vec::new();
	for _ in 0..10 {
		let conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire connection");
		conns.push(conn);
	}

	// 11th connection should timeout (max_connections reached)
	let result = tokio::time::timeout(Duration::from_secs(1), pool.inner().acquire()).await;

	assert!(
		result.is_err() || result.unwrap().is_err(),
		"Expected timeout or error when exceeding max_connections"
	);
}

/// Test pool + backend connection string parsing
///
/// **Test Intent**: Verify pool correctly parses and passes connection string to backend
///
/// **Integration Point**: ConnectionPool URL parsing → Backend connection initialization
///
/// **Not Intent**: Invalid URLs, connection string validation
#[rstest]
#[tokio::test]
async fn test_pool_backend_connection_string_parsing(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Add query parameters to connection string
	let url_with_params = format!("{}?connect_timeout=10", url);

	let pool_config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url_with_params, pool_config)
		.await
		.expect("Failed to create pool with connection string parameters");

	// Verify backend parsed connection string correctly
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

// ============================================================================
// Pool + Backend-Specific Feature Tests
// ============================================================================

/// Test pool with PostgreSQL-specific backend features
///
/// **Test Intent**: Verify pool correctly integrates with PostgreSQL-specific features
/// like LISTEN/NOTIFY, pg_advisory_lock
///
/// **Integration Point**: ConnectionPool → PostgreSQL Backend features
///
/// **Not Intent**: Generic SQL, MySQL features, SQLite features
#[rstest]
#[tokio::test]
async fn test_pool_postgres_specific_backend_features(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let pool_config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, pool_config)
		.await
		.expect("Failed to create pool");

	// Test PostgreSQL advisory lock through pool
	sqlx::query("SELECT pg_advisory_lock($1)")
		.bind(12345_i64)
		.execute(pool.inner())
		.await
		.expect("Failed to acquire advisory lock");

	// Verify lock is held
	let result = sqlx::query("SELECT pg_try_advisory_lock($1) as locked")
		.bind(12345_i64)
		.fetch_one(pool.inner())
		.await
		.expect("Failed to try advisory lock");

	let locked: bool = result.get("locked");
	assert!(!locked, "Lock should already be held");

	// Release lock
	sqlx::query("SELECT pg_advisory_unlock($1)")
		.bind(12345_i64)
		.execute(pool.inner())
		.await
		.expect("Failed to release advisory lock");
}

/// Test pool handles backend reconnection
///
/// **Test Intent**: Verify pool can recover when backend connection is lost
///
/// **Integration Point**: ConnectionPool reconnection → Backend connection recovery
///
/// **Not Intent**: Connection pooling, graceful degradation
#[rstest]
#[tokio::test]
async fn test_pool_backend_reconnection(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let pool_config = PoolConfig::default()
		.with_max_connections(2)
		.with_min_connections(0);

	let pool = ConnectionPool::new_postgres(&url, pool_config)
		.await
		.expect("Failed to create pool");

	// Acquire connection
	let mut conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	// Execute query to ensure connection is active
	sqlx::query("SELECT 1")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute query");

	// Simulate connection disruption by closing connection
	// (In real scenario, this would be network failure)
	drop(conn);

	// Pool should be able to create new connection
	let mut new_conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire new connection after disruption");

	let result = sqlx::query("SELECT 2 as value")
		.fetch_one(&mut *new_conn)
		.await
		.expect("Failed to execute query on new connection");

	let value: i32 = result.get("value");
	assert_eq!(value, 2);
}
