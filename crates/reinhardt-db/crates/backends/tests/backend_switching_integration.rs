//! Integration tests for database backend switching
//!
//! This test file validates the ability to switch between PostgreSQL, MySQL, and SQLite
//! backends dynamically, ensuring connection pool management, backend-specific query syntax,
//! transaction handling, and feature detection work correctly across different databases.
//!
//! ## Test Coverage
//!
//! - Backend switching between PostgreSQL/MySQL/SQLite
//! - Connection pool management across backends
//! - Backend-specific query syntax (placeholders, RETURNING, ON CONFLICT)
//! - Transaction handling per backend
//! - Backend feature detection and capability checks
//!
//! ## Requirements
//!
//! - PostgreSQL container via TestContainers
//! - Optional MySQL/SQLite containers for multi-database tests
//! - Real database operations with connection pooling
//! - 10+ comprehensive test cases

use reinhardt_backends::{DatabaseConnection, DatabaseType, QueryValue};
use rstest::*;
use serial_test::serial;
use sqlx::{MySqlPool, PgPool, SqlitePool};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage, ImageExt, core::WaitFor, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;

type PostgresContainer = ContainerAsync<Postgres>;
type MySqlContainer = ContainerAsync<GenericImage>;
type SqliteInMemory = ();

/// PostgreSQL container fixture with connection pool
#[fixture]
async fn postgres_container() -> (PostgresContainer, Arc<PgPool>, u16, String) {
	let postgres = Postgres::default()
		.start()
		.await
		.expect("Failed to start PostgreSQL container");

	// Retry port acquisition with exponential backoff (port mapping may not be immediately available)
	let port = {
		let mut retries = 0;
		let max_retries = 5;
		loop {
			match postgres.get_host_port_ipv4(5432).await {
				Ok(p) => break p,
				Err(e) => {
					if retries >= max_retries {
						panic!(
							"Failed to get PostgreSQL port after {} retries: {:?}",
							max_retries, e
						);
					}
					retries += 1;
					let backoff = std::time::Duration::from_millis(100 * (1 << retries));
					tokio::time::sleep(backoff).await;
				}
			}
		}
	};

	let url = format!("postgresql://postgres:postgres@localhost:{}/postgres", port);

	let pool = PgPool::connect(&url)
		.await
		.expect("Failed to connect to PostgreSQL");

	(postgres, Arc::new(pool), port, url)
}

/// MySQL container fixture with connection pool
#[fixture]
async fn mysql_container() -> (MySqlContainer, Arc<MySqlPool>, u16, String) {
	let mysql = GenericImage::new("mysql", "8.0")
		.with_wait_for(WaitFor::message_on_stderr("ready for connections"))
		.with_env_var("MYSQL_ROOT_PASSWORD", "test")
		.with_env_var("MYSQL_DATABASE", "mysql")
		.start()
		.await
		.expect("Failed to start MySQL container");

	let port = {
		let mut retries = 0;
		let max_retries = 5;
		loop {
			match mysql.get_host_port_ipv4(3306).await {
				Ok(p) => break p,
				Err(e) => {
					if retries >= max_retries {
						panic!(
							"Failed to get MySQL port after {} retries: {:?}",
							max_retries, e
						);
					}
					retries += 1;
					let backoff = std::time::Duration::from_millis(100 * (1 << retries));
					tokio::time::sleep(backoff).await;
				}
			}
		}
	};

	let url = format!("mysql://root:test@localhost:{}/mysql", port);

	// Retry connection with exponential backoff for MySQL 8.0
	let pool = {
		let mut retries = 0;
		let max_retries = 10;
		loop {
			match MySqlPool::connect(&url).await {
				Ok(pool) => break pool,
				Err(e) => {
					if retries >= max_retries {
						panic!(
							"Failed to connect to MySQL after {} retries: {}",
							max_retries, e
						);
					}
					retries += 1;
					let backoff = std::time::Duration::from_millis(100 * (1 << retries));
					tokio::time::sleep(backoff).await;
				}
			}
		}
	};

	(mysql, Arc::new(pool), port, url)
}

/// SQLite in-memory fixture
#[fixture]
async fn sqlite_fixture() -> (SqliteInMemory, Arc<SqlitePool>, String) {
	let url = "sqlite::memory:".to_string();
	let pool = SqlitePool::connect(&url)
		.await
		.expect("Failed to create SQLite in-memory pool");

	((), Arc::new(pool), url)
}

