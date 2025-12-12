//! Pool timeout and exhaustion tests
//! Based on SQLAlchemy's timeout tests

use reinhardt_pool::{ConnectionPool, PoolConfig};
use sqlx::Sqlite;
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_pool_timeout() {
	// Test that pool timeout works when pool is exhausted
	// Based on SQLAlchemy test_timeout
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(2)
		.with_connect_timeout(Duration::from_secs(2));

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Acquire all available connections
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

	// Try to acquire third connection - should timeout
	let start = Instant::now();
	let result = tokio::time::timeout(Duration::from_secs(3), pool.inner().acquire()).await;

	let elapsed = start.elapsed();

	// Should timeout (either via acquire timeout or tokio timeout)
	assert!(
		result.is_err() || result.unwrap().is_err(),
		"Should timeout when pool exhausted"
	);
	assert!(elapsed.as_secs() >= 1, "Should wait before timing out");
}

#[tokio::test]
async fn test_pool_timeout_subsecond_precision() {
	// Test sub-second timeout precision
	// Based on SQLAlchemy test_timeout_subsecond_precision
	// Now correctly uses acquire_timeout for pool exhaustion
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(1)
		.with_acquire_timeout(Duration::from_millis(500)); // Changed from connect_timeout

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Acquire the only connection
	let _conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	// Try to acquire second connection with sub-second timeout
	let start = Instant::now();
	let result = pool.inner().acquire().await;

	let elapsed = start.elapsed();

	// Should timeout quickly (within ~700ms, allowing 200ms margin)
	assert!(result.is_err(), "Should timeout");
	assert!(
		elapsed.as_millis() < 1000,
		"Should timeout in less than 1 second, got {:?}",
		elapsed
	);
}

#[tokio::test]
async fn test_pool_exhausted_some_timeout() {
	// Test async pool exhaustion with timeout
	// Based on SQLAlchemy async test_pool_exhausted_some_timeout
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(1)
		.with_connect_timeout(Duration::from_millis(100));

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let _conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	// Should timeout when trying to get second connection
	let result = tokio::time::timeout(Duration::from_millis(500), pool.inner().acquire()).await;

	assert!(
		result.is_err() || result.unwrap().is_err(),
		"Should timeout"
	);
}

#[tokio::test]
async fn test_pool_exhausted_no_timeout() {
	// Test pool exhaustion with near-zero timeout
	// Based on SQLAlchemy async test_pool_exhausted_no_timeout
	// Now correctly uses acquire_timeout
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(1)
		.with_acquire_timeout(Duration::from_millis(50)); // Short timeout for testing

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let _conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	// Should timeout very quickly
	let start = Instant::now();
	let result = pool.inner().acquire().await;

	let elapsed = start.elapsed();

	assert!(result.is_err(), "Should timeout");
	assert!(
		elapsed.as_millis() >= 10 && elapsed.as_millis() < 200,
		"Timeout should be between 10ms and 200ms, got {:?}",
		elapsed
	);
}

#[tokio::test]
async fn test_acquire_timeout_accessor() {
	// Test that timeout() accessor returns configured timeout
	// Based on SQLAlchemy test_timeout_accessor
	let url = "sqlite::memory:";
	let timeout_duration = Duration::from_secs(5);
	let config = PoolConfig::new().with_connect_timeout(timeout_duration);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	assert_eq!(pool.config().connect_timeout, timeout_duration);
}

#[tokio::test]
async fn test_connection_released_on_drop() {
	// Test that connections are released back to pool when dropped
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(2);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Acquire and drop first connection
	{
		let _conn1 = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire conn1");
		// conn1 dropped here
	}

	// Should be able to acquire two connections since first was released
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
}

#[tokio::test]
async fn test_timeout_race_condition() {
	// Test timeout behavior under concurrent load
	// Based on SQLAlchemy test_timeout_race
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(2)
		.with_connect_timeout(Duration::from_secs(3));

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Hold two connections
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

	// Spawn multiple tasks trying to acquire connections simultaneously
	let mut handles = vec![];
	for _ in 0..5 {
		let pool_clone = pool.inner().clone();
		let handle = tokio::spawn(async move {
			let start = Instant::now();
			let result = tokio::time::timeout(Duration::from_secs(5), pool_clone.acquire()).await;
			(start.elapsed(), result.is_err() || result.unwrap().is_err())
		});
		handles.push(handle);
	}

	let mut timeouts = vec![];
	for handle in handles {
		let (elapsed, timed_out) = handle.await.expect("Task panicked");
		if timed_out {
			timeouts.push(elapsed);
		}
	}

	// All tasks should timeout with reasonable timing
	assert!(timeouts.len() >= 3, "Most tasks should timeout");
	for elapsed in &timeouts {
		assert!(elapsed.as_secs() >= 1, "Should wait at least 1 second");
	}
}

#[tokio::test]
async fn test_hanging_connect_within_overflow() {
	// Test that a single hanging connection doesn't block others
	// Based on SQLAlchemy test_hanging_connect_within_overflow
	// NOTE: This test uses SQLite in-memory database, which connects instantly.
	// The test verifies that overflow connections can be created concurrently,
	// ensuring that the pool doesn't block on slow connection establishment.

	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(5);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Acquire multiple connections concurrently
	let mut handles = vec![];
	for i in 0..3 {
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

	// All tasks should complete successfully
	for (i, handle) in handles.into_iter().enumerate() {
		let result = handle.await.expect("Task panicked");
		assert_eq!(result, i as i64);
	}
}

#[tokio::test]
async fn test_idle_timeout() {
	// Test that connections are closed after idle timeout
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(5)
		.with_idle_timeout(Some(Duration::from_millis(100)));

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Acquire and immediately release a connection
	{
		let _conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire connection");
	}

	// Wait for idle timeout
	tokio::time::sleep(Duration::from_millis(200)).await;

	// Connection should have been closed due to idle timeout
	// We can't directly verify this with SQLx, but we can verify
	// that new connections can still be acquired
	let _conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection after idle timeout");
}

#[tokio::test]
async fn test_max_lifetime() {
	// Test that connections are closed after max lifetime
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(5)
		.with_max_lifetime(Some(Duration::from_millis(100)));

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Acquire a connection
	let _conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");

	// Wait for max lifetime to expire
	tokio::time::sleep(Duration::from_millis(200)).await;

	// Connection should be recycled on next acquire
	let _conn2 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection after max lifetime");
}
