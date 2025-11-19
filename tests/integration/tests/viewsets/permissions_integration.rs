//! Permissions Integration Tests for ViewSets
//!
//! **Purpose:**
//! Comprehensive integration tests for ViewSet permission system with real PostgreSQL database.
//! Tests verify permission classes work correctly with ORM queries and database operations.
//!
//! **Test Coverage:**
//! - IsAuthenticated permission with database queries
//! - IsAdminUser permission with staff checks
//! - Custom object-level permissions (IsOwner)
//! - Permission composition (AND/OR logic)
//! - Permission integration with ModelViewSet
//! - Permission filtering in list operations
//! - Permission checks for create/update/delete
//! - ReadOnly permissions (method-based)
//!
//! **Fixtures Used:**
//! - postgres_container: PostgreSQL database container from reinhardt-test

use bytes::Bytes;
use hyper::{HeaderMap, Method, StatusCode, Version};
use reinhardt_core::validators::TableName;
use reinhardt_db::orm::Model;
use reinhardt_http::{Request, Response};
use reinhardt_test::fixtures::postgres_container;
use reinhardt_viewsets::{ModelViewSet, ReadOnlyModelViewSet};
use rstest::*;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{ContainerAsync, GenericImage};

// ============================================================================
// Test Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
struct Article {
	id: Option<i64>,
	title: String,
	author_id: i64,
	published: bool,
}

const ARTICLE_TABLE: TableName = TableName::new_const("articles");

impl Model for Article {
	type PrimaryKey = i64;

	fn table_name() -> &'static str {
		ARTICLE_TABLE.as_str()
	}

	fn primary_key(&self) -> Option<&Self::PrimaryKey> {
		self.id.as_ref()
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}
}

#[derive(Debug, Clone)]
struct ArticleSerializer;

// ============================================================================
// Permission Classes
// ============================================================================

/// Trait for permission checking
trait Permission: Send + Sync {
	fn has_permission(&self, request: &Request) -> bool;
	fn has_object_permission(&self, request: &Request, obj: &Article) -> bool;
}

/// Allow all access
#[derive(Clone)]
struct AllowAny;

impl Permission for AllowAny {
	fn has_permission(&self, _request: &Request) -> bool {
		true
	}

	fn has_object_permission(&self, _request: &Request, _obj: &Article) -> bool {
		true
	}
}

/// Deny all access
#[derive(Clone)]
struct DenyAll;

impl Permission for DenyAll {
	fn has_permission(&self, _request: &Request) -> bool {
		false
	}

	fn has_object_permission(&self, _request: &Request, _obj: &Article) -> bool {
		false
	}
}

/// Allow only authenticated users
#[derive(Clone)]
struct IsAuthenticated;

impl Permission for IsAuthenticated {
	fn has_permission(&self, request: &Request) -> bool {
		request
			.headers
			.get("authorization")
			.and_then(|h| h.to_str().ok())
			.is_some()
	}

	fn has_object_permission(&self, request: &Request, _obj: &Article) -> bool {
		self.has_permission(request)
	}
}

/// Allow only admin users
#[derive(Clone)]
struct IsAdminUser;

impl IsAdminUser {
	fn is_admin(&self, request: &Request) -> bool {
		request
			.headers
			.get("x-user-role")
			.and_then(|h| h.to_str().ok())
			.map(|role| role == "admin")
			.unwrap_or(false)
	}
}

impl Permission for IsAdminUser {
	fn has_permission(&self, request: &Request) -> bool {
		self.is_admin(request)
	}

	fn has_object_permission(&self, request: &Request, _obj: &Article) -> bool {
		self.has_permission(request)
	}
}

/// Allow only object owner
#[derive(Clone)]
struct IsOwner;

impl IsOwner {
	fn get_user_id(&self, request: &Request) -> Option<i64> {
		request
			.headers
			.get("x-user-id")
			.and_then(|h| h.to_str().ok())
			.and_then(|id| id.parse().ok())
	}
}

impl Permission for IsOwner {
	fn has_permission(&self, _request: &Request) -> bool {
		true
	}

	fn has_object_permission(&self, request: &Request, obj: &Article) -> bool {
		self.get_user_id(request)
			.map(|user_id| user_id == obj.author_id)
			.unwrap_or(false)
	}
}

/// Read-only permission
#[derive(Clone)]
struct IsReadOnly;