async fn create_test_table_postgres(pool: &PgPool, table_name: &str) {
	let _ = sqlx::query(&format!("DROP TABLE IF EXISTS {}", table_name))
		.execute(pool)
		.await;

	sqlx::query(&format!(
		"CREATE TABLE {} (id SERIAL PRIMARY KEY, value TEXT, count INTEGER)",
		table_name
	))
	.execute(pool)
	.await
	.expect("Failed to create PostgreSQL test table");
}

#[cfg(feature = "mysql")]
async fn create_test_table_mysql(pool: &MySqlPool, table_name: &str) {
	let _ = sqlx::query(&format!("DROP TABLE IF EXISTS {}", table_name))
		.execute(pool)
		.await;

	sqlx::query(&format!(
		"CREATE TABLE {} (id INT AUTO_INCREMENT PRIMARY KEY, value VARCHAR(255), count INT)",
		table_name
	))
	.execute(pool)
	.await
	.expect("Failed to create MySQL test table");
}

async fn create_test_table_sqlite(pool: &SqlitePool, table_name: &str) {
	let _ = sqlx::query(&format!("DROP TABLE IF EXISTS {}", table_name))
		.execute(pool)
		.await;

	sqlx::query(&format!(
		"CREATE TABLE {} (id INTEGER PRIMARY KEY AUTOINCREMENT, value TEXT, count INTEGER)",
		table_name
	))
	.execute(pool)
	.await
	.expect("Failed to create SQLite test table");
}

async fn drop_test_table_postgres(pool: &PgPool, table_name: &str) {
	let _ = sqlx::query(&format!("DROP TABLE IF EXISTS {}", table_name))
		.execute(pool)
		.await;
}

#[cfg(feature = "mysql")]
async fn drop_test_table_mysql(pool: &MySqlPool, table_name: &str) {
	let _ = sqlx::query(&format!("DROP TABLE IF EXISTS {}", table_name))
		.execute(pool)
		.await;
}

async fn drop_test_table_sqlite(pool: &SqlitePool, table_name: &str) {
	let _ = sqlx::query(&format!("DROP TABLE IF EXISTS {}", table_name))
		.execute(pool)
		.await;
}

/// Test switching between PostgreSQL and SQLite backends
#[rstest]
#[tokio::test]
#[serial(backend_switching)]
async fn test_switch_between_postgres_and_sqlite(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
	#[future] sqlite_fixture: (SqliteInMemory, Arc<SqlitePool>, String),
) {
	let (_pg_container, pg_pool, _pg_port, pg_url) = postgres_container.await;
	let (_sqlite, sqlite_pool, sqlite_url) = sqlite_fixture.await;

	// Connect to PostgreSQL
	let pg_conn = DatabaseConnection::connect_postgres(&pg_url)
		.await
		.expect("Failed to connect to PostgreSQL");

	// Verify PostgreSQL backend type
	assert_eq!(
		pg_conn.backend().database_type(),
		DatabaseType::Postgres,
		"PostgreSQL backend type mismatch"
	);

	// Connect to SQLite
	let sqlite_conn = DatabaseConnection::connect_sqlite(&sqlite_url)
		.await
		.expect("Failed to connect to SQLite");

	// Verify SQLite backend type
	assert_eq!(
		sqlite_conn.backend().database_type(),
		DatabaseType::Sqlite,
		"SQLite backend type mismatch"
	);

	// Verify different placeholder styles
	assert_eq!(
		pg_conn.backend().placeholder(1),
		"$1",
		"PostgreSQL placeholder mismatch"
	);
	assert_eq!(
		sqlite_conn.backend().placeholder(1),
		"?",
		"SQLite placeholder mismatch"
	);

	// Cleanup
	drop(pg_conn);
	drop(sqlite_conn);
	drop(pg_pool);
	drop(sqlite_pool);
}

