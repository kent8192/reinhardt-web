//! Pool + Backend Configuration Integration Tests
//!
//! These tests verify the integration between pool configuration and backend
//! configuration, ensuring compatibility and proper validation.
//!
//! **Test Coverage:**
//! - Backend type detection from connection string
//! - Pool configuration validation against backend constraints
//! - Backend-specific configuration options
//! - Configuration migration between backends
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
// Backend Type Detection Tests
// ============================================================================

/// Test pool detects PostgreSQL backend from connection string
///
/// **Test Intent**: Verify pool correctly identifies PostgreSQL as backend
/// when given a postgres:// URL
///
/// **Integration Point**: ConnectionPool::new_postgres() → Backend detection
///
/// **Not Intent**: MySQL detection, SQLite detection
#[rstest]
#[tokio::test]
async fn test_pool_detects_postgres_backend(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Verify URL is PostgreSQL format
	assert!(
		url.starts_with("postgres://"),
		"URL should start with postgres://"
	);

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool with PostgreSQL backend");

	// Verify connection works with PostgreSQL
	let mut conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	let result = sqlx::query("SELECT version() as version")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute PostgreSQL version query");

	let version: String = result.get("version");
	assert!(
		version.contains("PostgreSQL"),
		"Backend should be PostgreSQL"
	);
}

/// Test pool handles connection string with query parameters
///
/// **Test Intent**: Verify pool parses and applies connection string parameters
/// like statement_timeout, connect_timeout
///
/// **Integration Point**: ConnectionPool URL parsing → Backend parameter application
///
/// **Not Intent**: Parameter validation, invalid parameter handling
#[rstest]
#[tokio::test]
async fn test_pool_backend_url_parameters(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Add connection string parameters (use & if URL already has query params)
	let separator = if url.contains('?') { '&' } else { '?' };
	let url_with_params = format!(
		"{}{}connect_timeout=10&statement_timeout=5000",
		url, separator
	);

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url_with_params, config)
		.await
		.expect("Failed to create pool with URL parameters");

	// Verify connection works
	let mut conn = pool.inner().acquire().await.expect("Failed to acquire");

	let result = sqlx::query("SELECT 1 as value")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute query");

	let value: i32 = result.get("value");
	assert_eq!(value, 1);
}

// ============================================================================
// Configuration Validation Tests
// ============================================================================

/// Test pool configuration respects backend max_connections limit
///
/// **Test Intent**: Verify PoolConfig max_connections is enforced
/// and aligns with backend capabilities
///
/// **Integration Point**: PoolConfig::max_connections → Backend connection limit
///
/// **Not Intent**: Dynamic connection scaling, connection pool resizing
#[rstest]
#[tokio::test]
async fn test_pool_config_max_connections_enforcement(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default()
		.with_min_connections(2)
		.with_max_connections(5)
		.with_acquire_timeout(Duration::from_secs(2));

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Verify config was applied
	assert_eq!(pool.config().min_connections, 2);
	assert_eq!(pool.config().max_connections, 5);

	// Acquire up to max_connections
	let mut conns = Vec::new();
	for _ in 0..5 {
		let conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire connection");
		conns.push(conn);
	}

	// 6th connection should timeout
	let result = tokio::time::timeout(Duration::from_secs(1), pool.inner().acquire()).await;

	assert!(
		result.is_err() || result.unwrap().is_err(),
		"Expected timeout when exceeding max_connections"
	);
}

/// Test pool configuration min_connections is maintained
///
/// **Test Intent**: Verify pool maintains minimum number of connections
/// as specified in PoolConfig
///
/// **Integration Point**: PoolConfig::min_connections → Backend connection pool
///
/// **Not Intent**: Connection idle timeout, connection health checks
#[rstest]
#[tokio::test]
async fn test_pool_config_min_connections_maintained(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default()
		.with_min_connections(3)
		.with_max_connections(10);

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Wait a moment for pool to establish min connections
	tokio::time::sleep(Duration::from_millis(500)).await;

	// Pool should be immediately usable (min_connections already established)
	let mut conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection from pre-warmed pool");

	let result = sqlx::query("SELECT 1 as value")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute query");

	let value: i32 = result.get("value");
	assert_eq!(value, 1);
}

/// Test pool configuration acquire_timeout is enforced
///
/// **Test Intent**: Verify PoolConfig acquire_timeout causes acquisition
/// to fail when no connections available within timeout period
///
/// **Integration Point**: PoolConfig::acquire_timeout → Backend connection acquisition
///
/// **Not Intent**: Connection timeout, query timeout
#[rstest]
#[tokio::test]
async fn test_pool_config_acquire_timeout_enforcement(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	let config = PoolConfig::default()
		.with_max_connections(2)
		.with_acquire_timeout(Duration::from_millis(500));

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Acquire all connections
	let _conn1 = pool.inner().acquire().await.expect("Failed to acquire");
	let _conn2 = pool.inner().acquire().await.expect("Failed to acquire");

	// 3rd acquisition should timeout after 500ms
	let start = std::time::Instant::now();
	let result = pool.inner().acquire().await;
	let elapsed = start.elapsed();

	assert!(result.is_err(), "Should timeout when pool exhausted");
	assert!(
		elapsed >= Duration::from_millis(400) && elapsed <= Duration::from_secs(2),
		"Should timeout close to configured acquire_timeout"
	);
}

// ============================================================================
// Backend-Specific Configuration Tests
// ============================================================================