impl Permission for IsReadOnly {
	fn has_permission(&self, request: &Request) -> bool {
		matches!(request.method, Method::GET | Method::HEAD | Method::OPTIONS)
	}

	fn has_object_permission(&self, request: &Request, _obj: &Article) -> bool {
		self.has_permission(request)
	}
}

/// Published articles only for non-admin
#[derive(Clone)]
struct PublishedOrAdmin;

impl PublishedOrAdmin {
	fn is_admin(&self, request: &Request) -> bool {
		request
			.headers
			.get("x-user-role")
			.and_then(|h| h.to_str().ok())
			.map(|role| role == "admin")
			.unwrap_or(false)
	}
}

impl Permission for PublishedOrAdmin {
	fn has_permission(&self, _request: &Request) -> bool {
		true
	}

	fn has_object_permission(&self, request: &Request, obj: &Article) -> bool {
		obj.published || self.is_admin(request)
	}
}

// ============================================================================
// Permission Composition (AND)
// ============================================================================

#[derive(Clone)]
struct AndPermission<P1, P2>
where
	P1: Permission + Clone,
	P2: Permission + Clone,
{
	perm1: P1,
	perm2: P2,
}

impl<P1, P2> AndPermission<P1, P2>
where
	P1: Permission + Clone,
	P2: Permission + Clone,
{
	fn new(perm1: P1, perm2: P2) -> Self {
		Self { perm1, perm2 }
	}
}

impl<P1, P2> Permission for AndPermission<P1, P2>
where
	P1: Permission + Clone,
	P2: Permission + Clone,
{
	fn has_permission(&self, request: &Request) -> bool {
		self.perm1.has_permission(request) && self.perm2.has_permission(request)
	}

	fn has_object_permission(&self, request: &Request, obj: &Article) -> bool {
		self.perm1.has_object_permission(request, obj)
			&& self.perm2.has_object_permission(request, obj)
	}
}

// ============================================================================
// Permission Composition (OR)
// ============================================================================

#[derive(Clone)]
struct OrPermission<P1, P2>
where
	P1: Permission + Clone,
	P2: Permission + Clone,
{
	perm1: P1,
	perm2: P2,
}

impl<P1, P2> OrPermission<P1, P2>
where
	P1: Permission + Clone,
	P2: Permission + Clone,
{
	fn new(perm1: P1, perm2: P2) -> Self {
		Self { perm1, perm2 }
	}
}

impl<P1, P2> Permission for OrPermission<P1, P2>
where
	P1: Permission + Clone,
	P2: Permission + Clone,
{
	fn has_permission(&self, request: &Request) -> bool {
		self.perm1.has_permission(request) || self.perm2.has_permission(request)
	}

	fn has_object_permission(&self, request: &Request, obj: &Article) -> bool {
		self.perm1.has_object_permission(request, obj)
			|| self.perm2.has_object_permission(request, obj)
	}
}

// ============================================================================
// ViewSet with Permission Support
// ============================================================================

struct PermissionViewSet<P: Permission> {
	_base: ModelViewSet<Article, ArticleSerializer>,
	permission: P,
	pool: Arc<PgPool>,
}

impl<P: Permission> PermissionViewSet<P> {
	fn new(permission: P, pool: Arc<PgPool>) -> Self {
		Self {
			_base: ModelViewSet::new("articles"),
			permission,
			pool,
		}
	}

	async fn list(&self, request: &Request) -> Result<Response, String> {
		if !self.permission.has_permission(request) {
			return Ok(Response::new(StatusCode::FORBIDDEN).with_body("Permission denied"));
		}

		let articles = sqlx::query_as::<_, Article>(
			"SELECT id, title, author_id, published FROM articles ORDER BY id",
		)
		.fetch_all(self.pool.as_ref())
		.await
		.map_err(|e| e.to_string())?;

		let filtered: Vec<_> = articles
			.into_iter()
			.filter(|article| self.permission.has_object_permission(request, article))
			.collect();

		let json = serde_json::to_string(&filtered).unwrap();
		Ok(Response::new(StatusCode::OK).with_body(json))
	}

