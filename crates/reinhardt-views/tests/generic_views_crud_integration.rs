//! Generic API Views CRUD Integration Tests
//!
//! Tests comprehensive CRUD functionality for Generic API Views with actual database operations:
//! - ListAPIView: Displaying lists of objects (empty and populated)
//! - CreateAPIView: Creating new objects with validation
//! - RetrieveAPIView: Retrieving single objects by ID
//! - UpdateAPIView: Full updates (PUT) and partial updates (PATCH)
//! - DestroyAPIView: Deleting objects
//! - Composite views: ListCreateAPIView, RetrieveUpdateDestroyAPIView
//! - Default ordering behavior
//!
//! **Test Category**: Happy Path (正常系)
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container
//!
//! **Test Data Schema:**
//! - articles(id SERIAL PRIMARY KEY, title TEXT NOT NULL, content TEXT NOT NULL,
//!   published BOOLEAN NOT NULL, view_count INT NOT NULL, created_at TIMESTAMP)

use bytes::Bytes;
use chrono::{DateTime, Utc};
use hyper::{HeaderMap, Method, StatusCode, Version};
use reinhardt_core::http::{Request, Response};
use reinhardt_core::macros::model;
use reinhardt_db::orm::{Model, init_database};
use reinhardt_serializers::JsonSerializer;
use reinhardt_test::fixtures::postgres_container;
use reinhardt_test::testcontainers::{ContainerAsync, GenericImage};
use reinhardt_views::{
	CreateAPIView, DestroyAPIView, ListAPIView, ListCreateAPIView, RetrieveAPIView,
	RetrieveUpdateDestroyAPIView, UpdateAPIView, View,
};
use rstest::*;
use sea_query::{ColumnDef, Iden, PostgresQueryBuilder, Table};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serial_test::serial;
use sqlx::{PgPool, Row};
use std::sync::Arc;

// ============================================================================
// Model Definitions
// ============================================================================

/// Article model for CRUD testing
#[allow(dead_code)]
#[model(app_label = "views_test", table_name = "articles")]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Article {
	#[field(primary_key = true)]
	id: Option<i64>,
	#[field(max_length = 200)]
	title: String,
	#[field(max_length = 5000)]
	content: String,
	published: bool,
	view_count: i32,
	#[field(null = true)]
	created_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Table Identifiers (for SeaQuery operations)
// ============================================================================

#[derive(Iden)]
enum Articles {
	Table,
	Id,
	Title,
	Content,
	Published,
	ViewCount,
	CreatedAt,
}

// ============================================================================
// Fixtures
// ============================================================================

/// Fixture: Initialize database connection
///
/// Dependencies: postgres_container (testcontainers)
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

/// Fixture: Setup articles table
#[fixture]
async fn articles_table(#[future] db_pool: Arc<PgPool>) -> Arc<PgPool> {
	let pool = db_pool.await;

	// Create articles table using SeaQuery
	let create_table_stmt = Table::create()
		.table(Articles::Table)
		.if_not_exists()
		.col(
			ColumnDef::new(Articles::Id)
				.big_integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(Articles::Title).string_len(200).not_null())
		.col(ColumnDef::new(Articles::Content).text().not_null())
		.col(
			ColumnDef::new(Articles::Published)
				.boolean()
				.not_null()
				.default(false),
		)
		.col(
			ColumnDef::new(Articles::ViewCount)
				.integer()
				.not_null()
				.default(0),
		)
		.col(ColumnDef::new(Articles::CreatedAt).timestamp())
		.to_owned();

	let sql = create_table_stmt.to_string(PostgresQueryBuilder);
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create articles table");

	pool
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Helper: Insert test articles directly into database
async fn insert_test_articles(pool: &PgPool, count: usize) -> Vec<Article> {
	let mut articles = Vec::new();

	for i in 1..=count {
		let article = Article::new(
			format!("Article {}", i),
			format!("Content for article {}", i),
			i % 2 == 0, // Even articles are published
			(i * 10) as i32,
			Some(Utc::now()),
		);

		let sql = "INSERT INTO articles (title, content, published, view_count, created_at) VALUES ($1, $2, $3, $4, $5) RETURNING id";
		let row = sqlx::query(sql)
			.bind(&article.title)
			.bind(&article.content)
			.bind(article.published)
			.bind(article.view_count)
			.bind(article.created_at)
			.fetch_one(pool)
			.await
			.expect("Failed to insert article");

		let id: i64 = row.get(0);
		let mut inserted_article = article;
		inserted_article.id = Some(id);
		articles.push(inserted_article);
	}

	articles
}

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

/// Helper: Create HTTP POST request with JSON body
fn create_post_request(uri: &str, json_body: &str) -> Request {
	Request::builder()
		.method(Method::POST)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::from(json_body.to_string()))
		.build()
		.expect("Failed to build request")
}

