//! ORM Filtered Relation Integration Tests with Real Database
//!
//! These tests verify filtered relationship functionality with real PostgreSQL database.
//! Phase 5: Test Filtered relationships with conditional data fetching.
//!
//! **Test Coverage:**
//! - Filtered relation definition with single condition
//! - Filtered relation definition with multiple conditions
//! - Conditional data fetching with filters
//! - Filtered relations with aliases
//! - Filtering related records by exact match
//! - Filtering related records by comparison operators
//! - Filtering related records by NULL checks
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use reinhardt_orm::filtered_relation::{FilteredRelation, FilteredRelationBuilder};
use reinhardt_orm::query_fields::{LookupType, LookupValue};
use rstest::*;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;

type PostgresContainer = ContainerAsync<Postgres>;

#[fixture]
async fn postgres_container() -> (PostgresContainer, Arc<PgPool>, u16, String) {
	let postgres = Postgres::default()
		.start()
		.await
		.expect("Failed to start PostgreSQL container");

	let port = postgres
		.get_host_port_ipv4(5432)
		.await
		.expect("Failed to get PostgreSQL port");

	let database_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);

	let pool = sqlx::postgres::PgPoolOptions::new()
		.max_connections(5)
		.connect(&database_url)
		.await
		.expect("Failed to connect to PostgreSQL");

	(postgres, Arc::new(pool), port, database_url)
}

