//! Database Backend Infrastructure Integration Tests
//!
//! This test module validates the integration of DynamicSettings with database backends
//! (PostgreSQL, MySQL) using TestContainers to spin up real database instances.
//!
//! ## Test Categories
//!
//! 1. **PostgreSQL Integration**: Connection, CRUD operations, schema
//! 2. **MySQL Integration**: Connection, CRUD operations, schema
//! 3. **Database-specific Features**: Transactions, concurrent access
//! 4. **Error Handling**: Connection failures, query errors
//!
//! ## Infrastructure
//!
//! - Uses TestContainers to start PostgreSQL 16 and MySQL 8 in Docker
//! - Each test gets isolated database instance
//! - Automatic cleanup after test completion

#[cfg(all(feature = "async", feature = "dynamic-database"))]
mod database_integration_tests {
	use reinhardt_settings::backends::database::DatabaseBackend;
	use reinhardt_settings::dynamic::DynamicSettings;
	use rstest::*;
	use serial_test::serial;
	use sqlx::AnyPool;
	use std::sync::{Arc, Once};
	use std::time::Duration;
	use testcontainers::runners::AsyncRunner;
	use testcontainers::{ContainerAsync, GenericImage, ImageExt};
	use tokio::time::sleep;

	// SQLxドライバーの初期化フラグ
	static INIT_DRIVERS: Once = Once::new();

	/// SQLxのAnyドライバーを初期化
	///
	/// SQLxのAnyデータベースは、実行時にドライバーレジストリに
	/// 具体的なドライバー（Postgres, MySQLなど）を登録する必要があります。
	/// この関数はOnceを使用して、テストセッション全体で1回だけ初期化を行います。
	fn init_sqlx_drivers() {
		INIT_DRIVERS.call_once(|| {
			sqlx::any::install_default_drivers();
		});
	}

	// ============================================================================
	// PostgreSQL Integration Tests
	// ============================================================================

	/// Fixture: Start PostgreSQL container for testing
	///
	/// Returns: (container, database_url)
	#[fixture]
	async fn postgres_container() -> (ContainerAsync<GenericImage>, String) {
		// SQLxドライバーを初期化（テストセッション全体で1回のみ実行）
		init_sqlx_drivers();

		let postgres_image = GenericImage::new("postgres", "16-alpine")
			.with_exposed_port(5432.into())
			.with_env_var("POSTGRES_PASSWORD", "test_password")
			.with_env_var("POSTGRES_DB", "test_db");

		let container = AsyncRunner::start(postgres_image)
			.await
			.expect("Failed to start PostgreSQL container");

		let host_port = container
			.get_host_port_ipv4(5432)
			.await
			.expect("Failed to get PostgreSQL port");

		let database_url = format!(
			"postgres://postgres:test_password@127.0.0.1:{}/test_db",
			host_port
		);

		// Wait for PostgreSQL to be ready
		sleep(Duration::from_secs(2)).await;

		// Initialize the database schema
		let backend = DatabaseBackend::new(&database_url)
			.await
			.expect("Failed to create PostgreSQL backend for initialization");

		backend
			.create_table()
			.await
			.expect("Failed to create settings table");

		(container, database_url)
	}

	/// Test: PostgreSQL backend basic connectivity
	///
	/// Why: Verifies that DatabaseBackend can connect to real PostgreSQL instance.
	#[rstest]
	#[serial(postgres)]
	#[tokio::test]
	async fn test_postgres_backend_connectivity(
		#[future] postgres_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, database_url) = postgres_container.await;

		let backend = DatabaseBackend::new(&database_url)
			.await
			.expect("Failed to create PostgreSQL backend");

		let dynamic = DynamicSettings::new(Arc::new(backend));

		// Just verify it can be created without errors
		let result = dynamic.get::<String>("nonexistent_key").await;
		assert!(result.is_ok(), "PostgreSQL backend should be operational");
	}

	/// Test: PostgreSQL backend set and get operations
	///
	/// Why: Verifies that values can be stored and retrieved from PostgreSQL.
	#[rstest]
	#[serial(postgres)]
	#[tokio::test]
	async fn test_postgres_backend_set_get(
		#[future] postgres_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, database_url) = postgres_container.await;

		let backend = DatabaseBackend::new(&database_url)
			.await
			.expect("Failed to create PostgreSQL backend");

		let dynamic = DynamicSettings::new(Arc::new(backend));

		// Set value
		dynamic
			.set("postgres.test.key", &"test_value", None)
			.await
			.expect("Failed to set value");

		// Get value
		let value: Option<String> = dynamic
			.get("postgres.test.key")
			.await
			.expect("Failed to get value");

		assert_eq!(
			value,
			Some("test_value".to_string()),
			"PostgreSQL backend should persist values"
		);
	}

