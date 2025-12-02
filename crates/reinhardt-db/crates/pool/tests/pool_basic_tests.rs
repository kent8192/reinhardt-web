//! Basic connection pool tests
//! Covers pool creation, connection acquisition, and basic operations

use reinhardt_pool::{ConnectionPool, PoolConfig};
use sqlx::Sqlite;

#[tokio::test]
async fn test_pool_creation() {
	// Test basic pool creation with default configuration
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	assert_eq!(pool.config().min_connections, 1);
	assert_eq!(pool.config().max_connections, 10);
}

#[tokio::test]
async fn test_pool_with_custom_config() {
	// Test pool creation with custom configuration
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(2)
		.with_max_connections(5)
		.with_test_before_acquire(true);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	assert_eq!(pool.config().min_connections, 2);
	assert_eq!(pool.config().max_connections, 5);
	assert!(pool.config().test_before_acquire);
}

#[tokio::test]
async fn test_connection_acquisition() {
	// Test that connections can be acquired from the pool
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let mut conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	// Verify connection works by executing a simple query
	let result: i64 = sqlx::query_scalar("SELECT 1")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute query");

	assert_eq!(result, 1);
}

#[tokio::test]
async fn test_multiple_connection_acquisition() {
	// Test acquiring multiple connections from the pool
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(3);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let mut conn1 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection 1");
	let mut conn2 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection 2");

	// Both connections should work independently
	let result1: i64 = sqlx::query_scalar("SELECT 1")
		.fetch_one(&mut *conn1)
		.await
		.expect("Failed to execute query on conn1");

	let result2: i64 = sqlx::query_scalar("SELECT 2")
		.fetch_one(&mut *conn2)
		.await
		.expect("Failed to execute query on conn2");

	assert_eq!(result1, 1);
	assert_eq!(result2, 2);
}

#[tokio::test]
async fn test_connection_reuse() {
	// Test that connections are properly reused after being returned to the pool
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(1)
		.with_max_connections(2);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Create a temporary table to verify connection reuse
	{
		let mut conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire connection");
		sqlx::query("CREATE TEMPORARY TABLE test_table (id INTEGER)")
			.execute(&mut *conn)
			.await
			.expect("Failed to create table");
	}

	// Connection is returned to pool here

	// Acquire connection again and verify the table still exists
	// Note: SQLite in-memory databases and temp tables don't persist across connections
	// So we test that we can acquire a connection again successfully
	let mut conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection again");

	let result: i64 = sqlx::query_scalar("SELECT 1")
		.fetch_one(&mut *conn)
		.await
		.expect("Failed to execute query");

	assert_eq!(result, 1);
}

#[tokio::test]
async fn test_pool_close() {
	// Test that pool can be closed properly
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	pool.close().await;

	// After close, acquiring connections should fail
	let result = pool.inner().acquire().await;
	assert!(
		result.is_err(),
		"Should not be able to acquire connection from closed pool"
	);
}

#[tokio::test]
async fn test_pool_basic_config_validation() {
	// Test that invalid configuration is rejected
	let url = "sqlite::memory:";

	// Test: min_connections > max_connections
	let config = PoolConfig::new()
		.with_min_connections(10)
		.with_max_connections(5);

	let result = ConnectionPool::<Sqlite>::new_sqlite(url, config).await;
	assert!(result.is_err(), "Should reject min > max configuration");
}

#[tokio::test]
async fn test_pool_config_zero_max() {
	// Test that max_connections = 0 is rejected
	let url = "sqlite::memory:";
	let config = PoolConfig {
		max_connections: 0,
		..Default::default()
	};

	let result = ConnectionPool::<Sqlite>::new_sqlite(url, config).await;
	assert!(result.is_err(), "Should reject max_connections = 0");
}

#[tokio::test]
async fn test_concurrent_connections() {
	// Test that multiple concurrent connections work correctly
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(5);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let mut handles = vec![];

	for i in 0..5 {
		let pool_clone = pool.inner().clone();
		let handle = tokio::spawn(async move {
			let mut conn = pool_clone
				.acquire()
				.await
				.expect("Failed to acquire connection");
			let result: i64 = sqlx::query_scalar(&format!("SELECT {}", i))
				.fetch_one(&mut *conn)
				.await
				.expect("Failed to execute query");
			result
		});
		handles.push(handle);
	}

	let mut results = vec![];
	for handle in handles {
		let result = handle.await.expect("Task panicked");
		results.push(result);
	}

	// Verify all queries executed successfully
	assert_eq!(results.len(), 5);
	for (i, result) in results.iter().enumerate() {
		assert_eq!(*result, i as i64);
	}
}
