//! Soft Delete Integration Tests
//!
//! Tests soft delete functionality with reinhardt-orm high-level API:
//! - Lifecycle (create → delete → restore)
//! - deleted_at filtering
//! - State transitions
//! - Edge cases (multiple records, already deleted)
//!
//! **Test Categories**: Normal cases, Error cases, Edge cases, State transition cases
//!
//! **Fixtures Used**:
//! - postgres_container: PostgreSQL database container

use chrono::{DateTime, Utc};
use reinhardt_orm::manager::{get_connection, init_database};
use reinhardt_orm::query::{Filter, FilterOperator, FilterValue};
use reinhardt_orm::{Model, SoftDeletable};
use reinhardt_test::fixtures::postgres_container;
use rstest::*;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Model Definition (using reinhardt-orm Model trait)
// ============================================================================

/// Article model with soft delete support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
	pub id: Option<i32>,
	pub title: String,
	pub content: String,
	pub deleted_at: Option<DateTime<Utc>>,
}

impl Article {
	/// Create a new Article instance with auto-generated new function
	pub fn new(title: String, content: String) -> Self {
		Self {
			id: None,
			title,
			content,
			deleted_at: None,
		}
	}
}

reinhardt_test::impl_test_model!(Article, i32, "articles", "articles");

impl SoftDeletable for Article {
	fn deleted_at(&self) -> Option<DateTime<Utc>> {
		self.deleted_at
	}

	fn set_deleted_at(&mut self, time: Option<DateTime<Utc>>) {
		self.deleted_at = time;
	}
}

// ============================================================================
// Test Helpers
// ============================================================================

/// Create articles table for testing
async fn setup_articles_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS articles (
			id SERIAL PRIMARY KEY,
			title TEXT NOT NULL,
			content TEXT NOT NULL,
			deleted_at TIMESTAMPTZ
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create articles table");
}

// ============================================================================
// Normal Cases: Soft Delete Lifecycle Tests
// ============================================================================

/// Test soft delete lifecycle (create → delete → restore)
///
/// **Test Intent**: Verify that the complete soft delete lifecycle (create → delete → restore) works correctly
///
/// **Integration Point**: Manager create/update + SoftDeletable trait
///
/// **Not Testing**: Physical deletion, multiple record operations
///
/// **Category**: Normal case
#[rstest]
#[tokio::test]
async fn test_soft_delete_lifecycle(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_articles_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	// Step 1: Create article using Manager API
	let manager = Article::objects();
	let mut article = Article::new("Test Article".to_string(), "Test Content".to_string());
	article = manager
		.create_with_conn(&conn, &article)
		.await
		.expect("Failed to create article");

	// Verify article is not deleted initially
	assert!(!article.is_deleted(), "New article should not be deleted");
	assert!(
		article.deleted_at().is_none(),
		"New article should not have deleted_at timestamp"
	);

	// Step 2: Soft delete the article
	article.set_deleted_at(Some(Utc::now()));
	article = manager
		.update_with_conn(&conn, &article)
		.await
		.expect("Failed to soft delete article");

	// Verify article is now soft deleted
	assert!(article.is_deleted(), "Article should be marked as deleted");
	assert!(
		article.deleted_at().is_some(),
		"Article should have deleted_at timestamp"
	);

	// Step 3: Restore the article
	article.set_deleted_at(None);
	article = manager
		.update_with_conn(&conn, &article)
		.await
		.expect("Failed to restore article");

	// Verify article is restored (deleted_at is NULL)
	assert!(
		!article.is_deleted(),
		"Restored article should not be deleted"
	);
	assert!(
		article.deleted_at().is_none(),
		"Restored article should not have deleted_at timestamp"
	);
}

