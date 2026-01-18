//! Integration tests for chaining multiple filters
//!
//! These tests verify that multiple FilterBackend implementations can be
//! chained together to build complex queries.
//!
//! **Test Coverage:**
//! 1. SearchFilter + OrderingFilter chain
//! 2. SearchFilter + OrderingFilter + RangeFilter chain
//! 3. Multiple SearchFilter chains (different fields)
//! 4. Filter chain with conditional application
//! 5. Filter chain execution order verification
//! 6. Filter chain with conflicting conditions
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container (reinhardt-test)
//! - filter_chain_test_db: Custom fixture providing database connection with test schema

use reinhardt_rest::filters::{FilterBackend, RangeFilter, SimpleOrderingBackend, SimpleSearchBackend};
use reinhardt_test::fixtures::testcontainers::{ContainerAsync, GenericImage, postgres_container};
use rstest::*;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;

// ========================================================================
// Custom Fixtures
// ========================================================================

/// Custom fixture providing PostgreSQL database with test schema for filter chaining
///
/// **Schema:**
/// - products: id, name, description, price, stock, category, created_at
///
/// **Integration Point**: postgres_container → filter_chain_test_db (fixture chaining)
#[fixture]
async fn filter_chain_test_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>, u16, String),
) -> (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>) {
	let (container, pool, _port, _url) = postgres_container.await;

	// Create products table
	sqlx::query(
		r#"
		CREATE TABLE products (
			id SERIAL PRIMARY KEY,
			name TEXT NOT NULL,
			description TEXT NOT NULL,
			price INTEGER NOT NULL,
			stock INTEGER NOT NULL,
			category TEXT NOT NULL,
			created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create products table");

	// Insert test data
	sqlx::query(
		r#"
		INSERT INTO products (name, description, price, stock, category) VALUES
		('Laptop', 'High-performance laptop for developers', 150000, 10, 'electronics'),
		('Mouse', 'Wireless ergonomic mouse', 5000, 50, 'electronics'),
		('Keyboard', 'Mechanical keyboard with RGB', 12000, 30, 'electronics'),
		('Monitor', '4K Ultra HD monitor', 80000, 15, 'electronics'),
		('Desk', 'Standing desk for home office', 45000, 8, 'furniture'),
		('Chair', 'Ergonomic office chair', 35000, 12, 'furniture'),
		('Bookshelf', 'Wooden bookshelf', 25000, 20, 'furniture'),
		('Lamp', 'LED desk lamp', 8000, 40, 'lighting'),
		('Headphones', 'Noise-cancelling headphones', 28000, 25, 'electronics'),
		('Webcam', 'Full HD webcam', 15000, 18, 'electronics')
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert products");

	(container, pool)
}

// ========================================================================
// Test 1: SearchFilter + OrderingFilter Chain
// ========================================================================

/// Test chaining SearchFilter and OrderingFilter
///
/// **Test Intent**: Verify that SearchFilter and OrderingFilter can be chained
/// to perform search and ordering in a single query.
///
/// **Integration Point**: SimpleSearchBackend → SimpleOrderingBackend → Combined SQL
///
/// **Verification**:
/// - Search filter applied (WHERE clause)
/// - Ordering filter applied (ORDER BY clause)
/// - Results match both filter criteria
/// - Correct SQL structure (WHERE before ORDER BY)
#[rstest]
#[tokio::test]
async fn test_search_ordering_chain(
	#[future] filter_chain_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>),
) {
	let (_container, pool) = filter_chain_test_db.await;

	// Create filter chain: Search "electronics" in category + Order by price DESC
	let search_backend = SimpleSearchBackend::new("search").with_field("category");

	let ordering_backend = SimpleOrderingBackend::new("ordering")
		.allow_field("price")
		.allow_field("name");

	let mut params = HashMap::new();
	params.insert("search".to_string(), "electronics".to_string());
	params.insert("ordering".to_string(), "-price".to_string());

	// Apply filters sequentially
	let base_sql = "SELECT * FROM products".to_string();
	let filtered_sql = search_backend
		.filter_queryset(&params, base_sql)
		.await
		.expect("Search filter failed");
	let filtered_sql = ordering_backend
		.filter_queryset(&params, filtered_sql)
		.await
		.expect("Ordering filter failed");

	// Verify SQL structure
	assert!(filtered_sql.contains("WHERE"));
	assert!(filtered_sql.contains("category LIKE '%electronics%'"));
	assert!(filtered_sql.contains("ORDER BY price DESC"));

	// Execute query and verify results
	let rows = sqlx::query(&filtered_sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	// Should return electronics products ordered by price DESC
	// Expected order: Laptop(150000), Monitor(80000), Headphones(28000), Webcam(15000), Keyboard(12000), Mouse(5000)
	assert_eq!(rows.len(), 6);

	let prices: Vec<i32> = rows
		.iter()
		.map(|r| r.try_get::<i32, _>("price").expect("Failed to get price"))
		.collect();
	assert_eq!(prices, vec![150000, 80000, 28000, 15000, 12000, 5000]);
}

// ========================================================================
// Test 2: SearchFilter + OrderingFilter + RangeFilter Chain
// ========================================================================

/// Test chaining three filters (SearchFilter, OrderingFilter, RangeFilter)
///
/// **Test Intent**: Verify that multiple filters can be chained together
/// to create complex queries with search, range, and ordering.
///
/// **Integration Point**: Multiple FilterBackends → Combined WHERE + ORDER BY SQL
///
/// **Verification**:
/// - All filter conditions applied
/// - Correct clause ordering (WHERE conditions → ORDER BY)
/// - Results match all filter criteria
#[rstest]
#[tokio::test]
async fn test_three_filter_chain(
	#[future] filter_chain_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>),
) {
	let (_container, pool) = filter_chain_test_db.await;

	// Create filter chain:
	// 1. Search "electronics" in category
	// 2. Price range: 10000 <= price <= 100000
	// 3. Order by price ASC
	let search_backend = SimpleSearchBackend::new("search").with_field("category");

	let ordering_backend = SimpleOrderingBackend::new("ordering").allow_field("price");

	let mut params = HashMap::new();
	params.insert("search".to_string(), "electronics".to_string());
	params.insert("ordering".to_string(), "price".to_string());

	// Apply search filter
	let base_sql = "SELECT * FROM products".to_string();
	let filtered_sql = search_backend
		.filter_queryset(&params, base_sql)
		.await
		.expect("Search filter failed");

	// Apply range filter (manual SQL construction for RangeFilter)
	let range_filter: RangeFilter<i32> = RangeFilter::new("price").gte(10000).lte(100000);
	let filtered_sql = format!(
		"{} AND price >= {} AND price <= {}",
		filtered_sql,
		range_filter.gte.as_ref().unwrap(),
		range_filter.lte.as_ref().unwrap()
	);

	// Apply ordering filter
	let filtered_sql = ordering_backend
		.filter_queryset(&params, filtered_sql)
		.await
		.expect("Ordering filter failed");

	// Verify SQL structure
	assert!(filtered_sql.contains("WHERE"));
	assert!(filtered_sql.contains("category LIKE '%electronics%'"));
	assert!(filtered_sql.contains("price >= 10000"));
	assert!(filtered_sql.contains("price <= 100000"));
	assert!(filtered_sql.contains("ORDER BY price ASC"));

	// Execute query and verify results
	let rows = sqlx::query(&filtered_sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	// Should return electronics products with price 10000-100000, ordered by price ASC
	// Expected: Keyboard(12000), Webcam(15000), Headphones(28000), Monitor(80000)
	// Excluded: Mouse(5000), Laptop(150000)
	assert_eq!(rows.len(), 4);

	let prices: Vec<i32> = rows
		.iter()
		.map(|r| r.try_get::<i32, _>("price").expect("Failed to get price"))
		.collect();
	assert_eq!(prices, vec![12000, 15000, 28000, 80000]);
}

// ========================================================================
// Test 3: Multiple SearchFilter Chains (Different Fields)
// ========================================================================

/// Test chaining multiple SearchFilters on different fields
///
/// **Test Intent**: Verify that multiple SearchFilters can be applied
/// to different fields simultaneously.
///
/// **Integration Point**: Multiple SimpleSearchBackend instances → Combined WHERE clauses
///
/// **Verification**:
/// - Multiple search conditions applied
/// - Different fields searched
/// - Results match all search criteria
#[rstest]
#[tokio::test]
async fn test_multiple_search_filter_chain(
	#[future] filter_chain_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>),
) {
	let (_container, pool) = filter_chain_test_db.await;

	// Create filter chain: Search "electronics" in category AND "Laptop" in name
	// NOTE: SimpleSearchBackend uses LIKE which is case-sensitive in PostgreSQL
	let category_search = SimpleSearchBackend::new("category_search").with_field("category");

	let name_search = SimpleSearchBackend::new("name_search").with_field("name");

	let mut params = HashMap::new();
	params.insert("category_search".to_string(), "electronics".to_string());
	params.insert("name_search".to_string(), "Laptop".to_string()); // Case-sensitive match

	// Apply filters sequentially
	let base_sql = "SELECT * FROM products".to_string();
	let filtered_sql = category_search
		.filter_queryset(&params, base_sql)
		.await
		.expect("Category search failed");
	let filtered_sql = name_search
		.filter_queryset(&params, filtered_sql)
		.await
		.expect("Name search failed");

	// Verify SQL structure
	assert!(filtered_sql.contains("WHERE"));
	assert!(filtered_sql.contains("category LIKE '%electronics%'"));
	assert!(filtered_sql.contains("name LIKE '%Laptop%'")); // Case-sensitive

	// Execute query and verify results
	let rows = sqlx::query(&filtered_sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	// Should return products matching both category AND name
	// Expected: Laptop (category=electronics, name contains "Laptop")
	assert_eq!(rows.len(), 1);

	let name: String = rows[0].try_get("name").expect("Failed to get name");
	let category: String = rows[0].try_get("category").expect("Failed to get category");
	assert_eq!(name, "Laptop"); // Exact case match
	assert_eq!(category, "electronics");
}

// ========================================================================
// Test 4: Filter Chain with Conditional Application
// ========================================================================

/// Test filter chain where some filters are conditionally applied
///
/// **Test Intent**: Verify that filters can be conditionally applied
/// based on parameter presence.
///
/// **Integration Point**: FilterBackend → Conditional SQL modification
///
/// **Verification**:
/// - Filters only applied when parameters present
/// - SQL structure changes based on parameters
/// - Results reflect conditional filtering
#[rstest]
#[tokio::test]
async fn test_conditional_filter_chain(
	#[future] filter_chain_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>),
) {
	let (_container, pool) = filter_chain_test_db.await;

	// Create filter chain with optional parameters
	let search_backend = SimpleSearchBackend::new("search").with_field("category");

	let ordering_backend = SimpleOrderingBackend::new("ordering").allow_field("price");

	// Case 1: Only search parameter provided (no ordering)
	let mut params = HashMap::new();
	params.insert("search".to_string(), "furniture".to_string());

	let base_sql = "SELECT * FROM products".to_string();
	let filtered_sql = search_backend
		.filter_queryset(&params, base_sql.clone())
		.await
		.expect("Search filter failed");
	let filtered_sql = ordering_backend
		.filter_queryset(&params, filtered_sql)
		.await
		.expect("Ordering filter failed");

	// Verify SQL: search applied, no ORDER BY
	assert!(filtered_sql.contains("category LIKE '%furniture%'"));
	assert!(!filtered_sql.contains("ORDER BY"));

	// Execute and verify
	let rows = sqlx::query(&filtered_sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	// Should return furniture products without specific ordering
	assert_eq!(rows.len(), 3); // Desk, Chair, Bookshelf

	// Case 2: Both search and ordering provided
	params.insert("ordering".to_string(), "price".to_string());

	let filtered_sql = search_backend
		.filter_queryset(&params, base_sql)
		.await
		.expect("Search filter failed");
	let filtered_sql = ordering_backend
		.filter_queryset(&params, filtered_sql)
		.await
		.expect("Ordering filter failed");

	// Verify SQL: both search and ordering applied
	assert!(filtered_sql.contains("category LIKE '%furniture%'"));
	assert!(filtered_sql.contains("ORDER BY price ASC"));

	// Execute and verify ordering
	let rows = sqlx::query(&filtered_sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	assert_eq!(rows.len(), 3);

	let prices: Vec<i32> = rows
		.iter()
		.map(|r| r.try_get::<i32, _>("price").expect("Failed to get price"))
		.collect();
	// Expected: Bookshelf(25000), Chair(35000), Desk(45000)
	assert_eq!(prices, vec![25000, 35000, 45000]);
}

// ========================================================================
// Test 5: Filter Chain Execution Order Verification
// ========================================================================

/// Test that filter chain execution order matters for SQL correctness
///
/// **Test Intent**: Verify that the order of filter application matters
/// for SQL structure and execution correctness. Incorrect order produces invalid SQL.
///
/// **Integration Point**: FilterBackend chaining order → SQL clause ordering
///
/// **Verification**:
/// - Correct order (Search → Ordering) produces valid SQL
/// - Incorrect order (Ordering → Search) produces invalid SQL structure
/// - SQL clause ordering (WHERE before ORDER BY) is critical
#[rstest]
#[tokio::test]
async fn test_filter_chain_execution_order(
	#[future] filter_chain_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>),
) {
	let (_container, pool) = filter_chain_test_db.await;

	let search_backend = SimpleSearchBackend::new("search").with_field("category");

	let ordering_backend = SimpleOrderingBackend::new("ordering").allow_field("price");

	let mut params = HashMap::new();
	params.insert("search".to_string(), "electronics".to_string());
	params.insert("ordering".to_string(), "-price".to_string());

	// Order 1: Search → Ordering (CORRECT ORDER)
	let base_sql = "SELECT * FROM products".to_string();
	let sql1 = search_backend
		.filter_queryset(&params, base_sql.clone())
		.await
		.expect("Search filter failed");
	let sql1 = ordering_backend
		.filter_queryset(&params, sql1)
		.await
		.expect("Ordering filter failed");

	// Verify correct SQL structure: WHERE before ORDER BY
	assert!(sql1.contains("WHERE"));
	assert!(sql1.contains("ORDER BY"));
	let where_pos = sql1.find("WHERE").unwrap();
	let order_pos = sql1.find("ORDER BY").unwrap();
	assert!(where_pos < order_pos, "WHERE must come before ORDER BY");

	// Execute query and verify it works
	let rows1 = sqlx::query(&sql1)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query 1 execution failed");

	assert_eq!(rows1.len(), 6); // Electronics products

	// Order 2: Ordering → Search (INCORRECT ORDER - produces invalid SQL)
	let sql2 = ordering_backend
		.filter_queryset(&params, base_sql)
		.await
		.expect("Ordering filter failed");

	// At this point, sql2 has ORDER BY clause
	assert!(sql2.contains("ORDER BY"));

	// Applying search filter after ORDER BY produces invalid SQL
	let sql2 = search_backend
		.filter_queryset(&params, sql2)
		.await
		.expect("Search filter failed");

	// SQL structure is invalid: ORDER BY comes before WHERE
	// This demonstrates why filter order matters
	assert!(sql2.contains("WHERE"));
	assert!(sql2.contains("ORDER BY"));

	// Attempting to execute this query should fail due to syntax error
	let result2 = sqlx::query(&sql2).fetch_all(pool.as_ref()).await;

	// Query execution fails because of invalid SQL syntax (ORDER BY before WHERE)
	assert!(result2.is_err(), "Invalid SQL should fail to execute");

	// Verify error message indicates syntax error
	let err_msg = result2.unwrap_err().to_string();
	assert!(err_msg.contains("syntax error") || err_msg.contains("42601"));
}

// ========================================================================
// Test 6: Filter Chain with Conflicting Conditions
// ========================================================================

/// Test filter chain behavior with conflicting filter conditions
///
/// **Test Intent**: Verify that conflicting filter conditions are handled
/// correctly (e.g., multiple conflicting range filters).
///
/// **Integration Point**: Multiple FilterBackends → Conflict resolution in SQL
///
/// **Verification**:
/// - Conflicting conditions produce expected results
/// - SQL structure reflects all conditions (AND logic)
/// - Empty result set when conditions are impossible to satisfy
#[rstest]
#[tokio::test]
async fn test_filter_chain_with_conflicts(
	#[future] filter_chain_test_db: (ContainerAsync<GenericImage>, Arc<sqlx::PgPool>),
) {
	let (_container, pool) = filter_chain_test_db.await;

	// Create conflicting range filters:
	// price >= 50000 AND price <= 30000 (impossible condition)
	let base_sql = "SELECT * FROM products".to_string();

	let filter1: RangeFilter<i32> = RangeFilter::new("price").gte(50000);
	let filter2: RangeFilter<i32> = RangeFilter::new("price").lte(30000);

	let filtered_sql = format!(
		"{} WHERE price >= {} AND price <= {}",
		base_sql,
		filter1.gte.as_ref().unwrap(),
		filter2.lte.as_ref().unwrap()
	);

	// Execute query with conflicting conditions
	let rows = sqlx::query(&filtered_sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	// Should return empty result set (no products satisfy both conditions)
	assert_eq!(rows.len(), 0);

	// Now test overlapping conditions (not conflicting):
	// price >= 20000 AND price <= 50000 (valid range)
	let filter3: RangeFilter<i32> = RangeFilter::new("price").gte(20000);
	let filter4: RangeFilter<i32> = RangeFilter::new("price").lte(50000);

	let filtered_sql = format!(
		"{} WHERE price >= {} AND price <= {}",
		base_sql,
		filter3.gte.as_ref().unwrap(),
		filter4.lte.as_ref().unwrap()
	);

	// Execute query with valid overlapping conditions
	let rows = sqlx::query(&filtered_sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Query execution failed");

	// Should return products in range 20000-50000
	// Expected: Bookshelf(25000), Headphones(28000), Chair(35000), Desk(45000)
	assert_eq!(rows.len(), 4);

	// Verify all results are within range
	for row in &rows {
		let price: i32 = row.try_get("price").expect("Failed to get price");
		assert!((20000..=50000).contains(&price));
	}
}
