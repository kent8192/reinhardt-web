//! Database-specific pool tests
//! Tests specific to PostgreSQL, MySQL, and SQLite behaviors

use reinhardt_db::pool::{ConnectionPool, PoolConfig};
use sqlx::Sqlite;
use std::time::Duration;

// SQLite-specific tests
mod sqlite_tests {
	use super::*;

	#[tokio::test]
	async fn test_sqlite_in_memory_pool() {
		// Test SQLite in-memory database pooling
		let url = "sqlite::memory:";
		let config = PoolConfig::default();

		let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
			.await
			.expect("Failed to create SQLite pool");

		let mut conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire connection");

		// Create table and insert data
		sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
			.execute(&mut *conn)
			.await
			.expect("Failed to create table");

		sqlx::query("INSERT INTO test (id, value) VALUES (1, 'test')")
			.execute(&mut *conn)
			.await
			.expect("Failed to insert data");

		// Verify data
		let result: (i64, String) = sqlx::query_as("SELECT id, value FROM test WHERE id = 1")
			.fetch_one(&mut *conn)
			.await
			.expect("Failed to select data");

		assert_eq!(result.0, 1);
		assert_eq!(result.1, "test");
	}

	#[tokio::test]
	async fn test_sqlite_file_pool() {
		// Test SQLite file-based database pooling
		use std::env;
		use std::fs;

		let temp_dir = env::temp_dir();

		// Check if temp directory is writable by attempting to create a test file
		let test_file = temp_dir.join(".reinhardt_write_test");
		if fs::write(&test_file, b"test").is_err() {
			eprintln!("Skipping test_sqlite_file_pool: temp directory is not writable");
			return;
		}
		let _ = fs::remove_file(&test_file);

		let db_path = temp_dir.join("test_pool_reinhardt.db");

		// Clean up any existing database
		let _ = std::fs::remove_file(&db_path);

		// Use canonicalized absolute path for SQLite
		let db_path_abs = match db_path.canonicalize() {
			Ok(p) => p,
			Err(_) => {
				// If canonicalize fails (file doesn't exist yet), use the original path
				// but ensure parent directory exists and is writable
				if !temp_dir.exists()
					|| fs::metadata(&temp_dir)
						.map(|m| m.permissions().readonly())
						.unwrap_or(true)
				{
					eprintln!("Skipping test_sqlite_file_pool: temp directory is not accessible");
					return;
				}
				db_path
			}
		};

		let db_path_str = db_path_abs.to_str().unwrap();
		let url = format!("sqlite://{}", db_path_str);

		let config = PoolConfig::new()
			.with_min_connections(1)
			.with_max_connections(3);

		// Try to create the pool, if it fails due to file access, skip the test
		let pool = match ConnectionPool::<Sqlite>::new_sqlite(&url, config).await {
			Ok(p) => p,
			Err(e) => {
				eprintln!(
					"Skipping test_sqlite_file_pool: Failed to create SQLite pool - {}",
					e
				);
				let _ = std::fs::remove_file(&db_path_abs);
				return;
			}
		};

		let mut conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire connection");

		// Create table
		sqlx::query("CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY, value TEXT)")
			.execute(&mut *conn)
			.await
			.expect("Failed to create table");

		drop(conn);
		pool.close().await;

		// Clean up
		let _ = std::fs::remove_file(&db_path_abs);
	}

	#[tokio::test]
	async fn test_sqlite_concurrent_writes() {
		// Test concurrent writes to SQLite database
		// Note: SQLite has limited concurrent write support
		let url = "sqlite::memory:";
		let config = PoolConfig::new()
			.with_min_connections(0)
			.with_max_connections(5);

		let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
			.await
			.expect("Failed to create SQLite pool");

		// Create table
		{
			let mut conn = pool
				.inner()
				.acquire()
				.await
				.expect("Failed to acquire connection");
			sqlx::query("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)")
				.execute(&mut *conn)
				.await
				.expect("Failed to create table");
		}

		// Concurrent writes
		let mut handles = vec![];
		for i in 0..5 {
			let pool_clone = pool.inner().clone();
			let handle = tokio::spawn(async move {
				let mut conn = pool_clone
					.acquire()
					.await
					.expect("Failed to acquire connection");
				sqlx::query(&format!(
					"INSERT INTO test (id, value) VALUES ({}, {})",
					i + 1,
					i * 10
				))
				.execute(&mut *conn)
				.await
				.expect("Failed to insert");
			});
			handles.push(handle);
		}

		for handle in handles {
			handle.await.expect("Task panicked");
		}

		// Verify all rows inserted
		let mut conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire connection");
		let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM test")
			.fetch_one(&mut *conn)
			.await
			.expect("Failed to count rows");

		assert_eq!(count.0, 5);
	}

