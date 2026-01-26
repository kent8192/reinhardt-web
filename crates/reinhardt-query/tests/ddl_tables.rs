//! Table operations integration tests
//!
//! Tests for CREATE/DROP TABLE operations.

use std::sync::Arc;

use rstest::rstest;

use reinhardt_query::prelude::*;
use reinhardt_query::types::{ColumnDef, ColumnType};

mod common;
use common::{MySqlContainer, PgContainer, mysql_ddl, postgres_ddl, unique_table_name};

/// Test basic CREATE TABLE on PostgreSQL
#[rstest]
#[tokio::test]
async fn test_postgres_create_table_basic(
	#[future] postgres_ddl: (PgContainer, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_ddl.await;
	let table_name = unique_table_name("test_users");

	// Build CREATE TABLE statement
	let mut stmt = Query::create_table();
	stmt.table(table_name.clone())
		.col(
			ColumnDef::new("id")
				.column_type(ColumnType::Integer)
				.not_null(true)
				.primary_key(true),
		)
		.col(
			ColumnDef::new("name")
				.column_type(ColumnType::String(None))
				.not_null(true),
		)
		.col(ColumnDef::new("email").column_type(ColumnType::String(None)));

	let builder = PostgresQueryBuilder::new();
	let (sql, _values) = builder.build_create_table(&stmt);

	// Execute CREATE TABLE
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Verify table exists
	let result: Option<i64> =
		sqlx::query_scalar("SELECT COUNT(*) FROM information_schema.tables WHERE table_name = $1")
			.bind(&table_name)
			.fetch_optional(pool.as_ref())
			.await
			.unwrap();

	assert_eq!(result, Some(1));

	// Cleanup
	sqlx::query(&format!(r#"DROP TABLE IF EXISTS "{}""#, table_name))
		.execute(pool.as_ref())
		.await
		.unwrap();
}

/// Test DROP TABLE on PostgreSQL
#[rstest]
#[tokio::test]
async fn test_postgres_drop_table(
	#[future] postgres_ddl: (PgContainer, Arc<sqlx::PgPool>, u16, String),
) {
	let (_container, pool, _port, _url) = postgres_ddl.await;
	let table_name = unique_table_name("test_users");

	// Create table
	let mut create_stmt = Query::create_table();
	create_stmt
		.table(table_name.clone())
		.col(
			ColumnDef::new("id")
				.column_type(ColumnType::Integer)
				.not_null(true)
				.primary_key(true),
		)
		.col(
			ColumnDef::new("name")
				.column_type(ColumnType::String(None))
				.not_null(true),
		);

	let builder = PostgresQueryBuilder::new();
	let (sql, _values) = builder.build_create_table(&create_stmt);
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Verify table exists
	let result: Option<i64> =
		sqlx::query_scalar("SELECT COUNT(*) FROM information_schema.tables WHERE table_name = $1")
			.bind(&table_name)
			.fetch_optional(pool.as_ref())
			.await
			.unwrap();
	assert_eq!(result, Some(1));

	// Drop table
	let mut drop_stmt = Query::drop_table();
	drop_stmt.table(table_name.clone());

	let (sql, _values) = builder.build_drop_table(&drop_stmt);
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to drop table");

	// Verify table no longer exists
	let result: Option<i64> =
		sqlx::query_scalar("SELECT COUNT(*) FROM information_schema.tables WHERE table_name = $1")
			.bind(&table_name)
			.fetch_optional(pool.as_ref())
			.await
			.unwrap();
	assert_eq!(result, Some(0));
}

/// Test basic CREATE TABLE on MySQL
#[rstest]
#[tokio::test]
async fn test_mysql_create_table_basic(
	#[future] mysql_ddl: (MySqlContainer, Arc<sqlx::MySqlPool>, u16, String),
) {
	let (_container, pool, _port, _url) = mysql_ddl.await;
	let table_name = unique_table_name("test_users");

	// Build CREATE TABLE statement
	let mut stmt = Query::create_table();
	stmt.table(table_name.clone())
		.col(
			ColumnDef::new("id")
				.column_type(ColumnType::Integer)
				.not_null(true)
				.primary_key(true)
				.auto_increment(true),
		)
		.col(
			ColumnDef::new("name")
				.column_type(ColumnType::String(None))
				.not_null(true),
		)
		.col(ColumnDef::new("email").column_type(ColumnType::String(None)));

	let builder = MySqlQueryBuilder::new();
	let (sql, _values) = builder.build_create_table(&stmt);

	// Execute CREATE TABLE
	sqlx::query(&sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create table");

	// Verify table exists
	let result: Option<i64> =
		sqlx::query_scalar("SELECT COUNT(*) FROM information_schema.tables WHERE table_name = ?")
			.bind(&table_name)
			.fetch_optional(pool.as_ref())
			.await
			.unwrap();

	assert_eq!(result, Some(1));

	// Cleanup
	sqlx::query(&format!("DROP TABLE IF EXISTS `{}`", table_name))
		.execute(pool.as_ref())
		.await
		.unwrap();
}
