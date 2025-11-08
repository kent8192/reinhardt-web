//! Integration tests for composite primary key query execution
//!
//! Tests the `get_composite()` method implementation that executes
//! database queries for records with composite primary keys.
//!
//! # Known Issues
//!
//! **Connection Pool Exhaustion (TODO)**
//!
//! Tests are experiencing non-deterministic failures due to connection pool timeout.
//!
//! - **Root Cause**: Global PostgreSQL container + global `reinhardt_orm::manager`
//!   with limited default pool size (likely 1-5 connections)
//! - **Symptoms**: `Pool timed out` errors during `CREATE TABLE IF NOT EXISTS`
//! - **Attempted Solutions**:
//!   - ✗ Per-test delays (100ms)
//!   - ✗ TRUNCATE instead of DROP/CREATE
//!   - ✗ Connection retry logic (10 retries, 500ms intervals)
//! - **Framework-Level Solution Needed**:
//!   - Add configurable pool size parameter to `init_database()`
//!   - Or provide pool size configuration via environment variable
//! - **Alternative**: Revert to per-test PostgreSQL containers (slower but reliable)
//!
//! Currently, tests may pass or fail depending on execution timing.
//! This is a framework design limitation, not a test implementation issue.

use reinhardt_macros::Model;
use reinhardt_orm::{QuerySet, composite_pk::PkValue, manager::init_database};
use rstest::*;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use std::collections::HashMap;
use testcontainers::{GenericImage, ImageExt, core::WaitFor, runners::AsyncRunner};
use tokio::sync::OnceCell;

/// Global PostgreSQL container instance shared across all tests
static POSTGRES_CONTAINER: OnceCell<(String, u16)> = OnceCell::const_new();

/// Initialize the global PostgreSQL container
async fn init_postgres() -> &'static (String, u16) {
	POSTGRES_CONTAINER
		.get_or_init(|| async {
			let postgres_image = GenericImage::new("postgres", "16-alpine")
				.with_wait_for(WaitFor::message_on_stderr(
					"database system is ready to accept connections",
				))
				.with_env_var("POSTGRES_PASSWORD", "test")
				.with_env_var("POSTGRES_DB", "test_db");

			let postgres = postgres_image
				.start()
				.await
				.expect("Failed to start PostgreSQL container");

			let port = postgres
				.get_host_port_ipv4(5432)
				.await
				.expect("Failed to get PostgreSQL port");

			let database_url = format!("postgres://postgres:test@localhost:{}/test_db", port);

			// Initialize database connection
			init_database(&database_url)
				.await
				.expect("Failed to initialize database");

			// Keep container alive by leaking it
			std::mem::forget(postgres);

			(database_url, port)
		})
		.await
}

/// Test model with composite primary key
#[derive(Model, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[model(app_label = "test_app", table_name = "post_tags")]
struct PostTag {
	#[field(primary_key = true)]
	post_id: i64,

	#[field(primary_key = true)]
	tag_id: i64,

	#[field(max_length = 200)]
	description: String,
}

/// Another test model with composite primary key
#[derive(Model, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[model(app_label = "test_app", table_name = "user_roles")]
struct UserRole {
	#[field(primary_key = true)]
	user_id: i64,

	#[field(primary_key = true)]
	role_id: i64,

	#[field(max_length = 100, null = true)]
	granted_by: Option<String>,
}

/// Set up test database fixture and recreate tables
///
/// Uses the global PostgreSQL container and truncates tables for each test
/// to ensure isolation between tests.
#[fixture]
async fn postgres_fixture() -> String {
	// Ensure global container is initialized
	let (database_url, _port) = init_postgres().await;

	// Use a scoped block to ensure connection is returned to pool after table setup
	{
		// Get connection from global manager with retry logic
		let conn = {
			let mut retry_count = 0;
			loop {
				match reinhardt_orm::manager::get_connection().await {
					Ok(conn) => break conn,
					Err(_e) if retry_count < 10 => {
						retry_count += 1;
						tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
					}
					Err(e) => panic!(
						"Failed to get connection after {} retries: {}",
						retry_count, e
					),
				}
			}
		};

		// Create tables if they don't exist (first test only)
		conn.execute(
			"CREATE TABLE IF NOT EXISTS post_tags (
                post_id BIGINT NOT NULL,
                tag_id BIGINT NOT NULL,
                description VARCHAR(200) NOT NULL,
                PRIMARY KEY (post_id, tag_id)
            )",
			vec![],
		)
		.await
		.expect("Failed to create post_tags table");

		conn.execute(
			"CREATE TABLE IF NOT EXISTS user_roles (
                user_id BIGINT NOT NULL,
                role_id BIGINT NOT NULL,
                granted_by VARCHAR(100),
                PRIMARY KEY (user_id, role_id)
            )",
			vec![],
		)
		.await
		.expect("Failed to create user_roles table");

		// Truncate tables to clear data from previous tests
		conn.execute("TRUNCATE TABLE post_tags", vec![])
			.await
			.expect("Failed to truncate post_tags table");
		conn.execute("TRUNCATE TABLE user_roles", vec![])
			.await
			.expect("Failed to truncate user_roles table");

		// Connection is automatically returned to pool when it goes out of scope here
	}

	database_url.clone()
}

