//! Index operations integration tests
//!
//! Tests for CREATE/DROP/ALTER INDEX and REINDEX operations.

use std::sync::Arc;

use rstest::rstest;

use reinhardt_query::prelude::*;

mod common;
use common::{
	MySqlContainer, PgContainer, mysql_ddl, postgres_ddl, unique_index_name, unique_table_name,
};

/// Test basic CREATE INDEX on PostgreSQL
#[rstest]
#[tokio::test]
async fn test_postgres_create_index_basic(
	#[future] postgres_ddl: (PgContainer, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_ddl.await;
	let table_name = unique_table_name("test_users");
	let index_name = unique_index_name("idx_email");

	// Create table first
	let create_table_sql = format!(
		r#"CREATE TABLE "{}" (id SERIAL PRIMARY KEY, email VARCHAR(255))"#,
		table_name
	);
	sqlx::query(&create_table_sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Build CREATE INDEX statement
	let mut stmt = Query::create_index();
	stmt.name(index_name.clone())
		.table(table_name.clone())
		.col("email");

	let builder = PostgresQueryBuilder::new();
	let (sql, _values) = builder.build_create_index(&stmt);

	// Execute CREATE INDEX
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create index");

	// Verify index exists
	let result: Option<i64> =
		sqlx::query_scalar("SELECT COUNT(*) FROM pg_indexes WHERE indexname = $1")
			.bind(&index_name)
			.fetch_optional(pool.as_ref())
			.await
			.unwrap();

	assert_eq!(result, Some(1));

	// Cleanup
	let drop_table_sql = format!(r#"DROP TABLE IF EXISTS "{}""#, table_name);
	sqlx::query(&drop_table_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// Test CREATE UNIQUE INDEX on PostgreSQL
#[rstest]
#[tokio::test]
async fn test_postgres_create_unique_index(
	#[future] postgres_ddl: (PgContainer, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_ddl.await;
	let table_name = unique_table_name("test_users");
	let index_name = unique_index_name("idx_email_unique");

	// Create table first
	let create_table_sql = format!(
		r#"CREATE TABLE "{}" (id SERIAL PRIMARY KEY, email VARCHAR(255))"#,
		table_name
	);
	sqlx::query(&create_table_sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Build CREATE UNIQUE INDEX statement
	let mut stmt = Query::create_index();
	stmt.name(index_name.clone())
		.table(table_name.clone())
		.col("email")
		.unique();

	let builder = PostgresQueryBuilder::new();
	let (sql, _values) = builder.build_create_index(&stmt);

	// Execute CREATE UNIQUE INDEX
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create unique index");

	// Verify index is unique
	let result: Option<bool> = sqlx::query_scalar(
		"SELECT indisunique FROM pg_indexes i
		JOIN pg_class c ON i.indexname = c.relname
		JOIN pg_index idx ON c.oid = idx.indexrelid
		WHERE i.indexname = $1",
	)
	.bind(&index_name)
	.fetch_optional(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(result, Some(true));

	// Cleanup
	let drop_table_sql = format!(r#"DROP TABLE IF EXISTS "{}""#, table_name);
	sqlx::query(&drop_table_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// Test DROP INDEX on PostgreSQL
#[rstest]
#[tokio::test]
async fn test_postgres_drop_index(
	#[future] postgres_ddl: (PgContainer, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_ddl.await;
	let table_name = unique_table_name("test_users");
	let index_name = unique_index_name("idx_email");

	// Create table and index
	let create_table_sql = format!(
		r#"CREATE TABLE "{}" (id SERIAL PRIMARY KEY, email VARCHAR(255))"#,
		table_name
	);
	sqlx::query(&create_table_sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	let create_index_sql = format!(
		r#"CREATE INDEX "{}" ON "{}" (email)"#,
		index_name, table_name
	);
	sqlx::query(&create_index_sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create index");

	// Verify index exists
	let result: Option<i64> =
		sqlx::query_scalar("SELECT COUNT(*) FROM pg_indexes WHERE indexname = $1")
			.bind(&index_name)
			.fetch_optional(pool.as_ref())
			.await
			.unwrap();
	assert_eq!(result, Some(1));

	// Build DROP INDEX statement
	let mut stmt = Query::drop_index();
	stmt.name(index_name.clone());

	let builder = PostgresQueryBuilder::new();
	let (sql, _values) = builder.build_drop_index(&stmt);

	// Execute DROP INDEX
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to drop index");

	// Verify index no longer exists
	let result: Option<i64> =
		sqlx::query_scalar("SELECT COUNT(*) FROM pg_indexes WHERE indexname = $1")
			.bind(&index_name)
			.fetch_optional(pool.as_ref())
			.await
			.unwrap();
	assert_eq!(result, Some(0));

	// Cleanup
	let drop_table_sql = format!(r#"DROP TABLE IF EXISTS "{}""#, table_name);
	sqlx::query(&drop_table_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// Test REINDEX on PostgreSQL
#[rstest]
#[tokio::test]
async fn test_postgres_reindex(
	#[future] postgres_ddl: (PgContainer, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_ddl.await;
	let table_name = unique_table_name("test_users");
	let index_name = unique_index_name("idx_email");

	// Create table and index
	let create_table_sql = format!(
		r#"CREATE TABLE "{}" (id SERIAL PRIMARY KEY, email VARCHAR(255))"#,
		table_name
	);
	sqlx::query(&create_table_sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	let create_index_sql = format!(
		r#"CREATE INDEX "{}" ON "{}" (email)"#,
		index_name, table_name
	);
	sqlx::query(&create_index_sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create index");

	// Build REINDEX statement
	let mut stmt = Query::reindex();
	stmt.index(index_name.clone());

	let builder = PostgresQueryBuilder::new();
	let (sql, _values) = builder.build_reindex(&stmt);

	// Execute REINDEX
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to reindex");

	// Verify index still exists after reindex
	let result: Option<i64> =
		sqlx::query_scalar("SELECT COUNT(*) FROM pg_indexes WHERE indexname = $1")
			.bind(&index_name)
			.fetch_optional(pool.as_ref())
			.await
			.unwrap();

	assert_eq!(result, Some(1));

	// Cleanup
	let drop_table_sql = format!(r#"DROP TABLE IF EXISTS "{}""#, table_name);
	sqlx::query(&drop_table_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// Test basic CREATE INDEX on MySQL
#[rstest]
#[tokio::test]
async fn test_mysql_create_index_basic(
	#[future] mysql_ddl: (MySqlContainer, Arc<sqlx::MySqlPool>, u16, String),
) {
	let (_container, pool, _port, _url) = mysql_ddl.await;
	let table_name = unique_table_name("test_users");
	let index_name = unique_index_name("idx_email");

	// Create table first
	let create_table_sql = format!(
		"CREATE TABLE `{}` (id INT AUTO_INCREMENT PRIMARY KEY, email VARCHAR(255))",
		table_name
	);
	sqlx::query(&create_table_sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Build CREATE INDEX statement
	let mut stmt = Query::create_index();
	stmt.name(index_name.clone())
		.table(table_name.clone())
		.col("email");

	let builder = MySqlQueryBuilder::new();
	let (sql, _values) = builder.build_create_index(&stmt);

	// Execute CREATE INDEX
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create index");

	// Verify index exists
	let result: Option<i64> = sqlx::query_scalar(
		"SELECT COUNT(*) FROM information_schema.statistics WHERE table_name = ? AND index_name = ?",
	)
	.bind(&table_name)
	.bind(&index_name)
	.fetch_optional(pool.as_ref())
	.await
	.unwrap();

	assert_eq!(result, Some(1));

	// Cleanup
	let drop_table_sql = format!("DROP TABLE IF EXISTS `{}`", table_name);
	sqlx::query(&drop_table_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// Test DROP INDEX on MySQL
#[rstest]
#[tokio::test]
async fn test_mysql_drop_index(
	#[future] mysql_ddl: (MySqlContainer, Arc<sqlx::MySqlPool>, u16, String),
) {
	let (_container, pool, _port, _url) = mysql_ddl.await;
	let table_name = unique_table_name("test_users");
	let index_name = unique_index_name("idx_email");

	// Create table and index
	let create_table_sql = format!(
		"CREATE TABLE `{}` (id INT AUTO_INCREMENT PRIMARY KEY, email VARCHAR(255))",
		table_name
	);
	sqlx::query(&create_table_sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	let create_index_sql = format!("CREATE INDEX `{}` ON `{}` (email)", index_name, table_name);
	sqlx::query(&create_index_sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create index");

	// Verify index exists
	let result: Option<i64> = sqlx::query_scalar(
		"SELECT COUNT(*) FROM information_schema.statistics WHERE table_name = ? AND index_name = ?",
	)
	.bind(&table_name)
	.bind(&index_name)
	.fetch_optional(pool.as_ref())
	.await
	.unwrap();
	assert_eq!(result, Some(1));

	// Build DROP INDEX statement (MySQL requires table name)
	let mut stmt = Query::drop_index();
	stmt.name(index_name.clone());
	stmt.table(table_name.clone());

	let builder = MySqlQueryBuilder::new();
	let (sql, _values) = builder.build_drop_index(&stmt);

	// Execute DROP INDEX
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to drop index");

	// Verify index no longer exists
	let result: Option<i64> = sqlx::query_scalar(
		"SELECT COUNT(*) FROM information_schema.statistics WHERE table_name = ? AND index_name = ?",
	)
	.bind(&table_name)
	.bind(&index_name)
	.fetch_optional(pool.as_ref())
	.await
	.unwrap();
	assert_eq!(result, Some(0));

	// Cleanup
	let drop_table_sql = format!("DROP TABLE IF EXISTS `{}`", table_name);
	sqlx::query(&drop_table_sql)
		.execute(pool.as_ref())
		.await
		.unwrap();
}
