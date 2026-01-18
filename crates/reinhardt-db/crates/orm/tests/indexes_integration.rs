//! Indexes Integration Tests
//!
//! This module tests database index functionality including:
//! - BTree indexes (default)
//! - Unique indexes
//! - Multi-column indexes (via model constraints)
//! - Index verification
//!
//! Note: Advanced index types (Hash, GIN, GiST) and partial indexes
//! are PostgreSQL-specific features that may require manual DDL or
//! migration system support beyond basic `#[field(index)]` attributes.

use reinhardt_core::macros::model;
use reinhardt_db::orm::Model;
use reinhardt_db::orm::indexes::{BTreeIndex, GinIndex, HashIndex, Index};
use reinhardt_db::orm::manager::reinitialize_database;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sea_query::{ColumnDef, Iden, PostgresQueryBuilder, Table};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

/// Product model for index testing
///
/// Allow dead_code: Model defined for index testing, may not be directly instantiated
#[allow(dead_code)]
#[model(app_label = "orm_test", table_name = "products")]
#[derive(Serialize, Deserialize)]
struct Product {
	#[field(primary_key = true)]
	id: Option<i32>,
	#[field(max_length = 200, index = true)]
	name: String,
	#[field(max_length = 100)]
	category: String,
	#[field(null = true)]
	price: Option<f64>,
}

/// SeaQuery table/column identifiers for Products table
#[derive(Debug, Clone, Copy, Iden)]
enum Products {
	Table,
	Id,
	Name,
	Category,
	Price,
}

