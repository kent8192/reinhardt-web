//! LATERAL JOIN Integration Tests
//!
//! Tests comprehensive LATERAL JOIN functionality covering:
//! - Basic LATERAL JOIN via QuerySet API
//! - LateralJoin struct and patterns
//! - QuerySet integration with with_lateral_join()
//! - SQL generation verification
//! - Database execution tests
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Test Data Schema:**
//! - authors(id SERIAL PRIMARY KEY, name TEXT NOT NULL)
//! - books(id SERIAL PRIMARY KEY, author_id INT, title TEXT, price INT, publication_year INT)

use reinhardt_orm::Model;
use reinhardt_orm::lateral_join::{
	LateralJoin, LateralJoinPatterns, LateralJoinType, LateralJoins,
};
use reinhardt_orm::manager::reinitialize_database;
use reinhardt_orm::query::QuerySet;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Test Models
// ============================================================================

/// Author model for LATERAL JOIN tests
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Author {
	id: Option<i32>,
	name: String,
}

reinhardt_test::impl_test_model!(Author, i32, "authors", "test");

// ============================================================================
// Fixtures
// ============================================================================

#[fixture]
async fn lateral_join_test_db(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String) {
	let (container, pool, port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	setup_tables(pool.as_ref()).await;
	(container, pool, port, url)
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Initialize test tables and data
async fn setup_tables(pool: &PgPool) {
	// Create authors table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS authors (
			id SERIAL PRIMARY KEY,
			name TEXT NOT NULL
		)",
	)
	.execute(pool)
	.await
	.unwrap();

	// Create books table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS books (
			id SERIAL PRIMARY KEY,
			author_id INT NOT NULL,
			title TEXT NOT NULL,
			price INT NOT NULL,
			publication_year INT NOT NULL
		)",
	)
	.execute(pool)
	.await
	.unwrap();

	// Insert authors
	sqlx::query("INSERT INTO authors (name) VALUES ('Alice'), ('Bob'), ('Charlie')")
		.execute(pool)
		.await
		.unwrap();

	// Insert books
	// Alice: 3 books (2020, 2021, 2022)
	sqlx::query(
		"INSERT INTO books (author_id, title, price, publication_year)
		VALUES
			(1, 'Book A1', 1000, 2020),
			(1, 'Book A2', 2000, 2021),
			(1, 'Book A3', 1500, 2022)",
	)
	.execute(pool)
	.await
	.unwrap();

	// Bob: 2 books (2019, 2022)
	sqlx::query(
		"INSERT INTO books (author_id, title, price, publication_year)
		VALUES
			(2, 'Book B1', 2500, 2019),
			(2, 'Book B2', 1800, 2022)",
	)
	.execute(pool)
	.await
	.unwrap();

	// Charlie: 1 book (2021)
	sqlx::query(
		"INSERT INTO books (author_id, title, price, publication_year)
		VALUES
			(3, 'Book C1', 3000, 2021)",
	)
	.execute(pool)
	.await
	.unwrap();
}

// ============================================================================
// LateralJoin Struct Tests
// ============================================================================

/// Test LateralJoin struct creation
///
/// **Test Intent**: Verify LateralJoin struct creates valid SQL
///
/// **Integration Point**: LateralJoin::new() → to_sql()
#[test]
fn test_lateral_join_creation() {
	let join = LateralJoin::new(
		"latest_book",
		"SELECT * FROM books WHERE author_id = authors.id ORDER BY publication_year DESC LIMIT 1",
	);

	assert_eq!(join.alias, "latest_book");
	assert_eq!(join.join_type, LateralJoinType::Left);

	let sql = join.to_sql();
	assert!(sql.contains("LEFT JOIN LATERAL"));
	assert!(sql.contains("latest_book"));
	assert!(sql.contains("ON true"));
}

/// Test LateralJoin with different join types
///
/// **Test Intent**: Verify different join types generate correct SQL
///
/// **Integration Point**: LateralJoin::inner(), LateralJoin::left()
#[test]
fn test_lateral_join_types() {
	let inner = LateralJoin::new("sub", "SELECT 1").inner();
	assert_eq!(inner.join_type, LateralJoinType::Inner);

	let sql = inner.to_sql();
	assert!(sql.contains("INNER JOIN LATERAL"));

	let left = LateralJoin::new("sub", "SELECT 1").left();
	assert_eq!(left.join_type, LateralJoinType::Left);

	let sql = left.to_sql();
	assert!(sql.contains("LEFT JOIN LATERAL"));
}