/// Test connection pool management across multiple backends
#[rstest]
#[tokio::test]
#[serial(backend_switching)]
async fn test_connection_pool_management_across_backends(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
	#[future] sqlite_fixture: (SqliteInMemory, Arc<SqlitePool>, String),
) {
	let (_pg_container, pg_pool, _pg_port, pg_url) = postgres_container.await;
	let (_sqlite, _sqlite_pool, sqlite_url) = sqlite_fixture.await;

	create_test_table_postgres(&pg_pool, "pool_test").await;

	// Create multiple connections to PostgreSQL
	let pg_conn1 = DatabaseConnection::connect_postgres_with_pool_size(&pg_url, Some(5))
		.await
		.expect("Failed to connect PostgreSQL conn1");
	let pg_conn2 = DatabaseConnection::connect_postgres_with_pool_size(&pg_url, Some(5))
		.await
		.expect("Failed to connect PostgreSQL conn2");

	// Insert data using first connection
	let insert_result = pg_conn1
		.backend()
		.execute(
			"INSERT INTO pool_test (value, count) VALUES ($1, $2)",
			vec![
				QueryValue::String("pooled".to_string()),
				QueryValue::Int(100),
			],
		)
		.await;
	assert!(insert_result.is_ok(), "Failed to insert via conn1");

	// Query data using second connection (should see data from first connection)
	let rows = pg_conn2
		.backend()
		.fetch_all(
			"SELECT value, count FROM pool_test WHERE value = $1",
			vec![QueryValue::String("pooled".to_string())],
		)
		.await
		.expect("Failed to query via conn2");

	assert_eq!(rows.len(), 1, "Expected 1 row via second connection");
	let value: String = rows[0].get("value").expect("Failed to get value");
	assert_eq!(value, "pooled", "Value mismatch");

	// Switch to SQLite with different pool
	let sqlite_conn = DatabaseConnection::connect_sqlite(&sqlite_url)
		.await
		.expect("Failed to connect to SQLite");

	assert_eq!(
		sqlite_conn.backend().database_type(),
		DatabaseType::Sqlite,
		"SQLite backend type mismatch"
	);

	// Cleanup
	drop_test_table_postgres(&pg_pool, "pool_test").await;
}

/// Test backend-specific placeholder syntax
#[cfg(feature = "mysql")]
#[rstest]
#[tokio::test]
#[serial(backend_switching)]
async fn test_backend_specific_placeholders(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
	#[future] mysql_container: (MySqlContainer, Arc<MySqlPool>, u16, String),
	#[future] sqlite_fixture: (SqliteInMemory, Arc<SqlitePool>, String),
) {
	let (_pg_container, pg_pool, _pg_port, pg_url) = postgres_container.await;
	let (_mysql_container, _mysql_pool, _mysql_port, mysql_url) = mysql_container.await;
	let (_sqlite, _sqlite_pool, sqlite_url) = sqlite_fixture.await;

	// PostgreSQL: $1, $2, $3 placeholders
	let pg_conn = DatabaseConnection::connect_postgres(&pg_url)
		.await
		.expect("Failed to connect to PostgreSQL");
	assert_eq!(pg_conn.backend().placeholder(1), "$1");
	assert_eq!(pg_conn.backend().placeholder(2), "$2");
	assert_eq!(pg_conn.backend().placeholder(10), "$10");

	// MySQL: ? placeholders
	let mysql_conn = DatabaseConnection::connect_mysql(&mysql_url)
		.await
		.expect("Failed to connect to MySQL");
	assert_eq!(mysql_conn.backend().placeholder(1), "?");
	assert_eq!(mysql_conn.backend().placeholder(2), "?");
	assert_eq!(mysql_conn.backend().placeholder(10), "?");

	// SQLite: ? placeholders
	let sqlite_conn = DatabaseConnection::connect_sqlite(&sqlite_url)
		.await
		.expect("Failed to connect to SQLite");
	assert_eq!(sqlite_conn.backend().placeholder(1), "?");
	assert_eq!(sqlite_conn.backend().placeholder(5), "?");

	// Verify placeholder usage in actual queries
	create_test_table_postgres(&pg_pool, "placeholder_test").await;

	let pg_insert = pg_conn
		.backend()
		.execute(
			"INSERT INTO placeholder_test (value, count) VALUES ($1, $2)",
			vec![
				QueryValue::String("postgres_placeholder".to_string()),
				QueryValue::Int(1),
			],
		)
		.await;
	assert!(pg_insert.is_ok(), "PostgreSQL placeholder insert failed");

	// Cleanup
	drop_test_table_postgres(&pg_pool, "placeholder_test").await;
}

