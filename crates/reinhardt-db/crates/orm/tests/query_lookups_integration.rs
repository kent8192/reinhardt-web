//! ORM Query Lookups Integration Tests
//!
//! Tests query lookup operators for Phase 2 Query Foundation, covering:
//! - Lookup operator SQL generation (exact, contains, gt, lte, in, not_in, etc.)
//! - Equivalence partitioning for each lookup category
//! - Decision table testing for IN/NOT IN and NULL handling
//! - Boundary value testing for comparison operators
//! - Combined lookup operator queries
//!
//! **Test Strategy:**
//! - Normal cases: All lookup types working correctly
//! - Equivalence partitioning: Test each lookup operator category (equality, comparison, containment, null)
//! - Decision Table: IN vs NOT IN, NULL handling scenarios
//! - Boundary Values: Test min/max values for numeric comparisons
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use chrono::{DateTime, Utc};
use reinhardt_orm::Model;
use reinhardt_orm::manager::{get_connection, init_database};
use reinhardt_orm::query::{Filter, FilterOperator, FilterValue};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Model Definitions
// ============================================================================

/// Products model for testing lookups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
	pub id: Option<i32>,
	pub name: String,
	pub description: String,
	pub price: i64,
	pub category: String,
	pub created_at: DateTime<Utc>,
}

reinhardt_test::impl_test_model!(Product, i32, "products");

/// Users model for testing NULL lookups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
	pub id: Option<i32>,
	pub name: String,
	pub email: Option<String>,
	pub age: Option<i32>,
}

reinhardt_test::impl_test_model!(User, i32, "users");

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Setup Products test table
async fn setup_products_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS products (
			id SERIAL PRIMARY KEY,
			name TEXT NOT NULL,
			description TEXT,
			price BIGINT NOT NULL,
			category TEXT NOT NULL,
			created_at TIMESTAMP DEFAULT NOW()
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create products table");
}

/// Setup Users test table with nullable fields
async fn setup_users_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			name TEXT NOT NULL,
			email TEXT,
			age INT
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create users table");
}

/// Insert test product data
async fn insert_test_products() {
	let conn = get_connection().await.expect("Failed to get connection");
	let manager = Product::objects();
	let test_data = vec![
		("Laptop", "High-end laptop", 150000_i64, "Electronics"),
		("Mouse", "Wireless mouse", 3000_i64, "Electronics"),
		("Keyboard", "Mechanical keyboard", 12000_i64, "Electronics"),
		("Monitor", "4K monitor", 45000_i64, "Electronics"),
		("Chair", "Office chair", 25000_i64, "Furniture"),
		("Desk", "Standing desk", 60000_i64, "Furniture"),
		("Lamp", "LED desk lamp", 5000_i64, "Furniture"),
		("Book", "Programming book", 4500_i64, "Books"),
	];

	for (name, description, price, category) in test_data {
		let product = Product {
			id: None,
			name: name.to_string(),
			description: description.to_string(),
			price,
			category: category.to_string(),
			created_at: Utc::now(),
		};
		manager
			.create_with_conn(&conn, &product)
			.await
			.expect("Failed to insert test data");
	}
}

/// Insert test user data with NULL values
async fn insert_test_users() {
	let conn = get_connection().await.expect("Failed to get connection");
	let manager = User::objects();
	let test_users = vec![
		User {
			id: None,
			name: "Alice".to_string(),
			email: Some("alice@example.com".to_string()),
			age: Some(30),
		},
		User {
			id: None,
			name: "Bob".to_string(),
			email: None,
			age: Some(25),
		},
		User {
			id: None,
			name: "Charlie".to_string(),
			email: Some("charlie@example.com".to_string()),
			age: None,
		},
		User {
			id: None,
			name: "David".to_string(),
			email: None,
			age: None,
		},
	];

	for user in test_users {
		manager
			.create_with_conn(&conn, &user)
			.await
			.expect("Failed to insert user");
	}
}