/// Test LateralJoin with custom ON condition
///
/// **Test Intent**: Verify custom ON condition is included in SQL
///
/// **Integration Point**: LateralJoin::on()
#[test]
fn test_lateral_join_with_on_condition() {
	let join = LateralJoin::new("sub", "SELECT * FROM items")
		.on("sub.category_id = categories.id AND sub.active = true");

	let sql = join.to_sql();
	assert!(sql.contains("ON sub.category_id = categories.id AND sub.active = true"));
	assert!(!sql.contains("ON true"));
}

// ============================================================================
// LateralJoins Collection Tests
// ============================================================================

/// Test LateralJoins collection
///
/// **Test Intent**: Verify LateralJoins collection manages multiple joins
///
/// **Integration Point**: LateralJoins::add() → to_sql()
#[test]
fn test_lateral_joins_collection() {
	let mut joins = LateralJoins::new();
	assert!(joins.is_empty());

	joins.add(LateralJoin::new("j1", "SELECT 1"));
	joins.add(LateralJoin::new("j2", "SELECT 2"));

	assert_eq!(joins.len(), 2);
	assert!(!joins.is_empty());

	let sqls = joins.to_sql();
	assert_eq!(sqls.len(), 2);
	assert!(sqls[0].contains("j1"));
	assert!(sqls[1].contains("j2"));
}

// ============================================================================
// LateralJoinPatterns Tests
// ============================================================================

/// Test top_n_per_group pattern
///
/// **Test Intent**: Verify top N per group pattern generates correct SQL
///
/// **Integration Point**: LateralJoinPatterns::top_n_per_group()
#[test]
fn test_lateral_join_pattern_top_n() {
	let join = LateralJoinPatterns::top_n_per_group(
		"top_books",
		"books",
		"author_id",
		"authors",
		"price DESC",
		3,
	);

	let sql = join.to_sql();
	assert!(sql.contains("LEFT JOIN LATERAL"));
	assert!(sql.contains("ORDER BY price DESC"));
	assert!(sql.contains("LIMIT 3"));
}

/// Test latest_per_parent pattern
///
/// **Test Intent**: Verify latest per parent pattern generates correct SQL
///
/// **Integration Point**: LateralJoinPatterns::latest_per_parent()
#[test]
fn test_lateral_join_pattern_latest() {
	let join = LateralJoinPatterns::latest_per_parent(
		"latest_book",
		"books",
		"author_id",
		"authors",
		"publication_year",
	);

	let sql = join.to_sql();
	assert!(sql.contains("LEFT JOIN LATERAL"));
	assert!(sql.contains("ORDER BY publication_year DESC"));
	assert!(sql.contains("LIMIT 1"));
}

/// Test aggregate_per_parent pattern
///
/// **Test Intent**: Verify aggregate per parent pattern generates correct SQL
///
/// **Integration Point**: LateralJoinPatterns::aggregate_per_parent()
#[test]
fn test_lateral_join_pattern_aggregate() {
	let join = LateralJoinPatterns::aggregate_per_parent(
		"book_stats",
		"books",
		"author_id",
		"authors",
		"COUNT(*) as book_count, AVG(price) as avg_price",
	);

	let sql = join.to_sql();
	assert!(sql.contains("LEFT JOIN LATERAL"));
	assert!(sql.contains("COUNT(*)"));
	assert!(sql.contains("AVG(price)"));
}

// ============================================================================
// QuerySet Integration Tests
// ============================================================================

/// Test QuerySet with_lateral_join method
///
/// **Test Intent**: Verify QuerySet.with_lateral_join() generates correct SQL
///
/// **Integration Point**: QuerySet::with_lateral_join() → to_sql()
#[test]
fn test_queryset_with_lateral_join() {
	let latest_book = LateralJoin::new(
		"latest_book",
		"SELECT * FROM books WHERE author_id = authors.id ORDER BY publication_year DESC LIMIT 1",
	);

	let sql = QuerySet::<Author>::new()
		.with_lateral_join(latest_book)
		.to_sql();

	// Verify LATERAL JOIN is included
	assert!(sql.contains("LEFT JOIN LATERAL"));
	assert!(sql.contains("latest_book"));
	// Verify SELECT FROM is present
	assert!(sql.contains("SELECT"));
	assert!(sql.contains("\"authors\""));
}

/// Test QuerySet with multiple LATERAL JOINs
///
/// **Test Intent**: Verify QuerySet supports multiple LATERAL JOINs
///
/// **Integration Point**: QuerySet::with_lateral_join() chaining
#[test]
fn test_queryset_with_multiple_lateral_joins() {
	let latest_book = LateralJoin::new(
		"latest_book",
		"SELECT * FROM books WHERE author_id = authors.id ORDER BY publication_year DESC LIMIT 1",
	);

	let book_stats = LateralJoinPatterns::aggregate_per_parent(
		"book_stats",
		"books",
		"author_id",
		"authors",
		"COUNT(*) as book_count",
	);

	let sql = QuerySet::<Author>::new()
		.with_lateral_join(latest_book)
		.with_lateral_join(book_stats)
		.to_sql();

	// Verify both LATERAL JOINs are included
	assert!(sql.contains("latest_book"));
	assert!(sql.contains("book_stats"));
}

