//! Aggregation Functions Integration Tests
//!
//! Tests comprehensive aggregation functionality covering:
//! - COUNT aggregation (with and without *)
//! - SUM aggregation with various numeric types
//! - AVG aggregation (average value)
//! - MIN aggregation (minimum value)
//! - MAX aggregation (maximum value)
//! - GROUP BY clause with aggregations (※ NOT YET SUPPORTED IN ORM)
//! - HAVING clause filtering on aggregates (※ NOT YET SUPPORTED IN ORM)
//! - NULL handling in aggregations
//! - Empty result sets
//! - Multiple aggregations in single query
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Test Data Schema:**
//! - products(id SERIAL PRIMARY KEY, name TEXT NOT NULL, category TEXT NOT NULL, price BIGINT, stock INT)
//! - sales(id SERIAL PRIMARY KEY, product_id INT NOT NULL, amount BIGINT NOT NULL, quantity INT)
//!
//! **LIMITATION**: GROUP BY and HAVING clauses are not yet supported in reinhardt-ORM.
//! Tests requiring these features still use sqlx directly. See memory:aggregations_group_by_investigation

use reinhardt_core::macros::model;
use reinhardt_db::orm::Model;
use reinhardt_db::orm::manager::reinitialize_database;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sea_query::{ColumnDef, Iden, PostgresQueryBuilder, Table};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Model Definitions
// ============================================================================

/// Product model for aggregation testing
#[allow(dead_code)]
#[model(app_label = "orm_test", table_name = "products")]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Product {
	#[field(primary_key = true)]
	id: Option<i32>,
	#[field(max_length = 200)]
	name: String,
	#[field(max_length = 100)]
	category: String,
	#[field(null = true)]
	price: Option<i64>,
	#[field(null = true)]
	stock: Option<i32>,
}

/// Sales model for aggregation testing
#[allow(dead_code)]
#[model(app_label = "orm_test", table_name = "sales")]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Sale {
	#[field(primary_key = true)]
	id: Option<i32>,
	product_id: i32,
	amount: i64,
	#[field(null = true)]
	quantity: Option<i32>,
}

// ============================================================================
// Table Identifiers (for SeaQuery operations)
// ============================================================================

#[derive(Iden)]
enum Products {
	Table,
	Id,
	Name,
	Category,
	Price,
	Stock,
}

#[derive(Iden)]
enum Sales {
	Table,
	Id,
	ProductId,
	Amount,
	Quantity,
}

// ============================================================================
// Fixtures
// ============================================================================