// ============================================================================
// Exact Lookup Tests (Equivalence partitioning:Equality Category)
// ============================================================================

/// Test exact lookup with string field
///
/// **Test Intent**: Verify exact match lookup for text fields works correctly
///
/// **Integration Point**: Manager::filter_by → FilterOperator::Eq → PostgreSQL exact match (=)
///
/// **Test Category**: Equivalence partitioning (Equality lookups)
///
/// **Not Intent**: Partial matching, case-insensitive matching
#[rstest]
#[tokio::test]
async fn test_exact_lookup_string(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");
	let manager = Product::objects();
	let filter = Filter::new(
		"name".to_string(),
		FilterOperator::Eq,
		FilterValue::String("Laptop".to_string()),
	);

	let products = manager
		.filter_by(filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute exact lookup");

	assert_eq!(products.len(), 1);

	let product = &products[0];
	assert_eq!(product.name, "Laptop");
	assert_eq!(product.price, 150000);
}

/// Test exact lookup with numeric field
///
/// **Test Intent**: Verify exact match lookup for numeric fields works correctly
///
/// **Integration Point**: Manager::filter_by → FilterOperator::Eq → PostgreSQL exact match for integers
///
/// **Test Category**: Equivalence partitioning (Equality lookups)
///
/// **Not Intent**: Range queries, comparison operators
#[rstest]
#[tokio::test]
async fn test_exact_lookup_numeric(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();
	let filter = Filter::new(
		"price".to_string(),
		FilterOperator::Eq,
		FilterValue::Int(3000),
	);

	let products = manager
		.filter_by(filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute numeric exact lookup");

	assert_eq!(products.len(), 1);

	let product = &products[0];
	assert_eq!(product.name, "Mouse");
	assert_eq!(product.price, 3000);
}

// ============================================================================
// Contains Lookup Tests (Equivalence partitioning:Containment Category)
// ============================================================================

/// Test contains lookup (LIKE %pattern%)
///
/// **Test Intent**: Verify substring matching lookup works correctly
///
/// **Integration Point**: Manager::filter_by → FilterOperator::Contains → PostgreSQL LIKE operator
///
/// **Test Category**: Equivalence partitioning (Containment lookups)
///
/// **Not Intent**: Exact match, starts_with, ends_with
#[rstest]
#[tokio::test]
async fn test_contains_lookup(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();
	let filter = Filter::new(
		"description".to_string(),
		FilterOperator::Contains,
		FilterValue::String("laptop".to_string()),
	);

	let products = manager
		.filter_by(filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute contains lookup");

	assert_eq!(products.len(), 1);

	let product = &products[0];
	assert_eq!(product.name, "Laptop");
	assert!(product.description.contains("laptop"));
}

/// Test startswith lookup (LIKE pattern%)
///
/// **Test Intent**: Verify prefix matching lookup works correctly
///
/// **Integration Point**: Manager::filter_by → FilterOperator::StartsWith → PostgreSQL LIKE with prefix pattern
///
/// **Test Category**: Equivalence partitioning (Containment lookups)
///
/// **Not Intent**: Contains, endswith, exact match
#[rstest]
#[tokio::test]
async fn test_startswith_lookup(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();
	let filter = Filter::new(
		"name".to_string(),
		FilterOperator::StartsWith,
		FilterValue::String("Key".to_string()),
	);

	let products = manager
		.filter_by(filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute startswith lookup");

	assert_eq!(products.len(), 1);

	let product = &products[0];
	assert_eq!(product.name, "Keyboard");
}

// ============================================================================
// Comparison Lookup Tests (Equivalence partitioning:Comparison Category + Boundary Values)
// ============================================================================

/// Test greater than (gt) lookup
///
/// **Test Intent**: Verify gt lookup correctly filters values greater than threshold
///
/// **Integration Point**: Manager::filter_by → FilterOperator::Gt → PostgreSQL > operator
///
/// **Test Category**: Equivalence partitioning (Comparison lookups) + Boundary Values
///
/// **Not Intent**: gte, exact match, lt
#[rstest]
#[tokio::test]
async fn test_gt_lookup(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();
	let filter = Filter::new(
		"price".to_string(),
		FilterOperator::Gt,
		FilterValue::Int(50000),
	);

	let mut products = manager
		.filter_by(filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute gt lookup");

	// Sort by price for consistent ordering
	products.sort_by_key(|p| p.price);

	assert_eq!(products.len(), 2); // Desk (60000) and Laptop (150000)

	let names: Vec<&str> = products.iter().map(|p| p.name.as_str()).collect();
	let prices: Vec<i64> = products.iter().map(|p| p.price).collect();

	assert_eq!(names, vec!["Desk", "Laptop"]);
	assert_eq!(prices, vec![60000, 150000]);

	// Verify all prices are > 50000
	for price in &prices {
		assert!(
			*price > 50000,
			"Price {} should be greater than 50000",
			price
		);
	}
}

/// Test less than or equal (lte) lookup with boundary value
///
/// **Test Intent**: Verify lte lookup includes boundary value and filters correctly
///
/// **Integration Point**: Manager::filter_by → FilterOperator::Lte → PostgreSQL <= operator
///
/// **Test Category**: Equivalence partitioning (Comparison lookups) + Boundary Values
///
/// **Not Intent**: lt (exclusive), gt, gte
#[rstest]
#[tokio::test]
async fn test_lte_lookup_boundary(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();
	let filter = Filter::new(
		"price".to_string(),
		FilterOperator::Lte,
		FilterValue::Int(5000),
	);

	let mut products = manager
		.filter_by(filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute lte lookup");

	// Sort by price for consistent ordering
	products.sort_by_key(|p| p.price);

	assert_eq!(products.len(), 3); // Mouse (3000), Book (4500), Lamp (5000)

	let names: Vec<&str> = products.iter().map(|p| p.name.as_str()).collect();
	let prices: Vec<i64> = products.iter().map(|p| p.price).collect();

	assert_eq!(names, vec!["Mouse", "Book", "Lamp"]);
	assert_eq!(prices, vec![3000, 4500, 5000]);

	// Verify all prices are <= 5000
	for price in &prices {
		assert!(*price <= 5000, "Price {} should be <= 5000", price);
	}

	// Boundary verification: 5000 should be included
	assert!(
		prices.contains(&5000),
		"Boundary value 5000 should be included"
	);
}

/// Test greater than or equal (gte) lookup
///
/// **Test Intent**: Verify gte lookup includes boundary value
///
/// **Integration Point**: Manager::filter_by → FilterOperator::Gte → PostgreSQL >= operator
///
/// **Test Category**: Boundary Values (inclusive lower bound)
///
/// **Not Intent**: gt (exclusive), lt, lte
#[rstest]
#[tokio::test]
async fn test_gte_lookup_boundary(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();
	let filter = Filter::new(
		"price".to_string(),
		FilterOperator::Gte,
		FilterValue::Int(12000),
	);

	let mut products = manager
		.filter_by(filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute gte lookup");

	// Sort by price for consistent ordering
	products.sort_by_key(|p| p.price);

	assert_eq!(products.len(), 5); // Keyboard, Chair, Monitor, Desk, Laptop

	let prices: Vec<i64> = products.iter().map(|p| p.price).collect();

	assert_eq!(prices, vec![12000, 25000, 45000, 60000, 150000]);

	// Verify all prices are >= 12000
	for price in &prices {
		assert!(
			*price >= 12000,
			"Price {} should be >= 12000 (inclusive boundary)",
			price
		);
	}

	// Boundary verification: 12000 should be included
	assert!(
		prices.contains(&12000),
		"Boundary value 12000 should be included"
	);
}

/// Test less than (lt) lookup
///
/// **Test Intent**: Verify lt lookup excludes boundary value (exclusive upper bound)
///
/// **Integration Point**: Manager::filter_by → FilterOperator::Lt → PostgreSQL < operator
///
/// **Test Category**: Boundary Values (exclusive upper bound)
///
/// **Not Intent**: lte (inclusive), gt, gte
#[rstest]
#[tokio::test]
async fn test_lt_lookup_exclusive_boundary(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();
	let filter = Filter::new(
		"price".to_string(),
		FilterOperator::Lt,
		FilterValue::Int(5000),
	);

	let mut products = manager
		.filter_by(filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute lt lookup");

	// Sort by price for consistent ordering
	products.sort_by_key(|p| p.price);

	assert_eq!(products.len(), 2); // Mouse (3000), Book (4500) - NOT Lamp (5000)

	let prices: Vec<i64> = products.iter().map(|p| p.price).collect();

	assert_eq!(prices, vec![3000, 4500]);

	// Verify all prices are < 5000
	for price in &prices {
		assert!(
			*price < 5000,
			"Price {} should be < 5000 (exclusive boundary)",
			price
		);
	}

	// Boundary verification: 5000 should NOT be included
	assert!(
		!prices.contains(&5000),
		"Boundary value 5000 should NOT be included (exclusive)"
	);
}

// ============================================================================
// IN Lookup Tests (Decision Table: IN vs NOT IN)
// ============================================================================

/// Test IN lookup with multiple values
///
/// **Test Intent**: Verify IN lookup correctly matches any value in the list
///
/// **Integration Point**: Manager::filter_by → FilterOperator::In → PostgreSQL IN operator
///
/// **Test Category**: Decision Table (IN operator)
///
/// **Not Intent**: NOT IN, single value match, range queries
#[rstest]
#[tokio::test]
async fn test_in_lookup(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();
	let categories = vec!["Electronics".to_string(), "Books".to_string()];
	let filter = Filter::new(
		"category".to_string(),
		FilterOperator::In,
		FilterValue::Array(categories),
	);

	let mut products = manager
		.filter_by(filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute IN lookup");

	// Sort by name for consistent ordering
	products.sort_by(|a, b| a.name.cmp(&b.name));

	assert_eq!(products.len(), 5); // 4 Electronics + 1 Book

	let names: Vec<&str> = products.iter().map(|p| p.name.as_str()).collect();
	let categories_result: Vec<&str> = products.iter().map(|p| p.category.as_str()).collect();

	assert_eq!(
		names,
		vec!["Book", "Keyboard", "Laptop", "Monitor", "Mouse"]
	);

	// Verify all results are either Electronics or Books
	for category in categories_result {
		assert!(
			category == "Electronics" || category == "Books",
			"Category {} should be either Electronics or Books",
			category
		);
	}
}

/// Test NOT IN lookup
///
/// **Test Intent**: Verify NOT IN lookup correctly excludes values in the list
///
/// **Integration Point**: Manager::filter_by → FilterOperator::NotIn → PostgreSQL NOT IN operator
///
/// **Test Category**: Decision Table (NOT IN operator)
///
/// **Not Intent**: IN, single value exclusion
#[rstest]
#[tokio::test]
async fn test_not_in_lookup(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();
	let categories = vec!["Electronics".to_string(), "Books".to_string()];
	let filter = Filter::new(
		"category".to_string(),
		FilterOperator::NotIn,
		FilterValue::Array(categories),
	);

	let mut products = manager
		.filter_by(filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute NOT IN lookup");

	// Sort by name for consistent ordering
	products.sort_by(|a, b| a.name.cmp(&b.name));

	assert_eq!(products.len(), 3); // 3 Furniture items

	let names: Vec<&str> = products.iter().map(|p| p.name.as_str()).collect();
	let categories_result: Vec<&str> = products.iter().map(|p| p.category.as_str()).collect();

	assert_eq!(names, vec!["Chair", "Desk", "Lamp"]);

	// Verify all results are NOT Electronics or Books
	for category in categories_result {
		assert_eq!(category, "Furniture");
		assert!(
			category != "Electronics" && category != "Books",
			"Category {} should NOT be Electronics or Books",
			category
		);
	}
}

// ============================================================================
// NULL Lookup Tests (Decision Table: NULL handling)
// ============================================================================

/// Test IS NULL lookup
///
/// **Test Intent**: Verify IS NULL lookup correctly identifies NULL values
///
/// **Integration Point**: Manager::filter_by → FilterOperator::Eq with FilterValue::Null → PostgreSQL IS NULL operator
///
/// **Test Category**: Decision Table (NULL handling - IS NULL branch)
///
/// **Not Intent**: IS NOT NULL, non-NULL values
#[rstest]
#[tokio::test]
async fn test_isnull_lookup(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_users_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_users().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = User::objects();
	let filter = Filter::new("email".to_string(), FilterOperator::Eq, FilterValue::Null);

	let mut users = manager
		.filter_by(filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute IS NULL lookup");

	// Sort by name for consistent ordering
	users.sort_by(|a, b| a.name.cmp(&b.name));

	assert_eq!(users.len(), 2); // Bob and David have NULL email

	let names: Vec<&str> = users.iter().map(|u| u.name.as_str()).collect();

	assert_eq!(names, vec!["Bob", "David"]);

	// Verify all emails are NULL
	for user in users {
		assert!(user.email.is_none(), "Email should be NULL");
	}
}

/// Test IS NOT NULL lookup
///
/// **Test Intent**: Verify IS NOT NULL lookup correctly excludes NULL values
///
/// **Integration Point**: Manager::filter_by → FilterOperator::Ne with FilterValue::Null → PostgreSQL IS NOT NULL operator
///
/// **Test Category**: Decision Table (NULL handling - IS NOT NULL branch)
///
/// **Not Intent**: IS NULL, NULL values
#[rstest]
#[tokio::test]
async fn test_isnotnull_lookup(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_users_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_users().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = User::objects();
	let filter = Filter::new("email".to_string(), FilterOperator::Ne, FilterValue::Null);

	let mut users = manager
		.filter_by(filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute IS NOT NULL lookup");

	// Sort by name for consistent ordering
	users.sort_by(|a, b| a.name.cmp(&b.name));

	assert_eq!(users.len(), 2); // Alice and Charlie have non-NULL email

	let names: Vec<&str> = users.iter().map(|u| u.name.as_str()).collect();

	assert_eq!(names, vec!["Alice", "Charlie"]);

	// Verify all emails are NOT NULL
	for user in users {
		assert!(user.email.is_some(), "Email should NOT be NULL");
	}
}

// ============================================================================
// Range Lookup Tests (BETWEEN operator)
// ============================================================================

/// Test BETWEEN lookup (range query)
///
/// **Test Intent**: Verify combined Gte + Lte filters emulate BETWEEN correctly
///
/// **Integration Point**: Multiple filters → PostgreSQL range query
///
/// **Test Category**: Equivalence partitioning (Range lookups) + Boundary Values
///
/// **Not Intent**: Individual gt/lt queries, OR conditions
///
/// **Note**: reinhardt-orm doesn't have BETWEEN operator, so we use Gte AND Lte
#[rstest]
#[tokio::test]
async fn test_range_lookup(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();

	// Apply two filters: *price >= 10000 AND *price <= 50000
	let filter_gte = Filter::new(
		"price".to_string(),
		FilterOperator::Gte,
		FilterValue::Int(10000),
	);
	let filter_lte = Filter::new(
		"price".to_string(),
		FilterOperator::Lte,
		FilterValue::Int(50000),
	);

	let mut products = manager
		.filter_by(filter_gte)
		.filter(filter_lte)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute BETWEEN lookup");

	// Sort by price for consistent ordering
	products.sort_by_key(|p| p.price);

	assert_eq!(products.len(), 3); // Keyboard (12000), Chair (25000), Monitor (45000)

	let names: Vec<&str> = products.iter().map(|p| p.name.as_str()).collect();
	let prices: Vec<i64> = products.iter().map(|p| p.price).collect();

	assert_eq!(names, vec!["Keyboard", "Chair", "Monitor"]);
	assert_eq!(prices, vec![12000, 25000, 45000]);

	// Verify all prices are within range [10000, 50000]
	for price in &prices {
		assert!(
			*price >= 10000 && *price <= 50000,
			"Price {} should be between 10000 and 50000 (inclusive)",
			price
		);
	}
}

// ============================================================================
// Combined Lookup Tests (Combined conditions)
// ============================================================================

/// Test combined lookups with AND condition
///
/// **Test Intent**: Verify multiple lookup operators can be combined with AND
///
/// **Integration Point**: Multiple filter_by calls → Multiple WHERE conditions with AND
///
/// **Test Category**: Combined conditions (Combined conditions)
///
/// **Not Intent**: Single condition, OR conditions
#[rstest]
#[tokio::test]
async fn test_combined_lookups_and(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();

	// Apply two filters: category = 'Electronics' AND *price > 10000
	let filter_category = Filter::new(
		"category".to_string(),
		FilterOperator::Eq,
		FilterValue::String("Electronics".to_string()),
	);
	let filter_price = Filter::new(
		"price".to_string(),
		FilterOperator::Gt,
		FilterValue::Int(10000),
	);

	let mut products = manager
		.filter_by(filter_category)
		.filter(filter_price)
		.all_with_db(&conn)
		.await
		.expect("Failed to execute combined AND lookup");

	// Sort by price for consistent ordering
	products.sort_by_key(|p| p.price);

	assert_eq!(products.len(), 3); // Keyboard, Monitor, Laptop

	let names: Vec<&str> = products.iter().map(|p| p.name.as_str()).collect();
	let prices: Vec<i64> = products.iter().map(|p| p.price).collect();
	let categories: Vec<&str> = products.iter().map(|p| p.category.as_str()).collect();

	assert_eq!(names, vec!["Keyboard", "Monitor", "Laptop"]);
	assert_eq!(prices, vec![12000, 45000, 150000]);

	// Verify all results satisfy BOTH conditions
	for (i, category) in categories.iter().enumerate() {
		assert_eq!(*category, "Electronics");
		assert!(prices[i] > 10000);
	}
}

/// Test combined lookups with OR condition
///
/// **Test Intent**: Demonstrate that reinhardt-orm currently doesn't support OR conditions directly
///
/// **Integration Point**: N/A - This test shows current limitation
///
/// **Test Category**: Combined conditions (Combined conditions with OR)
///
/// **Not Intent**: AND conditions, single condition
///
/// **Note**: reinhardt-orm doesn't have OR support yet, so we fetch both sets and merge
#[rstest]
#[tokio::test]
async fn test_combined_lookups_or(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();

	// Since reinhardt-orm doesn't support OR yet, we fetch both conditions separately and merge
	let filter_category = Filter::new(
		"category".to_string(),
		FilterOperator::Eq,
		FilterValue::String("Books".to_string()),
	);
	let filter_price = Filter::new(
		"price".to_string(),
		FilterOperator::Lt,
		FilterValue::Int(5000),
	);

	let products_books = manager
		.filter_by(filter_category)
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch Books");

	let products_cheap = manager
		.filter_by(filter_price)
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch cheap products");

	// Merge and deduplicate by ID
	let mut all_products = Vec::new();
	let mut seen_ids = std::collections::HashSet::new();

	for product in products_books.into_iter().chain(products_cheap.into_iter()) {
		if let Some(id) = product.id {
			if seen_ids.insert(id) {
				all_products.push(product);
			}
		}
	}

	// Sort by price for consistent ordering
	all_products.sort_by_key(|p| p.price);

	// Expected: Mouse (3000, price<5000), Book (4500, both conditions)
	assert_eq!(all_products.len(), 2);

	let names: Vec<&str> = all_products.iter().map(|p| p.name.as_str()).collect();
	let prices: Vec<i64> = all_products.iter().map(|p| p.price).collect();
	let categories: Vec<&str> = all_products.iter().map(|p| p.category.as_str()).collect();

	assert_eq!(names, vec!["Mouse", "Book"]);
	assert_eq!(prices, vec![3000, 4500]);

	// Verify all results satisfy AT LEAST ONE condition
	for i in 0..all_products.len() {
		let satisfies_category = categories[i] == "Books";
		let satisfies_price = prices[i] < 5000;

		assert!(
			satisfies_category || satisfies_price,
			"Result should satisfy at least one condition (category='Books' OR price<5000)"
		);
	}
}

/// Test complex combined lookups with nested conditions
///
/// **Test Intent**: Demonstrate that reinhardt-orm currently doesn't support nested OR/AND conditions
///
/// **Integration Point**: N/A - This test shows current limitation
///
/// **Test Category**: Combined conditions (Complex nested conditions)
///
/// **Not Intent**: Simple AND/OR, single level conditions
///
/// **Note**: reinhardt-orm doesn't have nested condition support yet, so we fetch sets and merge
#[rstest]
#[tokio::test]
async fn test_complex_combined_lookups(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_products_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	insert_test_products().await;

	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Product::objects();

	// (category = 'Electronics' AND *price > 40000) OR (category = 'Furniture' AND *price < 30000)
	// Fetch both condition sets separately and merge

	// First condition: Electronics AND *price > 40000
	let filter_electronics = Filter::new(
		"category".to_string(),
		FilterOperator::Eq,
		FilterValue::String("Electronics".to_string()),
	);
	let filter_expensive = Filter::new(
		"price".to_string(),
		FilterOperator::Gt,
		FilterValue::Int(40000),
	);

	let products_electronics_expensive = manager
		.filter_by(filter_electronics)
		.filter(filter_expensive)
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch Electronics > 40000");

	// Second condition: Furniture AND *price < 30000
	let filter_furniture = Filter::new(
		"category".to_string(),
		FilterOperator::Eq,
		FilterValue::String("Furniture".to_string()),
	);
	let filter_cheap = Filter::new(
		"price".to_string(),
		FilterOperator::Lt,
		FilterValue::Int(30000),
	);

	let products_furniture_cheap = manager
		.filter_by(filter_furniture)
		.filter(filter_cheap)
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch Furniture < 30000");

	// Merge and deduplicate
	let mut all_products = Vec::new();
	let mut seen_ids = std::collections::HashSet::new();

	for product in products_electronics_expensive
		.into_iter()
		.chain(products_furniture_cheap.into_iter())
	{
		if let Some(id) = product.id {
			if seen_ids.insert(id) {
				all_products.push(product);
			}
		}
	}

	// Sort by price for consistent ordering
	all_products.sort_by_key(|p| p.price);

	// Expected: Lamp (Furniture, 5000), Chair (Furniture, 25000), Monitor (Electronics, 45000), Laptop (Electronics, 150000)
	assert_eq!(all_products.len(), 4);

	let names: Vec<&str> = all_products.iter().map(|p| p.name.as_str()).collect();
	let prices: Vec<i64> = all_products.iter().map(|p| p.price).collect();
	let categories: Vec<&str> = all_products.iter().map(|p| p.category.as_str()).collect();

	assert_eq!(names, vec!["Lamp", "Chair", "Monitor", "Laptop"]);
	assert_eq!(prices, vec![5000, 25000, 45000, 150000]);

	// Verify each result satisfies the complex condition
	for i in 0..all_products.len() {
		let electronics_and_expensive = categories[i] == "Electronics" && prices[i] > 40000;
		let furniture_and_cheap = categories[i] == "Furniture" && prices[i] < 30000;

		assert!(
			electronics_and_expensive || furniture_and_cheap,
			"Result {} should satisfy: (Electronics AND price>40000) OR (Furniture AND price<30000)",
			names[i]
		);
	}
}
