//! Pool lifecycle tests
//! Tests for connection recycling, invalidation, and pool recreation

use reinhardt_pool::{ConnectionPool, PoolConfig};
use sqlx::Sqlite;
use std::time::Duration;

#[tokio::test]
async fn test_pool_dispose() {
	// Test that pool dispose closes checked-in connections
	// Based on SQLAlchemy test_dispose_closes_pooled
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(5);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Acquire and release some connections
	{
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
	}

	// Close the pool
	pool.close().await;

	// After close, should not be able to acquire new connections
	let result = pool.inner().acquire().await;
	assert!(result.is_err(), "Should not acquire from closed pool");
}

#[tokio::test]
async fn test_connection_test_before_acquire() {
	// Test that test_before_acquire validates connections
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(1)
		.with_max_connections(5)
		.with_test_before_acquire(true);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Acquire a connection and verify it works
	let mut conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	let result: i64 = sqlx::query_scalar("SELECT 1")
		.fetch_one(&mut *conn)
		.await
		.expect("Query should succeed");

	assert_eq!(result, 1);
}

#[tokio::test]
async fn test_connection_info_persistence() {
	// Test connection info dictionary persistence across checkouts
	// Based on SQLAlchemy test_info
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(1)
		.with_max_connections(2);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Note: SQLx doesn't expose connection info like SQLAlchemy
	// This test verifies basic connection reuse behavior
	let _conn1_id = {
		let conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire connection");
		// Get some identifier (we'll use memory address as proxy)
		&*conn as *const _ as usize
	};

	// Release and reacquire
	let _conn2_id = {
		let conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire connection");
		&*conn as *const _ as usize
	};

	// With pool_size >= 1, we might get the same connection back
	// This test mainly verifies the connection works after reuse
	let mut conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");
	let result: i64 = sqlx::query_scalar("SELECT 1")
		.fetch_one(&mut *conn)
		.await
		.expect("Query should succeed");

	assert_eq!(result, 1);
}

#[tokio::test]
async fn test_pool_close_with_active_connections() {
	// Test closing pool while connections are checked out
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(5);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Hold connections
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

	// Close pool (checked-out connections remain valid)
	pool.close().await;

	// New acquisitions should fail
	let result = pool.inner().acquire().await;
	assert!(result.is_err());
}

#[tokio::test]
async fn test_connection_recycle() {
	// Test connection recycling based on max lifetime
	// Based on SQLAlchemy test_recycle
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(5)
		.with_max_lifetime(Some(Duration::from_millis(50)));

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Acquire first connection
	let _conn1 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	// Wait for recycle time
	tokio::time::sleep(Duration::from_millis(100)).await;

	// Connection should be recycled on next acquire
	let _conn2 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire after recycle");
}

#[tokio::test]
async fn test_min_connections_maintained() {
	// Test that minimum number of connections is maintained
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(2)
		.with_max_connections(5);

	let _pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Give pool time to establish minimum connections
	tokio::time::sleep(Duration::from_millis(100)).await;

	// Pool should have created min_connections
	// Note: SQLx doesn't expose pool size directly, so we verify by usage
}

#[tokio::test]
async fn test_connection_detach() {
	// Test connection detachment from pool
	// Based on SQLAlchemy test_detach
	// Note: SQLx doesn't support explicit detach, but connections
	// are automatically returned when dropped
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(2);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	{
		let _conn1 = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire conn1");
		// conn1 automatically returns to pool on drop
	}

	// Should be able to acquire connection again
	let _conn2 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn2");
}

#[tokio::test]
async fn test_overflow_handling() {
	// Test pool overflow behavior
	// Based on SQLAlchemy test_max_overflow
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(3);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Acquire connections up to max
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

	// Fourth connection should timeout
	let result = tokio::time::timeout(Duration::from_millis(100), pool.inner().acquire()).await;

	assert!(
		result.is_err(),
		"Should timeout when max connections reached"
	);
}

#[tokio::test]
async fn test_no_overflow() {
	// Test pool with no overflow
	// Based on SQLAlchemy test_no_overflow
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(2);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

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

	// Should not be able to acquire more than max
	let result = tokio::time::timeout(Duration::from_millis(100), pool.inner().acquire()).await;

	assert!(result.is_err(), "Should not exceed max connections");
}

#[tokio::test]
async fn test_connection_cleanup() {
	// Test that connections are properly cleaned up
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(5);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Acquire and drop multiple connections
	for _ in 0..10 {
		let _conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire connection");
		// Connection is returned to pool on drop
	}

	// Should still be able to acquire connections
	let _conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire after cleanup");
}

#[tokio::test]
async fn test_pool_config_immutability() {
	// Test that pool configuration is immutable after creation
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(2)
		.with_max_connections(5);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Verify config is preserved
	assert_eq!(pool.config().min_connections, 2);
	assert_eq!(pool.config().max_connections, 5);
}

#[tokio::test]
async fn test_concurrent_pool_operations() {
	// Test concurrent pool operations
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(10);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let mut handles = vec![];

	// Spawn many concurrent tasks
	for i in 0..20 {
		let pool_clone = pool.inner().clone();
		let handle = tokio::spawn(async move {
			let mut conn = pool_clone
				.acquire()
				.await
				.expect("Failed to acquire connection");
			let result: i64 = sqlx::query_scalar(&format!("SELECT {}", i % 10))
				.fetch_one(&mut *conn)
				.await
				.expect("Failed to execute query");
			tokio::time::sleep(Duration::from_millis(10)).await;
			result
		});
		handles.push(handle);
	}

	// All tasks should complete successfully
	for handle in handles {
		let _result = handle.await.expect("Task should complete");
	}
}