/// Test RETURNING clause support detection
#[cfg(feature = "mysql")]
#[rstest]
#[tokio::test]
#[serial(backend_switching)]
async fn test_returning_clause_support(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
	#[future] mysql_container: (MySqlContainer, Arc<MySqlPool>, u16, String),
	#[future] sqlite_fixture: (SqliteInMemory, Arc<SqlitePool>, String),
) {
	let (_pg_container, _pg_pool, _pg_port, pg_url) = postgres_container.await;
	let (_mysql_container, _mysql_pool, _mysql_port, mysql_url) = mysql_container.await;
	let (_sqlite, _sqlite_pool, sqlite_url) = sqlite_fixture.await;

	// PostgreSQL supports RETURNING
	let pg_conn = DatabaseConnection::connect_postgres(&pg_url)
		.await
		.expect("Failed to connect to PostgreSQL");
	assert!(
		pg_conn.backend().supports_returning(),
		"PostgreSQL should support RETURNING"
	);

	// MySQL does NOT support RETURNING (as of MySQL 8.0)
	let mysql_conn = DatabaseConnection::connect_mysql(&mysql_url)
		.await
		.expect("Failed to connect to MySQL");
	assert!(
		!mysql_conn.backend().supports_returning(),
		"MySQL should not support RETURNING"
	);

	// SQLite supports RETURNING (since 3.35.0)
	let sqlite_conn = DatabaseConnection::connect_sqlite(&sqlite_url)
		.await
		.expect("Failed to connect to SQLite");
	assert!(
		sqlite_conn.backend().supports_returning(),
		"SQLite should support RETURNING"
	);
}

/// Test ON CONFLICT clause support detection
#[cfg(feature = "mysql")]
#[rstest]
#[tokio::test]
#[serial(backend_switching)]
async fn test_on_conflict_support(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
	#[future] mysql_container: (MySqlContainer, Arc<MySqlPool>, u16, String),
	#[future] sqlite_fixture: (SqliteInMemory, Arc<SqlitePool>, String),
) {
	let (_pg_container, _pg_pool, _pg_port, pg_url) = postgres_container.await;
	let (_mysql_container, _mysql_pool, _mysql_port, mysql_url) = mysql_container.await;
	let (_sqlite, _sqlite_pool, sqlite_url) = sqlite_fixture.await;

	// PostgreSQL supports ON CONFLICT
	let pg_conn = DatabaseConnection::connect_postgres(&pg_url)
		.await
		.expect("Failed to connect to PostgreSQL");
	assert!(
		pg_conn.backend().supports_on_conflict(),
		"PostgreSQL should support ON CONFLICT"
	);

	// MySQL does NOT support ON CONFLICT (uses INSERT ... ON DUPLICATE KEY UPDATE)
	let mysql_conn = DatabaseConnection::connect_mysql(&mysql_url)
		.await
		.expect("Failed to connect to MySQL");
	assert!(
		!mysql_conn.backend().supports_on_conflict(),
		"MySQL should not support ON CONFLICT"
	);

	// SQLite supports ON CONFLICT
	let sqlite_conn = DatabaseConnection::connect_sqlite(&sqlite_url)
		.await
		.expect("Failed to connect to SQLite");
	assert!(
		sqlite_conn.backend().supports_on_conflict(),
		"SQLite should support ON CONFLICT"
	);
}

/// Test transaction handling in PostgreSQL backend
#[rstest]
#[tokio::test]
#[serial(backend_switching)]
async fn test_transaction_handling_postgres(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_pg_container, pg_pool, _pg_port, _pg_url) = postgres_container.await;

	create_test_table_postgres(&pg_pool, "transaction_test").await;

	// Begin transaction
	let mut tx = pg_pool
		.begin()
		.await
		.expect("Failed to begin PostgreSQL transaction");

	// Insert data within transaction
	sqlx::query("INSERT INTO transaction_test (value, count) VALUES ($1, $2)")
		.bind("tx_value")
		.bind(42)
		.execute(&mut *tx)
		.await
		.expect("Failed to insert within transaction");

	// Data should not be visible outside transaction yet
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transaction_test")
		.fetch_one(&*pg_pool)
		.await
		.expect("Failed to count rows");
	assert_eq!(count, 0, "Data should not be visible before commit");

	// Commit transaction
	tx.commit().await.expect("Failed to commit transaction");

	// Data should now be visible
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transaction_test")
		.fetch_one(&*pg_pool)
		.await
		.expect("Failed to count rows after commit");
	assert_eq!(count, 1, "Data should be visible after commit");

	// Cleanup
	drop_test_table_postgres(&pg_pool, "transaction_test").await;
}

