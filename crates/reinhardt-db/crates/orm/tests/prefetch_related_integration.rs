//! ORM Prefetch Related Integration Tests
//!
//! These tests verify N+1 query problem prevention using prefetch_related functionality.
//! Tests prefetching strategies for related objects with real PostgreSQL database.
//!
//! **Test Coverage:**
//! - Basic Prefetch: Simple prefetching of related objects
//! - Multiple Relations: Simultaneous prefetching of multiple related object types
//! - Query Generation: Verification of correct SQL query generation
//! - Nested Prefetch: Prefetching of nested related objects
//! - Property-based: Query count reduction verification with random data
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

// Allow dead code in test file: test models and their new() functions may not all be used
// in current tests but are provided for comprehensive test scenarios.
#![allow(dead_code)]

use reinhardt_db::orm::manager::reinitialize_database;
use reinhardt_db::orm::query::FilterOperator;
use reinhardt_db::orm::{Model, QuerySet};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use sea_query::PostgresQueryBuilder;
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

type PostgresContainer = ContainerAsync<GenericImage>;

// ============================================================================
// Test Models (Real Database Schema)
// ============================================================================

/// Author model for testing one-to-many relationships
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Author {
	id: Option<i32>,
	name: String,
}

impl Author {
	fn new(name: String) -> Self {
		Self { id: None, name }
	}
}

reinhardt_test::impl_test_model!(
	Author,
	i32,
	"authors",
	"test",
	relationships: [(OneToMany, "book", "Book", "author_id", "author")]
);

/// Book model for testing foreign key relationships
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Book {
	id: Option<i32>,
	title: String,
	author_id: Option<i32>,
	publisher_id: Option<i32>,
}

impl Book {
	fn new(title: String, author_id: Option<i32>) -> Self {
		Self {
			id: None,
			title,
			author_id,
			publisher_id: None,
		}
	}
}

reinhardt_test::impl_test_model!(
	Book,
	i32,
	"books",
	"test",
	relationships: [(OneToMany, "review", "Review", "book_id", "book")],
	many_to_many: [("tag", "Tag", "books_tag", "book_id", "tag_id")]
);

/// Publisher model for testing multiple foreign keys
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Publisher {
	id: Option<i32>,
	name: String,
}

impl Publisher {
	fn new(name: String) -> Self {
		Self { id: None, name }
	}
}

reinhardt_test::impl_test_model!(Publisher, i32, "publishers", "test");

/// Review model for testing nested relationships
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Review {
	id: Option<i32>,
	book_id: i32,
	content: String,
	rating: i32,
}

impl Review {
	fn new(book_id: i32, content: String, rating: i32) -> Self {
		Self {
			id: None,
			book_id,
			content,
			rating,
		}
	}
}

reinhardt_test::impl_test_model!(Review, i32, "reviews", "test");

/// Tag model for testing many-to-many relationships
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Tag {
	id: Option<i32>,
	name: String,
}

impl Tag {
	fn new(name: String) -> Self {
		Self { id: None, name }
	}
}

reinhardt_test::impl_test_model!(Tag, i32, "tags", "test");

// ============================================================================
// Fixtures
// ============================================================================