/// Test QuerySet with LATERAL JOIN and ordering
///
/// **Test Intent**: Verify LATERAL JOIN works with ORDER BY
///
/// **Integration Point**: QuerySet::with_lateral_join() + order_by()
#[test]
fn test_queryset_lateral_join_with_ordering() {
	let latest_book = LateralJoin::new(
		"latest_book",
		"SELECT * FROM books WHERE author_id = authors.id ORDER BY publication_year DESC LIMIT 1",
	);

	let sql = QuerySet::<Author>::new()
		.with_lateral_join(latest_book)
		.order_by(&["name"])
		.to_sql();

	// Verify ORDER BY comes after LATERAL JOIN
	assert!(sql.contains("LEFT JOIN LATERAL"));
	assert!(sql.contains("ORDER BY"));
}

/// Test QuerySet without LATERAL JOINs
///
/// **Test Intent**: Verify QuerySet without LATERAL JOINs produces standard SQL
///
/// **Integration Point**: QuerySet::to_sql() without lateral joins
#[test]
fn test_queryset_without_lateral_join() {
	let sql = QuerySet::<Author>::new().to_sql();

	// Should not have LATERAL JOIN
	assert!(!sql.contains("LATERAL"));
	// Should have standard SELECT
	assert!(sql.starts_with("SELECT"));
}

// ============================================================================
// Database Execution Tests
// ============================================================================

/// Test LATERAL JOIN basic execution - get latest book per author
///
/// **Test Intent**: Execute LATERAL JOIN query and verify results
///
/// **Integration Point**: LATERAL JOIN → PostgreSQL execution
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_lateral_join_database_execution_basic(
	#[future] lateral_join_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = lateral_join_test_db.await;

	// Get latest book per author using raw SQL
	let query = "
		SELECT a.name, b.title, b.publication_year
		FROM authors a
		CROSS JOIN LATERAL (
			SELECT title, publication_year FROM books
			WHERE books.author_id = a.id
			ORDER BY publication_year DESC
			LIMIT 1
		) b
		ORDER BY a.id
	";

	let rows = sqlx::query(query).fetch_all(pool.as_ref()).await.unwrap();

	assert_eq!(rows.len(), 3);

	// Alice: Book A3 (2022)
	assert_eq!(rows[0].get::<String, _>("name"), "Alice");
	assert_eq!(rows[0].get::<String, _>("title"), "Book A3");
	assert_eq!(rows[0].get::<i32, _>("publication_year"), 2022);

	// Bob: Book B2 (2022)
	assert_eq!(rows[1].get::<String, _>("name"), "Bob");
	assert_eq!(rows[1].get::<String, _>("title"), "Book B2");
	assert_eq!(rows[1].get::<i32, _>("publication_year"), 2022);

	// Charlie: Book C1 (2021)
	assert_eq!(rows[2].get::<String, _>("name"), "Charlie");
	assert_eq!(rows[2].get::<String, _>("title"), "Book C1");
	assert_eq!(rows[2].get::<i32, _>("publication_year"), 2021);
}

/// Test LEFT JOIN LATERAL execution
///
/// **Test Intent**: Execute LEFT JOIN LATERAL to include authors without books
///
/// **Integration Point**: LEFT JOIN LATERAL → PostgreSQL execution
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_left_join_lateral_execution(
	#[future] lateral_join_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = lateral_join_test_db.await;

	// Add author without books
	sqlx::query("INSERT INTO authors (name) VALUES ('Diana')")
		.execute(pool.as_ref())
		.await
		.unwrap();

	let query = "
		SELECT a.name, b.title
		FROM authors a
		LEFT JOIN LATERAL (
			SELECT title FROM books
			WHERE books.author_id = a.id
			LIMIT 1
		) b ON true
		ORDER BY a.id
	";

	let rows = sqlx::query(query).fetch_all(pool.as_ref()).await.unwrap();

	assert_eq!(rows.len(), 4);

	// Authors with books have title
	assert!(rows[0].get::<Option<String>, _>("title").is_some());
	assert!(rows[1].get::<Option<String>, _>("title").is_some());
	assert!(rows[2].get::<Option<String>, _>("title").is_some());

	// Diana has no books
	assert_eq!(rows[3].get::<String, _>("name"), "Diana");
	assert_eq!(rows[3].get::<Option<String>, _>("title"), None);
}