/// Test transaction rollback in MySQL backend
#[cfg(feature = "mysql")]
#[rstest]
#[tokio::test]
#[serial(backend_switching)]
async fn test_transaction_rollback_mysql(
	#[future] mysql_container: (MySqlContainer, Arc<MySqlPool>, u16, String),
) {
	let (_mysql_container, mysql_pool, _mysql_port, _mysql_url) = mysql_container.await;

	create_test_table_mysql(&mysql_pool, "rollback_test").await;

	// Begin transaction
	let mut tx = mysql_pool
		.begin()
		.await
		.expect("Failed to begin MySQL transaction");

	// Insert data within transaction
	sqlx::query("INSERT INTO rollback_test (value, count) VALUES (?, ?)")
		.bind("rollback_value")
		.bind(99)
		.execute(&mut *tx)
		.await
		.expect("Failed to insert within transaction");

	// Rollback transaction
	tx.rollback().await.expect("Failed to rollback transaction");

	// Data should NOT be visible after rollback
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM rollback_test")
		.fetch_one(&*mysql_pool)
		.await
		.expect("Failed to count rows after rollback");
	assert_eq!(count, 0, "Data should not be visible after rollback");

	// Cleanup
	drop_test_table_mysql(&mysql_pool, "rollback_test").await;
}

/// Test transaction handling in SQLite backend
#[rstest]
#[tokio::test]
#[serial(backend_switching)]
async fn test_transaction_handling_sqlite(
	#[future] sqlite_fixture: (SqliteInMemory, Arc<SqlitePool>, String),
) {
	let (_sqlite, sqlite_pool, _sqlite_url) = sqlite_fixture.await;

	create_test_table_sqlite(&sqlite_pool, "tx_test").await;

	// Begin transaction
	let mut tx = sqlite_pool
		.begin()
		.await
		.expect("Failed to begin SQLite transaction");

	// Insert data within transaction
	sqlx::query("INSERT INTO tx_test (value, count) VALUES (?, ?)")
		.bind("sqlite_tx_value")
		.bind(123)
		.execute(&mut *tx)
		.await
		.expect("Failed to insert within transaction");

	// Commit transaction
	tx.commit().await.expect("Failed to commit transaction");

	// Verify data is visible
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tx_test")
		.fetch_one(&*sqlite_pool)
		.await
		.expect("Failed to count rows");
	assert_eq!(count, 1, "Data should be visible after commit");

	// Cleanup
	drop_test_table_sqlite(&sqlite_pool, "tx_test").await;
}

/// Test backend feature detection for all databases
#[cfg(feature = "mysql")]
#[rstest]
#[tokio::test]
#[serial(backend_switching)]
async fn test_backend_feature_detection(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
	#[future] mysql_container: (MySqlContainer, Arc<MySqlPool>, u16, String),
	#[future] sqlite_fixture: (SqliteInMemory, Arc<SqlitePool>, String),
) {
	let (_pg_container, _pg_pool, _pg_port, pg_url) = postgres_container.await;
	let (_mysql_container, _mysql_pool, _mysql_port, mysql_url) = mysql_container.await;
	let (_sqlite, _sqlite_pool, sqlite_url) = sqlite_fixture.await;

	// PostgreSQL feature set
	let pg_conn = DatabaseConnection::connect_postgres(&pg_url)
		.await
		.expect("Failed to connect to PostgreSQL");
	assert_eq!(pg_conn.backend().database_type(), DatabaseType::Postgres);
	assert!(pg_conn.backend().supports_returning());
	assert!(pg_conn.backend().supports_on_conflict());
	assert_eq!(pg_conn.backend().placeholder(1), "$1");

	// MySQL feature set
	let mysql_conn = DatabaseConnection::connect_mysql(&mysql_url)
		.await
		.expect("Failed to connect to MySQL");
	assert_eq!(mysql_conn.backend().database_type(), DatabaseType::Mysql);
	assert!(!mysql_conn.backend().supports_returning());
	assert!(!mysql_conn.backend().supports_on_conflict());
	assert_eq!(mysql_conn.backend().placeholder(1), "?");

	// SQLite feature set
	let sqlite_conn = DatabaseConnection::connect_sqlite(&sqlite_url)
		.await
		.expect("Failed to connect to SQLite");
	assert_eq!(sqlite_conn.backend().database_type(), DatabaseType::Sqlite);
	assert!(sqlite_conn.backend().supports_returning());
	assert!(sqlite_conn.backend().supports_on_conflict());
	assert_eq!(sqlite_conn.backend().placeholder(1), "?");
}