/// Helper: Create HTTP PUT request with JSON body
fn create_put_request(uri: &str, json_body: &str) -> Request {
	Request::builder()
		.method(Method::PUT)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::from(json_body.to_string()))
		.build()
		.expect("Failed to build request")
}

/// Helper: Create HTTP PATCH request with JSON body
fn create_patch_request(uri: &str, json_body: &str) -> Request {
	Request::builder()
		.method(Method::PATCH)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::from(json_body.to_string()))
		.build()
		.expect("Failed to build request")
}

/// Helper: Create HTTP DELETE request
fn create_delete_request(uri: &str) -> Request {
	Request::builder()
		.method(Method::DELETE)
		.uri(uri)
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.expect("Failed to build request")
}

/// Helper: Parse JSON response body
fn parse_json_response(response: &Response) -> Value {
	let body_str = String::from_utf8(response.body.to_vec()).expect("Invalid UTF-8 in response");
	serde_json::from_str(&body_str).expect("Invalid JSON in response")
}

// ============================================================================
// Tests
// ============================================================================

/// Test: ListAPIView returns empty list when no data exists
#[rstest]
#[tokio::test]
#[serial(generic_views_crud)]
async fn test_list_api_view_empty_list(#[future] articles_table: Arc<PgPool>) {
	let _pool = articles_table.await;

	let view = ListAPIView::<Article, JsonSerializer<Article>>::new();
	let request = create_get_request("/articles/");

	let response = view.dispatch(request).await.expect("Failed to dispatch");

	assert_eq!(response.status, StatusCode::OK);

	let json = parse_json_response(&response);
	assert!(json.is_array());
	assert_eq!(json.as_array().unwrap().len(), 0);
}

/// Test: ListAPIView returns list with multiple items
#[rstest]
#[tokio::test]
#[serial(generic_views_crud)]
async fn test_list_api_view_with_items(#[future] articles_table: Arc<PgPool>) {
	let pool = articles_table.await;

	// Insert 3 test articles
	let inserted = insert_test_articles(&pool, 3).await;

	let view = ListAPIView::<Article, JsonSerializer<Article>>::new();
	let request = create_get_request("/articles/");

	let response = view.dispatch(request).await.expect("Failed to dispatch");

	assert_eq!(response.status, StatusCode::OK);

	let json = parse_json_response(&response);
	assert!(json.is_array());
	let articles = json.as_array().unwrap();
	assert_eq!(articles.len(), 3);

	// Verify first article data
	assert_eq!(articles[0]["title"], inserted[0].title);
	assert_eq!(articles[0]["content"], inserted[0].content);
}

/// Test: CreateAPIView creates new article with valid data
#[rstest]
#[tokio::test]
#[serial(generic_views_crud)]
async fn test_create_api_view_success(#[future] articles_table: Arc<PgPool>) {
	let pool = articles_table.await;

	let view = CreateAPIView::<Article, JsonSerializer<Article>>::new();
	let json_body =
		r#"{"title":"New Article","content":"New content","published":true,"view_count":5}"#;
	let request = create_post_request("/articles/", json_body);

	let response = view.dispatch(request).await.expect("Failed to dispatch");

	assert_eq!(response.status, StatusCode::CREATED);

	// Verify article was saved to database
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM articles")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count articles");
	assert_eq!(count, 1);

	// Verify response contains Location header or body contains id
	let has_location = response.headers.get("Location").is_some();
	let body_str = String::from_utf8(response.body.to_vec()).unwrap();
	assert!(has_location || body_str.contains("id"));
}

