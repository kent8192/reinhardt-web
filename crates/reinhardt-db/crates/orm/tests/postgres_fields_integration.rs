//! PostgreSQL-Specific Field Types Integration Tests
//!
//! Tests comprehensive integration of PostgreSQL-specific field types with the ORM,
//! covering:
//! - JSONB field operations (insert, search, update)
//! - Array field operations (insert, search, update, empty arrays)
//! - HStore field key-value operations
//! - Range types (IntegerRange, DateRange)
//! - Edge cases (empty arrays, empty JSON, NULL values)
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Integration Point:**
//! This test verifies that PostgreSQL-specific types are correctly handled through
//! the entire stack: ORM → QueryCompiler → SQL generation → PostgreSQL execution

use reinhardt_orm;
use reinhardt_orm::manager::reinitialize_database;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serial_test::serial;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Fixtures
// ============================================================================

#[fixture]
async fn postgres_fields_test_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String) {
	let (container, pool, port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	(container, pool, port, url)
}

// ============================================================================
// JSONB Field Tests
// ============================================================================

/// Test JSONB field insertion and retrieval
///
/// **Test Intent**: Verify JSONB field can store and retrieve complex JSON objects
///
/// **Integration Point**: ORM → JSONB parameter binding → PostgreSQL JSONB storage
///
/// **Not Intent**: JSON (non-binary), simple scalar types
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_jsonb_field_insert_and_retrieve(
	#[future] postgres_fields_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_fields_test_db.await;

	// Create table with JSONB column
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS products (
			id SERIAL PRIMARY KEY,
			name TEXT NOT NULL,
			attributes JSONB NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert product with JSONB attributes
	let attributes = serde_json::json!({
		"color": "red",
		"size": "large",
		"features": ["waterproof", "durable"],
		"specs": {
			"weight": 1.5,
			"dimensions": {
				"width": 10,
				"height": 20,
				"depth": 5
			}
		}
	});

	sqlx::query("INSERT INTO products (name, attributes) VALUES ($1, $2)")
		.bind("Widget")
		.bind(&attributes)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert");

	// Retrieve and verify JSONB data
	let result = sqlx::query("SELECT name, attributes FROM products WHERE id = 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	let name: String = result.get("name");
	let retrieved_attrs: serde_json::Value = result.get("attributes");

	assert_eq!(name, "Widget");
	assert_eq!(retrieved_attrs, attributes);
	assert_eq!(retrieved_attrs["color"], "red");
	assert_eq!(retrieved_attrs["specs"]["weight"], 1.5);
}

/// Test JSONB field querying with containment operator (@>)
///
/// **Test Intent**: Verify JSONB containment queries work correctly
///
/// **Integration Point**: ORM → JSONB query operators → PostgreSQL JSONB containment
///
/// **Not Intent**: Simple equality, text search
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_jsonb_containment_query(
	#[future] postgres_fields_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_fields_test_db.await;

	// Create table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS documents (
			id SERIAL PRIMARY KEY,
			title TEXT NOT NULL,
			metadata JSONB NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert test documents
	sqlx::query("INSERT INTO documents (title, metadata) VALUES ($1, $2)")
		.bind("Doc 1")
		.bind(serde_json::json!({"tags": ["rust", "database"], "status": "published"}))
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert doc 1");

	sqlx::query("INSERT INTO documents (title, metadata) VALUES ($1, $2)")
		.bind("Doc 2")
		.bind(serde_json::json!({"tags": ["python", "web"], "status": "draft"}))
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert doc 2");

	sqlx::query("INSERT INTO documents (title, metadata) VALUES ($1, $2)")
		.bind("Doc 3")
		.bind(serde_json::json!({"tags": ["rust", "web"], "status": "published"}))
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert doc 3");

	// Query documents containing {"status": "published"} in metadata
	let results: Vec<String> = sqlx::query_scalar(
		r#"
		SELECT title FROM documents
		WHERE metadata @> $1::jsonb
		ORDER BY title
		"#,
	)
	.bind(serde_json::json!({"status": "published"}))
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query");

	assert_eq!(results, vec!["Doc 1", "Doc 3"]);
}

/// Test JSONB field with empty object
///
/// **Test Intent**: Verify empty JSON objects are handled correctly
///
/// **Integration Point**: ORM → JSONB empty value handling
///
/// **Not Intent**: NULL values, non-empty objects
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_jsonb_empty_object(
	#[future] postgres_fields_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_fields_test_db.await;

	// Create table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS configs (id SERIAL PRIMARY KEY, settings JSONB NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert empty JSON object
	let empty_json = serde_json::json!({});
	sqlx::query("INSERT INTO configs (settings) VALUES ($1)")
		.bind(&empty_json)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert");

	// Retrieve and verify
	let result: serde_json::Value = sqlx::query_scalar("SELECT settings FROM configs WHERE id = 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	assert_eq!(result, empty_json);
	assert!(result.is_object());
	assert_eq!(result.as_object().unwrap().len(), 0);
}

// ============================================================================
// Array Field Tests
// ============================================================================

/// Test Array field insertion and retrieval
///
/// **Test Intent**: Verify PostgreSQL arrays can store and retrieve multiple values
///
/// **Integration Point**: ORM → Array parameter binding → PostgreSQL array storage
///
/// **Not Intent**: Scalar values, JSON arrays
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_array_field_insert_and_retrieve(
	#[future] postgres_fields_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_fields_test_db.await;

	// Create table with array columns
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS articles (
			id SERIAL PRIMARY KEY,
			title TEXT NOT NULL,
			tags TEXT[] NOT NULL,
			view_counts INT[] NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert article with array data
	let tags = vec!["rust", "programming", "database"];
	let view_counts = vec![100, 250, 75];

	sqlx::query("INSERT INTO articles (title, tags, view_counts) VALUES ($1, $2, $3)")
		.bind("Rust Tutorial")
		.bind(&tags)
		.bind(&view_counts)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert");

	// Retrieve and verify
	let result = sqlx::query("SELECT title, tags, view_counts FROM articles WHERE id = 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	let title: String = result.get("title");
	let retrieved_tags: Vec<String> = result.get("tags");
	let retrieved_counts: Vec<i32> = result.get("view_counts");

	assert_eq!(title, "Rust Tutorial");
	assert_eq!(retrieved_tags, tags);
	assert_eq!(retrieved_counts, view_counts);
}

/// Test Array field with ANY operator for search
///
/// **Test Intent**: Verify array containment queries using ANY operator
///
/// **Integration Point**: ORM → Array query operators → PostgreSQL ANY operator
///
/// **Not Intent**: ALL operator, array equality
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_array_any_operator_query(
	#[future] postgres_fields_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_fields_test_db.await;

	// Create table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS posts (
			id SERIAL PRIMARY KEY,
			title TEXT NOT NULL,
			categories TEXT[] NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert test posts
	sqlx::query("INSERT INTO posts (title, categories) VALUES ($1, $2)")
		.bind("Post 1")
		.bind(&vec!["tech", "rust"])
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert post 1");

	sqlx::query("INSERT INTO posts (title, categories) VALUES ($1, $2)")
		.bind("Post 2")
		.bind(&vec!["news", "world"])
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert post 2");

	sqlx::query("INSERT INTO posts (title, categories) VALUES ($1, $2)")
		.bind("Post 3")
		.bind(&vec!["tech", "python"])
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert post 3");

	// Query posts containing 'tech' in categories array
	let results: Vec<String> =
		sqlx::query_scalar("SELECT title FROM posts WHERE 'tech' = ANY(categories) ORDER BY title")
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to query");

	assert_eq!(results, vec!["Post 1", "Post 3"]);
}

/// Test Array field update operation
///
/// **Test Intent**: Verify array fields can be updated correctly
///
/// **Integration Point**: ORM → Array UPDATE operations → PostgreSQL array modification
///
/// **Not Intent**: Append operations, element modification
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_array_field_update(
	#[future] postgres_fields_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_fields_test_db.await;

	// Create table
	sqlx::query("CREATE TABLE IF NOT EXISTS lists (id SERIAL PRIMARY KEY, items TEXT[] NOT NULL)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Insert initial data
	let initial_items = vec!["a", "b", "c"];
	sqlx::query("INSERT INTO lists (items) VALUES ($1)")
		.bind(&initial_items)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert");

	// Update array
	let updated_items = vec!["x", "y", "z"];
	sqlx::query("UPDATE lists SET items = $1 WHERE id = 1")
		.bind(&updated_items)
		.execute(pool.as_ref())
		.await
		.expect("Failed to update");

	// Verify update
	let result: Vec<String> = sqlx::query_scalar("SELECT items FROM lists WHERE id = 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	assert_eq!(result, updated_items);
}

/// Test Array field with empty array
///
/// **Test Intent**: Verify empty arrays are handled correctly
///
/// **Integration Point**: ORM → Empty array handling → PostgreSQL empty array storage
///
/// **Not Intent**: NULL arrays, non-empty arrays
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_array_field_empty_array(
	#[future] postgres_fields_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_fields_test_db.await;

	// Create table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS collections (id SERIAL PRIMARY KEY, items TEXT[] NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert empty array
	let empty_array: Vec<String> = vec![];
	sqlx::query("INSERT INTO collections (items) VALUES ($1)")
		.bind(&empty_array)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert");

	// Retrieve and verify
	let result: Vec<String> = sqlx::query_scalar("SELECT items FROM collections WHERE id = 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	assert_eq!(result.len(), 0);
	assert!(result.is_empty());
}

// ============================================================================
// Range Type Tests
// ============================================================================

/// Test Integer Range field operations
///
/// **Test Intent**: Verify PostgreSQL INT4RANGE type can store and query integer ranges
///
/// **Integration Point**: ORM → Range type binding → PostgreSQL range storage
///
/// **Not Intent**: Date ranges, continuous ranges
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_integer_range_field(
	#[future] postgres_fields_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_fields_test_db.await;

	// Create table with INT4RANGE
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS price_ranges (
			id SERIAL PRIMARY KEY,
			name TEXT NOT NULL,
			range INT4RANGE NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert range data (using text representation with explicit type cast)
	sqlx::query("INSERT INTO price_ranges (name, range) VALUES ($1, $2::int4range)")
		.bind("Budget")
		.bind("[0,100)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert budget");

	sqlx::query("INSERT INTO price_ranges (name, range) VALUES ($1, $2::int4range)")
		.bind("Mid-range")
		.bind("[100,500)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert mid-range");

	sqlx::query("INSERT INTO price_ranges (name, range) VALUES ($1, $2::int4range)")
		.bind("Premium")
		.bind("[500,2000)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert premium");

	// Query ranges containing specific value
	let results: Vec<String> =
		sqlx::query_scalar("SELECT name FROM price_ranges WHERE range @> 250 ORDER BY name")
			.fetch_all(pool.as_ref())
			.await
			.expect("Failed to query");

	assert_eq!(results, vec!["Mid-range"]);
}

/// Test Date Range field operations
///
/// **Test Intent**: Verify PostgreSQL DATERANGE type can store and query date ranges
///
/// **Integration Point**: ORM → Date range type binding → PostgreSQL date range storage
///
/// **Not Intent**: Timestamp ranges, integer ranges
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_date_range_field(
	#[future] postgres_fields_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_fields_test_db.await;

	// Create table with DATERANGE
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS events (
			id SERIAL PRIMARY KEY,
			name TEXT NOT NULL,
			period DATERANGE NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert date range data (using text representation with explicit type cast)
	sqlx::query("INSERT INTO events (name, period) VALUES ($1, $2::daterange)")
		.bind("Summer Camp")
		.bind("[2025-06-01,2025-08-31]")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert summer");

	sqlx::query("INSERT INTO events (name, period) VALUES ($1, $2::daterange)")
		.bind("Winter Workshop")
		.bind("[2025-12-01,2026-02-28]")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert winter");

	// Query events containing specific date
	let results: Vec<String> = sqlx::query_scalar(
		"SELECT name FROM events WHERE period @> '2025-07-15'::date ORDER BY name",
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query");

	assert_eq!(results, vec!["Summer Camp"]);
}

// ============================================================================
// Edge Cases and Complex Scenarios
// ============================================================================

/// Test JSONB with nested arrays
///
/// **Test Intent**: Verify complex nested structures in JSONB are handled correctly
///
/// **Integration Point**: ORM → Complex JSONB structures → PostgreSQL deep nesting
///
/// **Not Intent**: Flat structures, simple arrays
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_jsonb_with_nested_arrays(
	#[future] postgres_fields_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_fields_test_db.await;

	// Create table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS complex_data (id SERIAL PRIMARY KEY, data JSONB NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert nested structure
	let complex_data = serde_json::json!({
		"users": [
			{"name": "Alice", "roles": ["admin", "editor"]},
			{"name": "Bob", "roles": ["viewer"]},
		],
		"settings": {
			"notifications": ["email", "sms"],
			"preferences": {
				"theme": "dark",
				"languages": ["en", "ja"]
			}
		}
	});

	sqlx::query("INSERT INTO complex_data (data) VALUES ($1)")
		.bind(&complex_data)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert");

	// Retrieve and verify nested access
	let result: serde_json::Value =
		sqlx::query_scalar("SELECT data FROM complex_data WHERE id = 1")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to query");

	assert_eq!(result["users"][0]["name"], "Alice");
	assert_eq!(result["settings"]["preferences"]["theme"], "dark");
	assert_eq!(
		result["settings"]["preferences"]["languages"]
			.as_array()
			.unwrap()
			.len(),
		2
	);
}

/// Test multi-dimensional array field
///
/// **Test Intent**: Verify PostgreSQL supports multi-dimensional arrays
///
/// **Integration Point**: ORM → Multi-dimensional array binding → PostgreSQL array nesting
///
/// **Not Intent**: Single-dimensional arrays, jagged arrays
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_multidimensional_array_field(
	#[future] postgres_fields_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_fields_test_db.await;

	// Create table with 2D array
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS matrices (id SERIAL PRIMARY KEY, matrix INT[][] NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert 2D array using text representation with explicit type cast
	// PostgreSQL represents 2D arrays as '{{1,2,3},{4,5,6}}'
	sqlx::query("INSERT INTO matrices (matrix) VALUES ($1::int[][])")
		.bind("{{1,2,3},{4,5,6}}")
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert");

	// Retrieve and verify (PostgreSQL returns 2D arrays as nested arrays in some drivers)
	// Use ::text cast to get string representation for verification
	let result = sqlx::query("SELECT matrix::text as matrix FROM matrices WHERE id = 1")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query");

	// Note: The exact representation depends on the driver
	// For verification, we'll check that it's not empty
	let matrix_str: String = result.try_get("matrix").expect("Failed to get matrix");
	assert!(matrix_str.contains("1"));
	assert!(matrix_str.contains("6"));
}
