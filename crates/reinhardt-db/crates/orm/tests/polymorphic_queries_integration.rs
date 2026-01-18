//! ORM Polymorphic Relationship Integration Tests with Real Database
//!
//! These tests verify polymorphic relationship functionality with real PostgreSQL database.
//! Polymorphic relationships allow multiple different models to relate to a single entity
//! (e.g., Comments on both Articles and Posts).
//!
//! **Test Coverage:**
//! - Polymorphic ForeignKey creation with content_type pattern
//! - Generic FK queries across multiple content types
//! - Content type identification and filtering
//! - ExprTrait-based polymorphic queries
//! - Multiple relationships to same polymorphic table
//! - Constraint enforcement in polymorphic relationships
//! - JOINs with polymorphic content type tables
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container

use reinhardt_db::orm::manager::reinitialize_database;
use rstest::*;
use sea_query::{Alias, Expr, ExprTrait, Query};
use serde::{Deserialize, Serialize};
use serial_test::serial;
use sqlx::{PgPool, Row};
use std::sync::Arc;
use testcontainers::{ContainerAsync, runners::AsyncRunner};
use testcontainers_modules::postgres::Postgres;

type PostgresContainer = ContainerAsync<Postgres>;

#[fixture]
async fn polymorphic_test_db() -> (PostgresContainer, Arc<PgPool>, u16, String) {
	let postgres = Postgres::default()
		.start()
		.await
		.expect("Failed to start PostgreSQL container");

	let port = postgres
		.get_host_port_ipv4(5432)
		.await
		.expect("Failed to get PostgreSQL port");

	let database_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);

	reinitialize_database(&database_url).await.unwrap();

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

/// Represents a content type in the polymorphic relationship system
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct ContentType {
	id: Option<i32>,
	name: String,
	model_name: String,
}

/// Represents an article that can be commented on
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Article {
	id: Option<i32>,
	title: String,
	content: String,
}

/// Represents a post that can be commented on
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Post {
	id: Option<i32>,
	title: String,
	body: String,
}

/// Represents a comment that can be attached to any content type
#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Comment {
	id: Option<i32>,
	content: String,
	content_type_id: i32,
	object_id: i32,
}

// ============================================================================
// Polymorphic ForeignKey Integration Tests
// ============================================================================