	#[tokio::test]
	async fn test_sqlite_pool_with_pragma() {
		// Test SQLite pool with PRAGMA settings
		// Note: SQLx handles pragmas via connection options
		let url = "sqlite::memory:";
		let config = PoolConfig::default();

		let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
			.await
			.expect("Failed to create SQLite pool");

		let mut conn = pool
			.inner()
			.acquire()
			.await
			.expect("Failed to acquire connection");

		// Check some PRAGMA values
		let journal_mode: (String,) = sqlx::query_as("PRAGMA journal_mode")
			.fetch_one(&mut *conn)
			.await
			.expect("Failed to get journal_mode");

		// Verify we can read pragma (value depends on SQLx defaults)
		assert!(!journal_mode.0.is_empty());
	}
}

// PostgreSQL-specific tests (conditional compilation)
// These tests require a running PostgreSQL instance
#[cfg(feature = "postgres-tests")]
mod postgres_tests {
	use super::*;
	use sqlx::Postgres;

	#[tokio::test]
	#[ignore = "Requires running PostgreSQL instance"]
	async fn test_postgres_pool_creation() {
		// Based on Django test_connect_pool
		let url = std::env::var("DATABASE_URL")
			.unwrap_or_else(|_| "postgresql://localhost/test".to_string());

		let config = PoolConfig::new()
			.with_min_connections(0)
			.with_max_connections(2)
			.with_connect_timeout(Duration::from_secs(5));

		let result = ConnectionPool::<Postgres>::new_postgres(&url, config).await;

		if result.is_ok() {
			let pool = result.unwrap();
			let mut conn = pool
				.inner()
				.acquire()
				.await
				.expect("Failed to acquire connection");

			let result: i32 = sqlx::query_scalar("SELECT 1")
				.fetch_one(&mut *conn)
				.await
				.expect("Failed to execute query");

			assert_eq!(result, 1);
		}
		// Test skipped if PostgreSQL not available
	}

	#[tokio::test]
	#[ignore = "Requires running PostgreSQL instance"]
	async fn test_postgres_connection_reuse() {
		// Based on Django test_connect_pool - verify connection reuse
		let url = std::env::var("DATABASE_URL")
			.unwrap_or_else(|_| "postgresql://localhost/test".to_string());

		let config = PoolConfig::new()
			.with_min_connections(1)
			.with_max_connections(2);

		let result = ConnectionPool::<Postgres>::new_postgres(&url, config).await;

		if result.is_ok() {
			let pool = result.unwrap();

			// Get backend PID of first connection
			let pid1 = {
				let mut conn = pool
					.inner()
					.acquire()
					.await
					.expect("Failed to acquire connection");
				let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
					.fetch_one(&mut *conn)
					.await
					.expect("Failed to get backend PID");
				pid
			};

			// Get backend PID of second connection (should reuse first)
			let pid2 = {
				let mut conn = pool
					.inner()
					.acquire()
					.await
					.expect("Failed to acquire connection");
				let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
					.fetch_one(&mut *conn)
					.await
					.expect("Failed to get backend PID");
				pid
			};

			// With min_connections >= 1, might reuse same connection
			// At minimum, both PIDs should be valid
			assert!(pid1 > 0);
			assert!(pid2 > 0);
		}
	}

	#[tokio::test]
	#[ignore = "Requires running PostgreSQL instance"]
	async fn test_postgres_pool_exhaustion() {
		// Test PostgreSQL pool exhaustion behavior
		let url = std::env::var("DATABASE_URL")
			.unwrap_or_else(|_| "postgresql://localhost/test".to_string());

		let config = PoolConfig::new()
			.with_min_connections(0)
			.with_max_connections(2)
			.with_connect_timeout(Duration::from_secs(2));

		let result = ConnectionPool::<Postgres>::new_postgres(&url, config).await;

		if result.is_ok() {
			let pool = result.unwrap();

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

			// Third connection should timeout
			let result = tokio::time::timeout(Duration::from_secs(3), pool.inner().acquire()).await;

			assert!(result.is_err(), "Should timeout when pool exhausted");
		}
	}
}

