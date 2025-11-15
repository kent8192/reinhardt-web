//! Integration tests for BatchValidator with real database
//!
//! These tests verify that BatchValidator correctly executes database queries
//! for unique and unique_together validation checks.

use reinhardt_serializers::BatchValidator;
use sea_orm::{Database, DatabaseConnection};
use testcontainers::{GenericImage, ImageExt, core::WaitFor, runners::AsyncRunner};

/// Set up test database and create test tables
///
/// Returns:
/// - PostgreSQL container (must be kept alive for the test duration)
/// - Database URL string
async fn setup_test_db() -> (testcontainers::ContainerAsync<GenericImage>, String) {
	// Start PostgreSQL container
	let postgres = GenericImage::new("postgres", "16-alpine")
		.with_wait_for(WaitFor::message_on_stderr(
			"database system is ready to accept connections",
		))
		.with_env_var("POSTGRES_PASSWORD", "test")
		.with_env_var("POSTGRES_DB", "test_db")
		.start()
		.await
		.expect("Failed to start PostgreSQL container");

	let port = postgres
		.get_host_port_ipv4(5432)
		.await
		.expect("Failed to get PostgreSQL port");

	let database_url = format!("postgres://postgres:test@localhost:{}/test_db", port);

	// Initialize global database connection
	reinhardt_db::orm::manager::init_database(&database_url)
		.await
		.expect("Failed to initialize database");

	// Create connection for table setup
	let conn = Database::connect(&database_url)
		.await
		.expect("Failed to connect to database");

	// Create test tables
	create_test_tables(&conn).await;

	(postgres, database_url)
}

/// Create test tables for BatchValidator tests
async fn create_test_tables(conn: &DatabaseConnection) {
	use sea_orm::ConnectionTrait;

	// Create users table
	conn.execute_unprepared(
		"CREATE TABLE IF NOT EXISTS users (
            id SERIAL PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            username TEXT NOT NULL UNIQUE,
            first_name TEXT,
            last_name TEXT
        )",
	)
	.await
	.expect("Failed to create users table");

	// Create products table
	conn.execute_unprepared(
		"CREATE TABLE IF NOT EXISTS products (
            id SERIAL PRIMARY KEY,
            sku TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            price DECIMAL(10, 2)
        )",
	)
	.await
	.expect("Failed to create products table");

	// Insert test data
	conn.execute_unprepared(
		"INSERT INTO users (email, username, first_name, last_name) VALUES
            ('existing@example.com', 'existing_user', 'Existing', 'User'),
            ('alice@example.com', 'alice', 'Alice', 'Smith')",
	)
	.await
	.expect("Failed to insert test users");

	conn.execute_unprepared(
		"INSERT INTO products (sku, name, price) VALUES
            ('PROD-123', 'Test Product', 99.99)",
	)
	.await
	.expect("Failed to insert test products");
}

/// Test basic unique field validation with real database
#[tokio::test]
async fn test_batch_validator_basic_integration() {
	let (_container, _db_url) = setup_test_db().await;

	let mut validator = BatchValidator::new();
	assert_eq!(validator.pending_count(), 0);

	// Add checks for existing data (should fail validation)
	validator.add_unique_check("users", "email", "existing@example.com");
	validator.add_unique_check("users", "username", "existing_user");

	assert_eq!(validator.pending_count(), 2);

	// Execute validation
	let result = validator.execute().await;
	assert!(result.is_ok());
	let failures = result.unwrap();

	// Debug output
	eprintln!("Failures: {:?}", failures);

	// Both checks should fail (existing data found)
	assert_eq!(failures.len(), 2);

	// Verify failure messages
	assert!(failures.contains_key("users:email:existing@example.com"));
	assert!(failures.contains_key("users:username:existing_user"));

	// Test with new data (should pass validation)
	validator.clear();
	validator.add_unique_check("users", "email", "new@example.com");
	validator.add_unique_check("users", "username", "newuser");

	let result = validator.execute().await;
	assert!(result.is_ok());
	let failures = result.unwrap();

	// No failures expected for new data
	assert_eq!(failures.len(), 0);
}

/// Test mixed unique and unique_together checks with real database
#[tokio::test]
async fn test_batch_validator_mixed_checks_integration() {
	let (_container, _db_url) = setup_test_db().await;

	let mut validator = BatchValidator::new();

	// Add single field unique check (existing email)
	validator.add_unique_check("users", "email", "alice@example.com");

	// Add unique_together check (existing combination)
	validator.add_unique_together_check(
		"users",
		vec!["first_name".to_string(), "last_name".to_string()],
		vec!["Alice".to_string(), "Smith".to_string()],
	);

	// Add unique check for products (existing SKU)
	validator.add_unique_check("products", "sku", "PROD-123");

	assert_eq!(validator.pending_count(), 3);

	// Execute validation
	let result = validator.execute().await;
	assert!(result.is_ok());
	let failures = result.unwrap();

	// All 3 checks should fail (existing data found)
	assert_eq!(failures.len(), 3);

	// Verify specific failures
	assert!(failures.contains_key("users:email:alice@example.com"));
	assert!(failures.contains_key("users:first_name+last_name:Alice+Smith"));
	assert!(failures.contains_key("products:sku:PROD-123"));

	// Test with new data (should pass validation)
	validator.clear();
	validator.add_unique_check("users", "email", "bob@example.com");
	validator.add_unique_together_check(
		"users",
		vec!["first_name".to_string(), "last_name".to_string()],
		vec!["Bob".to_string(), "Jones".to_string()],
	);
	validator.add_unique_check("products", "sku", "PROD-456");

	let result = validator.execute().await;
	assert!(result.is_ok());
	let failures = result.unwrap();

	// No failures expected for new data
	assert_eq!(failures.len(), 0);
}

/// Test batch validator with empty checks
#[tokio::test]
async fn test_batch_validator_empty() {
	let (_container, _db_url) = setup_test_db().await;

	let validator = BatchValidator::new();
	assert_eq!(validator.pending_count(), 0);

	// Execute with no checks
	let result = validator.execute().await;
	assert!(result.is_ok());
	let failures = result.unwrap();
	assert_eq!(failures.len(), 0);
}