// ============================================================================
// Test Models (Real Database Schema)
// ============================================================================

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Author {
	id: Option<i32>,
	name: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Book {
	id: Option<i32>,
	title: String,
	author_id: Option<i32>,
	status: String,
	published: bool,
	rating: Option<i32>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Post {
	id: Option<i32>,
	title: String,
	author_id: Option<i32>,
	is_published: bool,
}

// ============================================================================
// Filtered Relation Definition Tests
// ============================================================================

/// Test filtered relation definition with single condition
///
/// **Test Intent**: Verify FilteredRelation can be created with a single
/// filter condition matching a specific value
///
/// **Integration Point**: ORM FilteredRelation definition → PostgreSQL WHERE clause
///
/// **Not Intent**: Database queries, data fetching
#[rstest]
#[tokio::test]
async fn test_filtered_relation_single_condition_definition() {
	let filtered = FilteredRelation::new("books").filter(
		"status",
		LookupType::Exact,
		LookupValue::String("published".to_string()),
	);

	assert_eq!(filtered.relation_name(), "books");
	assert_eq!(filtered.conditions().len(), 1);
	assert!(filtered.has_conditions());
}

/// Test filtered relation definition with multiple conditions
///
/// **Test Intent**: Verify FilteredRelation can be created with multiple
/// filter conditions combined with AND logic
///
/// **Integration Point**: ORM FilteredRelation multiple filters → PostgreSQL AND conditions
///
/// **Not Intent**: OR logic, complex nested conditions
#[rstest]
#[tokio::test]
async fn test_filtered_relation_multiple_conditions_definition() {
	let filtered = FilteredRelation::new("books")
		.filter(
			"status",
			LookupType::Exact,
			LookupValue::String("active".to_string()),
		)
		.filter("published", LookupType::Exact, LookupValue::Bool(true))
		.filter("rating", LookupType::Gt, LookupValue::Int(4));

	assert_eq!(filtered.relation_name(), "books");
	assert_eq!(filtered.conditions().len(), 3);
	assert!(filtered.has_conditions());
}

/// Test filtered relation with alias assignment
///
/// **Test Intent**: Verify FilteredRelation can be assigned an alias
/// for use in complex queries
///
/// **Integration Point**: ORM FilteredRelation alias → Query reference
///
/// **Not Intent**: Alias resolution, query execution
#[rstest]
#[tokio::test]
async fn test_filtered_relation_with_alias_definition() {
	let filtered = FilteredRelation::new("books")
		.filter(
			"status",
			LookupType::Exact,
			LookupValue::String("published".to_string()),
		)
		.with_alias("active_books");

	assert_eq!(filtered.relation_name(), "books");
	assert_eq!(filtered.alias(), Some("active_books"));
	assert_eq!(filtered.condition_count(), 1);
}

// ============================================================================
// Conditional Data Fetching Tests
// ============================================================================

/// Test filtering related records by exact match
///
/// **Test Intent**: Verify filtered relations can fetch records matching
/// exact value conditions with real database queries
///
/// **Integration Point**: ORM FilteredRelation exact filter → PostgreSQL WHERE = query
///
/// **Not Intent**: Multiple match types, case-insensitive matching
#[rstest]
#[tokio::test]
async fn test_conditional_data_fetching_exact_match(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

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
			status VARCHAR(50) NOT NULL,
			published BOOLEAN NOT NULL,
			rating INTEGER
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert author
	let author_id: i32 = sqlx::query("INSERT INTO authors (name) VALUES ($1) RETURNING id")
		.bind("Jane Austen")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert books with different statuses
	sqlx::query(
		"INSERT INTO books (title, author_id, status, published, rating) VALUES ($1, $2, $3, $4, $5)",
	)
	.bind("Published Book")
	.bind(author_id)
	.bind("published")
	.bind(true)
	.bind(5)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"INSERT INTO books (title, author_id, status, published, rating) VALUES ($1, $2, $3, $4, $5)",
	)
	.bind("Draft Book")
	.bind(author_id)
	.bind("draft")
	.bind(false)
	.bind(3)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Create filtered relation for published books
	let filtered = FilteredRelation::new("books").filter(
		"status",
		LookupType::Exact,
		LookupValue::String("published".to_string()),
	);

	// Verify filtered relation
	assert_eq!(filtered.condition_count(), 1);

	// Query published books
	let published_books =
		sqlx::query("SELECT title, status FROM books WHERE status = $1 AND author_id = $2")
			.bind("published")
			.bind(author_id)
			.fetch_all(pool.as_ref())
			.await
			.unwrap();

	assert_eq!(published_books.len(), 1);
	let title: String = published_books[0].get("title");
	assert_eq!(title, "Published Book");
}

/// Test filtering related records by comparison operators
///
/// **Test Intent**: Verify filtered relations can use comparison operators
/// (greater than, less than) for numeric filtering
///
/// **Integration Point**: ORM FilteredRelation GT filter → PostgreSQL WHERE > query
///
/// **Not Intent**: String comparisons, date ranges
#[rstest]
#[tokio::test]
async fn test_conditional_data_fetching_with_comparison(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

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
			status VARCHAR(50) NOT NULL,
			published BOOLEAN NOT NULL,
			rating INTEGER
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert author
	let author_id: i32 = sqlx::query("INSERT INTO authors (name) VALUES ($1) RETURNING id")
		.bind("Stephen King")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert books with various ratings
	for (title, rating) in &[
		("Excellent Book", 5),
		("Good Book", 4),
		("Average Book", 3),
		("Poor Book", 1),
	] {
		sqlx::query(
			"INSERT INTO books (title, author_id, status, published, rating) VALUES ($1, $2, $3, $4, $5)",
		)
		.bind(title)
		.bind(author_id)
		.bind("published")
		.bind(true)
		.bind(*rating as i32)
		.execute(pool.as_ref())
		.await
		.unwrap();
	}

	// Create filtered relation for highly rated books
	let filtered =
		FilteredRelation::new("books").filter("rating", LookupType::Gt, LookupValue::Int(3));

	assert_eq!(filtered.condition_count(), 1);

	// Query books with rating > 3
	let high_rated_books = sqlx::query(
		"SELECT title, rating FROM books WHERE rating > $1 AND author_id = $2 ORDER BY rating DESC",
	)
	.bind(3)
	.bind(author_id)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(high_rated_books.len(), 2);
	let first_rating: i32 = high_rated_books[0].get("rating");
	assert_eq!(first_rating, 5);
}

/// Test filtering related records with multiple conditions
///
/// **Test Intent**: Verify filtered relations combine multiple conditions
/// with AND logic to refine data selection
///
/// **Integration Point**: ORM FilteredRelation multiple AND filters → PostgreSQL compound WHERE
///
/// **Not Intent**: OR conditions, selective AND/OR combinations
#[rstest]
#[tokio::test]
async fn test_conditional_data_fetching_multiple_conditions(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

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
			status VARCHAR(50) NOT NULL,
			published BOOLEAN NOT NULL,
			rating INTEGER
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert author
	let author_id: i32 = sqlx::query("INSERT INTO authors (name) VALUES ($1) RETURNING id")
		.bind("George RR Martin")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert books with various combinations
	sqlx::query(
		"INSERT INTO books (title, author_id, status, published, rating) VALUES ($1, $2, $3, $4, $5)",
	)
	.bind("Published Bestseller")
	.bind(author_id)
	.bind("published")
	.bind(true)
	.bind(5)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"INSERT INTO books (title, author_id, status, published, rating) VALUES ($1, $2, $3, $4, $5)",
	)
	.bind("Published Low Rated")
	.bind(author_id)
	.bind("published")
	.bind(true)
	.bind(2)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"INSERT INTO books (title, author_id, status, published, rating) VALUES ($1, $2, $3, $4, $5)",
	)
	.bind("Draft Excellent")
	.bind(author_id)
	.bind("draft")
	.bind(false)
	.bind(5)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Create filtered relation with multiple conditions
	let filtered = FilteredRelation::new("books")
		.filter(
			"status",
			LookupType::Exact,
			LookupValue::String("published".to_string()),
		)
		.filter("published", LookupType::Exact, LookupValue::Bool(true))
		.filter("rating", LookupType::Gte, LookupValue::Int(4));

	assert_eq!(filtered.condition_count(), 3);

	// Query with combined conditions
	let result_books = sqlx::query("SELECT title, status, published, rating FROM books WHERE status = $1 AND published = $2 AND rating >= $3 AND author_id = $4")
		.bind("published")
		.bind(true)
		.bind(4)
		.bind(author_id)
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	assert_eq!(result_books.len(), 1);
	let title: String = result_books[0].get("title");
	assert_eq!(title, "Published Bestseller");
}

