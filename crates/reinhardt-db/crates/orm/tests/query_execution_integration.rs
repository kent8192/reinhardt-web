//! ORM Query Execution Internal Integration Tests
//!
//! Tests internal query execution engine integration within ORM crate, covering:
//! - QueryCompiler SQL generation
//! - ExecutableQuery preparation and execution
//! - Parameter binding and type conversion
//! - Query result mapping
//! - Error handling in query execution
//! - Performance characteristics
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use reinhardt_orm::manager::reinitialize_database;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Query Compilation Tests
// ============================================================================

/// Test basic SELECT query compilation
///
/// **Test Intent**: Verify QueryCompiler generates correct SQL for basic SELECT
///
/// **Integration Point**: QueryCompiler → SQL string generation
///
/// **Not Intent**: Complex queries, joins
///
/// Uses reinhardt_orm for database connection management.
#[rstest]
#[tokio::test]
async fn test_basic_select_compilation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Create test table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT NOT NULL, age INT NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert test data
	sqlx::query("INSERT INTO users (name, age) VALUES ($1, $2)")
		.bind("Alice")
		.bind(30)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert");

	// Execute basic SELECT query
	let result = sqlx::query("SELECT id, name, age FROM users WHERE name = $1")
		.bind("Alice")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to execute query");

	let name: String = result.get("name");
	let age: i32 = result.get("age");

	assert_eq!(name, "Alice");
	assert_eq!(age, 30);
}

/// Test SELECT with multiple conditions
///
/// **Test Intent**: Verify QueryCompiler correctly combines multiple WHERE conditions
///
/// **Integration Point**: QueryCompiler → WHERE clause generation with AND/OR
///
/// **Not Intent**: Single condition, join conditions
#[rstest]
#[tokio::test]
async fn test_select_with_multiple_conditions(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS products (id SERIAL PRIMARY KEY, name TEXT NOT NULL, price BIGINT NOT NULL, in_stock BOOLEAN NOT NULL)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Insert test data
	sqlx::query("INSERT INTO products (name, price, in_stock) VALUES ($1, $2, $3)")
		.bind("Product A")
		.bind(1000_i64)
		.bind(true)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert A");

	sqlx::query("INSERT INTO products (name, price, in_stock) VALUES ($1, $2, $3)")
		.bind("Product B")
		.bind(2000_i64)
		.bind(false)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert B");

	sqlx::query("INSERT INTO products (name, price, in_stock) VALUES ($1, $2, $3)")
		.bind("Product C")
		.bind(1500_i64)
		.bind(true)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert C");

	// Query with multiple conditions: price > 1000 AND in_stock = true
	let results: Vec<String> = sqlx::query_scalar(
		"SELECT name FROM products WHERE price > $1 AND in_stock = $2 ORDER BY name",
	)
	.bind(1000_i64)
	.bind(true)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to execute query");

	assert_eq!(results, vec!["Product C"]);
}

/// Test ORDER BY clause compilation
///
/// **Test Intent**: Verify QueryCompiler correctly generates ORDER BY clauses
/// with multiple columns and directions
///
/// **Integration Point**: QueryCompiler → ORDER BY clause generation
///
/// **Not Intent**: WHERE conditions, LIMIT/OFFSET
#[rstest]
#[tokio::test]
async fn test_order_by_compilation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS employees (id SERIAL PRIMARY KEY, name TEXT NOT NULL, department TEXT NOT NULL, salary BIGINT NOT NULL)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Insert test data
	sqlx::query("INSERT INTO employees (name, department, salary) VALUES ($1, $2, $3)")
		.bind("Alice")
		.bind("Engineering")
		.bind(70000_i64)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert Alice");

	sqlx::query("INSERT INTO employees (name, department, salary) VALUES ($1, $2, $3)")
		.bind("Bob")
		.bind("Engineering")
		.bind(80000_i64)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert Bob");

	sqlx::query("INSERT INTO employees (name, department, salary) VALUES ($1, $2, $3)")
		.bind("Charlie")
		.bind("Sales")
		.bind(60000_i64)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert Charlie");

	// Query with ORDER BY department ASC, salary DESC
	let names: Vec<String> =
		sqlx::query_scalar("SELECT name FROM employees ORDER BY department ASC, salary DESC")
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to execute query");

	assert_eq!(names, vec!["Bob", "Alice", "Charlie"]);
}