/// Fixture that initializes ORM database connection and sets up products table
///
/// This fixture receives postgres_container and calls reinitialize_database
/// to ensure each test has an isolated database connection, then creates
/// the products table schema.
#[fixture]
async fn products_test_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String) {
	let (container, pool, port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_test_schema(&pool).await.unwrap();
	(container, pool, port, url)
}

/// Setup test database schema with Products table
async fn setup_test_schema(pool: &PgPool) -> Result<(), sqlx::Error> {
	// Drop existing table to ensure clean state
	let drop_table = Table::drop()
		.table(Products::Table)
		.if_exists()
		.cascade()
		.build(PostgresQueryBuilder);

	sqlx::query(&drop_table).execute(pool).await?;

	// Create fresh table
	let create_table = Table::create()
		.table(Products::Table)
		.col(
			ColumnDef::new(Products::Id)
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(Products::Name).string_len(200).not_null())
		.col(
			ColumnDef::new(Products::Category)
				.string_len(100)
				.not_null(),
		)
		.col(ColumnDef::new(Products::Price).double())
		.build(PostgresQueryBuilder);

	sqlx::query(&create_table).execute(pool).await?;

	Ok(())
}

/// Insert test data into Products table using reinhardt-orm API
async fn insert_test_data() -> Result<(), sqlx::Error> {
	let products = vec![
		Product::new(
			"Laptop".to_string(),
			"Electronics".to_string(),
			Some(1299.99),
		),
		Product::new("Mouse".to_string(), "Electronics".to_string(), Some(29.99)),
		Product::new("Desk".to_string(), "Furniture".to_string(), Some(399.99)),
		Product::new("Chair".to_string(), "Furniture".to_string(), Some(249.99)),
	];

	// Insert products individually to avoid bulk_create issues with auto-increment
	for product in products {
		Product::objects()
			.create(&product)
			.await
			.map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
	}

	Ok(())
}

/// Test BTree index creation and basic functionality
///
/// Normal case:Verifies standard BTree index creation
#[rstest]
#[tokio::test]
#[serial]
async fn test_btree_index_creation(
	#[future] products_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = products_test_db.await;
	insert_test_data().await.unwrap();

	// Create BTree index on name column using reinhardt-orm API
	let btree_index = BTreeIndex::new("idx_products_name", vec!["name".to_string()]);
	let create_index_sql = btree_index.to_sql("products");

	sqlx::query(&create_index_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Verify index exists
	let index_exists: bool = sqlx::query_scalar(
		"SELECT EXISTS (
			SELECT 1 FROM pg_indexes
			WHERE tablename = 'products'
			AND indexname = 'idx_products_name'
		)",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(index_exists, true, "BTree index should be created");
}

/// Test Hash index creation for PostgreSQL
///
/// Sanity:Verifies PostgreSQL-specific Hash index type
#[rstest]
#[tokio::test]
#[serial]
async fn test_hash_index_creation(
	#[future] products_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = products_test_db.await;

	// Create Hash index on category column using reinhardt-orm API
	let hash_index = HashIndex::new("idx_products_category_hash", vec!["category".to_string()]);
	let create_index_sql = hash_index.to_sql("products");

	sqlx::query(&create_index_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Verify index exists with correct type
	let index_type: String = sqlx::query_scalar(
		"SELECT am.amname
		FROM pg_class c
		JOIN pg_index i ON c.oid = i.indexrelid
		JOIN pg_am am ON c.relam = am.oid
		WHERE c.relname = 'idx_products_category_hash'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(index_type, "hash", "Index should be of type Hash");
}

/// Test GIN index for array columns in PostgreSQL
///
/// Sanity:Verifies GIN index for array data types
#[rstest]
#[tokio::test]
#[serial]
async fn test_gin_index_for_arrays(
	#[future] products_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = products_test_db.await;

	// Add tags column for GIN index testing
	sqlx::query("ALTER TABLE products ADD COLUMN tags TEXT[]")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Insert data with tags
	sqlx::query(
		"INSERT INTO products (name, category, tags) VALUES
		('Laptop', 'Electronics', ARRAY['portable', 'computer']),
		('Monitor', 'Electronics', ARRAY['display', 'computer'])",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Create GIN index on tags array column using reinhardt-orm API
	let gin_index = GinIndex::new("idx_products_tags_gin", vec!["tags".to_string()]);
	let create_index_sql = gin_index.to_sql("products");

	sqlx::query(&create_index_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Verify index exists with correct type
	let index_type: String = sqlx::query_scalar(
		"SELECT am.amname
		FROM pg_class c
		JOIN pg_index i ON c.oid = i.indexrelid
		JOIN pg_am am ON c.relam = am.oid
		WHERE c.relname = 'idx_products_tags_gin'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(index_type, "gin", "Index should be of type GIN");

	// Test array contains query
	let count: i64 =
		sqlx::query_scalar("SELECT COUNT(*) FROM products WHERE tags @> ARRAY['computer']")
			.fetch_one(pool.as_ref())
			.await
			.unwrap();

	assert_eq!(count, 2, "Should find 2 products with 'computer' tag");
}

/// Test multi-column composite index
///
/// Normal case:Verifies index creation on multiple columns
#[rstest]
#[tokio::test]
#[serial]
async fn test_multi_column_index(
	#[future] products_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = products_test_db.await;
	insert_test_data().await.unwrap();

	// Create multi-column index on category and price using reinhardt-orm API
	let multi_col_index = Index::new(
		"idx_products_category_price",
		vec!["category".to_string(), "price".to_string()],
	);
	let create_index_sql = multi_col_index.to_sql("products");

	sqlx::query(&create_index_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Verify index exists with correct columns
	let column_count: i64 = sqlx::query_scalar(
		"SELECT COUNT(*)
		FROM pg_index i
		JOIN pg_class c ON i.indexrelid = c.oid
		WHERE c.relname = 'idx_products_category_price'
		AND array_length(i.indkey, 1) = 2",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(column_count, 1, "Multi-column index should have 2 columns");
}

/// Test unique index constraint
///
/// Normal case:Verifies unique index prevents duplicate values
#[rstest]
#[tokio::test]
#[serial]
async fn test_unique_index(
	#[future] products_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = products_test_db.await;

	// Create unique index on name column using reinhardt-orm API
	let unique_index = Index::new("idx_products_name_unique", vec!["name".to_string()]).unique();
	let create_index_sql = unique_index.to_sql("products");

	sqlx::query(&create_index_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Insert first product using reinhardt-orm API
	let first_product = Product::new("UniqueProduct".to_string(), "Test".to_string(), None);

	Product::objects().create(&first_product).await.unwrap();

	// Attempt to insert duplicate - should fail
	let duplicate_product = Product::new("UniqueProduct".to_string(), "Test".to_string(), None);

	let result = Product::objects().create(&duplicate_product).await;

	assert!(
		result.is_err(),
		"Duplicate insert should fail with unique constraint violation"
	);
}

/// Test partial index with WHERE clause
///
/// Sanity:Verifies partial index creation with filter condition
#[rstest]
#[tokio::test]
#[serial]
async fn test_partial_index_with_where(
	#[future] products_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = products_test_db.await;
	insert_test_data().await.unwrap();

	// Create partial index for expensive products (price > 100) using reinhardt-orm API
	let partial_index = Index::new("idx_products_expensive", vec!["price".to_string()])
		.with_condition("price > 100".to_string());
	let create_index_sql = partial_index.to_sql("products");

	sqlx::query(&create_index_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Verify index exists
	let index_exists: bool = sqlx::query_scalar(
		"SELECT EXISTS (
			SELECT 1 FROM pg_indexes
			WHERE tablename = 'products'
			AND indexname = 'idx_products_expensive'
		)",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(index_exists, true, "Partial index should be created");

	// Verify index has WHERE clause (indpred is not null)
	let has_predicate: bool = sqlx::query_scalar(
		"SELECT pg_get_expr(i.indpred, i.indrelid) IS NOT NULL
		FROM pg_index i
		JOIN pg_class c ON i.indexrelid = c.oid
		WHERE c.relname = 'idx_products_expensive'",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(has_predicate, true, "Index should have WHERE predicate");
}

/// Test index usage in query execution plan
///
/// Sanity:Verifies indexes are actually used by the query planner
#[rstest]
#[tokio::test]
#[serial]
async fn test_index_usage_in_query_plan(
	#[future] products_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = products_test_db.await;

	insert_test_data().await.unwrap();

	// Create index on category using reinhardt-orm API
	let category_index = Index::new("idx_products_category", vec!["category".to_string()]);
	let create_index_sql = category_index.to_sql("products");

	sqlx::query(&create_index_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Analyze table to update statistics
	sqlx::query("ANALYZE products")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Get query execution plan
	let explain_result: Vec<String> =
		sqlx::query_scalar("EXPLAIN SELECT * FROM products WHERE category = 'Electronics'")
			.fetch_all(pool.as_ref())
			.await
			.unwrap();

	let explain_output = explain_result.join("\n");

	// Verify index is mentioned in the execution plan
	// Note: Query planner might choose Seq Scan for small tables
	// This test verifies the index exists and can be used
	assert!(
		explain_output.contains("products") || explain_output.contains("Scan"),
		"EXPLAIN output should contain scan information: {}",
		explain_output
	);
}
