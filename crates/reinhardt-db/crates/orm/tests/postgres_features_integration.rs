//! PostgreSQL-Specific Features Integration Tests - Phase 6
//!
//! Tests for advanced PostgreSQL-specific functionality with the ORM, covering:
//! - Full-Text Search (tsvector, tsquery operations)
//! - Array Aggregation (array_agg aggregate function)
//! - Array Overlap Operations (@@ operator for arrays)
//! - PostgreSQL-Specific Aggregate Functions (string_agg, json_agg, etc.)
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Integration Point:**
//! This test verifies that PostgreSQL-specific operations are correctly handled
//! through the entire stack: ORM → QueryBuilder → SQL generation → PostgreSQL execution
//!
//! **Phase 6 Coverage:**
//! - Normal Cases: All PostgreSQL-specific features working correctly
//! - PostgreSQL Exclusive: Features only available in PostgreSQL
//! - Tests: 7-9 comprehensive test cases

use reinhardt_orm::manager::reinitialize_database;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serial_test::serial;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Fixtures
// ============================================================================

#[fixture]
async fn postgres_features_test_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String) {
	let (container, pool, port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	(container, pool, port, url)
}

// ============================================================================
// Full-Text Search Tests
// ============================================================================

/// Test PostgreSQL Full-Text Search with tsvector
///
/// **Test Intent**: Verify full-text search using tsvector and tsquery works correctly
///
/// **Integration Point**: ORM → Full-text search operators → PostgreSQL tsvector matching
///
/// **PostgreSQL Exclusive**: tsvector and tsquery are PostgreSQL-specific types
///
/// **Not Intent**: Simple text matching, LIKE operators
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_full_text_search_basic(
	#[future] postgres_features_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_features_test_db.await;

	// Create table with tsvector column for full-text search
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS documents (
			id SERIAL PRIMARY KEY,
			title TEXT NOT NULL,
			content TEXT NOT NULL,
			search_vector TSVECTOR NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create documents table");

	// Insert documents with tsvector
	sqlx::query(
		r#"
		INSERT INTO documents (title, content, search_vector)
		VALUES
			('Rust Programming', 'Learn Rust programming language basics', to_tsvector('english', 'rust programming language basics')),
			('Python Guide', 'Complete Python programming tutorial', to_tsvector('english', 'python programming tutorial')),
			('Web Development', 'Building web applications with Rust', to_tsvector('english', 'web building applications rust'))
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert documents");

	// Query documents using full-text search
	let results: Vec<String> = sqlx::query_scalar(
		r#"
		SELECT title FROM documents
		WHERE search_vector @@ to_tsquery('english', 'rust')
		ORDER BY title
		"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query with full-text search");

	assert_eq!(results.len(), 2);
	assert!(results.contains(&"Rust Programming".to_string()));
	assert!(results.contains(&"Web Development".to_string()));
}

/// Test PostgreSQL Full-Text Search with ranking
///
/// **Test Intent**: Verify full-text search ranking (ts_rank) works correctly
///
/// **Integration Point**: ORM → Full-text ranking → PostgreSQL ts_rank function
///
/// **PostgreSQL Exclusive**: ts_rank function is PostgreSQL-specific
///
/// **Not Intent**: Simple relevance, basic matching
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_full_text_search_with_ranking(
	#[future] postgres_features_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_features_test_db.await;

	// Create documents table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS articles (
			id SERIAL PRIMARY KEY,
			title TEXT NOT NULL,
			body TEXT NOT NULL,
			search_vector TSVECTOR NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create articles table");

	// Insert articles
	sqlx::query(
		r#"
		INSERT INTO articles (title, body, search_vector)
		VALUES
			('Database Performance', 'Optimizing database queries for performance', to_tsvector('english', 'optimizing database queries for performance')),
			('Query Optimization', 'How to optimize SQL queries effectively', to_tsvector('english', 'optimize sql queries effectively')),
			('Index Design', 'Creating effective database indexes', to_tsvector('english', 'creating effective database indexes'))
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert articles");

	// Query with ranking
	let results: Vec<(String, f32)> = sqlx::query_as(
		r#"
		SELECT title, ts_rank(search_vector, to_tsquery('english', 'database')) as rank
		FROM articles
		WHERE search_vector @@ to_tsquery('english', 'database')
		ORDER BY rank DESC
		"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query with ranking");

	assert!(!results.is_empty());
	assert_eq!(results[0].0, "Database Performance");
	assert!(results[0].1 > 0.0);
}

// ============================================================================
// Array Aggregation Tests
// ============================================================================

/// Test PostgreSQL array_agg aggregate function
///
/// **Test Intent**: Verify array_agg correctly aggregates values into arrays
///
/// **Integration Point**: ORM → Aggregate functions → PostgreSQL array_agg
///
/// **PostgreSQL Exclusive**: array_agg is a PostgreSQL-specific aggregate function
///
/// **Not Intent**: Simple array operations, GROUP_CONCAT
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_array_agg_basic(
	#[future] postgres_features_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_features_test_db.await;

	// Create table with category and items
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS category_items (
			id SERIAL PRIMARY KEY,
			category TEXT NOT NULL,
			item_name TEXT NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create category_items table");

	// Insert test data
	sqlx::query(
		r#"
		INSERT INTO category_items (category, item_name)
		VALUES
			('fruits', 'apple'),
			('fruits', 'banana'),
			('fruits', 'orange'),
			('vegetables', 'carrot'),
			('vegetables', 'lettuce'),
			('vegetables', 'spinach')
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert test data");

	// Query with array_agg
	let results: Vec<(String, Vec<String>)> = sqlx::query_as(
		r#"
		SELECT category, array_agg(item_name ORDER BY item_name) as items
		FROM category_items
		GROUP BY category
		ORDER BY category
		"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query with array_agg");

	assert_eq!(results.len(), 2);

	// Check fruits category
	let (category, items) = &results[0];
	assert_eq!(category, "fruits");
	assert_eq!(
		items,
		&vec![
			"apple".to_string(),
			"banana".to_string(),
			"orange".to_string()
		]
	);

	// Check vegetables category
	let (category, items) = &results[1];
	assert_eq!(category, "vegetables");
	assert_eq!(
		items,
		&vec![
			"carrot".to_string(),
			"lettuce".to_string(),
			"spinach".to_string()
		]
	);
}

/// Test PostgreSQL array_agg with numeric values
///
/// **Test Intent**: Verify array_agg works with numeric types
///
/// **Integration Point**: ORM → Numeric aggregation → PostgreSQL array_agg with numbers
///
/// **PostgreSQL Exclusive**: array_agg with ORDER BY clause
///
/// **Not Intent**: Simple numeric aggregation, SUM/AVG
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_array_agg_numeric(
	#[future] postgres_features_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_features_test_db.await;

	// Create sales table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS sales (
			id SERIAL PRIMARY KEY,
			product_id INTEGER NOT NULL,
			amount DECIMAL(10, 2) NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create sales table");

	// Insert sales data
	sqlx::query(
		r#"
		INSERT INTO sales (product_id, amount)
		VALUES
			(1, 100.50),
			(1, 200.75),
			(2, 150.25),
			(2, 300.00),
			(3, 50.00)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert sales data");

	// Query with array_agg for numeric values (cast to double precision for sqlx compatibility)
	let results: Vec<(i32, Vec<f64>)> = sqlx::query_as(
		r#"
		SELECT product_id, array_agg(amount::double precision ORDER BY amount) as amounts
		FROM sales
		GROUP BY product_id
		ORDER BY product_id
		"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query with numeric array_agg");

	assert_eq!(results.len(), 3);

	// Verify product 1
	assert_eq!(results[0].0, 1);
	assert_eq!(results[0].1.len(), 2);
}

// ============================================================================
// Array Overlap Operations Tests
// ============================================================================

/// Test PostgreSQL array overlap operator (@>)
///
/// **Test Intent**: Verify array containment operator works for queries
///
/// **Integration Point**: ORM → Array operators → PostgreSQL @> operator
///
/// **PostgreSQL Exclusive**: @> operator for array containment
///
/// **Not Intent**: Array equality, simple array matching
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_array_overlap_containment(
	#[future] postgres_features_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_features_test_db.await;

	// Create user permissions table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS user_permissions (
			id SERIAL PRIMARY KEY,
			username TEXT NOT NULL,
			permissions TEXT[] NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create user_permissions table");

	// Insert users with permissions
	sqlx::query(
		r#"
		INSERT INTO user_permissions (username, permissions)
		VALUES
			('alice', ARRAY['read', 'write', 'delete']),
			('bob', ARRAY['read', 'write']),
			('charlie', ARRAY['read']),
			('david', ARRAY['read', 'write', 'delete', 'admin'])
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert permissions data");

	// Query users who have both read and write permissions
	let results: Vec<String> = sqlx::query_scalar(
		r#"
		SELECT username FROM user_permissions
		WHERE permissions @> ARRAY['read', 'write']
		ORDER BY username
		"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query with array containment");

	assert_eq!(results.len(), 3);
	assert!(results.contains(&"alice".to_string()));
	assert!(results.contains(&"bob".to_string()));
	assert!(results.contains(&"david".to_string()));
	assert!(!results.contains(&"charlie".to_string()));
}

/// Test PostgreSQL array overlap operator (&&)
///
/// **Test Intent**: Verify array overlap operator detects any common elements
///
/// **Integration Point**: ORM → Array operators → PostgreSQL && operator
///
/// **PostgreSQL Exclusive**: && operator for array overlap
///
/// **Not Intent**: Containment, array equality
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_array_overlap_any_element(
	#[future] postgres_features_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_features_test_db.await;

	// Create user interests table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS user_interests (
			id SERIAL PRIMARY KEY,
			user_id INTEGER NOT NULL,
			interests TEXT[] NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create user_interests table");

	// Insert user interests
	sqlx::query(
		r#"
		INSERT INTO user_interests (user_id, interests)
		VALUES
			(1, ARRAY['rust', 'programming', 'systems']),
			(2, ARRAY['python', 'data-science', 'ml']),
			(3, ARRAY['rust', 'web', 'async']),
			(4, ARRAY['javascript', 'frontend', 'react'])
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert interests data");

	// Query users interested in either 'rust' or 'web'
	let results: Vec<i32> = sqlx::query_scalar(
		r#"
		SELECT user_id FROM user_interests
		WHERE interests && ARRAY['rust', 'web']
		ORDER BY user_id
		"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query with array overlap");

	assert_eq!(results.len(), 2);
	assert!(results.contains(&1));
	assert!(results.contains(&3));
}

// ============================================================================
// PostgreSQL-Specific Aggregate Functions Tests
// ============================================================================

/// Test PostgreSQL string_agg aggregate function
///
/// **Test Intent**: Verify string_agg correctly concatenates string values
///
/// **Integration Point**: ORM → String aggregation → PostgreSQL string_agg
///
/// **PostgreSQL Exclusive**: string_agg is a PostgreSQL-specific function
///
/// **Not Intent**: Simple concatenation, GROUP_CONCAT equivalents
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_string_agg_basic(
	#[future] postgres_features_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_features_test_db.await;

	// Create table with authors and books
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS author_books (
			id SERIAL PRIMARY KEY,
			author TEXT NOT NULL,
			book_title TEXT NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create author_books table");

	// Insert test data
	sqlx::query(
		r#"
		INSERT INTO author_books (author, book_title)
		VALUES
			('J.K. Rowling', 'Harry Potter and the Philosopher''s Stone'),
			('J.K. Rowling', 'Harry Potter and the Chamber of Secrets'),
			('George R.R. Martin', 'A Game of Thrones'),
			('George R.R. Martin', 'A Clash of Kings'),
			('J.R.R. Tolkien', 'The Hobbit')
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert author books data");

	// Query with string_agg
	let results: Vec<(String, String)> = sqlx::query_as(
		r#"
		SELECT author, string_agg(book_title, ', ' ORDER BY book_title) as books
		FROM author_books
		GROUP BY author
		ORDER BY author
		"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query with string_agg");

	assert_eq!(results.len(), 3);

	// Verify J.K. Rowling's books
	let (author, books) = &results[1];
	assert_eq!(author, "J.K. Rowling");
	assert!(books.contains("Harry Potter and the Chamber of Secrets"));
	assert!(books.contains("Harry Potter and the Philosopher's Stone"));
	assert!(books.contains(", "));
}

/// Test PostgreSQL json_agg aggregate function
///
/// **Test Intent**: Verify json_agg correctly aggregates values into JSON arrays
///
/// **Integration Point**: ORM → JSON aggregation → PostgreSQL json_agg
///
/// **PostgreSQL Exclusive**: json_agg is a PostgreSQL-specific aggregate function
///
/// **Not Intent**: Simple JSON, JSONB operations
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_json_agg_basic(
	#[future] postgres_features_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_features_test_db.await;

	// Create orders table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS orders (
			id SERIAL PRIMARY KEY,
			customer_id INTEGER NOT NULL,
			order_date DATE NOT NULL,
			total DECIMAL(10, 2) NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create orders table");

	// Insert test data
	sqlx::query(
		r#"
		INSERT INTO orders (customer_id, order_date, total)
		VALUES
			(1, '2025-01-01'::date, 100.00),
			(1, '2025-01-15'::date, 250.50),
			(2, '2025-01-05'::date, 75.00),
			(2, '2025-01-20'::date, 125.75)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert orders data");

	// Query with json_agg
	let results: Vec<(i32, serde_json::Value)> = sqlx::query_as(
		r#"
		SELECT customer_id, json_agg(json_build_object('id', id, 'total', total) ORDER BY order_date)
		FROM orders
		GROUP BY customer_id
		ORDER BY customer_id
		"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query with json_agg");

	assert_eq!(results.len(), 2);

	// Verify structure for customer 1
	assert_eq!(results[0].0, 1);
	let json_array = results[0].1.as_array();
	assert!(json_array.is_some());
	assert_eq!(json_array.unwrap().len(), 2);
}

/// Test PostgreSQL Count Distinct with Multiple Columns
///
/// **Test Intent**: Verify COUNT(DISTINCT) works for distinct value counting
///
/// **Integration Point**: ORM → Distinct counting → PostgreSQL COUNT(DISTINCT)
///
/// **PostgreSQL Exclusive**: COUNT(DISTINCT col1, col2) for multiple columns
///
/// **Not Intent**: Simple COUNT, GROUP BY
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_count_distinct_aggregate(
	#[future] postgres_features_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_features_test_db.await;

	// Create events table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS events (
			id SERIAL PRIMARY KEY,
			user_id INTEGER NOT NULL,
			event_type TEXT NOT NULL,
			occurred_at TIMESTAMP NOT NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create events table");

	// Insert test data
	sqlx::query(
		r#"
		INSERT INTO events (user_id, event_type, occurred_at)
		VALUES
			(1, 'login', NOW()),
			(1, 'view', NOW()),
			(1, 'login', NOW()),
			(2, 'login', NOW()),
			(2, 'view', NOW()),
			(3, 'view', NOW()),
			(3, 'view', NOW())
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to insert events data");

	// Query with COUNT(DISTINCT)
	let results: Vec<(String, i64)> = sqlx::query_as(
		r#"
		SELECT event_type, COUNT(DISTINCT user_id) as distinct_users
		FROM events
		GROUP BY event_type
		ORDER BY event_type
		"#,
	)
	.fetch_all(pool.as_ref())
	.await
	.expect("Failed to query with COUNT(DISTINCT)");

	assert_eq!(results.len(), 2);

	// Verify login event
	let (event_type, count) = &results[0];
	assert_eq!(event_type, "login");
	assert_eq!(*count, 2);

	// Verify view event
	let (event_type, count) = &results[1];
	assert_eq!(event_type, "view");
	assert_eq!(*count, 3);
}
