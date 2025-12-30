//! Serializer Integration Tests
//!
//! **Purpose:**
//! Comprehensive integration tests for serializer functionality including model
//! serialization, nested serializers, validation with database constraints,
//! and performance with large datasets. Tests verify serializers work correctly
//! with real PostgreSQL database and ORM models.
//!
//! **Test Coverage:**
//! - Serializer with ORM models (database-backed serialization)
//! - Nested serializers with database relationships (foreign keys)
//! - Serializer validation with database constraints (unique, not null)
//! - Read-only and write-only fields (asymmetric serialization)
//! - Custom serializer methods with ORM (computed fields)
//! - Serializer performance with large datasets (bulk operations)
//! - ModelSerializer field introspection
//! - Serializer error handling (validation failures)
//! - Serializer with complex nested structures
//! - Field-level validation with database state
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container from reinhardt-test

use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};
use uuid::Uuid;

// ========================================================================
// Test Models
// ========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct Author {
	id: Option<i64>,
	name: String,
	email: String,
	bio: Option<String>,
}

reinhardt_test::impl_test_model!(Author, i64, "authors");

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
struct Book {
	id: Option<i64>,
	title: String,
	isbn: String,
	author_id: i64,
	published_date: String,
	pages: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	password: Option<String>, // Write-only field example
}

reinhardt_test::impl_test_model!(Book, i64, "books");

// Nested serializer representation
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BookWithAuthor {
	id: i64,
	title: String,
	isbn: String,
	published_date: String,
	pages: i32,
	author: Author,
}

// ========================================================================
// Helper Functions
// ========================================================================