/// Test filtering active records (deleted_at IS NULL)
///
/// **Test Intent**: Verify that deleted records are excluded by deleted_at filtering
///
/// **Integration Point**: QuerySet filter with IS NULL check
///
/// **Not Testing**: Including deleted records, complex condition joins
///
/// **Category**: Normal case
#[rstest]
#[tokio::test]
async fn test_filter_active_records(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_articles_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Article::objects();

	// Insert 3 articles
	let article1 = Article::new("Active Article 1".to_string(), "Content 1".to_string());
	let article2 = Article::new("Active Article 2".to_string(), "Content 2".to_string());
	let mut article3 = Article::new("To Be Deleted".to_string(), "Content 3".to_string());

	let article1 = manager
		.create_with_conn(&conn, &article1)
		.await
		.expect("Failed to create article1");
	let article2 = manager
		.create_with_conn(&conn, &article2)
		.await
		.expect("Failed to create article2");
	article3 = manager
		.create_with_conn(&conn, &article3)
		.await
		.expect("Failed to create article3");

	// Soft delete one article
	article3.set_deleted_at(Some(Utc::now()));
	manager
		.update_with_conn(&conn, &article3)
		.await
		.expect("Failed to soft delete article3");

	// Query active records only using Filter
	let active_filter = Filter::new(
		"deleted_at".to_string(),
		FilterOperator::Eq,
		FilterValue::Null,
	);

	let queryset = manager.filter_by(active_filter);
	let active_articles = queryset
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch active articles");

	// Verify only 2 active articles are returned
	assert_eq!(
		active_articles.len(),
		2,
		"Should return only 2 active articles"
	);

	// Verify correct articles were returned
	let active_ids: Vec<i32> = active_articles.iter().filter_map(|a| a.id).collect();
	assert!(
		active_ids.contains(&article1.id.unwrap()),
		"Should include article1"
	);
	assert!(
		active_ids.contains(&article2.id.unwrap()),
		"Should include article2"
	);
	assert!(
		!active_ids.contains(&article3.id.unwrap()),
		"Should not include deleted article3"
	);
}

/// Test filtering deleted records (deleted_at IS NOT NULL)
///
/// **Test Intent**: Verify that only deleted records are retrieved by deleted_at filtering
///
/// **Integration Point**: QuerySet filter with IS NOT NULL check
///
/// **Not Testing**: Retrieving active records, physical deletion
///
/// **Category**: Normal case
#[rstest]
#[tokio::test]
async fn test_filter_deleted_records(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_articles_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Article::objects();

	// Insert 3 articles
	let article1 = Article::new("Active Article".to_string(), "Content 1".to_string());
	let mut article2 = Article::new("Deleted Article 1".to_string(), "Content 2".to_string());
	let mut article3 = Article::new("Deleted Article 2".to_string(), "Content 3".to_string());

	let _article1 = manager
		.create_with_conn(&conn, &article1)
		.await
		.expect("Failed to create article1");
	article2 = manager
		.create_with_conn(&conn, &article2)
		.await
		.expect("Failed to create article2");
	article3 = manager
		.create_with_conn(&conn, &article3)
		.await
		.expect("Failed to create article3");

	// Soft delete two articles
	article2.set_deleted_at(Some(Utc::now()));
	article2 = manager
		.update_with_conn(&conn, &article2)
		.await
		.expect("Failed to soft delete article2");

	article3.set_deleted_at(Some(Utc::now()));
	article3 = manager
		.update_with_conn(&conn, &article3)
		.await
		.expect("Failed to soft delete article3");

	// Query deleted records only using Filter (IS NOT NULL)
	let deleted_filter = Filter::new(
		"deleted_at".to_string(),
		FilterOperator::Ne,
		FilterValue::Null,
	);

	let queryset = manager.filter_by(deleted_filter);
	let deleted_articles = queryset
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch deleted articles");

	// Verify only 2 deleted articles are returned
	assert_eq!(
		deleted_articles.len(),
		2,
		"Should return only 2 deleted articles"
	);

	// Verify correct articles were returned
	let deleted_ids: Vec<i32> = deleted_articles.iter().filter_map(|a| a.id).collect();
	assert!(
		deleted_ids.contains(&article2.id.unwrap()),
		"Should include deleted article2"
	);
	assert!(
		deleted_ids.contains(&article3.id.unwrap()),
		"Should include deleted article3"
	);
}

// ============================================================================
// Error Cases: Error Handling Tests
// ============================================================================