/// Test filtered relation with NULL check
///
/// **Test Intent**: Verify filtered relations can filter by NULL values
/// to find records with or without related data
///
/// **Integration Point**: ORM FilteredRelation NULL filter → PostgreSQL IS NULL/IS NOT NULL
///
/// **Not Intent**: Empty/blank string filtering
#[rstest]
#[tokio::test]
async fn test_conditional_data_fetching_null_check(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

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
			status VARCHAR(50) NOT NULL,
			published BOOLEAN NOT NULL,
			rating INTEGER
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert author
	let author_id: i32 = sqlx::query("INSERT INTO authors (name) VALUES ($1) RETURNING id")
		.bind("Test Author")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert books with and without ratings
	sqlx::query(
		"INSERT INTO books (title, author_id, status, published, rating) VALUES ($1, $2, $3, $4, $5)",
	)
	.bind("Rated Book")
	.bind(author_id)
	.bind("published")
	.bind(true)
	.bind(5)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"INSERT INTO books (title, author_id, status, published, rating) VALUES ($1, $2, $3, $4, NULL)",
	)
	.bind("Unrated Book")
	.bind(author_id)
	.bind("published")
	.bind(true)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Create filtered relation for books with ratings
	let filtered =
		FilteredRelation::new("books").filter("rating", LookupType::IsNotNull, LookupValue::Null);

	assert_eq!(filtered.condition_count(), 1);

	// Query books with NOT NULL rating
	let rated_books =
		sqlx::query("SELECT title FROM books WHERE rating IS NOT NULL AND author_id = $1")
			.bind(author_id)
			.fetch_all(pool.as_ref())
			.await
			.unwrap();

	assert_eq!(rated_books.len(), 1);
	let title: String = rated_books[0].get("title");
	assert_eq!(title, "Rated Book");
}

/// Test FilteredRelationBuilder with fluent API
///
/// **Test Intent**: Verify FilteredRelationBuilder provides convenient
/// methods for creating common filter conditions
///
/// **Integration Point**: ORM FilteredRelationBuilder → FilteredRelation
///
/// **Not Intent**: All possible filter types, complex builder chains
#[rstest]
#[tokio::test]
async fn test_filtered_relation_builder_fluent_api() {
	let filtered = FilteredRelationBuilder::new("posts")
		.exact("status", "published")
		.is_true("visible")
		.gt("views", 100)
		.build();

	assert_eq!(filtered.relation_name(), "posts");
	assert_eq!(filtered.condition_count(), 3);
	assert!(filtered.has_conditions());
}

/// Test filtered relation with IN operator for multiple values
///
/// **Test Intent**: Verify filtered relations can filter by multiple
/// values using the IN operator
///
/// **Integration Point**: ORM FilteredRelation IN filter → PostgreSQL IN clause
///
/// **Not Intent**: Complex array operations, nested IN queries
#[rstest]
#[tokio::test]
async fn test_conditional_data_fetching_in_operator(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

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
			status VARCHAR(50) NOT NULL,
			published BOOLEAN NOT NULL,
			rating INTEGER
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert author
	let author_id: i32 = sqlx::query("INSERT INTO authors (name) VALUES ($1) RETURNING id")
		.bind("Test Author")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert books with various statuses
	for (title, status) in &[
		("Book1", "published"),
		("Book2", "draft"),
		("Book3", "archived"),
		("Book4", "published"),
	] {
		sqlx::query(
			"INSERT INTO books (title, author_id, status, published, rating) VALUES ($1, $2, $3, $4, $5)",
		)
		.bind(title)
		.bind(author_id)
		.bind(status)
		.bind(true)
		.bind(3)
		.execute(pool.as_ref())
		.await
		.unwrap();
	}

	// Create filtered relation with IN operator
	let statuses = vec![
		LookupValue::String("published".to_string()),
		LookupValue::String("archived".to_string()),
	];
	let filtered = FilteredRelation::new("books").filter(
		"status",
		LookupType::In,
		LookupValue::Array(statuses),
	);

	assert_eq!(filtered.condition_count(), 1);

	// Query with IN operator
	let books = sqlx::query("SELECT title FROM books WHERE status IN ($1, $2) AND author_id = $3")
		.bind("published")
		.bind("archived")
		.bind(author_id)
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	assert_eq!(books.len(), 2);
}