/// Test: RetrieveAPIView retrieves article by ID
#[rstest]
#[tokio::test]
#[serial(generic_views_crud)]
async fn test_retrieve_api_view_success(#[future] articles_table: Arc<PgPool>) {
	let pool = articles_table.await;

	// Insert test article
	let inserted = insert_test_articles(&pool, 1).await;
	let article_id = inserted[0].id.unwrap();

	let view = RetrieveAPIView::<Article, JsonSerializer<Article>>::new();
	let request = create_get_request(&format!("/articles/{}/", article_id));

	let response = view.dispatch(request).await.expect("Failed to dispatch");

	assert_eq!(response.status, StatusCode::OK);

	let json = parse_json_response(&response);
	assert_eq!(json["id"], article_id);
	assert_eq!(json["title"], inserted[0].title);
	assert_eq!(json["content"], inserted[0].content);
}

/// Test: UpdateAPIView performs full update with PUT
#[rstest]
#[tokio::test]
#[serial(generic_views_crud)]
async fn test_update_api_view_put(#[future] articles_table: Arc<PgPool>) {
	let pool = articles_table.await;

	// Insert test article
	let inserted = insert_test_articles(&pool, 1).await;
	let article_id = inserted[0].id.unwrap();

	let view = UpdateAPIView::<Article, JsonSerializer<Article>>::new();
	let json_body = r#"{"title":"Updated Title","content":"Updated content","published":false,"view_count":100}"#;
	let request = create_put_request(&format!("/articles/{}/", article_id), json_body);

	let response = view.dispatch(request).await.expect("Failed to dispatch");

	assert_eq!(response.status, StatusCode::OK);

	// Verify all fields were updated in database
	let updated: (String, String, bool, i32) =
		sqlx::query_as("SELECT title, content, published, view_count FROM articles WHERE id = $1")
			.bind(article_id)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to fetch updated article");

	assert_eq!(updated.0, "Updated Title");
	assert_eq!(updated.1, "Updated content");
	assert_eq!(updated.2, false);
	assert_eq!(updated.3, 100);
}

/// Test: UpdateAPIView performs partial update with PATCH
#[rstest]
#[tokio::test]
#[serial(generic_views_crud)]
async fn test_update_api_view_patch(#[future] articles_table: Arc<PgPool>) {
	let pool = articles_table.await;

	// Insert test article
	let inserted = insert_test_articles(&pool, 1).await;
	let article_id = inserted[0].id.unwrap();
	let original_content = inserted[0].content.clone();

	let view = UpdateAPIView::<Article, JsonSerializer<Article>>::new().with_partial(true);
	let json_body = r#"{"title":"Patched Title"}"#; // Only update title
	let request = create_patch_request(&format!("/articles/{}/", article_id), json_body);

	let response = view.dispatch(request).await.expect("Failed to dispatch");

	assert_eq!(response.status, StatusCode::OK);

	// Verify only title was updated, content remains unchanged
	let updated: (String, String) =
		sqlx::query_as("SELECT title, content FROM articles WHERE id = $1")
			.bind(article_id)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to fetch updated article");

	assert_eq!(updated.0, "Patched Title");
	assert_eq!(updated.1, original_content); // Content should be unchanged
}