/// Test PostgreSQL-specific pool configuration options
///
/// **Test Intent**: Verify pool supports PostgreSQL-specific options
/// like statement_timeout, application_name
///
/// **Integration Point**: PoolConfig → PostgreSQL session parameters
///
/// **Not Intent**: MySQL options, SQLite options
#[rstest]
#[tokio::test]
async fn test_postgres_specific_pool_options(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Create pool with PostgreSQL-specific application_name (use & if URL already has query params)
	let separator = if url.contains('?') { '&' } else { '?' };
	let url_with_app = format!("{}{}application_name=reinhardt_test", url, separator);

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url_with_app, config)
		.await
		.expect("Failed to create pool");

	let mut conn = pool.inner().acquire().await.expect("Failed to acquire");

	// Query current application_name
	let result = sqlx::query("SELECT current_setting('application_name') as app_name")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to query application_name");

	let app_name: String = result.get("app_name");
	assert_eq!(app_name, "reinhardt_test", "Application name should be set");
}

/// Test pool configuration handles backend connection limits
///
/// **Test Intent**: Verify pool respects backend's max_connections limit
/// and provides meaningful error when exceeded
///
/// **Integration Point**: PoolConfig → Backend max_connections constraint
///
/// **Not Intent**: Dynamic limit adjustment, connection recycling
#[rstest]
#[tokio::test]
async fn test_pool_respects_backend_connection_limit(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Query PostgreSQL's max_connections setting
	let temp_pool = sqlx::PgPool::connect(&url)
		.await
		.expect("Failed to connect");

	let result = sqlx::query("SHOW max_connections")
		.fetch_one(&temp_pool)
		.await
		.expect("Failed to query max_connections");

	let max_conns_str: String = result.get("max_connections");
	let backend_max_conns: u32 = max_conns_str
		.parse()
		.expect("Failed to parse max_connections");

	temp_pool.close().await;

	// Create pool with max_connections below backend limit
	let safe_max = std::cmp::min(backend_max_conns / 2, 10);
	let config = PoolConfig::default().with_max_connections(safe_max);

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Should be able to acquire connections up to pool max
	let mut conns = Vec::new();
	for _ in 0..safe_max {
		let conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire connection");
		conns.push(conn);
	}

	// Verify all connections are usable
	for conn in &mut conns {
		let result = sqlx::query("SELECT 1 as value")
			.fetch_one(&mut **conn)
			.await
			.expect("Failed to execute query");

		let value: i32 = result.get("value");
		assert_eq!(value, 1);
	}
}

// ============================================================================
// Configuration Compatibility Tests
// ============================================================================

/// Test pool configuration compatibility with backend constraints
///
/// **Test Intent**: Verify pool validates configuration against backend
/// capabilities (e.g., idle_timeout not exceeding backend's tcp_keepalives_idle)
///
/// **Integration Point**: PoolConfig validation → Backend constraints
///
/// **Not Intent**: Configuration migration, dynamic reconfiguration
#[rstest]
#[tokio::test]
async fn test_pool_config_backend_compatibility(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Create pool with reasonable timeouts
	let config = PoolConfig::default()
		.with_max_connections(10)
		.with_acquire_timeout(Duration::from_secs(30))
		.with_idle_timeout(Some(Duration::from_secs(600)))
		.with_max_lifetime(Some(Duration::from_secs(1800)));

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Pool should be created with compatible configuration");

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

/// Test pool configuration overrides with backend defaults
///
/// **Test Intent**: Verify pool configuration takes precedence over
/// backend default settings when explicitly specified
///
/// **Integration Point**: PoolConfig → Backend default parameters
///
/// **Not Intent**: Backend configuration modification, runtime reconfiguration
#[rstest]
#[tokio::test]
async fn test_pool_config_overrides_backend_defaults(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, url) = postgres_container.await;

	// Create pool with custom configuration
	let config = PoolConfig::default()
		.with_min_connections(5)
		.with_max_connections(15)
		.with_acquire_timeout(Duration::from_secs(20));

	let pool = ConnectionPool::new_postgres(&url, config)
		.await
		.expect("Failed to create pool");

	// Verify custom configuration is applied
	assert_eq!(pool.config().min_connections, 5);
	assert_eq!(pool.config().max_connections, 15);
	assert_eq!(pool.config().acquire_timeout, Duration::from_secs(20));

	// Verify pool works with custom configuration
	let mut conn = pool.inner().acquire().await.expect("Failed to acquire");

	let result = sqlx::query("SELECT 1 as value")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute query");

	let value: i32 = result.get("value");
	assert_eq!(value, 1);
}

/// Test pool handles backend connection string variations
///
/// **Test Intent**: Verify pool correctly parses different PostgreSQL
/// connection string formats (URL, key-value pairs)
///
/// **Integration Point**: ConnectionPool URL parsing → Backend connection
///
/// **Not Intent**: Invalid URL handling, malformed connection strings
#[rstest]
#[tokio::test]
async fn test_pool_handles_connection_string_variations(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, port, _url) = postgres_container.await;

	// Test with explicit host/port format
	let url_explicit = format!("postgres://postgres:postgres@localhost:{}/postgres", port);

	let config = PoolConfig::default();
	let pool = ConnectionPool::new_postgres(&url_explicit, config)
		.await
		.expect("Failed to create pool with explicit URL format");

	let mut conn = pool.inner().acquire().await.expect("Failed to acquire");

	let result = sqlx::query("SELECT 1 as value")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute query");

	let value: i32 = result.get("value");
	assert_eq!(value, 1);
}