/// Test basic polymorphic FK creation with content_type pattern
///
/// **Test Intent**: Verify polymorphic relationship tables are created correctly
/// with content_type identifier pattern
///
/// **Integration Point**: ORM polymorphic FK → PostgreSQL schema with content_type
///
/// **Not Intent**: Multi-table inheritance, complex polymorphism patterns
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_polymorphic_fk_creation(
	#[future] polymorphic_test_db: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = polymorphic_test_db.await;

	// Create content_type table
	sqlx::query(
		"CREATE TABLE content_types (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			model_name VARCHAR(255) NOT NULL UNIQUE
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Create content tables
	sqlx::query(
		"CREATE TABLE articles (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			content TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE posts (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			body TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Create polymorphic comments table
	sqlx::query(
		"CREATE TABLE comments (
			id SERIAL PRIMARY KEY,
			content TEXT NOT NULL,
			content_type_id INTEGER NOT NULL REFERENCES content_types(id),
			object_id INTEGER NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Verify tables were created
	let table_check = sqlx::query(
		"SELECT table_name FROM information_schema.tables
		 WHERE table_name IN ('content_types', 'articles', 'posts', 'comments')",
	)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(table_check.len(), 4);
}

/// Test polymorphic generic FK queries with content types
///
/// **Test Intent**: Verify querying comments for different content types works correctly
///
/// **Integration Point**: ORM polymorphic FK → PostgreSQL content_type filtering
///
/// **Not Intent**: Lazy loading, prefetch optimization
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_polymorphic_query_generic_fk(
	#[future] polymorphic_test_db: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = polymorphic_test_db.await;

	// Setup schema
	sqlx::query(
		"CREATE TABLE content_types (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			model_name VARCHAR(255) NOT NULL UNIQUE
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE articles (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			content TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE posts (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			body TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE comments (
			id SERIAL PRIMARY KEY,
			content TEXT NOT NULL,
			content_type_id INTEGER NOT NULL REFERENCES content_types(id),
			object_id INTEGER NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert content types
	let article_ct_id: i32 =
		sqlx::query("INSERT INTO content_types (name, model_name) VALUES ($1, $2) RETURNING id")
			.bind("article")
			.bind("Article")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	let post_ct_id: i32 =
		sqlx::query("INSERT INTO content_types (name, model_name) VALUES ($1, $2) RETURNING id")
			.bind("post")
			.bind("Post")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	// Insert content
	let article_id: i32 =
		sqlx::query("INSERT INTO articles (title, content) VALUES ($1, $2) RETURNING id")
			.bind("Article 1")
			.bind("Article content")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	let post_id: i32 = sqlx::query("INSERT INTO posts (title, body) VALUES ($1, $2) RETURNING id")
		.bind("Post 1")
		.bind("Post body")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert comments on different content types
	sqlx::query("INSERT INTO comments (content, content_type_id, object_id) VALUES ($1, $2, $3)")
		.bind("Comment on article")
		.bind(article_ct_id)
		.bind(article_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO comments (content, content_type_id, object_id) VALUES ($1, $2, $3)")
		.bind("Comment on post")
		.bind(post_ct_id)
		.bind(post_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Query comments for article content type
	let article_comments = sqlx::query(
		"SELECT c.content, ct.model_name FROM comments c
		 JOIN content_types ct ON c.content_type_id = ct.id
		 WHERE c.content_type_id = $1",
	)
	.bind(article_ct_id)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(article_comments.len(), 1);
	let content: String = article_comments[0].get("content");
	assert_eq!(content, "Comment on article");
}

/// Test accessing content via polymorphic relationship with JOIN
///
/// **Test Intent**: Verify JOINing polymorphic comments to their content works
///
/// **Integration Point**: ORM polymorphic FK → PostgreSQL JOIN with content_type
///
/// **Not Intent**: Complex join queries, aggregation
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_polymorphic_content_type_access(
	#[future] polymorphic_test_db: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = polymorphic_test_db.await;

	// Setup schema
	sqlx::query(
		"CREATE TABLE content_types (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			model_name VARCHAR(255) NOT NULL UNIQUE
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE articles (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			content TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE comments (
			id SERIAL PRIMARY KEY,
			content TEXT NOT NULL,
			content_type_id INTEGER NOT NULL REFERENCES content_types(id),
			object_id INTEGER NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert content type
	let ct_id: i32 =
		sqlx::query("INSERT INTO content_types (name, model_name) VALUES ($1, $2) RETURNING id")
			.bind("article")
			.bind("Article")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	// Insert article
	let article_id: i32 =
		sqlx::query("INSERT INTO articles (title, content) VALUES ($1, $2) RETURNING id")
			.bind("Deep Dive")
			.bind("This is detailed content")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	// Insert comment
	sqlx::query("INSERT INTO comments (content, content_type_id, object_id) VALUES ($1, $2, $3)")
		.bind("Great article!")
		.bind(ct_id)
		.bind(article_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Access content via polymorphic comment
	let result = sqlx::query(
		"SELECT c.content as comment_text, a.title, ct.model_name
		 FROM comments c
		 JOIN content_types ct ON c.content_type_id = ct.id
		 JOIN articles a ON c.object_id = a.id AND ct.id = c.content_type_id
		 WHERE c.id = (SELECT id FROM comments LIMIT 1)",
	)
	.fetch_one(pool.as_ref())
	.await
	.unwrap();

	let comment_text: String = result.get("comment_text");
	let title: String = result.get("title");
	let model_name: String = result.get("model_name");

	assert_eq!(comment_text, "Great article!");
	assert_eq!(title, "Deep Dive");
	assert_eq!(model_name, "Article");
}

/// Test polymorphic queries with ExprTrait for type-safe filtering
///
/// **Test Intent**: Verify ExprTrait-based polymorphic filtering works correctly
///
/// **Integration Point**: SeaQuery ExprTrait → Polymorphic content filtering
///
/// **Not Intent**: Complex expression building, optimization
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_polymorphic_query_with_expr(
	#[future] polymorphic_test_db: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = polymorphic_test_db.await;

	// Setup schema
	sqlx::query(
		"CREATE TABLE content_types (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			model_name VARCHAR(255) NOT NULL UNIQUE
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE articles (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			content TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE posts (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			body TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE comments (
			id SERIAL PRIMARY KEY,
			content TEXT NOT NULL,
			content_type_id INTEGER NOT NULL REFERENCES content_types(id),
			object_id INTEGER NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert content types
	let article_ct_id: i32 =
		sqlx::query("INSERT INTO content_types (name, model_name) VALUES ($1, $2) RETURNING id")
			.bind("article")
			.bind("Article")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	let post_ct_id: i32 =
		sqlx::query("INSERT INTO content_types (name, model_name) VALUES ($1, $2) RETURNING id")
			.bind("post")
			.bind("Post")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	// Insert content
	let article_id: i32 =
		sqlx::query("INSERT INTO articles (title, content) VALUES ($1, $2) RETURNING id")
			.bind("Article 1")
			.bind("Article content")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	let post_id: i32 = sqlx::query("INSERT INTO posts (title, body) VALUES ($1, $2) RETURNING id")
		.bind("Post 1")
		.bind("Post body")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert comments
	sqlx::query("INSERT INTO comments (content, content_type_id, object_id) VALUES ($1, $2, $3)")
		.bind("Comment on article")
		.bind(article_ct_id)
		.bind(article_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO comments (content, content_type_id, object_id) VALUES ($1, $2, $3)")
		.bind("Comment on post")
		.bind(post_ct_id)
		.bind(post_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Use ExprTrait to build type-safe query
	let mut select_stmt = Query::select();
	select_stmt
		.column((Alias::new("c"), Alias::new("content")))
		.column((Alias::new("c"), Alias::new("content_type_id")))
		.from_as(Alias::new("comments"), Alias::new("c"));

	// Filter using ExprTrait with AND condition
	select_stmt
		.and_where(Expr::col((Alias::new("c"), Alias::new("content_type_id"))).eq(article_ct_id));

	// Execute the built query
	let sql = select_stmt.to_string(sea_query::PostgresQueryBuilder);
	let results = sqlx::query(&sql).fetch_all(pool.as_ref()).await.unwrap();

	assert_eq!(results.len(), 1);
	let content: String = results[0].get("content");
	assert_eq!(content, "Comment on article");
}

/// Test multiple polymorphic relationships to same content type
///
/// **Test Intent**: Verify multiple models can relate to same polymorphic comments
///
/// **Integration Point**: ORM polymorphic FK → PostgreSQL multi-model references
///
/// **Not Intent**: Self-referential relationships, complex graphs
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_polymorphic_multiple_relations(
	#[future] polymorphic_test_db: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = polymorphic_test_db.await;

	// Setup schema
	sqlx::query(
		"CREATE TABLE content_types (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			model_name VARCHAR(255) NOT NULL UNIQUE
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE articles (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			content TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE posts (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			body TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE videos (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			url VARCHAR(500) NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE comments (
			id SERIAL PRIMARY KEY,
			content TEXT NOT NULL,
			content_type_id INTEGER NOT NULL REFERENCES content_types(id),
			object_id INTEGER NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert content types
	let article_ct: i32 =
		sqlx::query("INSERT INTO content_types (name, model_name) VALUES ($1, $2) RETURNING id")
			.bind("article")
			.bind("Article")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	let post_ct: i32 =
		sqlx::query("INSERT INTO content_types (name, model_name) VALUES ($1, $2) RETURNING id")
			.bind("post")
			.bind("Post")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	let video_ct: i32 =
		sqlx::query("INSERT INTO content_types (name, model_name) VALUES ($1, $2) RETURNING id")
			.bind("video")
			.bind("Video")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	// Insert content
	let article_id: i32 =
		sqlx::query("INSERT INTO articles (title, content) VALUES ($1, $2) RETURNING id")
			.bind("Article")
			.bind("Article content")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	let post_id: i32 = sqlx::query("INSERT INTO posts (title, body) VALUES ($1, $2) RETURNING id")
		.bind("Post")
		.bind("Post body")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	let video_id: i32 = sqlx::query("INSERT INTO videos (title, url) VALUES ($1, $2) RETURNING id")
		.bind("Video")
		.bind("https://example.com/video")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert comments on different content types
	sqlx::query("INSERT INTO comments (content, content_type_id, object_id) VALUES ($1, $2, $3)")
		.bind("Article comment 1")
		.bind(article_ct)
		.bind(article_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO comments (content, content_type_id, object_id) VALUES ($1, $2, $3)")
		.bind("Article comment 2")
		.bind(article_ct)
		.bind(article_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO comments (content, content_type_id, object_id) VALUES ($1, $2, $3)")
		.bind("Post comment")
		.bind(post_ct)
		.bind(post_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO comments (content, content_type_id, object_id) VALUES ($1, $2, $3)")
		.bind("Video comment")
		.bind(video_ct)
		.bind(video_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Verify each content type has correct number of comments
	let article_comments =
		sqlx::query("SELECT COUNT(*) as cnt FROM comments WHERE content_type_id = $1")
			.bind(article_ct)
			.fetch_one(pool.as_ref())
			.await
			.unwrap();

	let article_count: i64 = article_comments.get("cnt");
	assert_eq!(article_count, 2);

	let post_comments =
		sqlx::query("SELECT COUNT(*) as cnt FROM comments WHERE content_type_id = $1")
			.bind(post_ct)
			.fetch_one(pool.as_ref())
			.await
			.unwrap();

	let post_count: i64 = post_comments.get("cnt");
	assert_eq!(post_count, 1);

	let total = sqlx::query("SELECT COUNT(*) as cnt FROM comments")
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	let total_count: i64 = total.get("cnt");
	assert_eq!(total_count, 4);
}

/// Test polymorphic constraint enforcement
///
/// **Test Intent**: Verify foreign key constraints are enforced for content_type references
///
/// **Integration Point**: ORM polymorphic FK → PostgreSQL referential integrity
///
/// **Not Intent**: Data validation, business logic constraints
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_polymorphic_constraint_enforcement(
	#[future] polymorphic_test_db: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = polymorphic_test_db.await;

	// Setup schema
	sqlx::query(
		"CREATE TABLE content_types (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			model_name VARCHAR(255) NOT NULL UNIQUE
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE comments (
			id SERIAL PRIMARY KEY,
			content TEXT NOT NULL,
			content_type_id INTEGER NOT NULL REFERENCES content_types(id),
			object_id INTEGER NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Try to insert comment with non-existent content_type_id
	let result = sqlx::query(
		"INSERT INTO comments (content, content_type_id, object_id) VALUES ($1, $2, $3)",
	)
	.bind("Invalid comment")
	.bind(999)
	.bind(1)
	.execute(pool.as_ref())
	.await;

	// Constraint violation should be caught
	assert!(result.is_err());

	// Verify constraint violation reason
	match result {
		Err(e) => {
			let error_msg = e.to_string();
			assert!(error_msg.contains("foreign key") || error_msg.contains("constraint"));
		}
		Ok(_) => panic!("Expected constraint violation"),
	}
}

/// Test polymorphic JOINs with content type and content tables
///
/// **Test Intent**: Verify complex JOINs work for polymorphic relationships
///
/// **Integration Point**: ORM polymorphic FK → PostgreSQL multi-table JOIN
///
/// **Not Intent**: Join optimization, query planning
#[rstest]
#[tokio::test]
#[serial(orm_db)]
async fn test_polymorphic_join_with_content_type(
	#[future] polymorphic_test_db: (PostgresContainer, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = polymorphic_test_db.await;

	// Setup schema
	sqlx::query(
		"CREATE TABLE content_types (
			id SERIAL PRIMARY KEY,
			name VARCHAR(255) NOT NULL,
			model_name VARCHAR(255) NOT NULL UNIQUE
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE articles (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			content TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE posts (
			id SERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			body TEXT NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	sqlx::query(
		"CREATE TABLE comments (
			id SERIAL PRIMARY KEY,
			content TEXT NOT NULL,
			content_type_id INTEGER NOT NULL REFERENCES content_types(id),
			object_id INTEGER NOT NULL
		)",
	)
	.execute(pool.as_ref())
	.await
	.unwrap();

	// Insert content types
	let article_ct: i32 =
		sqlx::query("INSERT INTO content_types (name, model_name) VALUES ($1, $2) RETURNING id")
			.bind("article")
			.bind("Article")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	let post_ct: i32 =
		sqlx::query("INSERT INTO content_types (name, model_name) VALUES ($1, $2) RETURNING id")
			.bind("post")
			.bind("Post")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	// Insert content
	let article_id: i32 =
		sqlx::query("INSERT INTO articles (title, content) VALUES ($1, $2) RETURNING id")
			.bind("Test Article")
			.bind("Article content here")
			.fetch_one(pool.as_ref())
			.await
			.unwrap()
			.get("id");

	let post_id: i32 = sqlx::query("INSERT INTO posts (title, body) VALUES ($1, $2) RETURNING id")
		.bind("Test Post")
		.bind("Post body here")
		.fetch_one(pool.as_ref())
		.await
		.unwrap()
		.get("id");

	// Insert comments
	sqlx::query("INSERT INTO comments (content, content_type_id, object_id) VALUES ($1, $2, $3)")
		.bind("Great article!")
		.bind(article_ct)
		.bind(article_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	sqlx::query("INSERT INTO comments (content, content_type_id, object_id) VALUES ($1, $2, $3)")
		.bind("Nice post!")
		.bind(post_ct)
		.bind(post_id)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Complex JOIN to get comments with their content
	let results = sqlx::query(
		"SELECT c.id, c.content as comment, ct.model_name,
		        COALESCE(a.title, p.title) as content_title
		 FROM comments c
		 JOIN content_types ct ON c.content_type_id = ct.id
		 LEFT JOIN articles a ON ct.id = $1 AND c.object_id = a.id
		 LEFT JOIN posts p ON ct.id = $2 AND c.object_id = p.id
		 ORDER BY c.id",
	)
	.bind(article_ct)
	.bind(post_ct)
	.fetch_all(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(results.len(), 2);

	// Verify article comment
	let first_comment: String = results[0].get("comment");
	let first_title: String = results[0].get("content_title");
	let first_type: String = results[0].get("model_name");

	assert_eq!(first_comment, "Great article!");
	assert_eq!(first_title, "Test Article");
	assert_eq!(first_type, "Article");

	// Verify post comment
	let second_comment: String = results[1].get("comment");
	let second_title: String = results[1].get("content_title");
	let second_type: String = results[1].get("model_name");

	assert_eq!(second_comment, "Nice post!");
	assert_eq!(second_title, "Test Post");
	assert_eq!(second_type, "Post");
}