/// Test: DestroyAPIView deletes article successfully
#[rstest]
#[tokio::test]
#[serial(generic_views_crud)]
async fn test_destroy_api_view_success(#[future] articles_table: Arc<PgPool>) {
	let pool = articles_table.await;

	// Insert test article
	let inserted = insert_test_articles(&pool, 1).await;
	let article_id = inserted[0].id.unwrap();

	let view = DestroyAPIView::<Article>::new();
	let request = create_delete_request(&format!("/articles/{}/", article_id));

	let response = view.dispatch(request).await.expect("Failed to dispatch");

	assert_eq!(response.status, StatusCode::NO_CONTENT);

	// Verify article was deleted from database
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM articles WHERE id = $1")
		.bind(article_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count articles");
	assert_eq!(count, 0);

	// Verify subsequent retrieve returns 404
	let retrieve_view = RetrieveAPIView::<Article, JsonSerializer<Article>>::new();
	let retrieve_request = create_get_request(&format!("/articles/{}/", article_id));
	let retrieve_response = retrieve_view.dispatch(retrieve_request).await;
	assert!(
		retrieve_response.is_err() || retrieve_response.unwrap().status == StatusCode::NOT_FOUND
	);
}

/// Test: ListCreateAPIView supports both GET and POST
#[rstest]
#[tokio::test]
#[serial(generic_views_crud)]
async fn test_list_create_composite(#[future] articles_table: Arc<PgPool>) {
	let pool = articles_table.await;

	let view = ListCreateAPIView::<Article, JsonSerializer<Article>>::new();

	// Test GET (list)
	let get_request = create_get_request("/articles/");
	let get_response = view
		.dispatch(get_request)
		.await
		.expect("Failed to dispatch GET");
	assert_eq!(get_response.status, StatusCode::OK);

	// Test POST (create)
	let json_body =
		r#"{"title":"Composite Test","content":"Test content","published":true,"view_count":0}"#;
	let post_request = create_post_request("/articles/", json_body);
	let post_response = view
		.dispatch(post_request)
		.await
		.expect("Failed to dispatch POST");
	assert_eq!(post_response.status, StatusCode::CREATED);

	// Verify article was created
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM articles")
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count articles");
	assert_eq!(count, 1);
}

/// Test: RetrieveUpdateDestroyAPIView supports GET, PUT, and DELETE
#[rstest]
#[tokio::test]
#[serial(generic_views_crud)]
async fn test_retrieve_update_destroy_composite(#[future] articles_table: Arc<PgPool>) {
	let pool = articles_table.await;

	// Insert test article
	let inserted = insert_test_articles(&pool, 1).await;
	let article_id = inserted[0].id.unwrap();

	let view = RetrieveUpdateDestroyAPIView::<Article, JsonSerializer<Article>>::new();

	// Test GET (retrieve)
	let get_request = create_get_request(&format!("/articles/{}/", article_id));
	let get_response = view
		.dispatch(get_request)
		.await
		.expect("Failed to dispatch GET");
	assert_eq!(get_response.status, StatusCode::OK);

	// Test PUT (update)
	let json_body = r#"{"title":"Updated via RUD","content":"Updated content","published":true,"view_count":50}"#;
	let put_request = create_put_request(&format!("/articles/{}/", article_id), json_body);
	let put_response = view
		.dispatch(put_request)
		.await
		.expect("Failed to dispatch PUT");
	assert_eq!(put_response.status, StatusCode::OK);

	// Test DELETE (destroy)
	let delete_request = create_delete_request(&format!("/articles/{}/", article_id));
	let delete_response = view
		.dispatch(delete_request)
		.await
		.expect("Failed to dispatch DELETE");
	assert_eq!(delete_response.status, StatusCode::NO_CONTENT);

	// Verify article was deleted
	let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM articles WHERE id = $1")
		.bind(article_id)
		.fetch_one(pool.as_ref())
		.await
		.expect("Failed to count articles");
	assert_eq!(count, 0);
}

/// Test: ListAPIView applies default ordering (by -created_at)
#[rstest]
#[tokio::test]
#[serial(generic_views_crud)]
async fn test_default_ordering(#[future] articles_table: Arc<PgPool>) {
	let pool = articles_table.await;

	// Insert articles with different timestamps
	let mut articles = Vec::new();
	for i in 1..=3 {
		let created_at = Utc::now() - chrono::Duration::seconds((3 - i) as i64);
		let article = Article::new(
			format!("Article {}", i),
			format!("Content {}", i),
			true,
			0,
			Some(created_at),
		);

		let sql = "INSERT INTO articles (title, content, published, view_count, created_at) VALUES ($1, $2, $3, $4, $5) RETURNING id";
		let row = sqlx::query(sql)
			.bind(&article.title)
			.bind(&article.content)
			.bind(article.published)
			.bind(article.view_count)
			.bind(article.created_at)
			.fetch_one(pool.as_ref())
			.await
			.expect("Failed to insert article");

		let id: i64 = row.get(0);
		let mut inserted = article;
		inserted.id = Some(id);
		articles.push(inserted);
	}

	let view = ListAPIView::<Article, JsonSerializer<Article>>::new()
		.with_ordering(vec!["-created_at".to_string()]);
	let request = create_get_request("/articles/");

	let response = view.dispatch(request).await.expect("Failed to dispatch");

	assert_eq!(response.status, StatusCode::OK);

	let json = parse_json_response(&response);
	let result_articles = json.as_array().unwrap();

	// Verify descending order by created_at (newest first)
	assert_eq!(result_articles[0]["title"], "Article 3"); // Most recent
	assert_eq!(result_articles[1]["title"], "Article 2");
	assert_eq!(result_articles[2]["title"], "Article 1"); // Oldest
}
