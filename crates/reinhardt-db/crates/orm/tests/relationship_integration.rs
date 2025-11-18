//! ORM Relationship Integration Tests with Real Database
//!
//! These tests verify relationship functionality (ForeignKey, OneToOne, ManyToMany)
//! with real PostgreSQL database. Extracted from relationship_tests.rs.
//!
//! **Test Coverage:**
//! - ForeignKey: creation, access, reverse, NULL handling
//! - OneToOne: creation with UNIQUE constraint
//! - ManyToMany: junction table operations
//! - CASCADE DELETE behavior
//! - Multiple ForeignKeys to same model
//! - Self-referential ForeignKey
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

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
	country_id: Option<i32>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Book {
	id: Option<i32>,
	title: String,
	author_id: Option<i32>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Country {
	id: Option<i32>,
	name: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Profile {
	id: Option<i32>,
	bio: String,
	author_id: Option<i32>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Tag {
	id: Option<i32>,
	name: String,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct BookTag {
	id: Option<i32>,
	book_id: i32,
	tag_id: i32,
}

// ============================================================================
// ForeignKey Integration Tests
// ============================================================================

/// Test basic ForeignKey creation with real database
///
/// **Test Intent**: Verify ForeignKey relationship is created correctly
/// in PostgreSQL with INSERT and SELECT operations
///
/// **Integration Point**: ORM ForeignKey → PostgreSQL FOREIGN KEY constraint
///
/// **Not Intent**: Mock database, in-memory relationships
#[rstest]
#[tokio::test]
async fn test_foreign_key_creation(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create tables
	sqlx::query(
		"CREATE TABLE countries (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE authors (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			country_id INTEGER REFERENCES countries(id)
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert country
	let country_id: i32 = sqlx::query("INSERT INTO countries (name) VALUES ($1) RETURNING id")
		.bind("USA")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert author with ForeignKey
	let author_id: i32 =
		sqlx::query("INSERT INTO authors (name, country_id) VALUES ($1, $2) RETURNING id")
			.bind("John Doe")
			.bind(country_id)
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	// Verify ForeignKey relationship
	let result = sqlx::query("SELECT name, country_id FROM authors WHERE id = $1")
		.bind(author_id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	let name: String = result.get("name");
	let stored_country_id: i32 = result.get("country_id");

	assert_eq!(name, "John Doe");
	assert_eq!(stored_country_id, country_id);
}

/// Test accessing related object via ForeignKey with JOIN
///
/// **Test Intent**: Verify ForeignKey relationship allows JOIN operations
/// to access related data
///
/// **Integration Point**: ORM ForeignKey access → PostgreSQL JOIN query
///
/// **Not Intent**: Lazy loading, N+1 prevention
#[rstest]
#[tokio::test]
async fn test_foreign_key_access(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create tables
	sqlx::query(
		"CREATE TABLE countries (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE authors (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			country_id INTEGER REFERENCES countries(id)
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert data
	let country_id: i32 = sqlx::query("INSERT INTO countries (name) VALUES ($1) RETURNING id")
		.bind("USA")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	sqlx::query("INSERT INTO authors (name, country_id) VALUES ($1, $2)")
		.bind("Jane Smith")
		.bind(country_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Access related country via JOIN
	let result = sqlx::query(
		"SELECT c.name as country_name
		 FROM authors a
		 JOIN countries c ON a.country_id = c.id
		 WHERE a.name = $1",
	)
	.bind("Jane Smith")
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	let country_name: String = result.get("country_name");
	assert_eq!(country_name, "USA");
}

/// Test reverse ForeignKey (one-to-many relationship)
///
/// **Test Intent**: Verify reverse ForeignKey allows querying
/// all related objects (e.g., all books by an author)
///
/// **Integration Point**: ORM reverse ForeignKey → PostgreSQL reverse JOIN
///
/// **Not Intent**: Prefetch, eager loading
#[rstest]
#[tokio::test]
async fn test_reverse_foreign_key(
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
			author_id INTEGER REFERENCES authors(id)
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert author
	let author_id: i32 = sqlx::query("INSERT INTO authors (name) VALUES ($1) RETURNING id")
		.bind("Prolific Author")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert books
	for i in 1..=3 {
		sqlx::query("INSERT INTO books (title, author_id) VALUES ($1, $2)")
			.bind(format!("Book {}", i))
			.bind(author_id)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}

	// Query reverse relationship (author's books)
	let books = sqlx::query("SELECT title FROM books WHERE author_id = $1")
		.bind(author_id)
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	assert_eq!(books.len(), 3);
}

/// Test ForeignKey with NULL value
///
/// **Test Intent**: Verify ForeignKey allows NULL values when not constrained
///
/// **Integration Point**: ORM ForeignKey NULL → PostgreSQL NULL foreign key
///
/// **Not Intent**: NOT NULL constraint, default values
#[rstest]
#[tokio::test]
async fn test_null_foreign_key(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create table
	sqlx::query(
		"CREATE TABLE authors (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			country_id INTEGER
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert author with NULL country_id
	let author_id: i32 =
		sqlx::query("INSERT INTO authors (name, country_id) VALUES ($1, NULL) RETURNING id")
			.bind("Independent Author")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	// Verify NULL is stored
	let result = sqlx::query("SELECT country_id FROM authors WHERE id = $1")
		.bind(author_id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	let country_id: Option<i32> = result.get("country_id");
	assert!(country_id.is_none());
}

// ============================================================================
// OneToOne Relationship Integration Tests
// ============================================================================

/// Test OneToOne relationship creation with UNIQUE constraint
///
/// **Test Intent**: Verify OneToOne relationship enforces uniqueness
/// at database level with UNIQUE constraint
///
/// **Integration Point**: ORM OneToOne → PostgreSQL UNIQUE constraint
///
/// **Not Intent**: Application-level uniqueness, duplicate prevention in ORM
#[rstest]
#[tokio::test]
async fn test_one_to_one_creation(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create tables with UNIQUE constraint for OneToOne
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
		"CREATE TABLE profiles (
			id SERIAL PRIMARY KEY,
			bio TEXT NOT NULL,
			author_id INTEGER UNIQUE REFERENCES authors(id)
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert author
	let author_id: i32 = sqlx::query("INSERT INTO authors (name) VALUES ($1) RETURNING id")
		.bind("Profile Owner")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert profile (OneToOne)
	let profile_id: i32 =
		sqlx::query("INSERT INTO profiles (bio, author_id) VALUES ($1, $2) RETURNING id")
			.bind("This is my bio")
			.bind(author_id)
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	// Verify profile exists
	let result = sqlx::query("SELECT bio, author_id FROM profiles WHERE id = $1")
		.bind(profile_id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	let bio: String = result.get("bio");
	let stored_author_id: i32 = result.get("author_id");

	assert_eq!(bio, "This is my bio");
	assert_eq!(stored_author_id, author_id);

	// Attempt to insert duplicate profile (should fail due to UNIQUE constraint)
	let duplicate_result = sqlx::query("INSERT INTO profiles (bio, author_id) VALUES ($1, $2)")
		.bind("Duplicate bio")
		.bind(author_id)
		.execute(pool.as_ref())
		.await;

	assert!(
		duplicate_result.is_err(),
		"Duplicate OneToOne should violate UNIQUE constraint"
	);
}

// ============================================================================
// ManyToMany Relationship Integration Tests
// ============================================================================

/// Test ManyToMany relationship creation with junction table
///
/// **Test Intent**: Verify ManyToMany relationship uses junction table
/// for many-to-many associations
///
/// **Integration Point**: ORM ManyToMany → PostgreSQL junction table
///
/// **Not Intent**: Direct relationships, embedded collections
#[rstest]
#[tokio::test]
async fn test_many_to_many_creation(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create tables
	sqlx::query(
		"CREATE TABLE books (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE tags (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE book_tags (
			id SERIAL PRIMARY KEY,
			book_id INTEGER REFERENCES books(id),
			tag_id INTEGER REFERENCES tags(id),
			UNIQUE(book_id, tag_id)
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert book
	let book_id: i32 = sqlx::query("INSERT INTO books (title) VALUES ($1) RETURNING id")
		.bind("Test Book")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert tags
	let tag1_id: i32 = sqlx::query("INSERT INTO tags (name) VALUES ($1) RETURNING id")
		.bind("fiction")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	let tag2_id: i32 = sqlx::query("INSERT INTO tags (name) VALUES ($1) RETURNING id")
		.bind("bestseller")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Create ManyToMany associations
	sqlx::query("INSERT INTO book_tags (book_id, tag_id) VALUES ($1, $2)")
		.bind(book_id)
		.bind(tag1_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO book_tags (book_id, tag_id) VALUES ($1, $2)")
		.bind(book_id)
		.bind(tag2_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Query tags for book
	let tags = sqlx::query(
		"SELECT t.name
		 FROM tags t
		 JOIN book_tags bt ON t.id = bt.tag_id
		 WHERE bt.book_id = $1",
	)
	.bind(book_id)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(tags.len(), 2);
}

/// Test ManyToMany reverse relationship
///
/// **Test Intent**: Verify ManyToMany junction table allows
/// querying in both directions
///
/// **Integration Point**: ORM ManyToMany reverse → PostgreSQL reverse JOIN
///
/// **Not Intent**: Unidirectional relationships
#[rstest]
#[tokio::test]
async fn test_many_to_many_reverse(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create tables (same as above)
	sqlx::query(
		"CREATE TABLE books (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE tags (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE book_tags (
			id SERIAL PRIMARY KEY,
			book_id INTEGER REFERENCES books(id),
			tag_id INTEGER REFERENCES tags(id)
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert tag
	let tag_id: i32 = sqlx::query("INSERT INTO tags (name) VALUES ($1) RETURNING id")
		.bind("science")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert books
	let book1_id: i32 = sqlx::query("INSERT INTO books (title) VALUES ($1) RETURNING id")
		.bind("Physics 101")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	let book2_id: i32 = sqlx::query("INSERT INTO books (title) VALUES ($1) RETURNING id")
		.bind("Chemistry 101")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Create associations
	sqlx::query("INSERT INTO book_tags (book_id, tag_id) VALUES ($1, $2)")
		.bind(book1_id)
		.bind(tag_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO book_tags (book_id, tag_id) VALUES ($1, $2)")
		.bind(book2_id)
		.bind(tag_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Query books for tag (reverse direction)
	let books = sqlx::query(
		"SELECT b.title
		 FROM books b
		 JOIN book_tags bt ON b.id = bt.book_id
		 WHERE bt.tag_id = $1",
	)
	.bind(tag_id)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(books.len(), 2);
}

// ============================================================================
// CASCADE DELETE Integration Tests
// ============================================================================

/// Test CASCADE DELETE behavior with ForeignKey
///
/// **Test Intent**: Verify CASCADE DELETE constraint removes related objects
/// when parent is deleted
///
/// **Integration Point**: ORM CASCADE → PostgreSQL ON DELETE CASCADE
///
/// **Not Intent**: SET NULL, RESTRICT, manual cascade
#[rstest]
#[tokio::test]
async fn test_cascade_delete(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create tables with CASCADE
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
			author_id INTEGER REFERENCES authors(id) ON DELETE CASCADE
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert author
	let author_id: i32 = sqlx::query("INSERT INTO authors (name) VALUES ($1) RETURNING id")
		.bind("Author to Delete")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert books
	sqlx::query("INSERT INTO books (title, author_id) VALUES ($1, $2)")
		.bind("Book 1")
		.bind(author_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO books (title, author_id) VALUES ($1, $2)")
		.bind("Book 2")
		.bind(author_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Verify books exist
	let count_before: i64 = sqlx::query("SELECT COUNT(*) as count FROM books WHERE author_id = $1")
		.bind(author_id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("count");
	assert_eq!(count_before, 2);

	// Delete author (should CASCADE to books)
	sqlx::query("DELETE FROM authors WHERE id = $1")
		.bind(author_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Verify books were deleted by CASCADE
	let count_after: i64 = sqlx::query("SELECT COUNT(*) as count FROM books WHERE author_id = $1")
		.bind(author_id)
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("count");
	assert_eq!(count_after, 0, "Books should be deleted by CASCADE");
}

// ============================================================================
// Advanced Relationship Tests
// ============================================================================

/// Test multiple ForeignKeys to the same model
///
/// **Test Intent**: Verify multiple ForeignKeys can reference the same model
/// (e.g., sender_id and recipient_id both reference users)
///
/// **Integration Point**: ORM multiple FK → PostgreSQL multiple FK constraints
///
/// **Not Intent**: Self-referential FK, polymorphic FK
#[rstest]
#[tokio::test]
async fn test_multiple_foreign_keys_same_model(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create tables
	sqlx::query(
		"CREATE TABLE users (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE messages (
			id SERIAL PRIMARY KEY,
			sender_id INTEGER REFERENCES users(id),
			recipient_id INTEGER REFERENCES users(id),
			content TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert users
	let sender_id: i32 = sqlx::query("INSERT INTO users (name) VALUES ($1) RETURNING id")
		.bind("Sender")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	let recipient_id: i32 = sqlx::query("INSERT INTO users (name) VALUES ($1) RETURNING id")
		.bind("Recipient")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert message with two ForeignKeys to same model
	sqlx::query("INSERT INTO messages (sender_id, recipient_id, content) VALUES ($1, $2, $3)")
		.bind(sender_id)
		.bind(recipient_id)
		.bind("Hello!")
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Verify both ForeignKeys are valid
	let result = sqlx::query("SELECT sender_id, recipient_id FROM messages WHERE content = $1")
		.bind("Hello!")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	let stored_sender: i32 = result.get("sender_id");
	let stored_recipient: i32 = result.get("recipient_id");

	assert_eq!(stored_sender, sender_id);
	assert_eq!(stored_recipient, recipient_id);
	assert_ne!(stored_sender, stored_recipient);
}

/// Test self-referential ForeignKey
///
/// **Test Intent**: Verify ForeignKey can reference its own table
/// (e.g., parent_id references categories.id)
///
/// **Integration Point**: ORM self-referential FK → PostgreSQL self-reference
///
/// **Not Intent**: Circular dependencies, mutual references
#[rstest]
#[tokio::test]
async fn test_self_referential_foreign_key(
	#[future] postgres_container: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create table with self-referential ForeignKey
	sqlx::query(
		"CREATE TABLE categories (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			parent_id INTEGER REFERENCES categories(id)
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert root category
	let root_id: i32 =
		sqlx::query("INSERT INTO categories (name, parent_id) VALUES ($1, NULL) RETURNING id")
			.bind("Root")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	// Insert child categories
	sqlx::query("INSERT INTO categories (name, parent_id) VALUES ($1, $2)")
		.bind("Child 1")
		.bind(root_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO categories (name, parent_id) VALUES ($1, $2)")
		.bind("Child 2")
		.bind(root_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Query children
	let children = sqlx::query("SELECT name FROM categories WHERE parent_id = $1")
		.bind(root_id)
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	assert_eq!(children.len(), 2);
}