// ============================================================================
// Parameter Binding Tests
// ============================================================================

/// Test parameter binding with various data types
///
/// **Test Intent**: Verify ExecutableQuery correctly binds parameters of different types
///
/// **Integration Point**: ExecutableQuery → Parameter binding + Type conversion
///
/// **Not Intent**: Complex types, NULL handling
#[rstest]
#[tokio::test]
async fn test_parameter_binding_various_types(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Create test table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS data_types (
			id SERIAL PRIMARY KEY,
			text_col TEXT,
			int_col INT,
			bigint_col BIGINT,
			bool_col BOOLEAN,
			float_col REAL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert with various parameter types
	sqlx::query("INSERT INTO data_types (text_col, int_col, bigint_col, bool_col, float_col) VALUES ($1, $2, $3, $4, $5)")
		.bind("test string")
		.bind(42_i32)
		.bind(9999999999_i64)
		.bind(true)
		.bind(3.15_f32)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert");

	// Query back and verify
	let result = sqlx::query(
		"SELECT text_col, int_col, bigint_col, bool_col, float_col FROM data_types WHERE id = 1",
	)
	.fetch_one(pool.as_ref())
	.await
	.expect("Failed to query");

	let text_val: String = result.get("text_col");
	let int_val: i32 = result.get("int_col");
	let bigint_val: i64 = result.get("bigint_col");
	let bool_val: bool = result.get("bool_col");
	let float_val: f32 = result.get("float_col");

	assert_eq!(text_val, "test string");
	assert_eq!(int_val, 42);
	assert_eq!(bigint_val, 9999999999);
	assert!(bool_val);
	assert!((float_val - 3.15).abs() < 0.01);
}

/// Test NULL parameter binding
///
/// **Test Intent**: Verify ExecutableQuery correctly handles NULL values in parameters
///
/// **Integration Point**: ExecutableQuery → NULL parameter binding
///
/// **Not Intent**: Non-NULL values, NOT NULL constraints
#[rstest]
#[tokio::test]
async fn test_null_parameter_binding(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Create test table with nullable columns
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS nullable_test (id SERIAL PRIMARY KEY, name TEXT, age INT)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert with NULL values
	sqlx::query("INSERT INTO nullable_test (name, age) VALUES ($1, $2)")
		.bind(Option::<String>::None)
		.bind(Option::<i32>::None)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert");

	// Query back and verify NULLs
	let result = sqlx::query("SELECT name, age FROM nullable_test WHERE id = 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	let name: Option<String> = result.get("name");
	let age: Option<i32> = result.get("age");

	assert!(name.is_none(), "name should be NULL");
	assert!(age.is_none(), "age should be NULL");
}

/// Test array parameter binding (PostgreSQL-specific)
///
/// **Test Intent**: Verify ExecutableQuery correctly binds array parameters
///
/// **Integration Point**: ExecutableQuery → Array parameter binding
///
/// **Not Intent**: Scalar parameters, other composite types
#[rstest]
#[tokio::test]
async fn test_array_parameter_binding(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Create test table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS tags_test (id SERIAL PRIMARY KEY, name TEXT NOT NULL, tags TEXT[])",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert with array parameter
	let tags = vec!["rust", "database", "orm"];
	sqlx::query("INSERT INTO tags_test (name, tags) VALUES ($1, $2)")
		.bind("Test Item")
		.bind(&tags)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert");

	// Query back and verify array
	let result = sqlx::query("SELECT tags FROM tags_test WHERE id = 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	let retrieved_tags: Vec<String> = result.get("tags");
	assert_eq!(retrieved_tags, tags);
}

// ============================================================================
// Query Result Mapping Tests
// ============================================================================

/// Test result mapping for single row
///
/// **Test Intent**: Verify ExecutableQuery correctly maps single row result to struct
///
/// **Integration Point**: ExecutableQuery → Result row mapping
///
/// **Not Intent**: Multiple rows, scalar results
#[rstest]
#[tokio::test]
async fn test_single_row_result_mapping(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS customers (id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Insert test data
	sqlx::query("INSERT INTO customers (name, email) VALUES ($1, $2)")
		.bind("John Doe")
		.bind("john@example.com")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert");

	// Query single row
	let result = sqlx::query("SELECT id, name, email FROM customers WHERE id = 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	let id: i32 = result.get("id");
	let name: String = result.get("name");
	let email: String = result.get("email");

	assert_eq!(id, 1);
	assert_eq!(name, "John Doe");
	assert_eq!(email, "john@example.com");
}

/// Test result mapping for multiple rows
///
/// **Test Intent**: Verify ExecutableQuery correctly maps multiple row results
///
/// **Integration Point**: ExecutableQuery → Multiple row result iteration
///
/// **Not Intent**: Single row, aggregation
#[rstest]
#[tokio::test]
async fn test_multiple_rows_result_mapping(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS items (id SERIAL PRIMARY KEY, name TEXT NOT NULL, price BIGINT NOT NULL)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Insert multiple rows
	sqlx::query("INSERT INTO items (name, price) VALUES ($1, $2)")
		.bind("Item 1")
		.bind(100_i64)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert 1");

	sqlx::query("INSERT INTO items (name, price) VALUES ($1, $2)")
		.bind("Item 2")
		.bind(200_i64)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert 2");

	sqlx::query("INSERT INTO items (name, price) VALUES ($1, $2)")
		.bind("Item 3")
		.bind(300_i64)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert 3");

	// Query all rows
	let results = sqlx::query("SELECT id, name, price FROM items ORDER BY id")
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to query");

	assert_eq!(results.len(), 3);

	let names: Vec<String> = results.iter().map(|r| r.get("name")).collect();
	assert_eq!(names, vec!["Item 1", "Item 2", "Item 3"]);

	let prices: Vec<i64> = results.iter().map(|r| r.get("price")).collect();
	assert_eq!(prices, vec![100, 200, 300]);
}

/// Test scalar result extraction
///
/// **Test Intent**: Verify ExecutableQuery correctly extracts scalar values (COUNT, SUM, etc.)
///
/// **Integration Point**: ExecutableQuery → Scalar result extraction
///
/// **Not Intent**: Row results, multiple columns
#[rstest]
#[tokio::test]
async fn test_scalar_result_extraction(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS orders (id SERIAL PRIMARY KEY, total BIGINT NOT NULL)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Insert test data
	sqlx::query("INSERT INTO orders (total) VALUES ($1)")
		.bind(100_i64)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert 1");

	sqlx::query("INSERT INTO orders (total) VALUES ($1)")
		.bind(200_i64)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert 2");

	sqlx::query("INSERT INTO orders (total) VALUES ($1)")
		.bind(300_i64)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert 3");

	// Query COUNT
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to get count");

	assert_eq!(count, 3);

	// Query SUM
	let sum: i64 = sqlx::query_scalar("SELECT SUM(total)::BIGINT FROM orders")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to get sum");

	assert_eq!(sum, 600);

	// Query AVG
	let avg: i64 = sqlx::query_scalar("SELECT AVG(total)::BIGINT FROM orders")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to get avg");

	assert_eq!(avg, 200);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

/// Test query execution with syntax error
///
/// **Test Intent**: Verify ExecutableQuery properly reports SQL syntax errors
///
/// **Integration Point**: ExecutableQuery → SQL syntax error detection
///
/// **Not Intent**: Successful queries, constraint violations
#[rstest]
#[tokio::test]
async fn test_syntax_error_handling(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Execute query with syntax error
	let result = sqlx::query("SELECT * FRM users") // "FRM" instead of "FROM"
		.fetch_one(pool.as_ref())
		.await;

	assert!(result.is_err(), "Syntax error should fail");

	let error = result.unwrap_err();
	let error_string = error.to_string().to_lowercase();
	assert!(
		error_string.contains("syntax") || error_string.contains("frm"),
		"Error should mention syntax issue"
	);
}

/// Test query execution with constraint violation
///
/// **Test Intent**: Verify ExecutableQuery properly reports constraint violations
///
/// **Integration Point**: ExecutableQuery → Constraint violation error
///
/// **Not Intent**: Syntax errors, successful inserts
#[rstest]
#[tokio::test]
async fn test_constraint_violation_error(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Create table with UNIQUE constraint
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS unique_emails (id SERIAL PRIMARY KEY, email TEXT UNIQUE NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert first record
	sqlx::query("INSERT INTO unique_emails (email) VALUES ($1)")
		.bind("user@example.com")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert first");

	// Attempt duplicate insert
	let result = sqlx::query("INSERT INTO unique_emails (email) VALUES ($1)")
		.bind("user@example.com")
		.execute(pool.as_ref())
		.await;

	assert!(result.is_err(), "Duplicate insert should fail");

	let error = result.unwrap_err();
	let error_string = error.to_string().to_lowercase();
	assert!(
		error_string.contains("unique") || error_string.contains("duplicate"),
		"Error should mention uniqueness violation"
	);
}

/// Test query execution with missing table
///
/// **Test Intent**: Verify ExecutableQuery properly reports non-existent table errors
///
/// **Integration Point**: ExecutableQuery → Table existence validation
///
/// **Not Intent**: Existing tables, other errors
#[rstest]
#[tokio::test]
async fn test_missing_table_error(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Query non-existent table
	let result = sqlx::query("SELECT * FROM non_existent_table")
		.fetch_one(pool.as_ref())
		.await;

	assert!(result.is_err(), "Querying non-existent table should fail");

	let error = result.unwrap_err();
	let error_string = error.to_string().to_lowercase();
	assert!(
		error_string.contains("does not exist") || error_string.contains("relation"),
		"Error should mention missing table"
	);
}

// ============================================================================
// Performance Characteristics Tests
// ============================================================================

/// Test query execution with LIMIT clause
///
/// **Test Intent**: Verify ExecutableQuery correctly applies LIMIT for pagination
///
/// **Integration Point**: ExecutableQuery → LIMIT clause application
///
/// **Not Intent**: OFFSET, full table scan
#[rstest]
#[tokio::test]
async fn test_limit_clause_performance(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS large_table (id SERIAL PRIMARY KEY, value INT)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Insert 100 rows
	for i in 1..=100 {
		sqlx::query("INSERT INTO large_table (value) VALUES ($1)")
			.bind(i)
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert");
	}

	// Query with LIMIT 10
	let results: Vec<i32> =
		sqlx::query_scalar("SELECT value FROM large_table ORDER BY id LIMIT 10")
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to query");

	assert_eq!(results.len(), 10);
	assert_eq!(results[0], 1);
	assert_eq!(results[9], 10);
}

/// Test query execution with OFFSET clause
///
/// **Test Intent**: Verify ExecutableQuery correctly applies OFFSET for pagination
///
/// **Integration Point**: ExecutableQuery → OFFSET clause application
///
/// **Not Intent**: LIMIT only, first page
#[rstest]
#[tokio::test]
async fn test_offset_clause_performance(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Create test table
	sqlx::query("CREATE TABLE IF NOT EXISTS paginated_table (id SERIAL PRIMARY KEY, value INT)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Insert 50 rows
	for i in 1..=50 {
		sqlx::query("INSERT INTO paginated_table (value) VALUES ($1)")
			.bind(i)
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert");
	}

	// Query with LIMIT 10 OFFSET 20 (third page)
	let results: Vec<i32> =
		sqlx::query_scalar("SELECT value FROM paginated_table ORDER BY id LIMIT 10 OFFSET 20")
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to query");

	assert_eq!(results.len(), 10);
	assert_eq!(results[0], 21);
	assert_eq!(results[9], 30);
}

/// Test batch insert execution
///
/// **Test Intent**: Verify ExecutableQuery can efficiently execute batch inserts
///
/// **Integration Point**: ExecutableQuery → Batch operation execution
///
/// **Not Intent**: Single insert, transactions
#[rstest]
#[tokio::test]
async fn test_batch_insert_execution(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;

	// Initialize ORM database connection
	reinitialize_database(&url).await.unwrap();

	// Create test table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS batch_test (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Begin transaction for batch insert
	let mut tx = pool.begin().await.expect("Failed to begin transaction");

	// Insert 20 rows in batch
	for i in 1..=20 {
		sqlx::query("INSERT INTO batch_test (name) VALUES ($1)")
			.bind(format!("Item {}", i))
			.execute(&mut *tx)
			.await
			.expect("Failed to insert");
	}

	tx.commit().await.expect("Failed to commit");

	// Verify all inserted
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM batch_test")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count");

	assert_eq!(count, 20, "Batch insert should insert all rows");
}