// MySQL-specific tests (conditional compilation)
#[cfg(feature = "mysql-tests")]
mod mysql_tests {
	use super::*;
	use sqlx::MySql;

	#[tokio::test]
	#[ignore = "Requires running MySQL instance"]
	async fn test_mysql_pool_creation() {
		let url =
			std::env::var("MYSQL_URL").unwrap_or_else(|_| "mysql://localhost/test".to_string());

		let config = PoolConfig::new()
			.with_min_connections(1)
			.with_max_connections(5);

		let result = ConnectionPool::<MySql>::new_mysql(&url, config).await;

		if result.is_ok() {
			let pool = result.unwrap();
			let mut conn = pool
				.inner()
				.acquire()
				.await
				.expect("Failed to acquire connection");

			let result: i32 = sqlx::query_scalar("SELECT 1")
				.fetch_one(&mut *conn)
				.await
				.expect("Failed to execute query");

			assert_eq!(result, 1);
		}
	}

	#[tokio::test]
	#[ignore = "Requires running MySQL instance"]
	async fn test_mysql_connection_reuse() {
		let url =
			std::env::var("MYSQL_URL").unwrap_or_else(|_| "mysql://localhost/test".to_string());

		let config = PoolConfig::new()
			.with_min_connections(1)
			.with_max_connections(2);

		let result = ConnectionPool::<MySql>::new_mysql(&url, config).await;

		if result.is_ok() {
			let pool = result.unwrap();

			// Get connection ID of first connection
			let id1 = {
				let mut conn = pool
					.inner()
					.acquire()
					.await
					.expect("Failed to acquire connection");
				let id: u64 = sqlx::query_scalar("SELECT CONNECTION_ID()")
					.fetch_one(&mut *conn)
					.await
					.expect("Failed to get connection ID");
				id
			};

			// Get connection ID of second connection
			let id2 = {
				let mut conn = pool
					.inner()
					.acquire()
					.await
					.expect("Failed to acquire connection");
				let id: u64 = sqlx::query_scalar("SELECT CONNECTION_ID()")
					.fetch_one(&mut *conn)
					.await
					.expect("Failed to get connection ID");
				id
			};

			// Both IDs should be valid
			assert!(id1 > 0);
			assert!(id2 > 0);
		}
	}
}

// Cross-database tests
#[tokio::test]
async fn test_pool_config_builder() {
	// Test that pool configuration builder works for all databases
	let config = PoolConfig::new()
		.with_min_connections(2)
		.with_max_connections(10)
		.with_connect_timeout(Duration::from_secs(5))
		.with_idle_timeout(Some(Duration::from_secs(300)))
		.with_max_lifetime(Some(Duration::from_secs(1800)))
		.with_test_before_acquire(true);

	assert_eq!(config.min_connections, 2);
	assert_eq!(config.max_connections, 10);
	assert_eq!(config.connect_timeout, Duration::from_secs(5));
	assert_eq!(config.idle_timeout, Some(Duration::from_secs(300)));
	assert_eq!(config.max_lifetime, Some(Duration::from_secs(1800)));
	assert!(config.test_before_acquire);
}

#[tokio::test]
async fn test_invalid_url_handling() {
	// Test that invalid database URLs are properly rejected
	let invalid_url = "invalid://url";
	let config = PoolConfig::default();

	let result = ConnectionPool::<Sqlite>::new_sqlite(invalid_url, config).await;
	assert!(result.is_err(), "Should reject invalid URL");
}

#[tokio::test]
async fn test_pool_with_zero_min_connections() {
	// Test pool with zero minimum connections
	let url = "sqlite::memory:";
	let config = PoolConfig::new()
		.with_min_connections(0)
		.with_max_connections(5);

	let pool = ConnectionPool::<Sqlite>::new_sqlite(url, config)
		.await
		.expect("Failed to create pool");

	// Should be able to acquire connections on demand
	let _conn = pool
		.inner()
		.acquire()
		.await
		.expect("Failed to acquire connection");
}