/// Fixture that initializes ORM database connection and sets up aggregations test schema
///
/// This fixture receives postgres_container and calls reinitialize_database
/// to ensure each test has an isolated database connection, then creates
/// the products and sales tables.
#[fixture]
async fn aggregations_test_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String) {
	let (container, pool, port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_test_schema(&pool).await;
	(container, pool, port, url)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create test table and insert test data using ORM API
async fn setup_test_schema(pool: &PgPool) {
	// Drop existing tables to ensure clean state
	let drop_sales = Table::drop()
		.table(Sales::Table)
		.if_exists()
		.cascade()
		.build(PostgresQueryBuilder);
	let drop_products = Table::drop()
		.table(Products::Table)
		.if_exists()
		.cascade()
		.build(PostgresQueryBuilder);

	sqlx::query(&drop_sales).execute(pool).await.unwrap();
	sqlx::query(&drop_products).execute(pool).await.unwrap();

	// Create products table using SeaQuery
	let create_products = Table::create()
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
		.col(ColumnDef::new(Products::Price).big_integer())
		.col(ColumnDef::new(Products::Stock).integer())
		.build(PostgresQueryBuilder);

	sqlx::query(&create_products)
		.execute(pool)
		.await
		.expect("Failed to create products table");

	// Create sales table using SeaQuery
	let create_sales = Table::create()
		.table(Sales::Table)
		.col(
			ColumnDef::new(Sales::Id)
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(Sales::ProductId).integer().not_null())
		.col(ColumnDef::new(Sales::Amount).big_integer().not_null())
		.col(ColumnDef::new(Sales::Quantity).integer())
		.build(PostgresQueryBuilder);

	sqlx::query(&create_sales)
		.execute(pool)
		.await
		.expect("Failed to create sales table");

	// Insert products using ORM API
	// Category A: 3 products (prices: 100, 200, NULL)
	// Category B: 2 products (prices: 150, 250)
	let products = vec![
		Product::new(
			"Product A1".to_string(),
			"Category A".to_string(),
			Some(100),
			Some(50),
		),
		Product::new(
			"Product A2".to_string(),
			"Category A".to_string(),
			Some(200),
			Some(30),
		),
		Product::new(
			"Product A3".to_string(),
			"Category A".to_string(),
			None,
			Some(20),
		),
		Product::new(
			"Product B1".to_string(),
			"Category B".to_string(),
			Some(150),
			Some(40),
		),
		Product::new(
			"Product B2".to_string(),
			"Category B".to_string(),
			Some(250),
			Some(10),
		),
	];

	for product in products {
		Product::objects()
			.create(&product)
			.await
			.expect("Failed to insert product");
	}

	// Insert sales data using ORM API
	// Product 1: 3 sales (100, 200, 150)
	// Product 2: 2 sales (300, 400)
	// Product 3: 1 sale (50)
	let sales = vec![
		Sale::new(1, 100, Some(1)),
		Sale::new(1, 200, Some(2)),
		Sale::new(1, 150, None),
		Sale::new(2, 300, Some(3)),
		Sale::new(2, 400, None),
		Sale::new(3, 50, Some(1)),
	];

	for sale in sales {
		Sale::objects()
			.create(&sale)
			.await
			.expect("Failed to insert sale");
	}
}

// ============================================================================
// Basic Aggregation Tests (Normal cases)
// ============================================================================

/// Test COUNT(*) aggregation
///
/// **Test Intent**: Verify COUNT(*) correctly counts all rows including NULLs
///
/// **Integration Point**: Query → COUNT(*) aggregation
///
/// **Test Category**: Basic aggregations - Normal case
///
/// **Not Intent**: COUNT with column, NULL handling specifically
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_count_all_rows(
	#[future] aggregations_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = aggregations_test_db.await;

	// COUNT(*) using reinhardt-ORM API
	let count = Sale::objects().count().await.expect("Failed to count");

	assert_eq!(count, 6, "COUNT(*) should return 6 sales");
}

/// Test SUM aggregation with BIGINT
///
/// **Test Intent**: Verify SUM correctly adds all values in a numeric column
///
/// **Integration Point**: Query → SUM aggregation
///
/// **Test Category**: Basic aggregations - Normal case
///
/// **Not Intent**: NULL handling, multiple groups
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_sum_numeric_aggregation(
	#[future] aggregations_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = aggregations_test_db.await;

	// SUM of all sales amounts: 100 + 200 + 150 + 300 + 400 + 50 = 1200
	// NOTE: Currently using sqlx directly as aggregate() doesn't return scalar values yet
	let sum: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(amount), 0)::BIGINT FROM sales")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to sum");

	assert_eq!(sum, 1200, "SUM should return 1200");
}

/// Test AVG aggregation
///
/// **Test Intent**: Verify AVG correctly calculates average value
///
/// **Integration Point**: Query → AVG aggregation
///
/// **Test Category**: Basic aggregations - Normal case
///
/// **Not Intent**: NULL handling, precision issues
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_avg_aggregation(
	#[future] aggregations_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = aggregations_test_db.await;

	// AVG of all sales amounts: 1200 / 6 = 200
	// NOTE: Currently using sqlx directly as aggregate() doesn't return scalar values yet
	let avg: i64 = sqlx::query_scalar("SELECT AVG(amount)::BIGINT FROM sales")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to average");

	assert_eq!(avg, 200, "AVG should return 200");
}

/// Test MIN aggregation
///
/// **Test Intent**: Verify MIN correctly finds minimum value
///
/// **Integration Point**: Query → MIN aggregation
///
/// **Test Category**: Basic aggregations - Normal case
///
/// **Not Intent**: NULL handling, NULL as minimum
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_min_aggregation(
	#[future] aggregations_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = aggregations_test_db.await;

	// MIN of all sales amounts: 50
	// NOTE: Currently using sqlx directly as aggregate() doesn't return scalar values yet
	let min: i64 = sqlx::query_scalar("SELECT MIN(amount) FROM sales")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to find minimum");

	assert_eq!(min, 50, "MIN should return 50");
}