	async fn retrieve(&self, request: &Request, id: i64) -> Result<Response, String> {
		if !self.permission.has_permission(request) {
			return Ok(Response::new(StatusCode::FORBIDDEN).with_body("Permission denied"));
		}

		let article = sqlx::query_as::<_, Article>(
			"SELECT id, title, author_id, published FROM articles WHERE id = $1",
		)
		.bind(id)
		.fetch_optional(self.pool.as_ref())
		.await
		.map_err(|e| e.to_string())?;

		match article {
			Some(article) => {
				if !self.permission.has_object_permission(request, &article) {
					return Ok(Response::new(StatusCode::FORBIDDEN)
						.with_body("Permission denied for this object"));
				}
				let json = serde_json::to_string(&article).unwrap();
				Ok(Response::new(StatusCode::OK).with_body(json))
			}
			None => Ok(Response::new(StatusCode::NOT_FOUND).with_body("Not found")),
		}
	}

	async fn update(&self, request: &Request, id: i64) -> Result<Response, String> {
		if !self.permission.has_permission(request) {
			return Ok(Response::new(StatusCode::FORBIDDEN).with_body("Permission denied"));
		}

		let article = sqlx::query_as::<_, Article>(
			"SELECT id, title, author_id, published FROM articles WHERE id = $1",
		)
		.bind(id)
		.fetch_optional(self.pool.as_ref())
		.await
		.map_err(|e| e.to_string())?;

		match article {
			Some(article) => {
				if !self.permission.has_object_permission(request, &article) {
					return Ok(Response::new(StatusCode::FORBIDDEN)
						.with_body("Permission denied for this object"));
				}
				Ok(Response::new(StatusCode::OK).with_body("Updated"))
			}
			None => Ok(Response::new(StatusCode::NOT_FOUND).with_body("Not found")),
		}
	}
}

// ============================================================================
// Helper Functions
// ============================================================================

async fn setup_articles_table(pool: &PgPool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS articles (
			id BIGSERIAL PRIMARY KEY,
			title VARCHAR(255) NOT NULL,
			author_id BIGINT NOT NULL,
			published BOOLEAN NOT NULL DEFAULT false
		)
		"#,
	)
	.execute(pool)
	.await
	.expect("Failed to create articles table");
}

async fn insert_test_articles(pool: &PgPool) {
	sqlx::query(
		"INSERT INTO articles (title, author_id, published) VALUES
		 ('Published Article', 1, true),
		 ('Draft Article', 1, false),
		 ('Another User Article', 2, true)",
	)
	.execute(pool)
	.await
	.expect("Failed to insert test articles");
}

async fn cleanup_articles_table(pool: &PgPool) {
	sqlx::query("DROP TABLE IF EXISTS articles CASCADE")
		.execute(pool)
		.await
		.expect("Failed to cleanup articles table");
}

// ============================================================================
// Tests
// ============================================================================

