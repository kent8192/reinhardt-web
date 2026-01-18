//! Lambda Statement Query Caching Integration Tests
//!
//! Tests comprehensive query caching functionality covering:
//! - Cache hit/miss scenarios in normal operations
//! - Cache invalidation and state transitions
//! - Cache consistency with property-based testing
//! - Query execution with lambda statements
//! - Cache statistics tracking
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Test Data Schema:**
//! - users(id SERIAL PRIMARY KEY, name TEXT NOT NULL, email TEXT NOT NULL, active BOOLEAN NOT NULL)
//! - products(id SERIAL PRIMARY KEY, name TEXT NOT NULL, price BIGINT NOT NULL)

use reinhardt_db::orm::lambda_stmt::{CACHE_STATS, LambdaStmt, QUERY_CACHE};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sea_query::{Expr, ExprTrait, Iden, PostgresQueryBuilder, Query};
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Table Identifiers
// ============================================================================

#[derive(Iden)]
enum Users {
	Table,
	Id,
	Name,
	Active,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Initialize test database with schema
async fn setup_test_data(pool: &PgPool) {
	// Create users table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id SERIAL PRIMARY KEY,
			name TEXT NOT NULL,
			email TEXT NOT NULL,
			active BOOLEAN NOT NULL
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create users table");

	// Create products table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS products (
			id SERIAL PRIMARY KEY,
			name TEXT NOT NULL,
			price BIGINT NOT NULL
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create products table");

	// Insert test data
	sqlx::query(
		"INSERT INTO users (name, email, active) VALUES ($1, $2, $3), ($4, $5, $6), ($7, $8, $9)",
	)
	.bind("Alice")
	.bind("alice@example.com")
	.bind(true)
	.bind("Bob")
	.bind("bob@example.com")
	.bind(false)
	.bind("Charlie")
	.bind("charlie@example.com")
	.bind(true)
	.execute(pool)
	.await
	.expect("Failed to insert users");

	sqlx::query("INSERT INTO products (name, price) VALUES ($1, $2), ($3, $4)")
		.bind("Laptop")
		.bind(100000_i64)
		.bind("Mouse")
		.bind(3000_i64)
		.execute(pool)
		.await
		.expect("Failed to insert products");
}

/// Clear cache statistics for test isolation
fn reset_cache_stats() {
	QUERY_CACHE.clear();
	CACHE_STATS.write().unwrap().reset();
}

// ============================================================================
// Cache Hit/Miss Tests (Normal cases)
// ============================================================================

/// Test cache miss on first query execution
///
/// **Test Intent**: Verify that first execution results in a cache miss
///
/// **Integration Point**: LambdaStmt → QueryCache → CACHE_STATS
///
/// **Not Intent**: Complex queries, database interactions
#[rstest]
#[tokio::test]
async fn test_cache_miss_on_first_execution(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	reset_cache_stats();
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	let stmt = LambdaStmt::new("get_all_users", || {
		"SELECT id, name, email FROM users".to_string()
	});

	let initial_misses = CACHE_STATS.read().unwrap().misses;

	let result = stmt.execute().expect("Failed to execute statement");
	assert_eq!(result, "SELECT id, name, email FROM users");

	let final_misses = CACHE_STATS.read().unwrap().misses;
	assert_eq!(final_misses, initial_misses + 1);
	assert!(!stmt.is_cached() || QUERY_CACHE.get(&stmt.cache_key).is_some());
}

/// Test cache hit on subsequent query execution
///
/// **Test Intent**: Verify that second execution with same key results in cache hit
///
/// **Integration Point**: LambdaStmt → QueryCache → CACHE_STATS
///
/// **Not Intent**: Complex queries, cache invalidation
#[rstest]
#[tokio::test]
async fn test_cache_hit_on_subsequent_execution(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	reset_cache_stats();
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	let stmt = LambdaStmt::new("get_active_users", || {
		"SELECT id, name FROM users WHERE active = true".to_string()
	});

	let initial_hits = CACHE_STATS.read().unwrap().hits;

	// First execution - cache miss
	let result1 = stmt.execute().expect("Failed to execute first");
	assert_eq!(result1, "SELECT id, name FROM users WHERE active = true");

	// Second execution - cache hit
	let result2 = stmt.execute().expect("Failed to execute second");
	assert_eq!(result2, result1);

	let final_hits = CACHE_STATS.read().unwrap().hits;
	assert_eq!(final_hits, initial_hits + 1);
	assert!(stmt.is_cached());
}

/// Test multiple different queries generate different cache entries
///
/// **Test Intent**: Verify cache distinguishes between different query keys
///
/// **Integration Point**: QueryCache → cache key isolation
///
/// **Not Intent**: Cache invalidation, hit rate calculation
#[rstest]
#[tokio::test]
async fn test_multiple_queries_with_different_keys(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	reset_cache_stats();
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	let stmt1 = LambdaStmt::new("query_users", || "SELECT * FROM users".to_string());
	let stmt2 = LambdaStmt::new("query_products", || "SELECT * FROM products".to_string());

	let result1 = stmt1.execute().expect("Failed first");
	let result2 = stmt2.execute().expect("Failed second");

	assert_eq!(QUERY_CACHE.size(), 2);
	assert!(stmt1.is_cached());
	assert!(stmt2.is_cached());
	assert_ne!(result1, result2);
}

// ============================================================================
// Cache Invalidation Tests (State transition cases)
// ============================================================================

/// Test cache invalidation by clearing all entries
///
/// **Test Intent**: Verify cache.clear() removes all cached queries
///
/// **Integration Point**: QueryCache::clear → internal HashMap
///
/// **Not Intent**: Selective removal, cache size calculations
#[rstest]
#[tokio::test]
async fn test_cache_invalidation_on_clear(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	reset_cache_stats();
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	let stmt1 = LambdaStmt::new("query_1", || "SELECT 1".to_string());
	let stmt2 = LambdaStmt::new("query_2", || "SELECT 2".to_string());

	stmt1.execute().expect("Failed first");
	stmt2.execute().expect("Failed second");

	assert_eq!(QUERY_CACHE.size(), 2);
	assert!(stmt1.is_cached());

	QUERY_CACHE.clear();

	assert_eq!(QUERY_CACHE.size(), 0);
	assert!(QUERY_CACHE.get(&stmt1.cache_key).is_none());
	assert!(QUERY_CACHE.get(&stmt2.cache_key).is_none());
}

/// Test cache removal by specific key
///
/// **Test Intent**: Verify removing specific cache entry doesn't affect others
///
/// **Integration Point**: QueryCache::remove → HashMap::remove
///
/// **Not Intent**: Cache hit/miss calculations, batch operations
#[rstest]
#[tokio::test]
async fn test_cache_removal_by_key(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	reset_cache_stats();
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	let stmt1 = LambdaStmt::new("keep_this", || "SELECT 1".to_string());
	let stmt2 = LambdaStmt::new("remove_this", || "SELECT 2".to_string());

	stmt1.execute().expect("Failed first");
	stmt2.execute().expect("Failed second");

	assert_eq!(QUERY_CACHE.size(), 2);

	let removed = QUERY_CACHE.remove("remove_this");
	assert_eq!(removed, Some("SELECT 2".to_string()));
	assert_eq!(QUERY_CACHE.size(), 1);
	assert!(stmt1.is_cached());
	assert!(!QUERY_CACHE.contains("remove_this"));
}

/// Test cache state transitions through multiple operations
///
/// **Test Intent**: Verify cache consistency through add/remove/clear sequence
///
/// **Integration Point**: QueryCache → multiple operations in sequence
///
/// **Not Intent**: Concurrent access, performance metrics
#[rstest]
#[tokio::test]
async fn test_cache_state_transitions(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	reset_cache_stats();
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// Start: empty cache
	assert_eq!(QUERY_CACHE.size(), 0);

	// Add three entries
	QUERY_CACHE.set("key1".to_string(), "query1".to_string());
	assert_eq!(QUERY_CACHE.size(), 1);

	QUERY_CACHE.set("key2".to_string(), "query2".to_string());
	assert_eq!(QUERY_CACHE.size(), 2);

	QUERY_CACHE.set("key3".to_string(), "query3".to_string());
	assert_eq!(QUERY_CACHE.size(), 3);

	// Remove one
	QUERY_CACHE.remove("key2");
	assert_eq!(QUERY_CACHE.size(), 2);
	assert!(QUERY_CACHE.contains("key1"));
	assert!(QUERY_CACHE.contains("key3"));
	assert!(!QUERY_CACHE.contains("key2"));

	// Clear all
	QUERY_CACHE.clear();
	assert_eq!(QUERY_CACHE.size(), 0);
}

// ============================================================================
// Cache Consistency Tests (Property-based)
// ============================================================================

/// Test cache consistency property: stored value can be retrieved
///
/// **Test Intent**: Verify invariant that cache.set() followed by cache.get() returns same value
///
/// **Integration Point**: QueryCache::set → QueryCache::get
///
/// **Not Intent**: Performance, multiple threads
#[rstest]
#[tokio::test]
async fn test_cache_consistency_set_get_invariant(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	reset_cache_stats();
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	let test_pairs = vec![
		("users_active", "SELECT * FROM users WHERE active = true"),
		(
			"products_expensive",
			"SELECT * FROM products WHERE price > 50000",
		),
		("users_email", "SELECT id, email FROM users"),
	];

	for (key, query) in test_pairs.iter() {
		QUERY_CACHE.set(ToString::to_string(key), ToString::to_string(query));
		let retrieved = QUERY_CACHE.get(key);
		assert_eq!(retrieved, Some(ToString::to_string(query)));
	}
}

/// Test lambda statement caching with database execution
///
/// **Test Intent**: Verify lambda statements compile to valid SQL and cache properly
///
/// **Integration Point**: LambdaStmt → Database execution → CACHE_STATS
///
/// **Not Intent**: Complex SQL generation, performance analysis
#[rstest]
#[tokio::test]
async fn test_lambda_stmt_database_execution_with_cache(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	reset_cache_stats();
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	let stmt = LambdaStmt::new("select_active_users", || {
		"SELECT id, name, active FROM users WHERE active = true".to_string()
	});

	// First execution - miss
	let sql = stmt.execute().expect("Failed to execute");
	let rows = sqlx::query(&sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("Failed to fetch rows");

	assert_eq!(rows.len(), 2); // Alice and Charlie
	let initial_misses = CACHE_STATS.read().unwrap().misses;
	assert_eq!(initial_misses, 1);

	// Second execution - hit (same lambda key)
	let sql2 = stmt.execute().expect("Failed to execute second");
	assert_eq!(sql, sql2);

	let hits = CACHE_STATS.read().unwrap().hits;
	assert_eq!(hits, 1);
	assert!(stmt.is_cached());
}

/// Test SeaQuery integration with lambda caching
///
/// **Test Intent**: Verify lambda statements work with SeaQuery-built queries
///
/// **Integration Point**: SeaQuery QueryBuilder → lambda_stmt → cache
///
/// **Not Intent**: Complex SeaQuery features, performance
#[rstest]
#[tokio::test]
async fn test_seaquery_with_lambda_caching(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	reset_cache_stats();
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	let stmt = LambdaStmt::new("seaquery_users", || {
		let (query, _values) = Query::select()
			.column(Users::Id)
			.column(Users::Name)
			.from(Users::Table)
			.and_where(Expr::col(Users::Active).eq(true))
			.build(PostgresQueryBuilder);

		query
	});

	let sql = stmt.execute().expect("Failed first execution");

	// Verify it's a valid query by building it through SeaQuery again
	let (expected_query, _values) = Query::select()
		.column(Users::Id)
		.column(Users::Name)
		.from(Users::Table)
		.and_where(Expr::col(Users::Active).eq(true))
		.build(PostgresQueryBuilder);

	assert_eq!(sql, expected_query);

	// Second execution should hit cache
	let sql2 = stmt.execute().expect("Failed second execution");
	assert_eq!(sql, sql2);
	assert!(stmt.is_cached());
}
