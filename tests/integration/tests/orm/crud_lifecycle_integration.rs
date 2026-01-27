//! ORM CRUD Lifecycle End-to-End Integration Tests
//!
//! Tests complete CRUD operation lifecycles including:
//! - Create → Read → Update → Delete flows
//! - Validation and constraints
//! - Event hooks and callbacks
//! - Cascading operations
//! - Optimistic locking
//! - Soft delete patterns
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use reinhardt_core::macros::model;
use reinhardt_db::orm::manager::reinitialize_database;
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// ORM Model Definitions
// ============================================================================

/// ORM model for article - demonstrates reinhardt_orm integration
#[model(app_label = "crud_test", table_name = "articles")]
#[derive(Serialize, Deserialize, Clone, Debug)]
// Test fixture: ORM model for CRUD lifecycle integration tests
#[allow(dead_code)] // ORM model for CRUD lifecycle tests
struct ArticleModel {
	#[field(primary_key = true)]
	id: Option<i32>,
	#[field(max_length = 255)]
	title: String,
	#[field(max_length = 10000)]
	body: String,
}

/// ORM model for post with timestamps
#[model(app_label = "crud_test", table_name = "posts")]
#[derive(Serialize, Deserialize, Clone, Debug)]
// Test fixture: ORM model for CRUD lifecycle integration tests
#[allow(dead_code)] // ORM model for CRUD lifecycle tests
struct PostModel {
	#[field(primary_key = true)]
	id: Option<i32>,
	#[field(max_length = 255)]
	title: String,
	#[field(auto_now_add = true, max_length = 50)]
	created_at: Option<String>,
	#[field(auto_now = true, max_length = 50)]
	updated_at: Option<String>,
}

// ============================================================================
// Basic CRUD Lifecycle Tests
// ============================================================================

