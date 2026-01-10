//! Integration tests for BatchValidator with real database
//!
//! These tests verify that BatchValidator correctly executes database queries
//! for unique and unique_together validation checks.

use reinhardt_serializers::BatchValidator;
use reinhardt_test::fixtures::{ColumnDefinition, FieldType, Operation, SqlDialect};
use sea_orm::{Database, DatabaseConnection};
use sea_query::{Iden, PostgresQueryBuilder, Query};
use testcontainers::{GenericImage, ImageExt, core::WaitFor, runners::AsyncRunner};

/// Set up test database and create test tables
///
/// Returns:
/// - PostgreSQL container (must be kept alive for the test duration)
/// - Database URL string
async fn setup_test_db() -> (testcontainers::ContainerAsync<GenericImage>, String) {
	// Start PostgreSQL container
	let image = GenericImage::new("postgres", "16-alpine")
		.with_wait_for(WaitFor::message_on_stderr(
			"database system is ready to accept connections",
		))
		.with_env_var("POSTGRES_PASSWORD", "test")
		.with_env_var("POSTGRES_DB", "test_db");

	let postgres = AsyncRunner::start(image)
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

// Table identifiers for SeaQuery
#[derive(Iden)]
enum Users {
	Table,
	Email,
	Username,
	FirstName,
	LastName,
}

#[derive(Iden)]
enum Products {
	Table,
	Sku,
	Name,
	Price,
}

/// Create test tables for BatchValidator tests
async fn create_test_tables(conn: &DatabaseConnection) {
	use sea_orm::ConnectionTrait;

	// Create users table using Operation-based schema definition
	let mut id_column = ColumnDefinition::new("id", FieldType::Integer);
	id_column.primary_key = true;
	id_column.auto_increment = true;

	let mut email_column = ColumnDefinition::new("email", FieldType::Text);
	email_column.not_null = true;
	email_column.unique = true;

	let mut username_column = ColumnDefinition::new("username", FieldType::Text);
	username_column.not_null = true;
	username_column.unique = true;

	let first_name_column = ColumnDefinition::new("first_name", FieldType::Text);
	let last_name_column = ColumnDefinition::new("last_name", FieldType::Text);

	let users_table_op = Operation::CreateTable {
		name: "users".to_string(),
		columns: vec![
			id_column,
			email_column,
			username_column,
			first_name_column,
			last_name_column,
		],
		constraints: vec![],
		without_rowid: None,
		interleave_in_parent: None,
		partition: None,
	};

	let users_sql = users_table_op.to_sql(&SqlDialect::Postgres);
	conn.execute_unprepared(&users_sql)
		.await
		.expect("Failed to create users table");

	// Create products table using Operation-based schema definition
	let mut product_id = ColumnDefinition::new("id", FieldType::Integer);
	product_id.primary_key = true;
	product_id.auto_increment = true;

	let mut sku_column = ColumnDefinition::new("sku", FieldType::Text);
	sku_column.not_null = true;
	sku_column.unique = true;

	let mut name_column = ColumnDefinition::new("name", FieldType::Text);
	name_column.not_null = true;

	let price_column = ColumnDefinition::new(
		"price",
		FieldType::Decimal {
			precision: 10,
			scale: 2,
		},
	);

	let products_table_op = Operation::CreateTable {
		name: "products".to_string(),
		columns: vec![product_id, sku_column, name_column, price_column],
		constraints: vec![],
		without_rowid: None,
		interleave_in_parent: None,
		partition: None,
	};

	let products_sql = products_table_op.to_sql(&SqlDialect::Postgres);
	conn.execute_unprepared(&products_sql)
		.await
		.expect("Failed to create products table");

	// Insert test data using SeaQuery
	let insert_users = Query::insert()
		.into_table(Users::Table)
		.columns([
			Users::Email,
			Users::Username,
			Users::FirstName,
			Users::LastName,
		])
		.values_panic([
			"existing@example.com".into(),
			"existing_user".into(),
			"Existing".into(),
			"User".into(),
		])
		.values_panic([
			"alice@example.com".into(),
			"alice".into(),
			"Alice".into(),
			"Smith".into(),
		])
		.to_string(PostgresQueryBuilder);

	conn.execute_unprepared(&insert_users)
		.await
		.expect("Failed to insert test users");

	let insert_products = Query::insert()
		.into_table(Products::Table)
		.columns([Products::Sku, Products::Name, Products::Price])
		.values_panic(["PROD-123".into(), "Test Product".into(), 99.99.into()])
		.to_string(PostgresQueryBuilder);

	conn.execute_unprepared(&insert_products)
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
	let failures = result.unwrap();
	assert_eq!(failures.len(), 0);
}