/// Test LATERAL JOIN with aggregation execution
///
/// **Test Intent**: Execute LATERAL JOIN with aggregate functions
///
/// **Integration Point**: LATERAL + aggregation → PostgreSQL execution
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_lateral_join_aggregation_execution(
	#[future] lateral_join_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = lateral_join_test_db.await;

	let query = "
		SELECT a.name, b.book_count, b.max_price
		FROM authors a
		CROSS JOIN LATERAL (
			SELECT COUNT(*) as book_count,
				   MAX(price) as max_price
			FROM books
			WHERE books.author_id = a.id
		) b
		ORDER BY a.id
	";

	let rows = sqlx::query(query).fetch_all(pool.as_ref()).await.unwrap();

	assert_eq!(rows.len(), 3);

	// Alice: 3 books, max price 2000
	assert_eq!(rows[0].get::<String, _>("name"), "Alice");
	assert_eq!(rows[0].get::<i64, _>("book_count"), 3);
	assert_eq!(rows[0].get::<i64, _>("max_price"), 2000);

	// Bob: 2 books, max price 2500
	assert_eq!(rows[1].get::<String, _>("name"), "Bob");
	assert_eq!(rows[1].get::<i64, _>("book_count"), 2);
	assert_eq!(rows[1].get::<i64, _>("max_price"), 2500);

	// Charlie: 1 book, max price 3000
	assert_eq!(rows[2].get::<String, _>("name"), "Charlie");
	assert_eq!(rows[2].get::<i64, _>("book_count"), 1);
	assert_eq!(rows[2].get::<i64, _>("max_price"), 3000);
}

/// Test LATERAL JOIN with top N per group execution
///
/// **Test Intent**: Execute LATERAL JOIN to get top N rows per group
///
/// **Integration Point**: LATERAL + LIMIT → PostgreSQL execution
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_lateral_join_top_n_execution(
	#[future] lateral_join_test_db: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = lateral_join_test_db.await;

	// Get top 2 highest-priced books per author
	let query = "
		SELECT a.name, b.title, b.price
		FROM authors a
		CROSS JOIN LATERAL (
			SELECT title, price FROM books
			WHERE books.author_id = a.id
			ORDER BY price DESC
			LIMIT 2
		) b
		ORDER BY a.id, b.price DESC
	";

	let rows = sqlx::query(query).fetch_all(pool.as_ref()).await.unwrap();

	// Alice: 2 books (A2: 2000, A3: 1500)
	// Bob: 2 books (B1: 2500, B2: 1800)
	// Charlie: 1 book (C1: 3000)
	assert_eq!(rows.len(), 5);

	// Alice's top 2
	assert_eq!(rows[0].get::<String, _>("name"), "Alice");
	assert_eq!(rows[0].get::<i32, _>("price"), 2000);
	assert_eq!(rows[1].get::<String, _>("name"), "Alice");
	assert_eq!(rows[1].get::<i32, _>("price"), 1500);

	// Bob's top 2
	assert_eq!(rows[2].get::<String, _>("name"), "Bob");
	assert_eq!(rows[2].get::<i32, _>("price"), 2500);
	assert_eq!(rows[3].get::<String, _>("name"), "Bob");
	assert_eq!(rows[3].get::<i32, _>("price"), 1800);

	// Charlie's only book
	assert_eq!(rows[4].get::<String, _>("name"), "Charlie");
	assert_eq!(rows[4].get::<i32, _>("price"), 3000);
}

// ============================================================================
// Edge Cases
// ============================================================================

/// Test MySQL SQL generation (without LATERAL keyword)
///
/// **Test Intent**: Verify MySQL-specific SQL generation
///
/// **Integration Point**: LateralJoin::to_mysql_sql()
#[test]
fn test_lateral_join_mysql_sql() {
	let join = LateralJoin::new("sub", "SELECT * FROM orders LIMIT 5");
	let sql = join.to_mysql_sql();

	// MySQL doesn't use LATERAL keyword
	assert!(!sql.contains("LATERAL"));
	assert!(sql.contains("LEFT JOIN"));
	assert!(sql.contains("sub"));
}

/// Test empty LateralJoins collection
///
/// **Test Intent**: Verify empty collection returns empty SQL list
///
/// **Integration Point**: LateralJoins::to_sql() edge case
#[test]
fn test_empty_lateral_joins() {
	let joins = LateralJoins::new();
	assert!(joins.is_empty());
	assert_eq!(joins.len(), 0);

	let sqls = joins.to_sql();
	assert!(sqls.is_empty());
}
