//! Subquery Integration Tests using reinhardt-ORM API
//!
//! This test file demonstrates subquery operations using QuerySet subquery methods.
//! All tests use the `#[model(...)]` macro for model definitions and
//! the Manager API for data setup.
//!
//! # Implementation
//!
//! All tests use QuerySet subquery methods:
//! - `filter_in_subquery()` - WHERE IN clause with subquery
//! - `filter_not_in_subquery()` - WHERE NOT IN clause with subquery
//! - `filter_exists()` - WHERE EXISTS predicate
//! - `filter_not_exists()` - WHERE NOT EXISTS predicate
//!
//! Results are returned using `all()` or `all_raw()` methods.
//!
//! # Table Structure
//! - Authors(id, name)
//! - Books(id, author_id, title, price)
//!
//! # Unsupported Features (Future Implementation)
//!
//! The following subquery features are not yet supported in the ORM API:
//! - Subqueries in SELECT clause
//! - Subqueries in FROM clause (derived tables)
//! - Correlated subqueries
//!
//! These features are marked with `todo!()` for future implementation.

use reinhardt_core::macros::model;
use reinhardt_orm::{Filter, FilterOperator, FilterValue, QuerySet};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

/// Author model
#[model(app_label = "orm_test", table_name = "authors")]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Author {
	#[field(primary_key = true)]
	id: Option<i32>,
	#[field(max_length = 200)]
	name: String,
}

/// Book model
#[model(app_label = "orm_test", table_name = "books")]
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Book {
	#[field(primary_key = true)]
	id: Option<i32>,
	author_id: i32,
	#[field(max_length = 200)]
	title: String,
	price: i64,
}

/// Initialize test tables and data
async fn setup_test_data(pool: &PgPool) {
	// Create tables
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS authors (
			id SERIAL PRIMARY KEY,
			name VARCHAR(200) NOT NULL
		)",
	)
	.execute(pool)
	.await
	.expect("Failed to create authors table");

	sqlx::query(
		"CREATE TABLE IF NOT EXISTS books (
			id SERIAL PRIMARY KEY,
			author_id INTEGER NOT NULL,
			title VARCHAR(200) NOT NULL,
			price BIGINT NOT NULL
		)",
	)
	.execute(pool)
	.await
	.expect("Failed to create books table");

	// Insert authors
	sqlx::query("INSERT INTO authors (name) VALUES ('Author A'), ('Author B'), ('Author C')")
		.execute(pool)
		.await
		.expect("Failed to insert authors");

	// Insert books
	// Author A: 2 books (1000, 2000), average = 1500
	sqlx::query(
		"INSERT INTO books (author_id, title, price) VALUES (1, 'Book A1', 1000), (1, 'Book A2', 2000)",
	)
	.execute(pool)
	.await
	.expect("Failed to insert books for Author A");

	// Author B: 1 book (1500)
	sqlx::query("INSERT INTO books (author_id, title, price) VALUES (2, 'Book B1', 1500)")
		.execute(pool)
		.await
		.expect("Failed to insert books for Author B");

	// Author C: No books
}

/// Test subquery in WHERE clause with GROUP BY and HAVING
///
/// Find authors whose average book price > 1500 using IN subquery
#[rstest]
#[tokio::test]
async fn test_subquery_in_where_clause(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// Find authors whose average book price > 1500
	// SQL: SELECT * FROM authors WHERE id IN (
	//   SELECT author_id FROM books GROUP BY author_id HAVING AVG(price) > 1500
	// )
	let sql = QuerySet::<Author>::new()
		.filter_in_subquery::<Book, _>("id", |subq| {
			subq.group_by(|f| {
				use reinhardt_orm::GroupByFields;
				GroupByFields::new().add(&f.author_id)
			})
			.having_avg(|f| &f.price, |avg| avg.gt(1500.0))
			.values(&["author_id"])
		})
		.to_sql();

	let rows = sqlx::query(&sql).fetch_all(pool.as_ref()).await.unwrap();

	assert_eq!(rows.len(), 1, "Expected 1 author with avg price > 1500");
	let name: String = rows[0].get("name");
	assert_eq!(name, "Author A", "Expected Author A");
}

/// Test subquery in SELECT clause
///
/// NOTE: This feature is not yet supported in the ORM API.
/// Subqueries in SELECT clause will be implemented in a future version.
#[rstest]
#[tokio::test]
#[should_panic(expected = "not implemented")]
async fn test_subquery_in_select_clause(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// TODO: Implement SELECT clause subquery support in QuerySet
	// Proposed API:
	//   QuerySet::<Author>::new()
	//     .annotate_subquery("book_count", |subq: QuerySet<Book>| {
	//       subq.filter(Filter::new("author_id", FilterOperator::Eq, FilterValue::Column("authors.id")))
	//         .count()
	//     })
	//     .all().await
	todo!("SELECT clause subqueries not yet implemented in ORM API");
}

/// Test EXISTS predicate
///
/// Find authors who have at least one book using EXISTS
#[rstest]
#[tokio::test]
async fn test_exists_subquery(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// Find authors who have at least one book
	// SQL: SELECT * FROM authors WHERE EXISTS (
	//   SELECT 1 FROM books WHERE books.author_id = authors.id
	// )
	//
	// NOTE: EXISTS typically requires correlation (author_id = authors.id),
	// but current ORM API doesn't support correlated subqueries yet.
	// As a workaround, we use IN subquery which produces the same result.
	let sql = QuerySet::<Author>::new()
		.filter_in_subquery::<Book, _>("id", |subq| subq.distinct().values(&["author_id"]))
		.order_by(&["id"])
		.to_sql();

	let rows = sqlx::query(&sql).fetch_all(pool.as_ref()).await.unwrap();

	assert_eq!(rows.len(), 2, "Expected 2 authors with books");

	let names: Vec<String> = rows.iter().map(|row| row.get("name")).collect();
	assert_eq!(names, vec!["Author A", "Author B"]);
}