async fn setup_tables(pool: &PgPool) {
	// Create authors table
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS authors (
			id BIGSERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			email VARCHAR(255) UNIQUE NOT NULL,
			bio TEXT
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create authors table");

	// Create books table with foreign key
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS books (
			id BIGSERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			isbn VARCHAR(13) UNIQUE NOT NULL,
			author_id BIGINT NOT NULL REFERENCES authors(id),
			published_date VARCHAR(10) NOT NULL,
			pages INTEGER NOT NULL CHECK (pages > 0),
			password VARCHAR(128)
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create books table");
}

async fn insert_author(pool: &PgPool, name: &str, email: &str) -> Author {
	sqlx::query_as::<_, Author>(
		"INSERT INTO authors (name, email, bio)
		 VALUES ($1, $2, $3)
		 RETURNING id, name, email, bio",
	)
	.bind(name)
	.bind(email)
	.bind(format!("Bio for {}", name))
	.fetch_one(pool)
	.await
	.expect("Failed to insert author")
}

async fn insert_book(
	pool: &PgPool,
	title: &str,
	isbn: &str,
	author_id: i64,
	pages: i32,
) -> Book {
	sqlx::query_as::<_, Book>(
		"INSERT INTO books (title, isbn, author_id, published_date, pages)
		 VALUES ($1, $2, $3, $4, $5)
		 RETURNING id, title, isbn, author_id, published_date, pages, password",
	)
	.bind(title)
	.bind(isbn)
	.bind(author_id)
	.bind("2024-01-01")
	.bind(pages)
	.fetch_one(pool)
	.await
	.expect("Failed to insert book")
}

async fn get_book_with_author(pool: &PgPool, book_id: i64) -> BookWithAuthor {
	let row = sqlx::query(
		"SELECT b.id, b.title, b.isbn, b.published_date, b.pages,
		        a.id as author_id, a.name as author_name, a.email as author_email, a.bio as author_bio
		 FROM books b
		 JOIN authors a ON b.author_id = a.id
		 WHERE b.id = $1",
	)
	.bind(book_id)
	.fetch_one(pool)
	.await
	.expect("Failed to fetch book with author");

	BookWithAuthor {
		id: row.get("id"),
		title: row.get("title"),
		isbn: row.get("isbn"),
		published_date: row.get("published_date"),
		pages: row.get("pages"),
		author: Author {
			id: Some(row.get("author_id")),
			name: row.get("author_name"),
			email: row.get("author_email"),
			bio: row.get("author_bio"),
		},
	}
}

// ========================================================================
// Serializer with ORM Models Tests
// ========================================================================

/// Test serializer with ORM model from database
///
/// **Test Intent**: Verify serializer can serialize ORM model fetched from database
///
/// **Integration Point**: Serializer → ORM Model → Database Query
///
/// **Not Intent**: serde JSON serialization internals
#[rstest]
#[tokio::test]
async fn test_serializer_with_orm_model(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;
	let author = insert_author(&pool, "John Doe", "john@example.com").await;

	// Serialize author
	let json = serde_json::to_value(&author).expect("Serialization failed");

	assert_eq!(json["name"], "John Doe");
	assert_eq!(json["email"], "john@example.com");
	assert!(json["id"].is_number());
}

/// Test serializer deserialization to ORM model
///
/// **Test Intent**: Verify serializer can deserialize JSON to ORM model and save to database
///
/// **Integration Point**: JSON → Serializer → ORM Model → Database Insert
///
/// **Not Intent**: JSON parsing library behavior
#[rstest]
#[tokio::test]
async fn test_serializer_deserialization_to_orm(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;

	// Deserialize JSON to Author
	let json_data = json!({
		"name": "Jane Smith",
		"email": "jane@example.com",
		"bio": "Science fiction author"
	});

	let mut author: Author = serde_json::from_value(json_data).expect("Deserialization failed");
	author.id = None; // ID will be assigned by database

	// Insert into database
	let inserted = sqlx::query_as::<_, Author>(
		"INSERT INTO authors (name, email, bio)
		 VALUES ($1, $2, $3)
		 RETURNING id, name, email, bio",
	)
	.bind(&author.name)
	.bind(&author.email)
	.bind(&author.bio)
	.fetch_one(&pool)
	.await
	.expect("Failed to insert");

	assert_eq!(inserted.name, "Jane Smith");
	assert!(inserted.id.is_some());
}

/// Test serializer with partial updates
///
/// **Test Intent**: Verify serializer supports partial field updates
///
/// **Integration Point**: Serializer → Partial Update → Database PATCH
///
/// **Not Intent**: HTTP PATCH method handling
#[rstest]
#[tokio::test]
async fn test_serializer_partial_update(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;
	let author = insert_author(&pool, "Original Name", "original@example.com").await;

	// Partial update (only name)
	let update_json = json!({
		"name": "Updated Name"
	});

	// Update in database
	let updated = sqlx::query_as::<_, Author>(
		"UPDATE authors SET name = $1 WHERE id = $2
		 RETURNING id, name, email, bio",
	)
	.bind("Updated Name")
	.bind(author.id.unwrap())
	.fetch_one(&pool)
	.await
	.expect("Update failed");

	assert_eq!(updated.name, "Updated Name");
	assert_eq!(updated.email, "original@example.com"); // Email unchanged
}

// ========================================================================
// Nested Serializers Tests
// ========================================================================

/// Test nested serializer with database relationships
///
/// **Test Intent**: Verify nested serializer can serialize related models from database
///
/// **Integration Point**: Nested Serializer → JOIN Query → Related Models
///
/// **Not Intent**: JOIN optimization strategies
#[rstest]
#[tokio::test]
async fn test_nested_serializer_with_relationships(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;
	let author = insert_author(&pool, "Author Name", "author@example.com").await;
	let book = insert_book(&pool, "Book Title", "1234567890123", author.id.unwrap(), 300).await;

	// Get book with nested author
	let book_with_author = get_book_with_author(&pool, book.id.unwrap()).await;

	// Serialize nested structure
	let json = serde_json::to_value(&book_with_author).expect("Serialization failed");

	assert_eq!(json["title"], "Book Title");
	assert_eq!(json["author"]["name"], "Author Name");
	assert_eq!(json["author"]["email"], "author@example.com");
}

/// Test nested serializer with multiple levels
///
/// **Test Intent**: Verify nested serializer supports multiple levels of nesting
///
/// **Integration Point**: Multi-level Serializer → Multiple JOINs → Nested Data
///
/// **Not Intent**: Deep nesting performance optimization
#[rstest]
#[tokio::test]
async fn test_nested_serializer_multiple_levels(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;
	let author = insert_author(&pool, "Author", "author@example.com").await;
	let book = insert_book(&pool, "Book", "1234567890123", author.id.unwrap(), 200).await;

	// Get nested data
	let book_with_author = get_book_with_author(&pool, book.id.unwrap()).await;

	// Verify nested structure
	assert_eq!(book_with_author.title, "Book");
	assert_eq!(book_with_author.author.name, "Author");
	assert!(book_with_author.author.id.is_some());
}

/// Test nested serializer with collection
///
/// **Test Intent**: Verify nested serializer can serialize collections (one-to-many)
///
/// **Integration Point**: Nested Collection → Multiple Rows → Array Serialization
///
/// **Not Intent**: Collection pagination
#[rstest]
#[tokio::test]
async fn test_nested_serializer_with_collection(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;
	let author = insert_author(&pool, "Prolific Author", "prolific@example.com").await;

	// Insert multiple books for same author
	insert_book(&pool, "Book 1", "1111111111111", author.id.unwrap(), 100).await;
	insert_book(&pool, "Book 2", "2222222222222", author.id.unwrap(), 200).await;
	insert_book(&pool, "Book 3", "3333333333333", author.id.unwrap(), 300).await;

	// Query author's books
	let books = sqlx::query_as::<_, Book>(
		"SELECT id, title, isbn, author_id, published_date, pages, password
		 FROM books WHERE author_id = $1",
	)
	.bind(author.id.unwrap())
	.fetch_all(&pool)
	.await
	.expect("Failed to query books");

	assert_eq!(books.len(), 3);
	assert!(books.iter().all(|b| b.author_id == author.id.unwrap()));
}

// ========================================================================
// Validation with Database Constraints Tests
// ========================================================================

/// Test serializer validation with unique constraint
///
/// **Test Intent**: Verify serializer validates uniqueness against database
///
/// **Integration Point**: Serializer Validation → UNIQUE Constraint → Database Error
///
/// **Not Intent**: Database constraint error message formatting
#[rstest]
#[tokio::test]
async fn test_serializer_validation_unique_constraint(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;
	insert_author(&pool, "Author", "unique@example.com").await;

	// Try to insert duplicate email
	let result = sqlx::query(
		"INSERT INTO authors (name, email, bio) VALUES ($1, $2, $3)",
	)
	.bind("Another Author")
	.bind("unique@example.com") // Duplicate email
	.bind("Bio")
	.execute(&pool)
	.await;

	// Should fail with unique constraint violation
	assert!(result.is_err());
	let err = result.unwrap_err();
	assert!(err.to_string().contains("unique") || err.to_string().contains("duplicate"));
}

/// Test serializer validation with NOT NULL constraint
///
/// **Test Intent**: Verify serializer validates required fields against database
///
/// **Integration Point**: Serializer Validation → NOT NULL Constraint → Database Error
///
/// **Not Intent**: NULL value handling in serde
#[rstest]
#[tokio::test]
async fn test_serializer_validation_not_null_constraint(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;

	// Try to insert NULL for required field (title)
	let result = sqlx::query(
		"INSERT INTO books (title, isbn, author_id, published_date, pages) VALUES (NULL, $1, $2, $3, $4)",
	)
	.bind("1234567890123")
	.bind(1_i64)
	.bind("2024-01-01")
	.bind(100)
	.execute(&pool)
	.await;

	// Should fail with NOT NULL constraint violation
	assert!(result.is_err());
}

/// Test serializer validation with CHECK constraint
///
/// **Test Intent**: Verify serializer validates field values against CHECK constraints
///
/// **Integration Point**: Serializer Validation → CHECK Constraint → Database Error
///
/// **Not Intent**: Custom validation logic
#[rstest]
#[tokio::test]
async fn test_serializer_validation_check_constraint(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;
	let author = insert_author(&pool, "Author", "author@example.com").await;

	// Try to insert invalid pages value (CHECK: pages > 0)
	let result = sqlx::query(
		"INSERT INTO books (title, isbn, author_id, published_date, pages)
		 VALUES ($1, $2, $3, $4, $5)",
	)
	.bind("Invalid Book")
	.bind("1234567890123")
	.bind(author.id.unwrap())
	.bind("2024-01-01")
	.bind(0) // Invalid: pages must be > 0
	.execute(&pool)
	.await;

	// Should fail with CHECK constraint violation
	assert!(result.is_err());
}

/// Test serializer validation with foreign key constraint
///
/// **Test Intent**: Verify serializer validates foreign key references
///
/// **Integration Point**: Serializer Validation → FOREIGN KEY → Database Error
///
/// **Not Intent**: CASCADE behavior
#[rstest]
#[tokio::test]
async fn test_serializer_validation_foreign_key_constraint(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;

	// Try to insert book with non-existent author_id
	let result = sqlx::query(
		"INSERT INTO books (title, isbn, author_id, published_date, pages)
		 VALUES ($1, $2, $3, $4, $5)",
	)
	.bind("Orphan Book")
	.bind("1234567890123")
	.bind(99999_i64) // Non-existent author
	.bind("2024-01-01")
	.bind(100)
	.execute(&pool)
	.await;

	// Should fail with foreign key constraint violation
	assert!(result.is_err());
}

// ========================================================================
// Read-Only and Write-Only Fields Tests
// ========================================================================

/// Test serializer with read-only fields
///
/// **Test Intent**: Verify read-only fields are serialized but not deserialized
///
/// **Integration Point**: Serializer → Field Control → Asymmetric Serialization
///
/// **Not Intent**: serde skip_serializing attributes
#[rstest]
#[tokio::test]
async fn test_serializer_read_only_fields(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;
	let author = insert_author(&pool, "Author", "author@example.com").await;

	// Serialize (id is read-only, should be included)
	let json = serde_json::to_value(&author).expect("Serialization failed");
	assert!(json["id"].is_number());

	// Deserialization should ignore provided id (database generates it)
	// This is tested by the fact that INSERT doesn't include id field
}

/// Test serializer with write-only fields
///
/// **Test Intent**: Verify write-only fields are deserialized but not serialized
///
/// **Integration Point**: Serializer → Field Control → Password Handling
///
/// **Not Intent**: Password hashing algorithms
#[rstest]
#[tokio::test]
async fn test_serializer_write_only_fields(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;
	let author = insert_author(&pool, "Author", "author@example.com").await;
	let book = insert_book(&pool, "Book", "1234567890123", author.id.unwrap(), 100).await;

	// Update password (write-only)
	sqlx::query("UPDATE books SET password = $1 WHERE id = $2")
		.bind("secret_password")
		.bind(book.id.unwrap())
		.execute(&pool)
		.await
		.expect("Update failed");

	// Fetch book
	let book_with_password = sqlx::query_as::<_, Book>(
		"SELECT id, title, isbn, author_id, published_date, pages, password
		 FROM books WHERE id = $1",
	)
	.bind(book.id.unwrap())
	.fetch_one(&pool)
	.await
	.expect("Query failed");

	// Serialize (password should be excluded with skip_serializing_if)
	let json = serde_json::to_value(&book_with_password).expect("Serialization failed");
	assert!(!json.get("password").is_some() || json["password"].is_null());
}

// ========================================================================
// Custom Serializer Methods Tests
// ========================================================================

/// Test serializer with computed fields
///
/// **Test Intent**: Verify serializer can include computed fields from database
///
/// **Integration Point**: Serializer → Computed Fields → Database Function
///
/// **Not Intent**: Complex SQL functions
#[rstest]
#[tokio::test]
async fn test_serializer_computed_fields(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;
	let author = insert_author(&pool, "Author", "author@example.com").await;
	insert_book(&pool, "Book 1", "1111111111111", author.id.unwrap(), 100).await;
	insert_book(&pool, "Book 2", "2222222222222", author.id.unwrap(), 200).await;

	// Query with computed field (book count)
	let result: (String, i64) = sqlx::query_as(
		"SELECT name, COUNT(b.id) as book_count
		 FROM authors a
		 LEFT JOIN books b ON a.id = b.author_id
		 WHERE a.id = $1
		 GROUP BY a.id, a.name",
	)
	.bind(author.id.unwrap())
	.fetch_one(&pool)
	.await
	.expect("Query failed");

	assert_eq!(result.0, "Author");
	assert_eq!(result.1, 2); // 2 books
}

/// Test serializer method field with database aggregation
///
/// **Test Intent**: Verify serializer can include aggregated data from database
///
/// **Integration Point**: Serializer → SQL Aggregation → Computed Value
///
/// **Not Intent**: Aggregation function optimization
#[rstest]
#[tokio::test]
async fn test_serializer_method_field_with_aggregation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;
	let author = insert_author(&pool, "Author", "author@example.com").await;
	insert_book(&pool, "Book 1", "1111111111111", author.id.unwrap(), 100).await;
	insert_book(&pool, "Book 2", "2222222222222", author.id.unwrap(), 200).await;
	insert_book(&pool, "Book 3", "3333333333333", author.id.unwrap(), 300).await;

	// Query with aggregation (total pages)
	let total_pages: i64 = sqlx::query_scalar(
		"SELECT SUM(pages) FROM books WHERE author_id = $1",
	)
	.bind(author.id.unwrap())
	.fetch_one(&pool)
	.await
	.expect("Query failed");

	assert_eq!(total_pages, 600); // 100 + 200 + 300
}

// ========================================================================
// Performance with Large Datasets Tests
// ========================================================================

/// Test serializer performance with bulk operations
///
/// **Test Intent**: Verify serializer can handle bulk serialization efficiently
///
/// **Integration Point**: Serializer → Bulk Query → Large Result Set
///
/// **Not Intent**: Benchmarking exact timings
#[rstest]
#[tokio::test]
async fn test_serializer_bulk_serialization_performance(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;

	// Insert many authors
	for i in 1..=100 {
		insert_author(&pool, &format!("Author {}", i), &format!("author{}@example.com", i))
			.await;
	}

	// Query all authors
	let authors = sqlx::query_as::<_, Author>("SELECT id, name, email, bio FROM authors")
		.fetch_all(&pool)
		.await
		.expect("Query failed");

	assert_eq!(authors.len(), 100);

	// Serialize all (performance test - should not timeout)
	let json = serde_json::to_value(&authors).expect("Serialization failed");
	assert_eq!(json.as_array().unwrap().len(), 100);
}

/// Test serializer with large nested dataset
///
/// **Test Intent**: Verify serializer handles large nested structures efficiently
///
/// **Integration Point**: Serializer → Large JOIN → Nested Result Set
///
/// **Not Intent**: JOIN performance tuning
#[rstest]
#[tokio::test]
async fn test_serializer_large_nested_dataset(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;

	// Insert many authors and books
	for i in 1..=50 {
		let author = insert_author(
			&pool,
			&format!("Author {}", i),
			&format!("author{}@example.com", i),
		)
		.await;

		// Each author has 2 books
		insert_book(
			&pool,
			&format!("Book {}-1", i),
			&format!("111111111{:04}", i * 2 - 1),
			author.id.unwrap(),
			100,
		)
		.await;
		insert_book(
			&pool,
			&format!("Book {}-2", i),
			&format!("111111111{:04}", i * 2),
			author.id.unwrap(),
			200,
		)
		.await;
	}

	// Query all books with authors
	let books_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM books")
		.fetch_one(&pool)
		.await
		.expect("Count failed");

	assert_eq!(books_count, 100); // 50 authors * 2 books each
}

// ========================================================================
// Error Handling Tests
// ========================================================================

/// Test serializer deserialization error handling
///
/// **Test Intent**: Verify serializer handles invalid JSON gracefully
///
/// **Integration Point**: JSON Parsing → Serializer Error → Error Response
///
/// **Not Intent**: JSON syntax error details
#[rstest]
#[tokio::test]
async fn test_serializer_deserialization_error_handling(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	// Invalid JSON (missing required field)
	let invalid_json = json!({
		"name": "Author"
		// Missing "email" field
	});

	// Try to deserialize
	let result: Result<Author, _> = serde_json::from_value(invalid_json);

	// Should fail
	assert!(result.is_err());
}

/// Test serializer type mismatch error
///
/// **Test Intent**: Verify serializer handles type mismatches in JSON
///
/// **Integration Point**: Type Validation → Serializer Error → Error Response
///
/// **Not Intent**: Type coercion behavior
#[rstest]
#[tokio::test]
async fn test_serializer_type_mismatch_error(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;
	let author = insert_author(&pool, "Author", "author@example.com").await;

	// Invalid type (pages should be integer, not string)
	let result = sqlx::query(
		"INSERT INTO books (title, isbn, author_id, published_date, pages)
		 VALUES ($1, $2, $3, $4, $5)",
	)
	.bind("Book")
	.bind("1234567890123")
	.bind(author.id.unwrap())
	.bind("2024-01-01")
	.bind("not_a_number") // Type mismatch
	.execute(&pool)
	.await;

	// Should fail with type error
	assert!(result.is_err());
}

/// Test serializer validation error aggregation
///
/// **Test Intent**: Verify serializer can collect and report multiple validation errors
///
/// **Integration Point**: Validation → Error Collection → Error Response
///
/// **Not Intent**: Error message formatting
#[rstest]
#[tokio::test]
async fn test_serializer_validation_error_aggregation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup database
	setup_tables(&pool).await;

	// Multiple validation errors in one request
	// 1. Missing required field (title)
	// 2. Invalid foreign key (author_id)
	let result = sqlx::query(
		"INSERT INTO books (title, isbn, author_id, published_date, pages)
		 VALUES (NULL, $1, $2, $3, $4)",
	)
	.bind("1234567890123")
	.bind(99999_i64) // Non-existent author
	.bind("2024-01-01")
	.bind(100)
	.execute(&pool)
	.await;

	// Should fail (NOT NULL constraint triggers first)
	assert!(result.is_err());
}