/// Test inserting and querying data across multiple backends
#[rstest]
#[tokio::test]
#[serial(backend_switching)]
async fn test_insert_and_query_across_backends(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
	#[future] sqlite_fixture: (SqliteInMemory, Arc<SqlitePool>, String),
) {
	let (_pg_container, pg_pool, _pg_port, pg_url) = postgres_container.await;
	let (_sqlite, _sqlite_pool, sqlite_url) = sqlite_fixture.await;

	create_test_table_postgres(&pg_pool, "multi_backend_test").await;
	// Note: SQLite :memory: database is connection-specific,
	// so we create table via DatabaseConnection instead of pool

	// PostgreSQL
	let pg_conn = DatabaseConnection::connect_postgres(&pg_url)
		.await
		.expect("Failed to connect to PostgreSQL");
	pg_conn
		.backend()
		.execute(
			"INSERT INTO multi_backend_test (value, count) VALUES ($1, $2)",
			vec![
				QueryValue::String("postgres_data".to_string()),
				QueryValue::Int(10),
			],
		)
		.await
		.expect("Failed to insert into PostgreSQL");

	let pg_rows = pg_conn
		.backend()
		.fetch_all("SELECT value, count FROM multi_backend_test", vec![])
		.await
		.expect("Failed to query PostgreSQL");
	assert_eq!(pg_rows.len(), 1);
	let pg_value: String = pg_rows[0]
		.get("value")
		.expect("Failed to get PostgreSQL value");
	assert_eq!(pg_value, "postgres_data");

	// SQLite - create table via DatabaseConnection since :memory: is connection-specific
	let sqlite_conn = DatabaseConnection::connect_sqlite(&sqlite_url)
		.await
		.expect("Failed to connect to SQLite");
	// Create table on this connection
	sqlite_conn
		.backend()
		.execute(
			"CREATE TABLE IF NOT EXISTS multi_backend_test (id INTEGER PRIMARY KEY AUTOINCREMENT, value TEXT, count INTEGER)",
			vec![],
		)
		.await
		.expect("Failed to create SQLite table");
	sqlite_conn
		.backend()
		.execute(
			"INSERT INTO multi_backend_test (value, count) VALUES (?, ?)",
			vec![
				QueryValue::String("sqlite_data".to_string()),
				QueryValue::Int(30),
			],
		)
		.await
		.expect("Failed to insert into SQLite");

	let sqlite_rows = sqlite_conn
		.backend()
		.fetch_all("SELECT value, count FROM multi_backend_test", vec![])
		.await
		.expect("Failed to query SQLite");
	assert_eq!(sqlite_rows.len(), 1);
	let sqlite_value: String = sqlite_rows[0]
		.get("value")
		.expect("Failed to get SQLite value");
	assert_eq!(sqlite_value, "sqlite_data");

	// Cleanup
	drop_test_table_postgres(&pg_pool, "multi_backend_test").await;
	// Note: SQLite table was created via DatabaseConnection, cleanup via connection
	sqlite_conn
		.backend()
		.execute("DROP TABLE IF EXISTS multi_backend_test", vec![])
		.await
		.expect("Failed to drop SQLite table");
}