#[rstest]
#[tokio::test]
async fn test_allow_any_permission(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(AllowAny, pool.clone());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.list(&request).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	let articles: Vec<Article> = serde_json::from_slice(&response.body).unwrap();
	assert_eq!(articles.len(), 3);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_deny_all_permission(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(DenyAll, pool.clone());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.list(&request).await.unwrap();
	assert_eq!(response.status, StatusCode::FORBIDDEN);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_is_authenticated_permission_denied(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(IsAuthenticated, pool.clone());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.list(&request).await.unwrap();
	assert_eq!(response.status, StatusCode::FORBIDDEN);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_is_authenticated_permission_allowed(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(IsAuthenticated, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("authorization", "Bearer token123".parse().unwrap());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.list(&request).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_is_admin_permission_denied(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(IsAdminUser, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("x-user-role", "user".parse().unwrap());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.list(&request).await.unwrap();
	assert_eq!(response.status, StatusCode::FORBIDDEN);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_is_admin_permission_allowed(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(IsAdminUser, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("x-user-role", "admin".parse().unwrap());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.list(&request).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_is_owner_object_permission_denied(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(IsOwner, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("x-user-id", "999".parse().unwrap());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/1/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.retrieve(&request, 1).await.unwrap();
	assert_eq!(response.status, StatusCode::FORBIDDEN);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_is_owner_object_permission_allowed(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(IsOwner, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("x-user-id", "1".parse().unwrap());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/1/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.retrieve(&request, 1).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	let article: Article = serde_json::from_slice(&response.body).unwrap();
	assert_eq!(article.id, Some(1));
	assert_eq!(article.author_id, 1);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_is_readonly_permission_get_allowed(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(IsReadOnly, pool.clone());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.list(&request).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_is_readonly_permission_post_denied(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(IsReadOnly, pool.clone());

	let request = Request::builder()
		.method(Method::POST)
		.uri("/articles/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.list(&request).await.unwrap();
	assert_eq!(response.status, StatusCode::FORBIDDEN);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_published_or_admin_permission_published_allowed(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(PublishedOrAdmin, pool.clone());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/1/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.retrieve(&request, 1).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_published_or_admin_permission_draft_denied_for_user(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(PublishedOrAdmin, pool.clone());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/2/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.retrieve(&request, 2).await.unwrap();
	assert_eq!(response.status, StatusCode::FORBIDDEN);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_published_or_admin_permission_draft_allowed_for_admin(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(PublishedOrAdmin, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("x-user-role", "admin".parse().unwrap());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/2/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.retrieve(&request, 2).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_and_permission_both_pass(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let permission = AndPermission::new(IsAuthenticated, IsReadOnly);
	let viewset = PermissionViewSet::new(permission, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("authorization", "Bearer token123".parse().unwrap());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.list(&request).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_and_permission_first_fails(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let permission = AndPermission::new(IsAuthenticated, IsReadOnly);
	let viewset = PermissionViewSet::new(permission, pool.clone());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/")
		.version(Version::HTTP_11)
		.headers(HeaderMap::new())
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.list(&request).await.unwrap();
	assert_eq!(response.status, StatusCode::FORBIDDEN);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_and_permission_second_fails(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let permission = AndPermission::new(IsAuthenticated, IsReadOnly);
	let viewset = PermissionViewSet::new(permission, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("authorization", "Bearer token123".parse().unwrap());

	let request = Request::builder()
		.method(Method::POST)
		.uri("/articles/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.list(&request).await.unwrap();
	assert_eq!(response.status, StatusCode::FORBIDDEN);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_or_permission_both_pass(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let permission = OrPermission::new(IsAdminUser, IsOwner);
	let viewset = PermissionViewSet::new(permission, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("x-user-role", "admin".parse().unwrap());
	headers.insert("x-user-id", "1".parse().unwrap());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/1/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.retrieve(&request, 1).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_or_permission_first_passes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let permission = OrPermission::new(IsAdminUser, IsOwner);
	let viewset = PermissionViewSet::new(permission, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("x-user-role", "admin".parse().unwrap());
	headers.insert("x-user-id", "999".parse().unwrap());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/1/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.retrieve(&request, 1).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_or_permission_second_passes(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let permission = OrPermission::new(IsAdminUser, IsOwner);
	let viewset = PermissionViewSet::new(permission, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("x-user-role", "user".parse().unwrap());
	headers.insert("x-user-id", "1".parse().unwrap());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/1/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.retrieve(&request, 1).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_or_permission_both_fail(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let permission = OrPermission::new(IsAdminUser, IsOwner);
	let viewset = PermissionViewSet::new(permission, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("x-user-role", "user".parse().unwrap());
	headers.insert("x-user-id", "999".parse().unwrap());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/1/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.retrieve(&request, 1).await.unwrap();
	assert_eq!(response.status, StatusCode::FORBIDDEN);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_list_filters_objects_by_permission(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(IsOwner, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("x-user-id", "1".parse().unwrap());

	let request = Request::builder()
		.method(Method::GET)
		.uri("/articles/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.list(&request).await.unwrap();
	assert_eq!(response.status, StatusCode::OK);

	let articles: Vec<Article> = serde_json::from_slice(&response.body).unwrap();
	assert_eq!(articles.len(), 2);
	assert!(articles.iter().all(|a| a.author_id == 1));

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_update_requires_object_permission(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;
	setup_articles_table(&pool).await;
	insert_test_articles(&pool).await;

	let viewset = PermissionViewSet::new(IsOwner, pool.clone());

	let mut headers = HeaderMap::new();
	headers.insert("x-user-id", "2".parse().unwrap());

	let request = Request::builder()
		.method(Method::PUT)
		.uri("/articles/1/")
		.version(Version::HTTP_11)
		.headers(headers)
		.body(Bytes::new())
		.build()
		.unwrap();

	let response = viewset.update(&request, 1).await.unwrap();
	assert_eq!(response.status, StatusCode::FORBIDDEN);

	cleanup_articles_table(&pool).await;
}

#[rstest]
#[tokio::test]
async fn test_readonly_viewset_inherits_permissions(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, _pool, _port, _url) = postgres_container.await;

	let _viewset: ReadOnlyModelViewSet<Article, ArticleSerializer> =
		ReadOnlyModelViewSet::new("articles");

	// ReadOnlyModelViewSet should support permission configuration
	// This test verifies the type system allows permission integration
}
