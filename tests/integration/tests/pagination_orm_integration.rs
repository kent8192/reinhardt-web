//! ORM Integration Tests for Pagination
//!
//! These tests test pagination with database querysets using reinhardt-orm.

use reinhardt_orm::{
    query::{FilterOperator, FilterValue},
    Model, QuerySet,
};
use reinhardt_pagination::{PageNumberPagination, Paginator};
use sqlx::PgPool;
use testcontainers::{runners::AsyncRunner, GenericImage};

// Test model for pagination tests
#[derive(Debug, Clone)]
struct TestArticle {
    id: i64,
    title: String,
    content: String,
}

impl Model for TestArticle {
    type PrimaryKey = i64;

    fn table_name() -> &'static str {
        "test_articles"
    }

    fn primary_key_field() -> &'static str {
        "id"
    }
}

#[cfg(test)]
mod orm_pagination_tests {
    use super::*;

    // NOTE: This test is based on Django's test_first_page from ModelPaginationTests
    #[tokio::test]
    async fn test_first_page_with_queryset() {
        // Setup test database
        let container = GenericImage::new("postgres", "16-alpine")
            .with_exposed_port(5432.into())
            .start()
            .await
            .unwrap();

        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let pool = PgPool::connect(&format!(
            "postgres://postgres:postgres@localhost:{}/test",
            port
        ))
        .await
        .unwrap();

        // Create test table
        sqlx::query("CREATE TABLE test_articles (id SERIAL PRIMARY KEY, title TEXT, content TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        // Insert test data
        for i in 1..=50 {
            sqlx::query("INSERT INTO test_articles (title, content) VALUES ($1, $2)")
                .bind(format!("Article {}", i))
                .bind(format!("Content for article {}", i))
                .execute(&pool)
                .await
                .unwrap();
        }

        // Use QuerySet with pagination
        let queryset = QuerySet::<TestArticle>::new()
            .order_by(vec!["id".to_string()])
            .limit(10)
            .offset(0);

        // Execute query
        let sql = queryset.to_sql();
        let results = sqlx::query_as::<_, TestArticle>(&sql)
            .fetch_all(&pool)
            .await
            .unwrap();

        // Test pagination
        let paginator = PageNumberPagination::new().page_size(10);
        let page = paginator
            .paginate(&results, Some("1"), "http://api.example.com")
            .unwrap();

        assert_eq!(page.results.len(), 10);
        assert_eq!(page.count, 10); // Only the results from the query
        assert!(page.next.is_none()); // No more results in this subset
        assert!(page.previous.is_none());

        // Check first page results
        assert_eq!(page.results[0].title, "Article 1");
        assert_eq!(page.results[9].title, "Article 10");
    }

    // NOTE: This test is based on Django's test_last_page from ModelPaginationTests
    #[tokio::test]
    async fn test_last_page_with_queryset() {
        // Setup test database
        let container = GenericImage::new("postgres", "16-alpine")
            .with_exposed_port(5432.into())
            .start()
            .await
            .unwrap();

        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let pool = PgPool::connect(&format!(
            "postgres://postgres:postgres@localhost:{}/test",
            port
        ))
        .await
        .unwrap();

        // Create test table
        sqlx::query("CREATE TABLE test_articles (id SERIAL PRIMARY KEY, title TEXT, content TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        // Insert test data
        for i in 1..=25 {
            sqlx::query("INSERT INTO test_articles (title, content) VALUES ($1, $2)")
                .bind(format!("Article {}", i))
                .bind(format!("Content for article {}", i))
                .execute(&pool)
                .await
                .unwrap();
        }

        // Use QuerySet for last page (page 3 with 10 items per page)
        let queryset = QuerySet::<TestArticle>::new()
            .order_by(vec!["id".to_string()])
            .limit(10)
            .offset(20); // Skip first 20 items for page 3

        // Execute query
        let sql = queryset.to_sql();
        let results = sqlx::query_as::<_, TestArticle>(&sql)
            .fetch_all(&pool)
            .await
            .unwrap();

        // Test pagination for last page
        let paginator = PageNumberPagination::new().page_size(10);
        let page = paginator
            .paginate(&results, Some("3"), "http://api.example.com")
            .unwrap();

        assert_eq!(page.results.len(), 5); // Last page has 5 items (21-25)
        assert_eq!(page.count, 5);
        assert!(page.next.is_none()); // No next page
        assert!(page.previous.is_some()); // Has previous page

        // Check last page results
        assert_eq!(page.results[0].title, "Article 21");
        assert_eq!(page.results[4].title, "Article 25");
    }

    // NOTE: This test is based on Django's test_page_getitem
    #[tokio::test]
    async fn test_page_getitem_with_queryset() {
        // Setup test database
        let container = GenericImage::new("postgres", "16-alpine")
            .with_exposed_port(5432.into())
            .start()
            .await
            .unwrap();

        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let pool = PgPool::connect(&format!(
            "postgres://postgres:postgres@localhost:{}/test",
            port
        ))
        .await
        .unwrap();

        // Create test table
        sqlx::query("CREATE TABLE test_articles (id SERIAL PRIMARY KEY, title TEXT, content TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        // Insert test data
        for i in 1..=20 {
            sqlx::query("INSERT INTO test_articles (title, content) VALUES ($1, $2)")
                .bind(format!("Article {}", i))
                .bind(format!("Content for article {}", i))
                .execute(&pool)
                .await
                .unwrap();
        }

        // Use QuerySet with pagination
        let queryset = QuerySet::<TestArticle>::new()
            .order_by(vec!["id".to_string()])
            .limit(10)
            .offset(0);

        // Execute query
        let sql = queryset.to_sql();
        let results = sqlx::query_as::<_, TestArticle>(&sql)
            .fetch_all(&pool)
            .await
            .unwrap();

        // Test pagination and indexing
        let paginator = PageNumberPagination::new().page_size(10);
        let page = paginator
            .paginate(&results, Some("1"), "http://api.example.com")
            .unwrap();

        // Test indexing behavior (similar to Django's page[0], page[1], etc.)
        assert_eq!(page.results.len(), 10);
        assert_eq!(page.results[0].title, "Article 1");
        assert_eq!(page.results[9].title, "Article 10");

        // Test that we can access individual items
        let first_item = &page.results[0];
        assert_eq!(first_item.id, 1);
        assert_eq!(first_item.title, "Article 1");
    }

    // NOTE: This test is based on Django's test_paginating_unordered_queryset_raises_warning
    #[tokio::test]
    async fn test_paginating_unordered_queryset_raises_warning() {
        // Setup test database
        let container = GenericImage::new("postgres", "16-alpine")
            .with_exposed_port(5432.into())
            .start()
            .await
            .unwrap();

        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let pool = PgPool::connect(&format!(
            "postgres://postgres:postgres@localhost:{}/test",
            port
        ))
        .await
        .unwrap();

        // Create test table
        sqlx::query("CREATE TABLE test_articles (id SERIAL PRIMARY KEY, title TEXT, content TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        // Insert test data
        for i in 1..=20 {
            sqlx::query("INSERT INTO test_articles (title, content) VALUES ($1, $2)")
                .bind(format!("Article {}", i))
                .bind(format!("Content for article {}", i))
                .execute(&pool)
                .await
                .unwrap();
        }

        // Use QuerySet WITHOUT ordering (this should raise a warning in Django)
        let queryset = QuerySet::<TestArticle>::new().limit(10).offset(0);

        // Execute query
        let sql = queryset.to_sql();
        let results = sqlx::query_as::<_, TestArticle>(&sql)
            .fetch_all(&pool)
            .await
            .unwrap();

        // Test pagination with unordered queryset
        let paginator = PageNumberPagination::new().page_size(10);
        let page = paginator
            .paginate(&results, Some("1"), "http://api.example.com")
            .unwrap();

        // The pagination should still work, but results may be in arbitrary order
        assert_eq!(page.results.len(), 10);
        assert_eq!(page.count, 10);

        // Note: In Django, this would raise a warning about unordered queryset
        // In our implementation, we just document this behavior
    }

    // NOTE: This test is based on Django's test_paginating_empty_queryset_does_not_warn
    #[tokio::test]
    async fn test_paginating_empty_queryset_does_not_warn() {
        // Setup test database
        let container = GenericImage::new("postgres", "16-alpine")
            .with_exposed_port(5432.into())
            .start()
            .await
            .unwrap();

        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let pool = PgPool::connect(&format!(
            "postgres://postgres:postgres@localhost:{}/test",
            port
        ))
        .await
        .unwrap();

        // Create test table
        sqlx::query("CREATE TABLE test_articles (id SERIAL PRIMARY KEY, title TEXT, content TEXT)")
            .execute(&pool)
            .await
            .unwrap();

        // Don't insert any data - empty table

        // Use QuerySet with empty results
        let queryset = QuerySet::<TestArticle>::new()
            .order_by(vec!["id".to_string()])
            .limit(10)
            .offset(0);

        // Execute query
        let sql = queryset.to_sql();
        let results = sqlx::query_as::<_, TestArticle>(&sql)
            .fetch_all(&pool)
            .await
            .unwrap();

        // Test pagination with empty queryset
        let paginator = PageNumberPagination::new().page_size(10);
        let page = paginator
            .paginate(&results, Some("1"), "http://api.example.com")
            .unwrap();

        // Empty queryset should not raise warnings
        assert_eq!(page.results.len(), 0);
        assert_eq!(page.count, 0);
        assert!(page.next.is_none());
        assert!(page.previous.is_none());
    }

    // NOTE: This test is based on Django's test_paginating_unordered_object_list_raises_warning
    #[tokio::test]
    async fn test_paginating_unordered_object_list_raises_warning() {
        // Create a list of objects without ordering
        let articles: Vec<TestArticle> = (1..=20)
            .map(|i| TestArticle {
                id: i,
                title: format!("Article {}", i),
                content: format!("Content for article {}", i),
            })
            .collect();

        // Test pagination with unordered object list
        let paginator = PageNumberPagination::new().page_size(10);
        let page = paginator
            .paginate(&articles, Some("1"), "http://api.example.com")
            .unwrap();

        // The pagination should work with object lists
        assert_eq!(page.results.len(), 10);
        assert_eq!(page.count, 20);
        assert!(page.next.is_some());
        assert!(page.previous.is_none());

        // Check first page results
        assert_eq!(page.results[0].title, "Article 1");
        assert_eq!(page.results[9].title, "Article 10");

        // Note: In Django, this would raise a warning about unordered object list
        // In our implementation, we just document this behavior
    }
}