	/// Test: PostgreSQL backend with different value types
	///
	/// Why: Verifies that PostgreSQL backend correctly serializes/deserializes different types.
	#[rstest]
	#[serial(postgres)]
	#[tokio::test]
	async fn test_postgres_backend_value_types(
		#[future] postgres_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, database_url) = postgres_container.await;

		let backend = DatabaseBackend::new(&database_url)
			.await
			.expect("Failed to create PostgreSQL backend");

		let dynamic = DynamicSettings::new(Arc::new(backend));

		// String
		dynamic.set("pg.string_key", &"hello", None).await.unwrap();
		let s: Option<String> = dynamic.get("pg.string_key").await.unwrap();
		assert_eq!(s, Some("hello".to_string()));

		// Integer
		dynamic.set("pg.int_key", &42, None).await.unwrap();
		let i: Option<i64> = dynamic.get("pg.int_key").await.unwrap();
		assert_eq!(i, Some(42));

		// Boolean
		dynamic.set("pg.bool_key", &true, None).await.unwrap();
		let b: Option<bool> = dynamic.get("pg.bool_key").await.unwrap();
		assert_eq!(b, Some(true));

		// Array
		let arr = vec!["a", "b", "c"];
		dynamic.set("pg.array_key", &arr, None).await.unwrap();
		let a: Option<Vec<String>> = dynamic.get("pg.array_key").await.unwrap();
		assert_eq!(
			a,
			Some(vec!["a".to_string(), "b".to_string(), "c".to_string()])
		);
	}

	/// Test: PostgreSQL backend concurrent access
	///
	/// Why: Verifies that multiple connections can access PostgreSQL concurrently.
	#[rstest]
	#[serial(postgres)]
	#[tokio::test]
	async fn test_postgres_backend_concurrent_access(
		#[future] postgres_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, database_url) = postgres_container.await;

		let backend = Arc::new(
			DatabaseBackend::new(&database_url)
				.await
				.expect("Failed to create PostgreSQL backend"),
		);

		let dynamic = Arc::new(DynamicSettings::new(backend));

		// Spawn multiple concurrent tasks
		let mut handles = vec![];

		for i in 0..5 {
			let dynamic_clone = Arc::clone(&dynamic);
			let handle = tokio::spawn(async move {
				let key = format!("pg.concurrent.key.{}", i);
				let value = format!("value_{}", i);

				// Set value
				dynamic_clone
					.set(&key, &value, None)
					.await
					.expect("Failed to set value");

				// Get value
				let retrieved: Option<String> =
					dynamic_clone.get(&key).await.expect("Failed to get value");

				assert_eq!(retrieved, Some(value));
			});
			handles.push(handle);
		}

		// Wait for all tasks to complete
		for handle in handles {
			handle.await.expect("Task panicked");
		}
	}

	/// Test: PostgreSQL backend update existing key
	///
	/// Why: Verifies that values can be updated in PostgreSQL.
	#[rstest]
	#[serial(postgres)]
	#[tokio::test]
	async fn test_postgres_backend_update(
		#[future] postgres_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, database_url) = postgres_container.await;

		let backend = DatabaseBackend::new(&database_url)
			.await
			.expect("Failed to create PostgreSQL backend");

		let dynamic = DynamicSettings::new(Arc::new(backend));

		// Set initial value
		dynamic
			.set("pg.update.key", &"initial", None)
			.await
			.expect("Failed to set initial value");

		let value1: Option<String> = dynamic.get("pg.update.key").await.unwrap();
		assert_eq!(value1, Some("initial".to_string()));

		// Update value
		dynamic
			.set("pg.update.key", &"updated", None)
			.await
			.expect("Failed to update value");

		let value2: Option<String> = dynamic.get("pg.update.key").await.unwrap();
		assert_eq!(
			value2,
			Some("updated".to_string()),
			"Value should be updated"
		);
	}

	// ============================================================================
	// MySQL Integration Tests
	// ============================================================================

