//! Database integration tests (DI + DB + sea-query)
//!
//! Tests dependency injection with database operations:
//! 1. Database connection pool injection
//! 2. Repository pattern with DI
//! 3. Transaction scoped dependencies
//! 4. sea-query builder injection

use reinhardt_di::{DiResult, Injectable, InjectionContext, SingletonScope};
use reinhardt_test::fixtures::testcontainers::{ContainerAsync, GenericImage, postgres_container};
use rstest::*;
use sea_query::{Alias, ColumnDef, Expr, ExprTrait, PostgresQueryBuilder, Query, Table};
use sqlx::PgPool;
use std::sync::Arc;

// Database connection service
#[derive(Clone)]
struct DatabaseService {
	pool: Arc<PgPool>,
}

#[async_trait::async_trait]
impl Injectable for DatabaseService {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let pool_wrapper = ctx.get_singleton::<Arc<PgPool>>().ok_or_else(|| {
			reinhardt_di::DiError::NotFound("Database pool not found".to_string())
		})?;
		// Unwrap the outer Arc to get Arc<PgPool>
		let pool = Arc::clone(&*pool_wrapper);

		Ok(DatabaseService { pool })
	}
}

// User repository with DI
#[derive(Clone)]
struct UserRepository {
	db: DatabaseService,
}

impl UserRepository {
	async fn create_user(&self, name: &str) -> Result<i32, sqlx::Error> {
		let query = Query::insert()
			.into_table(Alias::new("users"))
			.columns([Alias::new("name")])
			.values_panic([name.into()])
			.returning_col(Alias::new("id"))
			.to_string(PostgresQueryBuilder);

		let row: (i32,) = sqlx::query_as(&query)
			.fetch_one(self.db.pool.as_ref())
			.await?;

		Ok(row.0)
	}

	async fn get_user(&self, id: i32) -> Result<String, sqlx::Error> {
		let query = Query::select()
			.column(Alias::new("name"))
			.from(Alias::new("users"))
			.and_where(Expr::col(Alias::new("id")).eq(id))
			.to_string(PostgresQueryBuilder);

		let row: (String,) = sqlx::query_as(&query)
			.fetch_one(self.db.pool.as_ref())
			.await?;

		Ok(row.0)
	}
}

#[async_trait::async_trait]
impl Injectable for UserRepository {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let db = DatabaseService::inject(ctx).await?;
		Ok(UserRepository { db })
	}
}

#[rstest]
#[tokio::test]
async fn test_inject_database_connection(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup DI context with pool
	let singleton = Arc::new(SingletonScope::new());
	singleton.set(pool.clone());
	let ctx = InjectionContext::builder(singleton).build();

	// Create users table using sea-query
	let create_table = Table::create()
		.table(Alias::new("users"))
		.col(
			ColumnDef::new(Alias::new("id"))
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(Alias::new("name")).string().not_null())
		.to_string(PostgresQueryBuilder);

	sqlx::query(&create_table)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Inject DatabaseService
	let service = DatabaseService::inject(&ctx).await.unwrap();

	// Verify pool is shared
	assert!(Arc::ptr_eq(&service.pool, &pool));
}

#[rstest]
#[tokio::test]
async fn test_repository_pattern_with_di(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup DI context
	let singleton = Arc::new(SingletonScope::new());
	singleton.set(pool.clone());
	let ctx = InjectionContext::builder(singleton).build();

	// Create users table
	let create_table = Table::create()
		.table(Alias::new("users"))
		.col(
			ColumnDef::new(Alias::new("id"))
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(Alias::new("name")).string().not_null())
		.to_string(PostgresQueryBuilder);

	sqlx::query(&create_table)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Inject UserRepository
	let repo = UserRepository::inject(&ctx).await.unwrap();

	// Create user
	let user_id = repo.create_user("Alice").await.unwrap();
	assert_eq!(user_id, 1);

	// Get user
	let name = repo.get_user(user_id).await.unwrap();
	assert_eq!(name, "Alice");
}

#[rstest]
#[tokio::test]
async fn test_transaction_scope(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Create users table
	let create_table = Table::create()
		.table(Alias::new("users"))
		.col(
			ColumnDef::new(Alias::new("id"))
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(Alias::new("name")).string().not_null())
		.to_string(PostgresQueryBuilder);

	sqlx::query(&create_table)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Start transaction
	let mut tx = pool.begin().await.unwrap();

	// Insert in transaction
	let insert = Query::insert()
		.into_table(Alias::new("users"))
		.columns([Alias::new("name")])
		.values_panic(["Bob".into()])
		.to_string(PostgresQueryBuilder);

	sqlx::query(&insert).execute(&mut *tx).await.unwrap();

	// Rollback
	tx.rollback().await.unwrap();

	// Verify user not inserted
	let select = Query::select()
		.column(Alias::new("name"))
		.from(Alias::new("users"))
		.to_string(PostgresQueryBuilder);

	let rows: Vec<(String,)> = sqlx::query_as(&select)
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	assert_eq!(rows.len(), 0);
}

#[rstest]
#[tokio::test]
async fn test_sea_query_builder_injection(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_container.await;

	// Setup DI context
	let singleton = Arc::new(SingletonScope::new());
	singleton.set(pool.clone());
	let ctx = InjectionContext::builder(singleton).build();

	// Create users table
	let create_table = Table::create()
		.table(Alias::new("users"))
		.col(
			ColumnDef::new(Alias::new("id"))
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new(Alias::new("name")).string().not_null())
		.to_string(PostgresQueryBuilder);

	sqlx::query(&create_table)
		.execute(pool.as_ref())
		.await
		.unwrap();

	// Inject DatabaseService
	let db_service = DatabaseService::inject(&ctx).await.unwrap();

	// Use sea-query builder
	let insert = Query::insert()
		.into_table(Alias::new("users"))
		.columns([Alias::new("name")])
		.values_panic(["Charlie".into()])
		.to_string(PostgresQueryBuilder);

	sqlx::query(&insert)
		.execute(db_service.pool.as_ref())
		.await
		.unwrap();

	// Verify insertion
	let select = Query::select()
		.column(Alias::new("name"))
		.from(Alias::new("users"))
		.and_where(Expr::col(Alias::new("name")).eq("Charlie"))
		.to_string(PostgresQueryBuilder);

	let row: (String,) = sqlx::query_as(&select)
		.fetch_one(db_service.pool.as_ref())
		.await
		.unwrap();

	assert_eq!(row.0, "Charlie");
}