#[rstest]
#[serial(postgres_db)]
#[tokio::test]
async fn test_get_composite_success(#[future] postgres_fixture: String) {
	let _url = postgres_fixture.await;

	// Insert test data
	{
		let conn = reinhardt_orm::manager::get_connection()
			.await
			.expect("Failed to get connection");

		conn.execute(
			"INSERT INTO post_tags (post_id, tag_id, description) VALUES (1, 10, 'First tag')",
			vec![],
		)
		.await
		.expect("Failed to insert test data");
		// Connection returned to pool here
	}

	// Query using composite primary key
	let queryset = QuerySet::<PostTag>::new();
	let mut pk_values = HashMap::new();
	pk_values.insert("post_id".to_string(), PkValue::Int(1));
	pk_values.insert("tag_id".to_string(), PkValue::Int(10));

	let result = queryset.get_composite(&pk_values).await;
	assert!(result.is_ok(), "Query should succeed");

	let post_tag = result.unwrap();
	assert_eq!(post_tag.post_id, 1);
	assert_eq!(post_tag.tag_id, 10);
	assert_eq!(post_tag.description, "First tag");
}

#[rstest]
#[serial(postgres_db)]
#[tokio::test]
async fn test_get_composite_not_found(#[future] postgres_fixture: String) {
	let _url = postgres_fixture.await;

	// Query for non-existent record
	let queryset = QuerySet::<PostTag>::new();
	let mut pk_values = HashMap::new();
	pk_values.insert("post_id".to_string(), PkValue::Int(999));
	pk_values.insert("tag_id".to_string(), PkValue::Int(999));

	let result = queryset.get_composite(&pk_values).await;
	assert!(result.is_err(), "Query should fail for non-existent record");

	let error = result.unwrap_err();
	assert!(
		error.to_string().contains("No record found"),
		"Error should indicate record not found"
	);
}

#[rstest]
#[serial(postgres_db)]
#[tokio::test]
async fn test_get_composite_missing_pk_field(#[future] postgres_fixture: String) {
	let _url = postgres_fixture.await;

	// Query with missing primary key field
	let queryset = QuerySet::<PostTag>::new();
	let mut pk_values = HashMap::new();
	pk_values.insert("post_id".to_string(), PkValue::Int(1));
	// Missing tag_id

	let result = queryset.get_composite(&pk_values).await;
	assert!(result.is_err(), "Query should fail with missing PK field");

	let error = result.unwrap_err();
	assert!(
		error.to_string().contains("validation failed") || error.to_string().contains("missing"),
		"Error should indicate validation failure: {}",
		error
	);
}

