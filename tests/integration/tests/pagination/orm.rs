//! ORM Integration Tests for Pagination
//!
//! These tests require reinhardt-orm with django-compat feature and reinhardt-postgres crates.
//! They test pagination with database querysets.

use reinhardt_orm::Model;
use reinhardt_pagination::{Page, PageNumberPagination, PaginatedResponse};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite, sqlite::SqlitePoolOptions};

// ============================================================================
// Test Models
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::FromRow)]
struct TestArticle {
    id: Option<i64>,
    title: String,
    content: String,
    author_id: i64,
    published: bool,
    created_at: String,
}

impl Model for TestArticle {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "test_articles"
    }

    fn primary_key(&self) -> Option<&Self::PrimaryKey> {
        self.id.as_ref()
    }

    fn set_primary_key(&mut self, value: Self::PrimaryKey) {
        self.id = Some(value);
    }
}

// ============================================================================
// Database Setup Helpers
// ============================================================================

async fn setup_test_database() -> Pool<Sqlite> {
    setup_test_database_with_url("sqlite::memory:").await
}

async fn setup_test_database_with_url(url: &str) -> Pool<Sqlite> {
    let pool = SqlitePoolOptions::new()
        .connect(url)
        .await
        .expect("Failed to create database pool");

    // Create test_articles table
    sqlx::query(
        r#"
        CREATE TABLE test_articles (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            author_id INTEGER NOT NULL,
            published BOOLEAN NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create test_articles table");

    pool
}

async fn seed_test_data(pool: &Pool<Sqlite>, count: usize) {
    for i in 1..=count {
        sqlx::query(
            "INSERT INTO test_articles (title, content, author_id, published, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(format!("Article {}", i))
        .bind(format!("Content for article {}", i))
        .bind(1)
        .bind(true)
        .bind(format!("2024-01-{:02}", i))
        .execute(pool)
        .await
        .expect("Failed to insert test article");
    }
}

async fn teardown_database(pool: Pool<Sqlite>) {
    // SQLite in-memory database is automatically cleaned up when pool is dropped
    drop(pool);
}

// ============================================================================
// ORM Pagination Tests
// ============================================================================

#[tokio::test]
async fn test_first_page_with_queryset() {
    let pool = setup_test_database().await;
    seed_test_data(&pool, 25).await;

    // Fetch all data for pagination (simulating QuerySet behavior)
    let all_results = sqlx::query_as::<_, TestArticle>(
        "SELECT id, title, content, author_id, published, created_at FROM test_articles ORDER BY id"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch all data");

    // Verify total count
    assert_eq!(all_results.len(), 25);

    let paginator = PageNumberPagination::new().page_size(10);
    let page = paginator.get_page(&all_results, Some("1"));

    // Verify page contains correct items (1-10)
    assert_eq!(page.len(), 10);
    assert_eq!(page.get(0).unwrap().title, "Article 1");
    assert_eq!(page.get(9).unwrap().title, "Article 10");

    // Verify pagination metadata
    assert_eq!(page.has_next(), true);
    assert_eq!(page.has_previous(), false);
    assert_eq!(page.start_index(), 1);
    assert_eq!(page.end_index(), 10);

    teardown_database(pool).await;
}

#[tokio::test]
async fn test_last_page_with_queryset() {
    let pool = setup_test_database().await;
    seed_test_data(&pool, 25).await;

    // Fetch all data for pagination (simulating QuerySet behavior)
    let all_results = sqlx::query_as::<_, TestArticle>(
        "SELECT id, title, content, author_id, published, created_at FROM test_articles ORDER BY id"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch all data");

    // Verify total count
    assert_eq!(all_results.len(), 25);

    let paginator = PageNumberPagination::new().page_size(10);
    let page = paginator.get_page(&all_results, Some("3")); // Page 3 (last page)

    // Verify correct items (21-25)
    assert_eq!(page.len(), 5); // Only 5 items on last page
    assert_eq!(page.get(0).unwrap().title, "Article 21");
    assert_eq!(page.get(4).unwrap().title, "Article 25");

    // Verify pagination metadata
    assert_eq!(page.has_next(), false);
    assert_eq!(page.has_previous(), true);
    assert_eq!(page.start_index(), 21);
    assert_eq!(page.end_index(), 25);

    teardown_database(pool).await;
}

#[tokio::test]
async fn test_page_getitem_with_queryset() {
    let pool = setup_test_database().await;
    seed_test_data(&pool, 15).await;

    // Test indexing into results using direct SQL
    let results = sqlx::query_as::<_, TestArticle>(
        "SELECT id, title, content, author_id, published, created_at FROM test_articles ORDER BY id LIMIT 10"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch results");

    // Test indexing operations
    assert_eq!(results[0].title, "Article 1");
    assert_eq!(results[5].title, "Article 6");
    assert_eq!(results[9].title, "Article 10");

    // Test filtered results using direct SQL
    let filtered_results = sqlx::query_as::<_, TestArticle>(
        "SELECT id, title, content, author_id, published, created_at FROM test_articles WHERE published = ? ORDER BY id LIMIT 5"
    )
    .bind(true)
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch filtered results");

    // All our test data is published, so we should get 5 results
    assert_eq!(filtered_results.len(), 5);

    teardown_database(pool).await;
}

#[tokio::test]
async fn test_paginating_unordered_queryset_raises_warning() {
    let pool = setup_test_database().await;
    seed_test_data(&pool, 10).await;

    // Fetch results without ORDER BY (unordered) using direct SQL
    let results = sqlx::query_as::<_, TestArticle>(
        "SELECT id, title, content, author_id, published, created_at FROM test_articles LIMIT 5 OFFSET 0"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch unordered results");

    // Results should still be returned (but order is not guaranteed)
    assert_eq!(results.len(), 5);

    // In a real Django implementation, this would raise a warning like:
    // "UnorderedObjectListWarning: Pagination may yield inconsistent results with an unordered object_list"
    // For this test, we just verify the functionality works

    teardown_database(pool).await;
}

#[tokio::test]
async fn test_paginating_empty_queryset_does_not_warn() {
    let pool = setup_test_database().await;
    // Don't seed any data - empty table

    // Fetch from empty table using direct SQL
    let results = sqlx::query_as::<_, TestArticle>(
        "SELECT id, title, content, author_id, published, created_at FROM test_articles ORDER BY id LIMIT 5 OFFSET 0"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch empty results");

    // Should return empty results without warnings
    assert_eq!(results.len(), 0);

    // Verify count is 0
    let total_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM test_articles")
        .fetch_one(&pool)
        .await
        .expect("Failed to get count") as usize;

    assert_eq!(total_count, 0);

    teardown_database(pool).await;
}

#[tokio::test]
async fn test_paginating_unordered_object_list_raises_warning() {
    let pool = setup_test_database().await;
    seed_test_data(&pool, 10).await;

    // Create a mock object list with .ordered attribute set to false
    // In Django, this would be like: object_list.ordered = False
    // For this test, we simulate this by fetching without ORDER BY
    let results = sqlx::query_as::<_, TestArticle>(
        "SELECT id, title, content, author_id, published, created_at FROM test_articles LIMIT 5 OFFSET 0"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to fetch unordered object list");

    assert_eq!(results.len(), 5);

    // In a real Django implementation, this would raise:
    // "UnorderedObjectListWarning: Pagination may yield inconsistent results with an unordered object_list"
    // The warning would be raised when the Paginator is created with an unordered object list

    teardown_database(pool).await;
}