	/// Fixture: Start MySQL container for testing
	///
	/// Returns: (container, database_url)
	#[fixture]
	async fn mysql_container() -> (ContainerAsync<GenericImage>, String) {
		// SQLxドライバーを初期化（テストセッション全体で1回のみ実行）
		init_sqlx_drivers();

		let mysql_image = GenericImage::new("mysql", "8")
			.with_exposed_port(3306.into())
			.with_env_var("MYSQL_ROOT_PASSWORD", "test_password")
			.with_env_var("MYSQL_DATABASE", "test_db");

		let container = AsyncRunner::start(mysql_image)
			.await
			.expect("Failed to start MySQL container");

		let host_port = container
			.get_host_port_ipv4(3306)
			.await
			.expect("Failed to get MySQL port");

		let database_url = format!("mysql://root:test_password@127.0.0.1:{}/test_db", host_port);

		// Wait for MySQL to be ready (MySQL takes longer to start)
		// Retry connection with exponential backoff
		let mut retry_count = 0;
		let max_retries = 10;

		loop {
			match DatabaseBackend::new(&database_url).await {
				Ok(_backend) => {
					// Create table using MySQL-specific SQL
					let pool = AnyPool::connect(&database_url)
						.await
						.expect("Failed to connect for table creation");

					sqlx::query(
						"CREATE TABLE IF NOT EXISTS settings (
							`key` VARCHAR(255) NOT NULL PRIMARY KEY,
							`value` TEXT NOT NULL,
							`expire_date` TEXT
						) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4",
					)
					.execute(&pool)
					.await
					.expect("Failed to create settings table");

					sqlx::query(
						"CREATE INDEX IF NOT EXISTS idx_settings_expire_date ON settings(`expire_date`)",
					)
					.execute(&pool)
					.await
					.expect("Failed to create expire_date index");

					return (container, database_url);
				}
				Err(_e) if retry_count < max_retries => {
					retry_count += 1;
					sleep(Duration::from_millis(500 * 2_u64.pow(retry_count as u32))).await;
				}
				Err(e) => {
					panic!(
						"Failed to initialize MySQL after {} retries: {}",
						max_retries, e
					);
				}
			}
		}
	}

	/// Test: MySQL backend basic connectivity
	///
	/// Why: Verifies that DatabaseBackend can connect to real MySQL instance.
	#[rstest]
	#[serial(mysql)]
	#[tokio::test]
	async fn test_mysql_backend_connectivity(
		#[future] mysql_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, database_url) = mysql_container.await;

		let backend = DatabaseBackend::new(&database_url)
			.await
			.expect("Failed to create MySQL backend");

		let dynamic = DynamicSettings::new(Arc::new(backend));

		// Just verify it can be created without errors
		let result = dynamic.get::<String>("nonexistent_key").await;
		assert!(result.is_ok(), "MySQL backend should be operational");
	}

	/// Test: MySQL backend set and get operations
	///
	/// Why: Verifies that values can be stored and retrieved from MySQL.
	#[rstest]
	#[serial(mysql)]
	#[tokio::test]
	async fn test_mysql_backend_set_get(
		#[future] mysql_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, database_url) = mysql_container.await;

		let backend = DatabaseBackend::new(&database_url)
			.await
			.expect("Failed to create MySQL backend");

		let dynamic = DynamicSettings::new(Arc::new(backend));

		// Set value
		dynamic
			.set("mysql.test.key", &"test_value", None)
			.await
			.expect("Failed to set value");

		// Get value
		let value: Option<String> = dynamic
			.get("mysql.test.key")
			.await
			.expect("Failed to get value");

		assert_eq!(
			value,
			Some("test_value".to_string()),
			"MySQL backend should persist values"
		);
	}

	/// Test: MySQL backend concurrent access
	///
	/// Why: Verifies that multiple connections can access MySQL concurrently.
	#[rstest]
	#[serial(mysql)]
	#[tokio::test]
	async fn test_mysql_backend_concurrent_access(
		#[future] mysql_container: (ContainerAsync<GenericImage>, String),
	) {
		let (_container, database_url) = mysql_container.await;

		let backend = Arc::new(
			DatabaseBackend::new(&database_url)
				.await
				.expect("Failed to create MySQL backend"),
		);

		let dynamic = Arc::new(DynamicSettings::new(backend));

		// Spawn multiple concurrent tasks
		let mut handles = vec![];

		for i in 0..5 {
			let dynamic_clone = Arc::clone(&dynamic);
			let handle = tokio::spawn(async move {
				let key = format!("mysql.concurrent.key.{}", i);
				let value = format!("value_{}", i);

				// Set value
				dynamic_clone
					.set(&key, &value, None)
					.await
					.expect("Failed to set value");

				// Get value
				let retrieved: Option<String> =
					dynamic_clone.get(&key).await.expect("Failed to get value");

				assert_eq!(retrieved, Some(value));
			});
			handles.push(handle);
		}

		// Wait for all tasks to complete
		for handle in handles {
			handle.await.expect("Task panicked");
		}
	}
}
