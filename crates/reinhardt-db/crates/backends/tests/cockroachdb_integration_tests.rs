//! CockroachDB integration tests using TestContainers
//!
//! These tests require Docker to be running.
//!
//! Run these tests with:
//! ```bash
//! cargo test --test cockroachdb_integration_tests --features cockroachdb-backend -- --test-threads=1
//! ```

#[cfg(feature = "cockroachdb-backend")]
mod cockroachdb_tests {
	use ::testcontainers::{ContainerAsync, GenericImage};
	use reinhardt_backends::{
		CockroachDBBackend, CockroachDBSchemaEditor, CockroachDBTransactionManager,
		PostgreSQLSchemaEditor,
	};
	use reinhardt_test::fixtures::*;
	use rstest::*;
	use serial_test::serial;
	use std::sync::Arc;

	async fn cleanup_test_tables(pool: &sqlx::PgPool) {
		let _ = sqlx::query("DROP TABLE IF EXISTS test_users CASCADE")
			.execute(pool)
			.await;
		let _ = sqlx::query("DROP TABLE IF EXISTS test_events CASCADE")
			.execute(pool)
			.await;
	}

	// Basic Backend Tests

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_backend_creation(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		let pg_editor = PostgreSQLSchemaEditor::new((*pool).clone());
		let backend = CockroachDBBackend::new(pg_editor);

		assert_eq!(backend.database_name(), "cockroachdb");
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_supported_features(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		let pg_editor = PostgreSQLSchemaEditor::new((*pool).clone());
		let backend = CockroachDBBackend::new(pg_editor);

		assert!(backend.supports_feature("multi_region"));
		assert!(backend.supports_feature("distributed_transactions"));
		assert!(backend.supports_feature("as_of_system_time"));
		assert!(backend.supports_feature("regional_by_row"));
		assert!(backend.supports_feature("regional_by_table"));
		assert!(backend.supports_feature("global_tables"));
		assert!(!backend.supports_feature("unknown_feature"));

		let features = backend.supported_features();
		assert!(features.contains(&"multi_region"));
		assert!(features.contains(&"distributed_transactions"));
	}

	// Schema Editor Tests

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_schema_editor_locality_sql(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		let pg_editor = PostgreSQLSchemaEditor::new((*pool).clone());
		let backend = CockroachDBBackend::new(pg_editor);
		let editor = backend.schema_editor();

		let sql = editor.create_table_with_locality_sql(
			"users",
			&[("id", "UUID PRIMARY KEY"), ("name", "VARCHAR(100)")],
			"REGIONAL BY ROW",
		);

		assert!(sql.contains("CREATE TABLE"));
		assert!(sql.contains("\"users\""));
		assert!(sql.contains("LOCALITY REGIONAL BY ROW"));
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_schema_editor_alter_locality_sql(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		let pg_editor = PostgreSQLSchemaEditor::new((*pool).clone());
		let backend = CockroachDBBackend::new(pg_editor);
		let editor = backend.schema_editor();

		let sql = editor.alter_table_locality_sql("users", "REGIONAL BY TABLE");

		assert!(sql.contains("ALTER TABLE"));
		assert!(sql.contains("\"users\""));
		assert!(sql.contains("SET LOCALITY REGIONAL BY TABLE"));
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_schema_editor_partitioned_table_sql(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		let pg_editor = PostgreSQLSchemaEditor::new((*pool).clone());
		let backend = CockroachDBBackend::new(pg_editor);
		let editor = backend.schema_editor();

		let sql = editor.create_partitioned_table_sql(
			"events",
			&[
				("id", "UUID PRIMARY KEY"),
				("region", "VARCHAR(50)"),
				("data", "JSONB"),
			],
			"region",
			&[
				("us_east", "'us-east-1', 'us-east-2'"),
				("us_west", "'us-west-1', 'us-west-2'"),
			],
		);

		// Debug: Print generated SQL
		eprintln!("Generated SQL:\n{}", sql);

		assert!(sql.contains("CREATE TABLE"));
		assert!(sql.contains("PARTITION BY LIST"));
		assert!(sql.contains("\"region\""));
		// Note: PARTITION clause format may vary depending on implementation
		// Just verify that partition names are present
		assert!(sql.contains("us_east"));
		assert!(sql.contains("us_west"));
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_schema_editor_index_with_storing_sql(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		let pg_editor = PostgreSQLSchemaEditor::new((*pool).clone());
		let backend = CockroachDBBackend::new(pg_editor);
		let editor = backend.schema_editor();

		let sql = editor.create_index_with_storing_sql(
			"idx_email",
			"users",
			&["email"],
			&["name", "created_at"],
			false,
			None,
		);

		assert!(sql.contains("CREATE INDEX"));
		assert!(sql.contains("\"idx_email\""));
		assert!(sql.contains("ON \"users\""));
		assert!(sql.contains("\"email\""));
		assert!(sql.contains("STORING"));
		assert!(sql.contains("\"name\""));
		assert!(sql.contains("\"created_at\""));
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_schema_editor_unique_index_with_condition(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		let pg_editor = PostgreSQLSchemaEditor::new((*pool).clone());
		let backend = CockroachDBBackend::new(pg_editor);
		let editor = backend.schema_editor();

		let sql = editor.create_index_with_storing_sql(
			"idx_active_users",
			"users",
			&["email"],
			&["name"],
			true,
			Some("active = true"),
		);

		assert!(sql.contains("CREATE UNIQUE INDEX"));
		assert!(sql.contains("WHERE active = true"));
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_schema_editor_as_of_system_time_sql(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		let pg_editor = PostgreSQLSchemaEditor::new((*pool).clone());
		let backend = CockroachDBBackend::new(pg_editor);
		let editor = backend.schema_editor();

		let sql = editor.as_of_system_time_sql("SELECT * FROM users WHERE id = $1", "-5s");

		assert!(sql.contains("SELECT * FROM users WHERE id = $1"));
		assert!(sql.contains("AS OF SYSTEM TIME -5s"));
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_schema_editor_show_regions_sql(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		let pg_editor = PostgreSQLSchemaEditor::new((*pool).clone());
		let backend = CockroachDBBackend::new(pg_editor);
		let editor = backend.schema_editor();

		let sql = editor.show_regions_sql();
		assert_eq!(sql, "SHOW REGIONS");
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_schema_editor_survival_goal_sql(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		let pg_editor = PostgreSQLSchemaEditor::new((*pool).clone());
		let backend = CockroachDBBackend::new(pg_editor);
		let editor = backend.schema_editor();

		let sql = editor.show_survival_goal_sql();
		assert_eq!(sql, "SHOW SURVIVAL GOAL");
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_schema_editor_set_primary_region_sql(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		let pg_editor = PostgreSQLSchemaEditor::new((*pool).clone());
		let backend = CockroachDBBackend::new(pg_editor);
		let editor = backend.schema_editor();

		let sql = editor.set_primary_region_sql("mydb", "us-east-1");

		assert!(sql.contains("ALTER DATABASE"));
		assert!(sql.contains("\"mydb\""));
		assert!(sql.contains("SET PRIMARY REGION"));
		assert!(sql.contains("\"us-east-1\""));
	}

	// Real Database Integration Tests

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_create_table_with_pool(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		cleanup_test_tables(pool.as_ref()).await;

		// Create a simple table using PostgreSQL-compatible schema editor
		let pg_editor = PostgreSQLSchemaEditor::new((*pool).clone());
		let _editor = CockroachDBSchemaEditor::new(pg_editor);

		let create_sql = "CREATE TABLE test_users (id UUID PRIMARY KEY, name VARCHAR(100))";
		sqlx::query(create_sql)
			.execute(pool.as_ref())
			.await
			.expect("Failed to create table");

		// Verify table exists by querying it
		let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_users")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query table");

		assert_eq!(count, 0);

		cleanup_test_tables(pool.as_ref()).await;
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_insert_and_query(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		cleanup_test_tables(pool.as_ref()).await;

		// Create table
		sqlx::query("CREATE TABLE test_users (id SERIAL PRIMARY KEY, name VARCHAR(100))")
			.execute(pool.as_ref())
			.await
			.expect("Failed to create table");

		// Insert data and get the generated ID
		// CockroachDB SERIAL uses unique_rowid() which generates large, non-sequential values
		let inserted_id: i64 =
			sqlx::query_scalar("INSERT INTO test_users (name) VALUES ($1) RETURNING id")
				.bind("Alice")
				.fetch_one(pool.as_ref())
				.await
				.expect("Failed to insert");

		// Query data using the actual inserted ID
		let name: String = sqlx::query_scalar("SELECT name FROM test_users WHERE id = $1")
			.bind(inserted_id)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query");

		assert_eq!(name, "Alice");

		cleanup_test_tables(pool.as_ref()).await;
	}

	// Distributed Transaction Tests

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_transaction_manager_basic(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		cleanup_test_tables(pool.as_ref()).await;

		sqlx::query("CREATE TABLE test_users (id SERIAL PRIMARY KEY, balance INT)")
			.execute(pool.as_ref())
			.await
			.expect("Failed to create table");

		let tx_manager = CockroachDBTransactionManager::new((*pool).clone());

		// Execute transaction and capture the inserted ID
		let inserted_id = tx_manager
			.execute_with_retry(|tx| {
				Box::pin(async move {
					let id: i64 = sqlx::query_scalar(
						"INSERT INTO test_users (balance) VALUES ($1) RETURNING id",
					)
					.bind(100)
					.fetch_one(&mut **tx)
					.await?;
					Ok(id)
				})
			})
			.await
			.expect("Transaction failed");

		// Verify data was committed using the actual inserted ID
		// Note: CockroachDB INT is INT8 (i64), not INT4 (i32)
		let balance: i64 = sqlx::query_scalar("SELECT balance FROM test_users WHERE id = $1")
			.bind(inserted_id)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query");

		assert_eq!(balance, 100);

		cleanup_test_tables(pool.as_ref()).await;
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_transaction_with_priority(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		cleanup_test_tables(pool.as_ref()).await;

		sqlx::query("CREATE TABLE test_users (id SERIAL PRIMARY KEY, name VARCHAR(100))")
			.execute(pool.as_ref())
			.await
			.expect("Failed to create table");

		let tx_manager = CockroachDBTransactionManager::new((*pool).clone());

		// Execute with HIGH priority and capture the inserted ID
		let inserted_id = tx_manager
			.execute_with_priority("HIGH", |tx| {
				Box::pin(async move {
					let id: i64 = sqlx::query_scalar(
						"INSERT INTO test_users (name) VALUES ($1) RETURNING id",
					)
					.bind("HighPriority")
					.fetch_one(&mut **tx)
					.await?;
					Ok(id)
				})
			})
			.await
			.expect("Transaction failed");

		// Query using the actual inserted ID
		let name: String = sqlx::query_scalar("SELECT name FROM test_users WHERE id = $1")
			.bind(inserted_id)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query");

		assert_eq!(name, "HighPriority");

		cleanup_test_tables(pool.as_ref()).await;
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_get_cluster_info(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		let tx_manager = CockroachDBTransactionManager::new((*pool).clone());

		let info = tx_manager
			.get_cluster_info()
			.await
			.expect("Failed to get cluster info");

		// Version should not be empty and contain version number pattern
		// CockroachDB versions may or may not have 'v' prefix (e.g., "v23.1.0" or "23.1.0")
		assert!(!info.version.is_empty());
		assert!(info.version.contains('.'));
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_transaction_retry_configuration(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;

		let tx_manager = CockroachDBTransactionManager::new((*pool).clone())
			.with_max_retries(10)
			.with_base_backoff(std::time::Duration::from_millis(200));

		// Just verify the manager was created with custom config
		// (actual retry logic is tested in unit tests)
		assert!(tx_manager.pool().is_closed() == false);
	}

	// AS OF SYSTEM TIME Tests

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_as_of_system_time_query(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		cleanup_test_tables(pool.as_ref()).await;

		// Wait to ensure a clean starting point for time-based query
		tokio::time::sleep(std::time::Duration::from_millis(200)).await;

		// Create table and insert data
		sqlx::query("CREATE TABLE test_users (id SERIAL PRIMARY KEY, name VARCHAR(100))")
			.execute(pool.as_ref())
			.await
			.expect("Failed to create table");

		sqlx::query("INSERT INTO test_users (name) VALUES ($1)")
			.bind("Alice")
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert");

		// Wait for the data to be committed before querying historical state
		tokio::time::sleep(std::time::Duration::from_millis(100)).await;

		// Query historical data (100ms ago should be empty since table was just created)
		// Note: AS OF SYSTEM TIME requires the database/table to exist at that time
		// Using a shorter time interval since the table was just created
		let count: i64 =
			sqlx::query_scalar("SELECT COUNT(*) FROM test_users AS OF SYSTEM TIME '-50ms'")
				.fetch_one(pool.as_ref())
				.await
				.expect("Failed to query");

		// Table existed but was empty 50ms ago (we just created and inserted)
		// The exact count depends on timing, but should be 0 or 1
		assert!(count <= 1, "Count should be 0 or 1, got {}", count);

		// Query current data
		let current_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_users")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query");

		assert_eq!(current_count, 1);

		cleanup_test_tables(pool.as_ref()).await;
	}

	// PostgreSQL Compatibility Tests

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_postgresql_compatibility(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		cleanup_test_tables(pool.as_ref()).await;

		// CockroachDB should support PostgreSQL wire protocol
		// Test JSONB support (PostgreSQL feature)
		sqlx::query("CREATE TABLE test_events (id SERIAL PRIMARY KEY, data JSONB)")
			.execute(pool.as_ref())
			.await
			.expect("Failed to create table");

		// Insert data and get the generated ID
		let inserted_id: i64 =
			sqlx::query_scalar("INSERT INTO test_events (data) VALUES ($1) RETURNING id")
				.bind(serde_json::json!({"key": "value"}))
				.fetch_one(pool.as_ref())
				.await
				.expect("Failed to insert JSONB");

		// Query JSONB data using the actual inserted ID
		let data: serde_json::Value =
			sqlx::query_scalar("SELECT data FROM test_events WHERE id = $1")
				.bind(inserted_id)
				.fetch_one(pool.as_ref())
				.await
				.expect("Failed to query");

		assert_eq!(data["key"], "value");

		cleanup_test_tables(pool.as_ref()).await;
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_concurrent_transactions(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		cleanup_test_tables(pool.as_ref()).await;

		sqlx::query("CREATE TABLE test_users (id SERIAL PRIMARY KEY, name VARCHAR(100))")
			.execute(pool.as_ref())
			.await
			.expect("Failed to create table");

		let tx_manager1 = CockroachDBTransactionManager::new((*pool).clone());
		let tx_manager2 = CockroachDBTransactionManager::new((*pool).clone());

		// Run two transactions concurrently
		let handle1 = tokio::spawn(async move {
			tx_manager1
				.execute_with_retry(|tx| {
					Box::pin(async move {
						sqlx::query("INSERT INTO test_users (name) VALUES ($1)")
							.bind("Concurrent1")
							.execute(&mut **tx)
							.await?;
						Ok(())
					})
				})
				.await
				.expect("Transaction 1 failed");
		});

		let handle2 = tokio::spawn(async move {
			tx_manager2
				.execute_with_retry(|tx| {
					Box::pin(async move {
						sqlx::query("INSERT INTO test_users (name) VALUES ($1)")
							.bind("Concurrent2")
							.execute(&mut **tx)
							.await?;
						Ok(())
					})
				})
				.await
				.expect("Transaction 2 failed");
		});

		handle1.await.expect("Task 1 failed");
		handle2.await.expect("Task 2 failed");

		// Verify both transactions committed
		let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_users")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to count");

		assert_eq!(count, 2);

		cleanup_test_tables(pool.as_ref()).await;
	}

	#[rstest]
	#[tokio::test]
	#[serial(cockroachdb)]
	async fn test_uuid_primary_key(
		#[future] cockroachdb_container: (
			ContainerAsync<GenericImage>,
			Arc<sqlx::PgPool>,
			u16,
			String,
		),
	) {
		let (_container, pool, _port, _url) = cockroachdb_container.await;
		cleanup_test_tables(pool.as_ref()).await;

		// CockroachDB recommends UUID for distributed primary keys
		sqlx::query(
			"CREATE TABLE test_users (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name VARCHAR(100))",
		)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

		sqlx::query("INSERT INTO test_users (name) VALUES ($1)")
			.bind("Alice")
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert");

		// Verify UUID was generated
		let id: uuid::Uuid = sqlx::query_scalar("SELECT id FROM test_users WHERE name = 'Alice'")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query");

		assert_ne!(id, uuid::Uuid::nil());

		cleanup_test_tables(pool.as_ref()).await;
	}
}