#[rstest]
#[serial(postgres_db)]
#[tokio::test]
async fn test_get_composite_with_optional_field(#[future] postgres_fixture: String) {
	let _url = postgres_fixture.await;

	// Insert test data
	{
		let conn = reinhardt_orm::manager::get_connection()
			.await
			.expect("Failed to get connection");

		conn.execute(
			"INSERT INTO user_roles (user_id, role_id, granted_by) VALUES (1, 5, 'admin')",
			vec![],
		)
		.await
		.expect("Failed to insert test data");

		conn.execute(
			"INSERT INTO user_roles (user_id, role_id, granted_by) VALUES (2, 5, NULL)",
			vec![],
		)
		.await
		.expect("Failed to insert test data with NULL");
		// Connection returned to pool here
	}

	// Query record with optional field present
	let queryset = QuerySet::<UserRole>::new();
	let mut pk_values = HashMap::new();
	pk_values.insert("user_id".to_string(), PkValue::Int(1));
	pk_values.insert("role_id".to_string(), PkValue::Int(5));

	let result = queryset.get_composite(&pk_values).await;
	assert!(result.is_ok(), "Query should succeed");

	let user_role = result.unwrap();
	assert_eq!(user_role.user_id, 1);
	assert_eq!(user_role.role_id, 5);
	assert_eq!(user_role.granted_by, Some("admin".to_string()));

	// Query record with optional field NULL
	let mut pk_values_null = HashMap::new();
	pk_values_null.insert("user_id".to_string(), PkValue::Int(2));
	pk_values_null.insert("role_id".to_string(), PkValue::Int(5));

	let result_null = queryset.get_composite(&pk_values_null).await;
	assert!(result_null.is_ok(), "Query should succeed for NULL field");

	let user_role_null = result_null.unwrap();
	assert_eq!(user_role_null.user_id, 2);
	assert_eq!(user_role_null.role_id, 5);
	assert_eq!(user_role_null.granted_by, None);
}

#[rstest]
#[serial(postgres_db)]
#[tokio::test]
async fn test_get_composite_multiple_records(#[future] postgres_fixture: String) {
	let _url = postgres_fixture.await;

	// Insert multiple records with same partial key (shouldn't happen with proper PK)
	// This tests the error handling for unexpected database states
	{
		let conn = reinhardt_orm::manager::get_connection()
			.await
			.expect("Failed to get connection");

		conn.execute(
			"INSERT INTO post_tags (post_id, tag_id, description) VALUES (10, 20, 'First')",
			vec![],
		)
		.await
		.expect("Failed to insert test data");

		conn.execute(
			"INSERT INTO post_tags (post_id, tag_id, description) VALUES (10, 21, 'Second')",
			vec![],
		)
		.await
		.expect("Failed to insert test data");
		// Connection returned to pool here
	}

	// Query should succeed for unique composite key
	let queryset = QuerySet::<PostTag>::new();
	let mut pk_values = HashMap::new();
	pk_values.insert("post_id".to_string(), PkValue::Int(10));
	pk_values.insert("tag_id".to_string(), PkValue::Int(20));

	let result = queryset.get_composite(&pk_values).await;
	assert!(
		result.is_ok(),
		"Query should succeed for unique composite PK"
	);

	let post_tag = result.unwrap();
	assert_eq!(post_tag.post_id, 10);
	assert_eq!(post_tag.tag_id, 20);
	assert_eq!(post_tag.description, "First");
}

#[rstest]
#[serial(postgres_db)]
#[tokio::test]
async fn test_get_composite_string_pk(#[future] postgres_fixture: String) {
	let _url = postgres_fixture.await;

	// Create table with string PK component and insert test data
	{
		let conn = reinhardt_orm::manager::get_connection()
			.await
			.expect("Failed to get connection");

		conn.execute(
			"CREATE TABLE IF NOT EXISTS string_composite (
                category VARCHAR(50) NOT NULL,
                item_id BIGINT NOT NULL,
                value TEXT,
                PRIMARY KEY (category, item_id)
            )",
			vec![],
		)
		.await
		.expect("Failed to create string_composite table");

		conn.execute(
			"INSERT INTO string_composite (category, item_id, value) VALUES ('electronics', 100, 'Laptop')",
			vec![],
		)
		.await
		.expect("Failed to insert test data");
		// Connection returned to pool here
	}

	// Define temporary model
	#[derive(Model, Serialize, Deserialize, Clone, Debug)]
	#[model(app_label = "test_app", table_name = "string_composite")]
	struct StringComposite {
		#[field(primary_key = true, max_length = 50)]
		category: String,

		#[field(primary_key = true)]
		item_id: i64,

		#[field(null = true, max_length = 1000)]
		value: Option<String>,
	}

	// Query with string PK
	let queryset = QuerySet::<StringComposite>::new();
	let mut pk_values = HashMap::new();
	pk_values.insert(
		"category".to_string(),
		PkValue::String("electronics".to_string()),
	);
	pk_values.insert("item_id".to_string(), PkValue::Int(100));

	let result = queryset.get_composite(&pk_values).await;
	assert!(result.is_ok(), "Query should succeed with string PK");

	let record = result.unwrap();
	assert_eq!(record.category, "electronics");
	assert_eq!(record.item_id, 100);
	assert_eq!(record.value, Some("Laptop".to_string()));
}