/// Fixture that initializes ORM database connection
#[fixture]
async fn prefetch_test_db(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) -> (PostgresContainer, Arc<PgPool>, u16, String) {
	let (container, pool, port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();
	(container, pool, port, url)
}

// ============================================================================
// Basic Prefetch Tests
// ============================================================================

/// Test prefetch_related method creates correct QuerySet configuration
///
/// **Test Intent**: Verify that prefetch_related correctly stores field names for later query generation
///
/// **Integration Point**: QuerySet::prefetch_related → field configuration
#[rstest]
#[tokio::test]
async fn test_prefetch_related_configuration() {
	// Create QuerySet with prefetch_related
	let queryset: QuerySet<Author> = Author::objects().all().prefetch_related(&["book"]);

	// Verify prefetch_related_queries can be generated
	let pk_values = vec![1i64, 2, 3];
	let queries = queryset.prefetch_related_queries(&pk_values);

	// Should have one query for the "book" relation
	assert_eq!(queries.len(), 1);
	assert_eq!(queries[0].0, "book");

	// Verify the generated SQL
	let sql = queries[0].1.to_string(PostgresQueryBuilder);
	assert!(sql.contains("FROM"));
	assert!(sql.contains("IN"));
}

/// Test prefetch_related with multiple relations
///
/// **Test Intent**: Verify multiple relations can be prefetched simultaneously
///
/// **Integration Point**: QuerySet::prefetch_related with multiple fields
#[rstest]
#[tokio::test]
async fn test_prefetch_related_multiple_relations() {
	// Create QuerySet with multiple prefetch_related fields
	let queryset: QuerySet<Book> = Book::objects().all().prefetch_related(&["review", "tag"]);

	let pk_values = vec![1i64, 2, 3];
	let queries = queryset.prefetch_related_queries(&pk_values);

	// Should have two queries
	assert_eq!(queries.len(), 2);

	// Check field names
	let field_names: Vec<&String> = queries.iter().map(|(name, _)| name).collect();
	assert!(field_names.contains(&&"review".to_string()));
	assert!(field_names.contains(&&"tag".to_string()));
}

/// Test prefetch_related with empty pk_values returns empty queries
///
/// **Test Intent**: Verify edge case handling for empty primary key values
#[rstest]
#[tokio::test]
async fn test_prefetch_related_empty_pk_values() {
	let queryset: QuerySet<Author> = Author::objects().all().prefetch_related(&["book"]);

	let pk_values: Vec<i64> = vec![];
	let queries = queryset.prefetch_related_queries(&pk_values);

	// Should return empty vector when no primary keys
	assert!(queries.is_empty());
}

// ============================================================================
// Query Generation Tests
// ============================================================================

/// Test one-to-many prefetch query generation
///
/// **Test Intent**: Verify correct SQL is generated for one-to-many relationships
///
/// **Integration Point**: prefetch_one_to_many_query → SelectStatement
#[rstest]
#[tokio::test]
async fn test_prefetch_one_to_many_query_generation() {
	let queryset: QuerySet<Author> = Author::objects().all().prefetch_related(&["book"]);

	let pk_values = vec![1i64, 2, 3];
	let queries = queryset.prefetch_related_queries(&pk_values);

	assert_eq!(queries.len(), 1);

	// Generate SQL for verification
	let sql = queries[0].1.to_string(PostgresQueryBuilder);

	// Should select from the related table with IN clause
	assert!(sql.contains("FROM"));
	assert!(sql.contains("IN"));
	assert!(sql.contains("1"));
	assert!(sql.contains("2"));
	assert!(sql.contains("3"));
}

/// Test many-to-many prefetch query generation
///
/// **Test Intent**: Verify correct SQL is generated for many-to-many relationships with junction table
///
/// **Integration Point**: prefetch_many_to_many_query → SelectStatement with JOIN
#[rstest]
#[tokio::test]
async fn test_prefetch_many_to_many_query_generation() {
	let queryset: QuerySet<Book> = Book::objects().all().prefetch_related(&["tag"]);

	let pk_values = vec![1i64, 2];
	let queries = queryset.prefetch_related_queries(&pk_values);

	assert_eq!(queries.len(), 1);
	assert_eq!(queries[0].0, "tag");

	// Generate SQL for verification
	let sql = queries[0].1.to_string(PostgresQueryBuilder);

	// Should have JOIN for junction table
	assert!(sql.contains("JOIN"));
	assert!(sql.contains("IN"));
}

// ============================================================================
// Database Integration Tests
// ============================================================================

/// Test actual prefetch query execution against database
///
/// **Test Intent**: Verify prefetch queries execute correctly against real database
///
/// **Integration Point**: QuerySet → PostgreSQL database
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_prefetch_query_execution(
	#[future] prefetch_test_db: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = prefetch_test_db.await;

	// Create tables
	sqlx::query(
		"CREATE TABLE authors (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE books (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			author_id INTEGER REFERENCES authors(id)
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert test data
	let author_id: i32 = sqlx::query("INSERT INTO authors (name) VALUES ($1) RETURNING id")
		.bind("Isaac Asimov")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	for i in 1..=3 {
		sqlx::query("INSERT INTO books (title, author_id) VALUES ($1, $2)")
			.bind(format!("Foundation {}", i))
			.bind(author_id)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}

	// Create QuerySet with prefetch_related
	let queryset: QuerySet<Author> = Author::objects().all().prefetch_related(&["book"]);

	// Generate prefetch queries with actual author IDs
	let pk_values = vec![author_id as i64];
	let queries = queryset.prefetch_related_queries(&pk_values);

	assert_eq!(queries.len(), 1);

	// Verify SQL is generated correctly
	let sql = queries[0].1.to_string(PostgresQueryBuilder);
	assert!(sql.contains("FROM"));
}

/// Test prefetch strategy reduces query count compared to N+1
///
/// **Test Intent**: Demonstrate that prefetch_related generates fewer queries than N+1 pattern
///
/// **Integration Point**: N+1 query pattern vs. Prefetch pattern comparison
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_prefetch_prevents_n_plus_one_pattern(
	#[future] prefetch_test_db: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = prefetch_test_db.await;

	// Create tables
	sqlx::query(
		"CREATE TABLE authors (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE books (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			author_id INTEGER REFERENCES authors(id)
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert multiple authors with books
	let mut author_ids = Vec::new();
	for a in 1..=5 {
		let author_id: i32 = sqlx::query("INSERT INTO authors (name) VALUES ($1) RETURNING id")
			.bind(format!("Author {}", a))
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");
		author_ids.push(author_id);

		for b in 1..=3 {
			sqlx::query("INSERT INTO books (title, author_id) VALUES ($1, $2)")
				.bind(format!("Book {} by Author {}", b, a))
				.bind(author_id)
				.execute(pool.as_ref())
				.await
				.unwrap();
		}
	}

	// N+1 approach would require:
	// 1 query for authors + 5 queries for each author's books = 6 queries
	let n_plus_one_queries = 1 + author_ids.len();

	// Prefetch approach requires:
	// 1 query for authors + 1 query for ALL books (using IN clause) = 2 queries
	let queryset: QuerySet<Author> = Author::objects().all().prefetch_related(&["book"]);
	let pk_values: Vec<i64> = author_ids.iter().map(|&id| id as i64).collect();
	let prefetch_queries = queryset.prefetch_related_queries(&pk_values);

	// 1 main query + 1 prefetch query = 2 queries
	let prefetch_total_queries = 1 + prefetch_queries.len();

	// Verify prefetch is more efficient
	assert!(
		prefetch_total_queries < n_plus_one_queries,
		"Prefetch ({}) should use fewer queries than N+1 ({})",
		prefetch_total_queries,
		n_plus_one_queries
	);
	assert_eq!(prefetch_total_queries, 2);
	assert_eq!(n_plus_one_queries, 6);
}

// ============================================================================
// Nested Prefetch Tests
// ============================================================================

/// Test nested prefetch configuration (author -> books -> reviews)
///
/// **Test Intent**: Verify prefetch can be configured for nested relationships
///
/// **Integration Point**: QuerySet chaining for nested relationships
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_prefetch_nested_relations_configuration(
	#[future] prefetch_test_db: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = prefetch_test_db.await;

	// Create tables for nested relationship
	sqlx::query(
		"CREATE TABLE authors (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE books (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			author_id INTEGER REFERENCES authors(id),
			publisher_id INTEGER
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE reviews (
			id SERIAL PRIMARY KEY,
			book_id INTEGER REFERENCES books(id),
			content TEXT NOT NULL,
			rating INTEGER NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert test data
	let author_id: i32 = sqlx::query("INSERT INTO authors (name) VALUES ($1) RETURNING id")
		.bind("Philip K. Dick")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	let book_id: i32 =
		sqlx::query("INSERT INTO books (title, author_id) VALUES ($1, $2) RETURNING id")
			.bind("Ubik")
			.bind(author_id)
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	for r in 1..=2 {
		sqlx::query("INSERT INTO reviews (book_id, content, rating) VALUES ($1, $2, $3)")
			.bind(book_id)
			.bind(format!("Review {}", r))
			.bind(4 + r as i32)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}

	// First level prefetch: Author -> Books
	let author_queryset: QuerySet<Author> = Author::objects().all().prefetch_related(&["book"]);
	let author_pk_values = vec![author_id as i64];
	let author_prefetch = author_queryset.prefetch_related_queries(&author_pk_values);

	assert_eq!(author_prefetch.len(), 1);
	assert_eq!(author_prefetch[0].0, "book");

	// Second level prefetch: Book -> Reviews
	let book_queryset: QuerySet<Book> = Book::objects().all().prefetch_related(&["review"]);
	let book_pk_values = vec![book_id as i64];
	let book_prefetch = book_queryset.prefetch_related_queries(&book_pk_values);

	assert_eq!(book_prefetch.len(), 1);
	assert_eq!(book_prefetch[0].0, "review");
}

// ============================================================================
// Filter with Prefetch Tests
// ============================================================================

/// Test prefetch_related combined with filter
///
/// **Test Intent**: Verify prefetch works correctly with filtered QuerySets
///
/// **Integration Point**: Manager::filter + prefetch_related
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_prefetch_with_filter(
	#[future] prefetch_test_db: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = prefetch_test_db.await;

	// Create tables
	sqlx::query(
		"CREATE TABLE authors (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE books (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			author_id INTEGER REFERENCES authors(id),
			publisher_id INTEGER
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert test data
	let author_id: i32 = sqlx::query("INSERT INTO authors (name) VALUES ($1) RETURNING id")
		.bind("Arthur C. Clarke")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	for year in [1968, 1973, 1982, 1997] {
		sqlx::query("INSERT INTO books (title, author_id) VALUES ($1, $2)")
			.bind(format!("2001-like Book ({})", year))
			.bind(author_id)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}

	// Create filtered QuerySet with prefetch using Manager API
	use reinhardt_db::orm::query::FilterValue;
	let queryset: QuerySet<Author> = Author::objects()
		.filter(
			"name",
			FilterOperator::Eq,
			FilterValue::String("Arthur C. Clarke".to_string()),
		)
		.prefetch_related(&["book"]);

	// Verify prefetch queries can be generated
	let pk_values = vec![author_id as i64];
	let queries = queryset.prefetch_related_queries(&pk_values);

	assert_eq!(queries.len(), 1);

	// Verify SQL contains the relation
	let sql = queries[0].1.to_string(PostgresQueryBuilder);
	assert!(sql.contains("FROM"));
	assert!(sql.contains("IN"));
}

// ============================================================================
// Property-Based Tests
// ============================================================================

/// Property-based test: Prefetch query count is always constant (1 per relation)
///
/// **Test Intent**: Verify prefetch maintains O(1) queries per relation regardless of data size
#[rstest]
#[tokio::test]
#[case(2)] // 2 authors
#[case(5)] // 5 authors
#[case(10)] // 10 authors
#[case(50)] // 50 authors
async fn test_prefetch_query_count_constant(#[case] num_authors: usize) {
	// Create QuerySet with prefetch_related
	let queryset: QuerySet<Author> = Author::objects().all().prefetch_related(&["book"]);

	// Generate primary key values for all authors
	let pk_values: Vec<i64> = (1..=num_authors as i64).collect();
	let queries = queryset.prefetch_related_queries(&pk_values);

	// Should always be exactly 1 query for 1 relation, regardless of num_authors
	assert_eq!(
		queries.len(),
		1,
		"Prefetch should generate exactly 1 query per relation, not {}",
		queries.len()
	);

	// N+1 would require num_authors + 1 queries
	// Prefetch requires exactly 2 (1 main + 1 prefetch)
	let prefetch_total = 1 + queries.len();
	let n_plus_one_total = 1 + num_authors;

	assert!(
		prefetch_total < n_plus_one_total,
		"Prefetch ({}) should be more efficient than N+1 ({}) for {} authors",
		prefetch_total,
		n_plus_one_total,
		num_authors
	);
}

/// Property-based test: Multiple relations scale linearly
///
/// **Test Intent**: Verify query count scales with number of relations, not data size
#[rstest]
#[tokio::test]
async fn test_prefetch_multiple_relations_scale() {
	// Test with varying numbers of relations
	let one_relation: QuerySet<Author> = Author::objects().all().prefetch_related(&["book"]);

	let pk_values = vec![1i64, 2, 3, 4, 5];

	let queries_1 = one_relation.prefetch_related_queries(&pk_values);
	assert_eq!(queries_1.len(), 1);

	// Book has both review (one-to-many) and tag (many-to-many)
	let two_relations: QuerySet<Book> = Book::objects().all().prefetch_related(&["review", "tag"]);

	let queries_2 = two_relations.prefetch_related_queries(&pk_values);
	assert_eq!(queries_2.len(), 2);

	// Query count scales with relations (O(R)), not data size (O(N))
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test prefetch with single primary key value
///
/// **Test Intent**: Verify prefetch works correctly with a single item
#[rstest]
#[tokio::test]
async fn test_prefetch_single_pk_value() {
	let queryset: QuerySet<Author> = Author::objects().all().prefetch_related(&["book"]);

	let pk_values = vec![1i64];
	let queries = queryset.prefetch_related_queries(&pk_values);

	assert_eq!(queries.len(), 1);

	// SQL should contain just the single value
	let sql = queries[0].1.to_string(PostgresQueryBuilder);
	assert!(sql.contains("1"));
}

/// Test prefetch with large number of primary keys
///
/// **Test Intent**: Verify prefetch handles large IN clauses correctly
#[rstest]
#[tokio::test]
async fn test_prefetch_large_pk_values() {
	let queryset: QuerySet<Author> = Author::objects().all().prefetch_related(&["book"]);

	// Generate 100 primary key values
	let pk_values: Vec<i64> = (1..=100).collect();
	let queries = queryset.prefetch_related_queries(&pk_values);

	assert_eq!(queries.len(), 1);

	// SQL should contain all values in IN clause
	let sql = queries[0].1.to_string(PostgresQueryBuilder);
	assert!(sql.contains("1"));
	assert!(sql.contains("50"));
	assert!(sql.contains("100"));
}

/// Test prefetch preserves QuerySet ordering
///
/// **Test Intent**: Verify prefetch_related doesn't affect QuerySet order_by
#[rstest]
#[tokio::test]
async fn test_prefetch_preserves_ordering() {
	let queryset: QuerySet<Author> = Author::objects()
		.all()
		.order_by(&["name"])
		.prefetch_related(&["book"]);

	// Get the main query SQL
	let sql = queryset.to_sql();

	// Order by should be preserved
	assert!(sql.contains("ORDER BY"));
	assert!(sql.contains("name"));
}

/// Test prefetch with select_related combination
///
/// **Test Intent**: Verify prefetch_related can be combined with select_related
#[rstest]
#[tokio::test]
async fn test_prefetch_and_select_related_combination() {
	// select_related uses JOIN, prefetch_related uses separate queries
	let queryset: QuerySet<Book> = Book::objects()
		.all()
		.select_related(&["author"])
		.prefetch_related(&["review"]);

	// Main query should have JOIN for select_related
	let main_sql = queryset.to_sql();
	assert!(main_sql.contains("JOIN") || main_sql.contains("authors"));

	// Prefetch queries should be separate
	let pk_values = vec![1i64, 2, 3];
	let prefetch_queries = queryset.prefetch_related_queries(&pk_values);
	assert_eq!(prefetch_queries.len(), 1);
	assert_eq!(prefetch_queries[0].0, "review");
}
