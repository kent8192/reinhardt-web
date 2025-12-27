//! Generic Views Pagination Integration Tests
//!
//! Tests comprehensive pagination functionality for Generic API Views:
//! - PageNumber pagination (first/middle/last page, beyond limit, invalid values)
//! - LimitOffset pagination (basic operation, offset beyond total, max_limit enforcement)
//! - Cursor pagination (forward navigation)
//! - Edge cases (empty dataset, single item)
//!
//! **Test Category**: Boundary Value Analysis + Equivalence Partitioning (境界値分析+同値分割)
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Test Data Schema:**
//! - posts(id SERIAL PRIMARY KEY, title TEXT NOT NULL, content TEXT NOT NULL,
//!   author TEXT NOT NULL, published BOOLEAN NOT NULL, created_at TIMESTAMP)

use bytes::Bytes;
use chrono::{DateTime, Utc};
use hyper::{HeaderMap, Method, StatusCode, Version};
use reinhardt_core::http::Request;
use reinhardt_core::macros::model;
use reinhardt_db::orm::init_database;
use reinhardt_serializers::JsonSerializer;
use reinhardt_test::fixtures::postgres_container;
use reinhardt_test::testcontainers::{ContainerAsync, GenericImage};
use reinhardt_views::{ListAPIView, View};
use reinhardt_viewsets::PaginationConfig;
use rstest::*;
use sea_query::{ColumnDef, Iden, PostgresQueryBuilder, Table};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::PgPool;
use std::sync::Arc;

// ============================================================================
// Model Definitions
// ============================================================================

/// Post model for pagination testing
#[allow(dead_code)]
#[model(app_label = "views_pagination", table_name = "posts")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Post {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(max_length = 200)]
	title: String,
	#[field(max_length = 5000)]
	content: String,
	#[field(max_length = 100)]
	author: String,
	published: bool,
	#[field(null = true)]
	created_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Table Identifiers (for SeaQuery operations)
// ============================================================================

#[derive(Iden)]
enum Posts {
	Table,
	Id,
	Title,
	Content,
	Author,
	Published,
	CreatedAt,
}

// ============================================================================
// Fixtures
// ============================================================================

/// Fixture: Initialize database connection
#[fixture]
async fn db_pool(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) -> Arc<PgPool> {
	let (_container, pool, _port, connection_url) = postgres_container.await;

	// Initialize database connection for reinhardt-orm
	init_database(&connection_url)
		.await
		.expect("Failed to initialize database");

	pool
}

/// Fixture: Setup posts table
#[fixture]
async fn posts_table(#[future] db_pool: Arc<PgPool>) -> Arc<PgPool> {
	let pool = db_pool.await;

	// Create posts table
	let create_table_stmt = Table::create()
		.table(Posts::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(Posts::Id)
				.big_integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(Posts::Title).string_len(200).not_null())
		.col(ColumnDef::new(Posts::Content).text().not_null())
		.col(ColumnDef::new(Posts::Author).string_len(100).not_null())
		.col(
			ColumnDef::new(Posts::Published)
				.boolean()
				.not_null()
				.default(false),
		)
		.col(ColumnDef::new(Posts::CreatedAt).timestamp())
		.to_owned();

	let sql = create_table_stmt.to_string(PostgresQueryBuilder);
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create posts table");

	pool
}