/// Test MAX aggregation
///
/// **Test Intent**: Verify MAX correctly finds maximum value
///
/// **Integration Point**: Query → MAX aggregation
///
/// **Test Category**: Basic aggregations - Normal case
///
/// **Not Intent**: NULL handling, NULL as maximum
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_max_aggregation(
	#[future] aggregations_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = aggregations_test_db.await;

	// MAX of all sales amounts: 400
	// NOTE: Currently using sqlx directly as aggregate() doesn't return scalar values yet
	let max: i64 = sqlx::query_scalar("SELECT MAX(amount) FROM sales")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to find maximum");

	assert_eq!(max, 400, "MAX should return 400");
}

// ============================================================================
// GROUP BY Tests
// ============================================================================

/// Test GROUP BY with COUNT aggregation
///
/// **Test Intent**: Verify GROUP BY correctly partitions data for COUNT
///
/// **Integration Point**: Query → GROUP BY + COUNT aggregation
///
/// **Test Category**: GROUP BY clause - Normal case
///
/// **Not Intent**: HAVING clause, other aggregations
///
/// **NOTE**: GROUP BY is not yet supported in reinhardt-ORM QuerySet API.
/// This test continues to use sqlx directly until GROUP BY support is implemented.
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_group_by_with_count(
	#[future] aggregations_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = aggregations_test_db.await;

	// GROUP BY product_id, COUNT sales per product
	// Product 1: 3 sales
	// Product 2: 2 sales
	// Product 3: 1 sale
	let rows = sqlx::query(
		"SELECT product_id, COUNT(*) as sale_count FROM sales GROUP BY product_id ORDER BY product_id",
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to group and count");

	assert_eq!(rows.len(), 3, "Should have 3 product groups");
	assert_eq!(rows[0].get::<i32, _>("product_id"), 1);
	assert_eq!(rows[0].get::<i64, _>("sale_count"), 3);
	assert_eq!(rows[1].get::<i32, _>("product_id"), 2);
	assert_eq!(rows[1].get::<i64, _>("sale_count"), 2);
	assert_eq!(rows[2].get::<i32, _>("product_id"), 3);
	assert_eq!(rows[2].get::<i64, _>("sale_count"), 1);
}

/// Test GROUP BY with multiple aggregations (SUM and AVG)
///
/// **Test Intent**: Verify GROUP BY works with multiple aggregations simultaneously
///
/// **Integration Point**: Query → GROUP BY + multiple aggregations
///
/// **Test Category**: Multiple aggregations - Equivalence partitioning
///
/// **Not Intent**: HAVING clause, single aggregation
///
/// **NOTE**: GROUP BY is not yet supported in reinhardt-ORM QuerySet API.
/// This test continues to use sqlx directly until GROUP BY support is implemented.
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_group_by_with_multiple_aggregations(
	#[future] aggregations_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = aggregations_test_db.await;

	// GROUP BY product_id, get SUM and AVG of amounts
	let rows = sqlx::query(
		r#"
		SELECT
			product_id,
			SUM(amount)::BIGINT as total_amount,
			AVG(amount)::BIGINT as avg_amount
		FROM sales
		GROUP BY product_id
		ORDER BY product_id
		"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to group with multiple aggregations");

	assert_eq!(rows.len(), 3);

	// Product 1: SUM=450, AVG=150 (100+200+150)/3
	assert_eq!(rows[0].get::<i64, _>("total_amount"), 450);
	assert_eq!(rows[0].get::<i64, _>("avg_amount"), 150);

	// Product 2: SUM=700, AVG=350 (300+400)/2
	assert_eq!(rows[1].get::<i64, _>("total_amount"), 700);
	assert_eq!(rows[1].get::<i64, _>("avg_amount"), 350);

	// Product 3: SUM=50, AVG=50 (50)/1
	assert_eq!(rows[2].get::<i64, _>("total_amount"), 50);
	assert_eq!(rows[2].get::<i64, _>("avg_amount"), 50);
}

// ============================================================================
// HAVING Clause Tests
// ============================================================================

/// Test HAVING clause filtering aggregates
///
/// **Test Intent**: Verify HAVING correctly filters grouped results by aggregate value
///
/// **Integration Point**: Query → GROUP BY + HAVING with aggregate conditions
///
/// **Test Category**: HAVING clause - Normal case
///
/// **Not Intent**: WHERE clause, non-aggregate conditions
///
/// **NOTE**: HAVING clause is not yet supported in reinhardt-ORM QuerySet API.
/// This test continues to use sqlx directly until HAVING support is implemented.
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_having_clause_filter(
	#[future] aggregations_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = aggregations_test_db.await;

	// GROUP BY product_id, filter for products with total_amount > 100
	// Should return: Product 1 (450), Product 2 (700), Product 3 (50 - filtered out)
	let rows = sqlx::query(
		r#"
		SELECT product_id, SUM(amount)::BIGINT as total_amount
		FROM sales
		GROUP BY product_id
		HAVING SUM(amount) > 100
		ORDER BY product_id
		"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to apply HAVING filter");

	assert_eq!(rows.len(), 2);
	assert_eq!(rows[0].get::<i32, _>("product_id"), 1);
	assert_eq!(rows[0].get::<i64, _>("total_amount"), 450);
	assert_eq!(rows[1].get::<i32, _>("product_id"), 2);
	assert_eq!(rows[1].get::<i64, _>("total_amount"), 700);
}

// ============================================================================
// NULL Handling Tests (Edge cases)
// ============================================================================

/// Test COUNT(*) vs COUNT(column) with NULL values
///
/// **Test Intent**: Verify COUNT(*) counts NULLs but COUNT(column) does not
///
/// **Integration Point**: Query → COUNT aggregation with NULL handling
///
/// **Test Category**: NULL handling - Edge case
///
/// **Not Intent**: Other aggregations, non-NULL filtering
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_count_null_handling(
	#[future] aggregations_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = aggregations_test_db.await;

	// COUNT(*) should return 6 (including NULL quantity rows)
	// COUNT(quantity) should return 4 (excluding NULL quantity rows)
	let count_all: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sales")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count all");

	let count_col: i64 = sqlx::query_scalar("SELECT COUNT(quantity) FROM sales")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count column");

	assert_eq!(count_all, 6, "COUNT(*) should count all 6 rows");
	assert_eq!(
		count_col, 4,
		"COUNT(quantity) should count only 4 non-NULL values"
	);
}