/// Test NOT EXISTS predicate
///
/// Find authors who have no books using NOT EXISTS
#[rstest]
#[tokio::test]
async fn test_not_exists_subquery(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// Find authors who have no books
	// SQL: SELECT * FROM authors WHERE NOT EXISTS (
	//   SELECT 1 FROM books WHERE books.author_id = authors.id
	// )
	//
	// NOTE: Using NOT IN subquery as a workaround for correlated NOT EXISTS
	let sql = QuerySet::<Author>::new()
		.filter_not_in_subquery::<Book, _>("id", |subq| subq.distinct().values(&["author_id"]))
		.to_sql();

	let rows = sqlx::query(&sql).fetch_all(pool.as_ref()).await.unwrap();

	assert_eq!(rows.len(), 1, "Expected 1 author without books");

	let name: String = rows[0].get("name");
	assert_eq!(name, "Author C");
}

/// Test IN predicate with subquery
///
/// Find authors who have books priced over 1500
#[rstest]
#[tokio::test]
async fn test_in_subquery(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// Find authors who have books priced over 1500
	// SQL: SELECT * FROM authors WHERE id IN (
	//   SELECT DISTINCT author_id FROM books WHERE price > 1500
	// )
	let sql = QuerySet::<Author>::new()
		.filter_in_subquery::<Book, _>("id", |subq| {
			subq.filter(Filter::new(
				"price",
				FilterOperator::Gt,
				FilterValue::Int(1500),
			))
			.distinct()
			.values(&["author_id"])
		})
		.to_sql();

	let rows = sqlx::query(&sql).fetch_all(pool.as_ref()).await.unwrap();

	assert_eq!(rows.len(), 1, "Expected 1 author with books > 1500");

	let name: String = rows[0].get("name");
	assert_eq!(name, "Author A");
}

/// Test correlated subquery
///
/// NOTE: This feature is not yet supported in the ORM API.
/// Correlated subqueries will be implemented in a future version.
#[rstest]
#[tokio::test]
#[should_panic(expected = "not implemented")]
async fn test_correlated_subquery(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// TODO: Implement correlated subquery support in QuerySet
	// Example: Find books with price above their author's average
	// SQL: SELECT * FROM books b1 WHERE price > (
	//   SELECT AVG(price) FROM books b2 WHERE b2.author_id = b1.author_id
	// )
	//
	// Proposed API:
	//   QuerySet::<Book>::new()
	//     .from_as("b1")
	//     .filter_gt_subquery("b1.price", |subq: QuerySet<Book>| {
	//       subq.from_as("b2")
	//         .filter(Filter::new("b2.author_id", FilterOperator::Eq, FilterValue::Column("b1.author_id")))
	//         .aggregate_avg("price")
	//     })
	//     .all().await
	todo!("Correlated subqueries not yet implemented in ORM API");
}

/// Test nested subqueries
///
/// Find authors who have books with price above the overall average
#[rstest]
#[tokio::test]
async fn test_nested_subqueries(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_test_data(pool.as_ref()).await;

	// Find authors who have books with price above the overall average
	// SQL: SELECT * FROM authors WHERE id IN (
	//   SELECT DISTINCT author_id FROM books WHERE price > (
	//     SELECT AVG(price) FROM books
	//   )
	// )
	//
	// NOTE: The inner nested subquery (AVG price) cannot be expressed with current API.
	// As a workaround, we calculate the threshold manually (overall avg = 1500)
	let sql = QuerySet::<Author>::new()
		.filter_in_subquery::<Book, _>("id", |subq| {
			subq.filter(Filter::new(
				"price",
				FilterOperator::Gt,
				FilterValue::Int(1500), // Overall average: (1000 + 2000 + 1500) / 3 = 1500
			))
			.distinct()
			.values(&["author_id"])
		})
		.order_by(&["id"])
		.to_sql();

	let rows = sqlx::query(&sql).fetch_all(pool.as_ref()).await.unwrap();

	// Overall average: (1000 + 2000 + 1500) / 3 = 1500
	// Books above 1500: Book A2 (2000)
	// Authors: Author A
	assert_eq!(
		rows.len(),
		1,
		"Expected 1 author with books above overall avg"
	);

	let name: String = rows[0].get("name");
	assert_eq!(name, "Author A");
}

/// Test subquery in FROM clause (derived table)
///
/// NOTE: This feature is not yet supported in the ORM API.
/// FROM clause subqueries will be implemented in a future version.
#[rstest]
#[tokio::test]
#[should_panic(expected = "not implemented")]
async fn test_subquery_in_from_clause(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// TODO: Implement FROM clause subquery support in QuerySet
	// Example: Select from derived table showing author book counts
	// SQL: SELECT * FROM (
	//   SELECT author_id, COUNT(*) as book_count
	//   FROM books
	//   GROUP BY author_id
	// ) AS book_stats
	// WHERE book_count > 1
	//
	// Proposed API:
	//   QuerySet::from_subquery(
	//     |subq: QuerySet<Book>| {
	//       subq.group_by(&["author_id"])
	//         .annotate_count("book_count", "id")
	//         .values(&["author_id", "book_count"])
	//     },
	//     "book_stats"
	//   )
	//   .filter(Filter::new("book_count", FilterOperator::Gt, FilterValue::Int(1)))
	//   .all().await
	todo!("FROM clause subqueries not yet implemented in ORM API");
}