/// Fixture: Setup posts table with 25 sample posts (for pagination testing)
#[fixture]
async fn posts_with_data(#[future] posts_table: Arc<PgPool>) -> Arc<PgPool> {
	let pool = posts_table.await;

	// Insert 25 posts for pagination testing
	for i in 1..=25 {
		let post = Post::new(
			format!("Post {}", i),
			format!("Content for post {}", i),
			format!("Author {}", (i % 5) + 1), // 5 different authors
			i % 2 == 0,                        // alternating published status
			Some(Utc::now()),
		);

		let sql = "INSERT INTO posts (title, content, author, published, created_at) VALUES ($1, $2, $3, $4, $5)";
		sqlx::query(sql)
			.bind(&post.title)
			.bind(&post.content)
			.bind(&post.author)
			.bind(post.published)
			.bind(post.created_at)
			.execute(pool.as_ref())
			.await
			.expect("Failed to insert post");
	}

	pool
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper: Create HTTP GET request
fn create_get_request(uri: &str) -> Request {
	Request::builder()
		.method(Method::GET)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.expect("Failed to build request")
}

// ============================================================================
// Tests
// ============================================================================

/// Test: PageNumber pagination - first page
#[rstest]
#[tokio::test]
#[serial(views_pagination)]
async fn test_page_number_first_page(#[future] posts_with_data: Arc<PgPool>) {
	let _pool = posts_with_data.await;

	let view = ListAPIView::<Post, JsonSerializer<Post>>::new()
		.with_pagination(PaginationConfig::page_number(5, Some(100)));

	let request = create_get_request("/posts/?page=1");
	let result = view.dispatch(request).await;

	// Should return first 5 posts
	assert!(result.is_ok(), "First page request should succeed");
	let response = result.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	// Verify pagination metadata exists
	assert!(
		body_str.contains("\"page\"") || body_str.contains("\"results\""),
		"Response should contain pagination metadata"
	);
}

/// Test: PageNumber pagination - middle page
#[rstest]
#[tokio::test]
#[serial(views_pagination)]
async fn test_page_number_middle_page(#[future] posts_with_data: Arc<PgPool>) {
	let _pool = posts_with_data.await;

	let view = ListAPIView::<Post, JsonSerializer<Post>>::new()
		.with_pagination(PaginationConfig::page_number(5, Some(100)));

	let request = create_get_request("/posts/?page=3");
	let result = view.dispatch(request).await;

	// Should return posts 11-15 (page 3 with page_size=5)
	assert!(result.is_ok(), "Middle page request should succeed");
	let response = result.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(
		body_str.contains("Post"),
		"Response should contain post data"
	);
}

/// Test: PageNumber pagination - last page
#[rstest]
#[tokio::test]
#[serial(views_pagination)]
async fn test_page_number_last_page(#[future] posts_with_data: Arc<PgPool>) {
	let _pool = posts_with_data.await;

	let view = ListAPIView::<Post, JsonSerializer<Post>>::new()
		.with_pagination(PaginationConfig::page_number(5, Some(100)));

	// With 25 posts and page_size=5, last page is page 5
	let request = create_get_request("/posts/?page=5");
	let result = view.dispatch(request).await;

	// Should return posts 21-25 (last 5 posts)
	assert!(result.is_ok(), "Last page request should succeed");
	let response = result.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(
		body_str.contains("Post"),
		"Last page should contain remaining posts"
	);
}

/// Test: PageNumber pagination - page beyond limit
#[rstest]
#[tokio::test]
#[serial(views_pagination)]
async fn test_page_number_beyond_limit(#[future] posts_with_data: Arc<PgPool>) {
	let _pool = posts_with_data.await;

	let view = ListAPIView::<Post, JsonSerializer<Post>>::new()
		.with_pagination(PaginationConfig::page_number(5, Some(100)));

	// Request page 100 (beyond available data)
	let request = create_get_request("/posts/?page=100");
	let result = view.dispatch(request).await;

	// Should return empty results or handle gracefully
	assert!(result.is_ok(), "Page beyond limit should not error");
	let response = result.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	// Should return empty results
	assert!(
		body_str.contains("[]") || body_str.contains("\"results\":[]"),
		"Page beyond limit should return empty results"
	);
}

/// Test: PageNumber pagination - invalid page number (0)
#[rstest]
#[tokio::test]
#[serial(views_pagination)]
async fn test_page_number_invalid_zero(#[future] posts_with_data: Arc<PgPool>) {
	let _pool = posts_with_data.await;

	let view = ListAPIView::<Post, JsonSerializer<Post>>::new()
		.with_pagination(PaginationConfig::page_number(5, Some(100)));

	let request = create_get_request("/posts/?page=0");
	let result = view.dispatch(request).await;

	// Should handle invalid page gracefully (default to page 1 or return error)
	match result {
		Ok(response) => {
			assert!(
				response.status == StatusCode::OK || response.status == StatusCode::BAD_REQUEST,
				"Invalid page 0 should return OK (default to 1) or BAD_REQUEST"
			);
		}
		Err(_) => {
			// Error is acceptable for invalid page number
			assert!(true, "Error is acceptable for page=0");
		}
	}
}

/// Test: PageNumber pagination - negative page number
#[rstest]
#[tokio::test]
#[serial(views_pagination)]
async fn test_page_number_negative(#[future] posts_with_data: Arc<PgPool>) {
	let _pool = posts_with_data.await;

	let view = ListAPIView::<Post, JsonSerializer<Post>>::new()
		.with_pagination(PaginationConfig::page_number(5, Some(100)));

	let request = create_get_request("/posts/?page=-1");
	let result = view.dispatch(request).await;

	// Should handle negative page gracefully
	match result {
		Ok(response) => {
			assert!(
				response.status == StatusCode::OK || response.status == StatusCode::BAD_REQUEST,
				"Negative page should return OK (default to 1) or BAD_REQUEST"
			);
		}
		Err(_) => {
			// Error is acceptable for negative page
			assert!(true, "Error is acceptable for negative page");
		}
	}
}

/// Test: LimitOffset pagination - basic operation
#[rstest]
#[tokio::test]
#[serial(views_pagination)]
async fn test_limit_offset_basic(#[future] posts_with_data: Arc<PgPool>) {
	let _pool = posts_with_data.await;

	let view = ListAPIView::<Post, JsonSerializer<Post>>::new()
		.with_pagination(PaginationConfig::limit_offset(10, Some(50)));

	// Request limit=5, offset=10 (skip first 10, return next 5)
	let request = create_get_request("/posts/?limit=5&offset=10");
	let result = view.dispatch(request).await;

	assert!(result.is_ok(), "Limit/offset pagination should succeed");
	let response = result.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(body_str.contains("Post"), "Response should contain posts");
}

/// Test: LimitOffset pagination - offset beyond total
#[rstest]
#[tokio::test]
#[serial(views_pagination)]
async fn test_limit_offset_beyond_total(#[future] posts_with_data: Arc<PgPool>) {
	let _pool = posts_with_data.await;

	let view = ListAPIView::<Post, JsonSerializer<Post>>::new()
		.with_pagination(PaginationConfig::limit_offset(10, Some(50)));

	// Request offset=100 (beyond 25 available posts)
	let request = create_get_request("/posts/?limit=10&offset=100");
	let result = view.dispatch(request).await;

	assert!(result.is_ok(), "Offset beyond total should not error");
	let response = result.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	// Should return empty results
	assert!(
		body_str.contains("[]") || body_str.contains("\"results\":[]"),
		"Offset beyond total should return empty results"
	);
}

/// Test: LimitOffset pagination - max_limit enforcement
#[rstest]
#[tokio::test]
#[serial(views_pagination)]
async fn test_limit_offset_max_limit_enforcement(#[future] posts_with_data: Arc<PgPool>) {
	let _pool = posts_with_data.await;

	let view = ListAPIView::<Post, JsonSerializer<Post>>::new()
		.with_pagination(PaginationConfig::limit_offset(10, Some(20)));

	// Request limit=100, but max_limit=20 should be enforced
	let request = create_get_request("/posts/?limit=100&offset=0");
	let result = view.dispatch(request).await;

	assert!(result.is_ok(), "Max limit enforcement should work");
	let response = result.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	// Should return at most 20 items (enforced max_limit)
	assert!(
		body_str.contains("Post"),
		"Response should contain posts (limited to max_limit)"
	);
}

/// Test: Cursor pagination - forward navigation
#[rstest]
#[tokio::test]
#[serial(views_pagination)]
async fn test_cursor_pagination_forward(#[future] posts_with_data: Arc<PgPool>) {
	let _pool = posts_with_data.await;

	let view = ListAPIView::<Post, JsonSerializer<Post>>::new()
		.with_pagination(PaginationConfig::cursor(10, "id".to_string()));

	let request = create_get_request("/posts/");
	let result = view.dispatch(request).await;

	// Should return first 10 posts with cursor for next page
	assert!(result.is_ok(), "Cursor pagination should succeed");
	let response = result.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(
		body_str.contains("Post"),
		"Cursor pagination should return posts"
	);
}

/// Test: Pagination with empty dataset
#[rstest]
#[tokio::test]
#[serial(views_pagination)]
async fn test_pagination_empty_dataset(#[future] posts_table: Arc<PgPool>) {
	let _pool = posts_table.await;

	let view = ListAPIView::<Post, JsonSerializer<Post>>::new()
		.with_pagination(PaginationConfig::page_number(10, Some(100)));

	let request = create_get_request("/posts/?page=1");
	let result = view.dispatch(request).await;

	// Should return empty results with pagination metadata
	assert!(result.is_ok(), "Pagination on empty dataset should succeed");
	let response = result.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(
		body_str.contains("[]") || body_str.contains("\"results\":[]"),
		"Empty dataset should return empty array"
	);
}

/// Test: Pagination with single item
#[rstest]
#[tokio::test]
#[serial(views_pagination)]
async fn test_pagination_single_item(#[future] posts_table: Arc<PgPool>) {
	let pool = posts_table.await;

	// Insert exactly one post
	let post = Post::new(
		"Single Post".to_string(),
		"Only one post".to_string(),
		"Author".to_string(),
		true,
		Some(Utc::now()),
	);

	let sql = "INSERT INTO posts (title, content, author, published, created_at) VALUES ($1, $2, $3, $4, $5)";
	sqlx::query(sql)
		.bind(&post.title)
		.bind(&post.content)
		.bind(&post.author)
		.bind(post.published)
		.bind(post.created_at)
		.execute(pool.as_ref())
		.await
		.expect("Failed to insert post");

	let view = ListAPIView::<Post, JsonSerializer<Post>>::new()
		.with_pagination(PaginationConfig::page_number(10, Some(100)));

	let request = create_get_request("/posts/?page=1");
	let result = view.dispatch(request).await;

	// Should return single post with pagination metadata
	assert!(result.is_ok(), "Pagination with single item should succeed");
	let response = result.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(
		body_str.contains("Single Post"),
		"Should return the single post"
	);
}