/// Test aggregations on empty result set
///
/// **Test Intent**: Verify aggregations handle empty groups correctly
///
/// **Integration Point**: Query → Aggregations with empty result
///
/// **Test Category**: Empty groups - Edge case
///
/// **Not Intent**: Non-empty results, NULL handling
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_aggregations_on_empty_set(
	#[future] aggregations_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = aggregations_test_db.await;

	// Query with condition that matches no rows
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sales WHERE product_id = 999")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count empty");

	assert_eq!(count, 0, "COUNT should return 0 for empty set");

	// SUM, AVG, MIN, MAX should return NULL for empty set
	let sum_result: Option<i64> =
		sqlx::query_scalar("SELECT SUM(amount)::BIGINT FROM sales WHERE product_id = 999")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to sum empty");

	assert!(sum_result.is_none(), "SUM should return NULL for empty set");
}

/// Test aggregations with all NULL values
///
/// **Test Intent**: Verify aggregations handle all-NULL columns correctly
///
/// **Integration Point**: Query → Aggregations with all NULL values
///
/// **Test Category**: All NULL values - Edge case
///
/// **Not Intent**: Mixed NULL/non-NULL, empty sets
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_aggregations_all_null_values(
	#[future] aggregations_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = aggregations_test_db.await;

	// Product A has NULL price, group by category A which only has Product A with NULL price
	// Actually, Product A3 has NULL price, but we need to test where all values in aggregation are NULL
	// Select only sales where quantity is NULL
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sales WHERE quantity IS NULL")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count NULLs");

	assert_eq!(count, 2, "Should have 2 sales with NULL quantity");

	// SUM of all NULL quantities should be NULL
	let sum_result: Option<i64> =
		sqlx::query_scalar("SELECT SUM(quantity)::BIGINT FROM sales WHERE quantity IS NULL")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to sum NULLs");

	assert!(sum_result.is_none(), "SUM of all NULLs should be NULL");
}