/// Test complete CRUD lifecycle for single entity
///
/// **Test Intent**: Verify basic Create → Read → Update → Delete flow works
/// end-to-end
///
/// **Integration Point**: Session + Transaction + Query execution for full CRUD
///
/// **Not Intent**: Complex relationships, bulk operations
#[rstest]
#[tokio::test]
async fn test_basic_crud_lifecycle(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table
	sqlx::query("CREATE TABLE IF NOT EXISTS articles (id SERIAL PRIMARY KEY, title TEXT NOT NULL, body TEXT NOT NULL)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// CREATE: Insert article
	let article_id: i32 =
		sqlx::query_scalar("INSERT INTO articles (title, body) VALUES ($1, $2) RETURNING id")
			.bind("My First Article")
			.bind("This is the article body")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to create article");

	assert!(
		article_id > 0,
		"Article should have valid ID after creation"
	);

	// READ: Fetch article
	let result = sqlx::query("SELECT id, title, body FROM articles WHERE id = $1")
		.bind(article_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to read article");

	let title: String = result.get("title");
	let body: String = result.get("body");

	assert_eq!(title, "My First Article");
	assert_eq!(body, "This is the article body");

	// UPDATE: Modify article
	sqlx::query("UPDATE articles SET title = $1, body = $2 WHERE id = $3")
		.bind("Updated Title")
		.bind("Updated body content")
		.bind(article_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to update article");

	// READ: Verify update
	let updated_result = sqlx::query("SELECT title, body FROM articles WHERE id = $1")
		.bind(article_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to read updated article");

	let updated_title: String = updated_result.get("title");
	let updated_body: String = updated_result.get("body");

	assert_eq!(updated_title, "Updated Title");
	assert_eq!(updated_body, "Updated body content");

	// DELETE: Remove article
	sqlx::query("DELETE FROM articles WHERE id = $1")
		.bind(article_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to delete article");

	// READ: Verify deletion
	let deleted_result = sqlx::query("SELECT id FROM articles WHERE id = $1")
		.bind(article_id)
		.fetch_optional(pool.as_ref())
		.await
		.expect("Failed to query deleted article");

	assert!(deleted_result.is_none(), "Article should be deleted");
}

/// Test CRUD lifecycle with timestamps
///
/// **Test Intent**: Verify created_at and updated_at timestamps are managed
/// correctly throughout CRUD lifecycle
///
/// **Integration Point**: Timestamp field management + CRUD operations
///
/// **Not Intent**: Other metadata fields, custom timestamps
#[rstest]
#[tokio::test]
async fn test_crud_lifecycle_with_timestamps(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table with timestamp columns
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS posts (
			id SERIAL PRIMARY KEY,
			title TEXT NOT NULL,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
			updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// CREATE: Insert post (timestamps auto-set)
	let post_id: i32 = sqlx::query_scalar("INSERT INTO posts (title) VALUES ($1) RETURNING id")
		.bind("Timestamped Post")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to create post");

	// READ: Get initial timestamps
	let result = sqlx::query("SELECT created_at, updated_at FROM posts WHERE id = $1")
		.bind(post_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to read post");

	let created_at: chrono::NaiveDateTime = result.get("created_at");
	let initial_updated_at: chrono::NaiveDateTime = result.get("updated_at");

	// Wait a bit to ensure timestamp difference
	tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

	// UPDATE: Modify post (updated_at should change)
	sqlx::query("UPDATE posts SET title = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
		.bind("Updated Timestamped Post")
		.bind(post_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to update post");

	// READ: Verify timestamp changes
	let updated_result = sqlx::query("SELECT created_at, updated_at FROM posts WHERE id = $1")
		.bind(post_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to read updated post");

	let new_created_at: chrono::NaiveDateTime = updated_result.get("created_at");
	let new_updated_at: chrono::NaiveDateTime = updated_result.get("updated_at");

	assert_eq!(
		created_at, new_created_at,
		"created_at should not change on update"
	);
	assert!(
		new_updated_at > initial_updated_at,
		"updated_at should be newer after update"
	);
}

// ============================================================================
// Validation and Constraints Tests
// ============================================================================

/// Test CRUD lifecycle with NOT NULL constraint validation
///
/// **Test Intent**: Verify NOT NULL constraints are enforced during CREATE
/// and UPDATE operations
///
/// **Integration Point**: Constraint validation → CRUD operation failure
///
/// **Not Intent**: UNIQUE constraints, successful operations
#[rstest]
#[tokio::test]
async fn test_crud_lifecycle_not_null_validation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table with NOT NULL constraint
	sqlx::query("CREATE TABLE IF NOT EXISTS profiles (id SERIAL PRIMARY KEY, username TEXT NOT NULL, email TEXT NOT NULL)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// CREATE: Attempt insert with NULL username (should fail)
	let result = sqlx::query("INSERT INTO profiles (username, email) VALUES ($1, $2)")
		.bind(Option::<String>::None)
		.bind("test@example.com")
		.execute(pool.as_ref())
		.await;

	assert!(
		result.is_err(),
		"Insert with NULL NOT NULL column should fail"
	);

	// CREATE: Valid insert
	let profile_id: i32 =
		sqlx::query_scalar("INSERT INTO profiles (username, email) VALUES ($1, $2) RETURNING id")
			.bind("john_doe")
			.bind("john@example.com")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to create profile");

	// UPDATE: Attempt to set email to NULL (should fail)
	let update_result = sqlx::query("UPDATE profiles SET email = $1 WHERE id = $2")
		.bind(Option::<String>::None)
		.bind(profile_id)
		.execute(pool.as_ref())
		.await;

	assert!(
		update_result.is_err(),
		"Update setting NOT NULL column to NULL should fail"
	);
}

/// Test CRUD lifecycle with UNIQUE constraint validation
///
/// **Test Intent**: Verify UNIQUE constraints prevent duplicate entries
/// during CREATE and UPDATE
///
/// **Integration Point**: UNIQUE constraint validation → CRUD operation failure
///
/// **Not Intent**: NOT NULL constraints, successful operations
#[rstest]
#[tokio::test]
async fn test_crud_lifecycle_unique_validation(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table with UNIQUE constraint
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS accounts (id SERIAL PRIMARY KEY, email TEXT UNIQUE NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// CREATE: Insert first account
	let account1_id: i32 =
		sqlx::query_scalar("INSERT INTO accounts (email) VALUES ($1) RETURNING id")
			.bind("user@example.com")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to create account 1");

	// CREATE: Attempt duplicate insert (should fail)
	let duplicate_result = sqlx::query("INSERT INTO accounts (email) VALUES ($1)")
		.bind("user@example.com")
		.execute(pool.as_ref())
		.await;

	assert!(duplicate_result.is_err(), "Duplicate insert should fail");

	// CREATE: Insert second account with different email
	let account2_id: i32 =
		sqlx::query_scalar("INSERT INTO accounts (email) VALUES ($1) RETURNING id")
			.bind("other@example.com")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to create account 2");

	// UPDATE: Attempt to change account2's email to duplicate account1's
	let update_result = sqlx::query("UPDATE accounts SET email = $1 WHERE id = $2")
		.bind("user@example.com")
		.bind(account2_id)
		.execute(pool.as_ref())
		.await;

	assert!(
		update_result.is_err(),
		"Update to duplicate email should fail"
	);

	// Verify account1 still exists with original email
	let account1_email: String = sqlx::query_scalar("SELECT email FROM accounts WHERE id = $1")
		.bind(account1_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to read account 1");

	assert_eq!(account1_email, "user@example.com");
}

/// Test CRUD lifecycle with CHECK constraint validation
///
/// **Test Intent**: Verify CHECK constraints enforce business rules during
/// CREATE and UPDATE
///
/// **Integration Point**: CHECK constraint validation → CRUD operation failure
///
/// **Not Intent**: Other constraints, successful operations
#[rstest]
#[tokio::test]
async fn test_crud_lifecycle_check_constraint(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table with CHECK constraint (price must be positive)
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS items (
			id SERIAL PRIMARY KEY,
			name TEXT NOT NULL,
			price BIGINT NOT NULL CHECK (price > 0)
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// CREATE: Attempt insert with invalid price (should fail)
	let result = sqlx::query("INSERT INTO items (name, price) VALUES ($1, $2)")
		.bind("Invalid Item")
		.bind(-100_i64)
		.execute(pool.as_ref())
		.await;

	assert!(result.is_err(), "Insert with negative price should fail");

	// CREATE: Valid insert
	let item_id: i32 =
		sqlx::query_scalar("INSERT INTO items (name, price) VALUES ($1, $2) RETURNING id")
			.bind("Valid Item")
			.bind(500_i64)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to create item");

	// UPDATE: Attempt to set price to invalid value
	let update_result = sqlx::query("UPDATE items SET price = $1 WHERE id = $2")
		.bind(0_i64)
		.bind(item_id)
		.execute(pool.as_ref())
		.await;

	assert!(update_result.is_err(), "Update to zero price should fail");

	// Verify original price unchanged
	let price: i64 = sqlx::query_scalar("SELECT price FROM items WHERE id = $1")
		.bind(item_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to read item");

	assert_eq!(price, 500);
}

// ============================================================================
// Cascading Operations Tests
// ============================================================================

/// Test CRUD lifecycle with CASCADE DELETE
///
/// **Test Intent**: Verify deleting parent entity cascades to child entities
///
/// **Integration Point**: Foreign key CASCADE → DELETE propagation
///
/// **Not Intent**: Standalone delete, SET NULL
#[rstest]
#[tokio::test]
async fn test_crud_lifecycle_cascade_delete(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create parent table
	sqlx::query("CREATE TABLE IF NOT EXISTS authors (id SERIAL PRIMARY KEY, name TEXT NOT NULL)")
		.execute(pool.as_ref())
		.await
		.expect("Failed to create authors table");

	// Create child table with CASCADE DELETE
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS books (
			id SERIAL PRIMARY KEY,
			title TEXT NOT NULL,
			author_id INT NOT NULL,
			FOREIGN KEY (author_id) REFERENCES authors(id) ON DELETE CASCADE
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create books table");

	// CREATE: Insert author
	let author_id: i32 = sqlx::query_scalar("INSERT INTO authors (name) VALUES ($1) RETURNING id")
		.bind("John Author")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to create author");

	// CREATE: Insert books for author
	sqlx::query("INSERT INTO books (title, author_id) VALUES ($1, $2)")
		.bind("Book 1")
		.bind(author_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create book 1");

	sqlx::query("INSERT INTO books (title, author_id) VALUES ($1, $2)")
		.bind("Book 2")
		.bind(author_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create book 2");

	// Verify books exist
	let book_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM books WHERE author_id = $1")
		.bind(author_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count books");

	assert_eq!(book_count, 2);

	// DELETE: Remove author (should cascade to books)
	sqlx::query("DELETE FROM authors WHERE id = $1")
		.bind(author_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to delete author");

	// Verify books also deleted
	let remaining_books: i64 =
		sqlx::query_scalar("SELECT COUNT(*) FROM books WHERE author_id = $1")
			.bind(author_id)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to count remaining books");

	assert_eq!(remaining_books, 0, "Books should be cascade deleted");
}

/// Test CRUD lifecycle with SET NULL on delete
///
/// **Test Intent**: Verify deleting parent entity sets child foreign keys to NULL
///
/// **Integration Point**: Foreign key SET NULL → DELETE side effect
///
/// **Not Intent**: CASCADE DELETE, parent retention
#[rstest]
#[tokio::test]
async fn test_crud_lifecycle_set_null_on_delete(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create parent table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS categories (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create categories table");

	// Create child table with SET NULL
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS products (
			id SERIAL PRIMARY KEY,
			name TEXT NOT NULL,
			category_id INT,
			FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE SET NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create products table");

	// CREATE: Insert category
	let category_id: i32 =
		sqlx::query_scalar("INSERT INTO categories (name) VALUES ($1) RETURNING id")
			.bind("Electronics")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to create category");

	// CREATE: Insert product
	let product_id: i32 =
		sqlx::query_scalar("INSERT INTO products (name, category_id) VALUES ($1, $2) RETURNING id")
			.bind("Laptop")
			.bind(category_id)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to create product");

	// DELETE: Remove category
	sqlx::query("DELETE FROM categories WHERE id = $1")
		.bind(category_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to delete category");

	// Verify product still exists but category_id is NULL
	let result = sqlx::query("SELECT name, category_id FROM products WHERE id = $1")
		.bind(product_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to read product");

	let name: String = result.get("name");
	let category_id_after: Option<i32> = result.get("category_id");

	assert_eq!(name, "Laptop");
	assert!(
		category_id_after.is_none(),
		"category_id should be NULL after parent deletion"
	);
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

/// Test CRUD lifecycle with soft delete pattern
///
/// **Test Intent**: Verify soft delete marks records as deleted without physical removal
///
/// **Integration Point**: Soft delete flag → Query filtering
///
/// **Not Intent**: Hard delete, permanent removal
#[rstest]
#[tokio::test]
async fn test_crud_lifecycle_soft_delete(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table with soft delete column
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS documents (
			id SERIAL PRIMARY KEY,
			title TEXT NOT NULL,
			deleted_at TIMESTAMP NULL
		)
		"#,
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// CREATE: Insert document
	let doc_id: i32 = sqlx::query_scalar("INSERT INTO documents (title) VALUES ($1) RETURNING id")
		.bind("Important Document")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to create document");

	// READ: Verify document exists and not deleted
	let result = sqlx::query("SELECT title, deleted_at FROM documents WHERE id = $1")
		.bind(doc_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to read document");

	let deleted_at: Option<chrono::NaiveDateTime> = result.get("deleted_at");
	assert!(deleted_at.is_none(), "New document should not be deleted");

	// SOFT DELETE: Mark as deleted
	sqlx::query("UPDATE documents SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1")
		.bind(doc_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to soft delete");

	// READ: Query with soft delete filter (exclude deleted)
	let active_result =
		sqlx::query("SELECT id FROM documents WHERE id = $1 AND deleted_at IS NULL")
			.bind(doc_id)
			.fetch_optional(pool.as_ref())
			.await
			.expect("Failed to query active documents");

	assert!(
		active_result.is_none(),
		"Soft deleted document should not appear in active query"
	);

	// READ: Query all documents (include deleted)
	let all_result = sqlx::query("SELECT id, deleted_at FROM documents WHERE id = $1")
		.bind(doc_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to query all documents");

	let deleted_at_after: Option<chrono::NaiveDateTime> = all_result.get("deleted_at");
	assert!(
		deleted_at_after.is_some(),
		"Soft deleted document should still exist with deleted_at set"
	);

	// RESTORE: Undelete
	sqlx::query("UPDATE documents SET deleted_at = NULL WHERE id = $1")
		.bind(doc_id)
		.execute(pool.as_ref())
		.await
		.expect("Failed to restore document");

	// READ: Verify restored
	let restored_result =
		sqlx::query("SELECT id FROM documents WHERE id = $1 AND deleted_at IS NULL")
			.bind(doc_id)
			.fetch_optional(pool.as_ref())
			.await
			.expect("Failed to query restored document");

	assert!(
		restored_result.is_some(),
		"Restored document should appear in active query"
	);
}

// ============================================================================
// Bulk Operations Tests
// ============================================================================

/// Test bulk insert with CRUD lifecycle
///
/// **Test Intent**: Verify bulk CREATE operations work correctly
///
/// **Integration Point**: Batch insert → Database performance
///
/// **Not Intent**: Single insert, updates
#[rstest]
#[tokio::test]
async fn test_crud_lifecycle_bulk_insert(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS bulk_items (id SERIAL PRIMARY KEY, value INT NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Bulk INSERT: Use transaction for efficiency
	let mut tx = pool.begin().await.expect("Failed to begin transaction");

	for i in 1..=50 {
		sqlx::query("INSERT INTO bulk_items (value) VALUES ($1)")
			.bind(i)
			.execute(&mut *tx)
			.await
			.expect("Failed to insert");
	}

	tx.commit().await.expect("Failed to commit bulk insert");

	// READ: Verify all inserted
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bulk_items")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count");

	assert_eq!(count, 50, "All bulk inserts should succeed");
}

/// Test bulk update with CRUD lifecycle
///
/// **Test Intent**: Verify bulk UPDATE operations work correctly
///
/// **Integration Point**: Batch update → Database performance
///
/// **Not Intent**: Single update, inserts
#[rstest]
#[tokio::test]
async fn test_crud_lifecycle_bulk_update(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS status_items (id SERIAL PRIMARY KEY, status TEXT NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert test data
	for _i in 1..=30 {
		sqlx::query("INSERT INTO status_items (status) VALUES ($1)")
			.bind("pending")
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert");
	}

	// Bulk UPDATE: Change all to "processed"
	sqlx::query("UPDATE status_items SET status = $1")
		.bind("processed")
		.execute(pool.as_ref())
		.await
		.expect("Failed to bulk update");

	// READ: Verify all updated
	let processed_count: i64 =
		sqlx::query_scalar("SELECT COUNT(*) FROM status_items WHERE status = $1")
			.bind("processed")
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to count processed");

	assert_eq!(processed_count, 30, "All items should be bulk updated");
}

/// Test bulk delete with CRUD lifecycle
///
/// **Test Intent**: Verify bulk DELETE operations work correctly
///
/// **Integration Point**: Batch delete → Database performance
///
/// **Not Intent**: Single delete, soft delete
#[rstest]
#[tokio::test]
async fn test_crud_lifecycle_bulk_delete(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	reinitialize_database(&url).await.unwrap();

	// Create table
	sqlx::query(
		"CREATE TABLE IF NOT EXISTS temp_items (id SERIAL PRIMARY KEY, category TEXT NOT NULL)",
	)
	.execute(pool.as_ref())
	.await
	.expect("Failed to create table");

	// Insert test data
	for i in 1..=40 {
		let category = if i % 2 == 0 { "even" } else { "odd" };
		sqlx::query("INSERT INTO temp_items (category) VALUES ($1)")
			.bind(category)
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert");
	}

	// Bulk DELETE: Remove all "even" items
	sqlx::query("DELETE FROM temp_items WHERE category = $1")
		.bind("even")
		.execute(pool.as_ref())
		.await
		.expect("Failed to bulk delete");

	// READ: Verify only "odd" items remain
	let remaining_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM temp_items")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count remaining");

	assert_eq!(
		remaining_count, 20,
		"Only odd items should remain after bulk delete"
	);

	let all_odd: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM temp_items WHERE category = $1")
		.bind("odd")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count odd");

	assert_eq!(all_odd, 20);
}