/// Test attempting to soft delete already deleted record
///
/// **Test Intent**: Verify behavior when attempting to delete an already deleted record
///
/// **Integration Point**: UPDATE with WHERE deleted_at IS NOT NULL check
///
/// **Not Testing**: Deleting new records, restore operations
///
/// **Category**: Error case
#[rstest]
#[tokio::test]
async fn test_soft_delete_already_deleted_record(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_articles_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Article::objects();

	// Create and soft delete article
	let mut article = Article::new("Test Article".to_string(), "Test Content".to_string());
	article = manager
		.create_with_conn(&conn, &article)
		.await
		.expect("Failed to create article");

	article.set_deleted_at(Some(Utc::now()));
	article = manager
		.update_with_conn(&conn, &article)
		.await
		.expect("Failed to soft delete article");

	// Get first deletion timestamp
	let first_deleted_at = article.deleted_at().expect("deleted_at should be set");

	// Wait a moment to ensure timestamp difference if updated
	tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

	// Attempt to soft delete again (idempotent operation)
	article.set_deleted_at(Some(Utc::now()));
	let result = manager.update_with_conn(&conn, &article).await;

	// Operation should succeed (idempotent)
	assert!(result.is_ok(), "Re-deleting should succeed");

	let updated_article = result.unwrap();
	let second_deleted_at = updated_article
		.deleted_at()
		.expect("deleted_at should still be set");

	// Timestamp should be updated (newer)
	assert!(
		second_deleted_at >= first_deleted_at,
		"Second deletion timestamp should be >= first"
	);
}

/// Test restoring non-deleted record
///
/// **Test Intent**: Verify behavior when attempting to restore a non-deleted record (idempotency)
///
/// **Integration Point**: UPDATE with WHERE deleted_at IS NULL check
///
/// **Not Testing**: Restoring deleted records, error occurrence
///
/// **Category**: Error case
#[rstest]
#[tokio::test]
async fn test_restore_non_deleted_record(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_articles_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Article::objects();

	// Create active article (not deleted)
	let mut article = Article::new("Active Article".to_string(), "Content".to_string());
	article = manager
		.create_with_conn(&conn, &article)
		.await
		.expect("Failed to create article");

	// Verify article is not deleted
	assert!(!article.is_deleted(), "Article should not be deleted");

	// Attempt to restore (should be no-op)
	article.set_deleted_at(None);
	let result = manager.update_with_conn(&conn, &article).await;

	// Operation should succeed (idempotent)
	assert!(result.is_ok(), "Restoring active record should succeed");

	let updated_article = result.unwrap();

	// Verify deleted_at is still NULL
	assert!(
		updated_article.deleted_at().is_none(),
		"Article should still not be deleted"
	);
}

// ============================================================================
// State Transition Cases: State Transition Tests
// ============================================================================

/// Test state transitions: Active → Deleted → Restored → Deleted
///
/// **Test Intent**: Verify that multiple state transitions (Active → Deleted → Restored → Deleted) work correctly
///
/// **Integration Point**: Multiple UPDATE operations with state tracking
///
/// **Not Testing**: Single transition, concurrent access
///
/// **Category**: State transition case
#[rstest]
#[tokio::test]
async fn test_multiple_state_transitions(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_articles_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Article::objects();

	let mut article = Article::new("Test Article".to_string(), "Content".to_string());
	article = manager
		.create_with_conn(&conn, &article)
		.await
		.expect("Failed to create article");

	// State 1: Active
	assert!(!article.is_deleted(), "Should start as Active");

	// State 2: Active → Deleted
	article.set_deleted_at(Some(Utc::now()));
	article = manager
		.update_with_conn(&conn, &article)
		.await
		.expect("Failed to delete");

	assert!(article.is_deleted(), "Should be Deleted");

	// State 3: Deleted → Restored
	article.set_deleted_at(None);
	article = manager
		.update_with_conn(&conn, &article)
		.await
		.expect("Failed to restore");

	assert!(!article.is_deleted(), "Should be Restored (Active)");

	// State 4: Restored → Deleted again
	article.set_deleted_at(Some(Utc::now()));
	article = manager
		.update_with_conn(&conn, &article)
		.await
		.expect("Failed to delete again");

	assert!(article.is_deleted(), "Should be Deleted again");
}

// ============================================================================
// Edge Cases: Edge Case Tests
// ============================================================================

