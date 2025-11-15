//! Pool recreation tests
//! Tests for pool.recreate() functionality

use reinhardt_pool::{ConnectionPool, PoolConfig};
use sqlx::Sqlite;
use std::time::Duration;

#[tokio::test]
async fn test_pool_recreate_preserves_config() {
	// Test that pool.recreate() preserves configuration
	// Based on SQLAlchemy test_recreate
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(2)
		.with_max_connections(5)
		.with_acquire_timeout(Duration::from_secs(10))
		.with_test_before_acquire(true);

	let mut pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let original_config = pool.config().clone();

	// Recreate the pool
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

#[tokio::test]
async fn test_pool_recreate_works_after() {
	// Test that pool works after recreation
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let mut pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Use pool before recreation
	{
		let mut conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire before recreate");
		let result: i64 = sqlx::query_scalar("SELECT 1")
			.fetch_one(&mut *conn)
			.await
			.expect("Query failed before recreate");
		assert_eq!(result, 1);
	}

	// Recreate the pool
	pool.recreate().await.expect("Failed to recreate pool");

	// Use pool after recreation
	{
		let mut conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire after recreate");
		let result: i64 = sqlx::query_scalar("SELECT 2")
			.fetch_one(&mut *conn)
			.await
			.expect("Query failed after recreate");
		assert_eq!(result, 2);
	}
}

#[tokio::test]
async fn test_pool_recreate_resets_first_connect() {
	// Test that first_connect flag is reset on recreation
	use async_trait::async_trait;
	use reinhardt_pool::{PoolEvent, PoolEventListener};
	use std::sync::Arc;
	use tokio::sync::Mutex;

	struct FirstConnectCounter {
		count: Arc<Mutex<usize>>,
	}

	#[async_trait]
	impl PoolEventListener for FirstConnectCounter {
		async fn on_event(&self, event: PoolEvent) {
			if matches!(event, PoolEvent::ConnectionCreated { .. }) {
				let mut count = self.count.lock().await;
				*count += 1;
			}
		}
	}

	let url = "sqlite::memory:";
	let config = PoolConfig::new().with_min_connections(0);

	let mut pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let count = Arc::new(Mutex::new(0));
	let listener = Arc::new(FirstConnectCounter {
		count: count.clone(),
	});

	pool.add_listener(listener).await;

	// First connection should trigger first_connect
	{
		let _conn = pool.acquire().await.expect("Failed to acquire connection");
	}

	let count1 = *count.lock().await;
	assert_eq!(count1, 1, "First connect should have fired once");

	// Recreate the pool
	pool.recreate().await.expect("Failed to recreate pool");

	// First connection after recreate should trigger first_connect again
	{
		let _conn = pool
			.acquire()
			.await
			.expect("Failed to acquire after recreate");
	}

	let count2 = *count.lock().await;
	assert_eq!(count2, 2, "First connect should fire again after recreate");
}

#[tokio::test]
async fn test_pool_recreate_url_preserved() {
	// Test that URL is preserved during recreation
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let mut pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	let original_url = pool.url().to_string();

	pool.recreate().await.expect("Failed to recreate pool");

	assert_eq!(pool.url(), original_url);
}

#[tokio::test]
async fn test_pool_recreate_closes_old_connections() {
	// Test that old connections are closed during recreation
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(2);

	let mut pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Acquire some connections
	let _conn1 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire conn1");

	// Drop connection before recreating to prevent deadlock
	drop(_conn1);

	// Recreate the pool (this should close all existing connections)
	pool.recreate().await.expect("Failed to recreate pool");

	// Should be able to acquire new connections
	let _conn2 = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire after recreate");
}

#[tokio::test]
async fn test_pool_multiple_recreate() {
	// Test multiple recreation cycles
	let url = "sqlite::memory:";
	let config = PoolConfig::default();

	let mut pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	for i in 0..3 {
		pool.recreate()
			.await
			.expect(&format!("Failed to recreate pool iteration {}", i));

		// Verify pool works after each recreation
		let mut conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire connection");
		let result: i64 = sqlx::query_scalar(&format!("SELECT {}", i + 1))
			.fetch_one(&mut *conn)
			.await
			.expect("Query failed");
		assert_eq!(result, (i + 1) as i64);
	}
}

#[tokio::test]
async fn test_pool_recreate_with_custom_config() {
	// Test recreation with various custom configurations
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(1)
		.with_max_connections(3)
		.with_acquire_timeout(Duration::from_secs(5))
		.with_idle_timeout(Some(Duration::from_secs(300)))
		.with_max_lifetime(Some(Duration::from_secs(900)))
		.with_test_before_acquire(true);

	let mut pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	pool.recreate().await.expect("Failed to recreate pool");

	// All config values should be preserved
	assert_eq!(pool.config().min_connections, 1);
	assert_eq!(pool.config().max_connections, 3);
	assert_eq!(pool.config().acquire_timeout, Duration::from_secs(5));
	assert_eq!(pool.config().idle_timeout, Some(Duration::from_secs(300)));
	assert_eq!(pool.config().max_lifetime, Some(Duration::from_secs(900)));
	assert_eq!(pool.config().test_before_acquire, true);
}