/// Test concurrent operations across different backends
#[rstest]
#[tokio::test]
#[serial(backend_switching)]
async fn test_concurrent_operations_across_backends(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
	#[future] sqlite_fixture: (SqliteInMemory, Arc<SqlitePool>, String),
) {
	let (_pg_container, pg_pool, _pg_port, pg_url) = postgres_container.await;
	let (_sqlite, _sqlite_pool, sqlite_url) = sqlite_fixture.await;

	create_test_table_postgres(&pg_pool, "concurrent_test").await;
	// Note: SQLite :memory: database is connection-specific,
	// so we create table via DatabaseConnection instead of pool
	// create_test_table_sqlite(&sqlite_pool, "concurrent_test").await;

	// Create connections
	let pg_conn = DatabaseConnection::connect_postgres(&pg_url)
		.await
		.expect("Failed to connect to PostgreSQL");
	let sqlite_conn = DatabaseConnection::connect_sqlite(&sqlite_url)
		.await
		.expect("Failed to connect to SQLite");
	// Create table on this SQLite connection
	sqlite_conn
		.backend()
		.execute(
			"CREATE TABLE IF NOT EXISTS concurrent_test (id INTEGER PRIMARY KEY AUTOINCREMENT, value TEXT, count INTEGER)",
			vec![],
		)
		.await
		.expect("Failed to create SQLite table");

	// Spawn concurrent insert tasks
	let pg_task = {
		let pg_conn = pg_conn.clone();
		tokio::spawn(async move {
			for i in 0..5 {
				pg_conn
					.backend()
					.execute(
						"INSERT INTO concurrent_test (value, count) VALUES ($1, $2)",
						vec![
							QueryValue::String(format!("pg_{}", i)),
							QueryValue::Int(i as i64),
						],
					)
					.await
					.expect("Failed to insert into PostgreSQL");
			}
		})
	};

	let sqlite_task = {
		let sqlite_conn = sqlite_conn.clone();
		tokio::spawn(async move {
			for i in 0..5 {
				sqlite_conn
					.backend()
					.execute(
						"INSERT INTO concurrent_test (value, count) VALUES (?, ?)",
						vec![
							QueryValue::String(format!("sqlite_{}", i)),
							QueryValue::Int(i as i64),
						],
					)
					.await
					.expect("Failed to insert into SQLite");
			}
		})
	};

	// Wait for both tasks to complete
	pg_task.await.expect("PostgreSQL task panicked");
	sqlite_task.await.expect("SQLite task panicked");

	// Verify data in both backends
	let pg_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM concurrent_test")
		.fetch_one(&*pg_pool)
		.await
		.expect("Failed to count PostgreSQL rows");
	assert_eq!(pg_count, 5, "Expected 5 rows in PostgreSQL");

	// Note: SQLite count via DatabaseConnection since table was created there
	// Fetch all rows to count them (workaround for COUNT(*) type issue)
	let sqlite_rows = sqlite_conn
		.backend()
		.fetch_all("SELECT * FROM concurrent_test", vec![])
		.await
		.expect("Failed to fetch SQLite rows");
	let sqlite_count = sqlite_rows.len();
	assert_eq!(sqlite_count, 5, "Expected 5 rows in SQLite");

	// Cleanup
	drop_test_table_postgres(&pg_pool, "concurrent_test").await;
	// Note: SQLite table was created via DatabaseConnection, cleanup via connection
	sqlite_conn
		.backend()
		.execute("DROP TABLE IF EXISTS concurrent_test", vec![])
		.await
		.expect("Failed to drop SQLite table");
}

/// Test backend downcasting for accessing database-specific features
#[rstest]
#[tokio::test]
#[serial(backend_switching)]
async fn test_backend_downcasting(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
	#[future] sqlite_fixture: (SqliteInMemory, Arc<SqlitePool>, String),
) {
	let (_pg_container, _pg_pool, _pg_port, pg_url) = postgres_container.await;
	let (_sqlite, _sqlite_pool, sqlite_url) = sqlite_fixture.await;

	// PostgreSQL backend downcasting
	let pg_conn = DatabaseConnection::connect_postgres(&pg_url)
		.await
		.expect("Failed to connect to PostgreSQL");

	let pg_backend = pg_conn.backend();
	let pg_any = pg_backend.as_any();

	// Verify downcasting works (we can't directly access PostgresBackend without exposing it,
	// but we can verify the Any trait works)
	assert!(
		pg_any.is::<reinhardt_backends::dialect::PostgresBackend>(),
		"PostgreSQL backend should downcast to PostgresBackend"
	);

	// SQLite backend downcasting
	let sqlite_conn = DatabaseConnection::connect_sqlite(&sqlite_url)
		.await
		.expect("Failed to connect to SQLite");

	let sqlite_backend = sqlite_conn.backend();
	let sqlite_any = sqlite_backend.as_any();

	assert!(
		sqlite_any.is::<reinhardt_backends::dialect::SqliteBackend>(),
		"SQLite backend should downcast to SqliteBackend"
	);

	// Cross-backend downcasting should fail
	assert!(
		!pg_any.is::<reinhardt_backends::dialect::SqliteBackend>(),
		"PostgreSQL backend should not downcast to SqliteBackend"
	);
	assert!(
		!sqlite_any.is::<reinhardt_backends::dialect::PostgresBackend>(),
		"SQLite backend should not downcast to PostgresBackend"
	);
}