/// Test bulk soft delete of multiple records
///
/// **Test Intent**: Verify that bulk soft deletion of multiple records works correctly
///
/// **Integration Point**: Bulk UPDATE operations
///
/// **Not Testing**: Single record deletion, transaction control
///
/// **Category**: Edge case
#[rstest]
#[tokio::test]
async fn test_bulk_soft_delete(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_articles_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Article::objects();

	// Insert 5 articles
	let mut articles = Vec::new();
	for i in 1..=5 {
		let article = Article::new(format!("Article {}", i), format!("Content {}", i));
		let created = manager
			.create_with_conn(&conn, &article)
			.await
			.expect("Failed to create article");
		articles.push(created);
	}

	// Bulk soft delete articles 2, 3, 4 (indices 1, 2, 3)
	let mut to_delete = vec![
		articles[1].clone(),
		articles[2].clone(),
		articles[3].clone(),
	];

	// Set deleted_at for each
	let now = Utc::now();
	for article in to_delete.iter_mut() {
		article.set_deleted_at(Some(now));
	}

	// Update each record
	for article in to_delete.iter() {
		manager
			.update_with_conn(&conn, article)
			.await
			.expect("Failed to bulk delete");
	}

	// Verify exactly 3 records were deleted
	let deleted_filter = Filter::new(
		"deleted_at".to_string(),
		FilterOperator::Ne,
		FilterValue::Null,
	);
	let deleted_articles_check = manager
		.filter_by(deleted_filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch deleted articles");

	assert_eq!(
		deleted_articles_check.len(),
		3,
		"Should have deleted exactly 3 articles"
	);
}

/// Test soft delete with concurrent modifications
///
/// **Test Intent**: Verify that consistency is maintained when soft deletion and content updates are performed on the same record
///
/// **Integration Point**: Multiple UPDATE operations on same record
///
/// **Not Testing**: Transaction isolation levels, lock control
///
/// **Category**: Edge case
#[rstest]
#[tokio::test]
async fn test_soft_delete_with_updates(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_articles_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Article::objects();

	let mut article = Article::new("Original Title".to_string(), "Original Content".to_string());
	article = manager
		.create_with_conn(&conn, &article)
		.await
		.expect("Failed to create article");

	// Soft delete the article
	article.set_deleted_at(Some(Utc::now()));
	article = manager
		.update_with_conn(&conn, &article)
		.await
		.expect("Failed to delete");

	// Update content while deleted
	article.title = "Updated Title".to_string();
	article.content = "Updated Content".to_string();
	article = manager
		.update_with_conn(&conn, &article)
		.await
		.expect("Failed to update");

	// Verify deleted_at is preserved and content is updated
	assert_eq!(article.title, "Updated Title", "Title should be updated");
	assert_eq!(
		article.content, "Updated Content",
		"Content should be updated"
	);
	assert!(
		article.deleted_at().is_some(),
		"deleted_at should still be set (not cleared by update)"
	);
}

/// Test querying with mixed active and deleted records
///
/// **Test Intent**: Verify that filtering works correctly when active and deleted records are mixed
///
/// **Integration Point**: Complex WHERE clauses with AND/OR + IS NULL/IS NOT NULL
///
/// **Not Testing**: Simple fetch all, single condition filter
///
/// **Category**: Edge case
#[rstest]
#[tokio::test]
async fn test_mixed_active_deleted_queries(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, url) = postgres_container.await;
	setup_articles_table(pool.as_ref()).await;

	// Initialize reinhardt-orm database connection
	init_database(&url)
		.await
		.expect("Failed to initialize database");
	let conn = get_connection().await.expect("Failed to get connection");

	let manager = Article::objects();

	// Insert 10 articles
	for i in 1..=10 {
		let article = Article::new(format!("Article {}", i), format!("Content {}", i));
		manager
			.create_with_conn(&conn, &article)
			.await
			.expect("Failed to create article");
	}

	// Delete articles with even IDs (2, 4, 6, 8, 10)
	// Note: We need to fetch them first, then update
	let all_articles = manager
		.all()
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch all articles");
	for mut article in all_articles {
		if let Some(id) = article.id {
			if id % 2 == 0 {
				article.set_deleted_at(Some(Utc::now()));
				manager
					.update_with_conn(&conn, &article)
					.await
					.expect("Failed to delete even articles");
			}
		}
	}

	// Query 1: All records (ignore deletion status)
	let total_articles = manager
		.all()
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch all articles");
	assert_eq!(total_articles.len(), 10, "Should have 10 total articles");

	// Query 2: Active only
	let active_filter = Filter::new(
		"deleted_at".to_string(),
		FilterOperator::Eq,
		FilterValue::Null,
	);
	let active_articles = manager
		.filter_by(active_filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch active articles");
	assert_eq!(active_articles.len(), 5, "Should have 5 active articles");

	// Query 3: Deleted only
	let deleted_filter = Filter::new(
		"deleted_at".to_string(),
		FilterOperator::Ne,
		FilterValue::Null,
	);
	let deleted_articles = manager
		.filter_by(deleted_filter)
		.all_with_db(&conn)
		.await
		.expect("Failed to fetch deleted articles");
	assert_eq!(deleted_articles.len(), 5, "Should have 5 deleted articles");
}
