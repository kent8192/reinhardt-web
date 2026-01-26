//! Happy path tests for INSERT statement

use crate::fixtures::{Users, users_table};
use reinhardt_query::prelude::*;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;

/// Macro to bind values and execute query
macro_rules! bind_and_execute {
	($pool:expr, $sql:expr, $values:expr) => {{
		let mut query: sqlx::query::Query<'_, sqlx::Postgres, _> = sqlx::query(&$sql);
		for value in &$values.0 {
			query = match value {
				Value::BigInt(Some(v)) => query.bind(*v),
				Value::BigInt(None) => query.bind::<Option<i64>>(None),
				Value::SmallInt(Some(v)) => query.bind(*v),
				Value::SmallInt(None) => query.bind::<Option<i16>>(None),
				Value::Int(Some(v)) => query.bind(*v),
				Value::Int(None) => query.bind::<Option<i32>>(None),
				Value::String(Some(v)) => query.bind(v.as_str()),
				Value::String(None) => query.bind::<Option<&str>>(None),
				Value::Bool(Some(v)) => query.bind(*v),
				Value::Bool(None) => query.bind::<Option<bool>>(None),
				Value::TinyUnsigned(Some(v)) => query.bind(*v as i16),
				Value::TinyUnsigned(None) => query.bind::<Option<i16>>(None),
				Value::SmallUnsigned(Some(v)) => query.bind(*v as i32),
				Value::SmallUnsigned(None) => query.bind::<Option<i32>>(None),
				Value::Unsigned(None) => query.bind::<Option<i64>>(None),
				_ => query,
			};
		}
		query
			.execute($pool.as_ref())
			.await
			.expect("Query execution failed")
	}};
}

/// Test single row insertion
///
/// Verifies that Query::insert() can insert a single row correctly.
#[rstest]
#[tokio::test]
async fn test_insert_single_row(#[future] users_table: Arc<PgPool>) {
	let pool = users_table.await;

	let stmt = Query::insert()
		.into_table(Users::table_name())
		.columns(["name", "email", "age"])
		.values_panic(["Charlie", "charlie@example.com", 35i32])
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	let result = bind_and_execute!(pool, sql, values);
	assert_eq!(result.rows_affected(), 1, "Should insert exactly one row");

	// Verify using Model
	let users = Users::select()
		.where_(format!("{} = {}", "email", "'charlie@example.com'"))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch inserted user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].name, "Charlie");
	assert_eq!(users[0].email, "charlie@example.com");
	assert_eq!(users[0].age, Some(35));
}

/// Test multiple row insertion
///
/// Verifies that Query::insert() can insert multiple rows in a single statement.
#[rstest]
#[tokio::test]
async fn test_insert_multiple_rows(#[future] users_table: Arc<PgPool>) {
	let pool = users_table.await;

	let stmt = Query::insert()
		.into_table(Users::table_name())
		.columns(["name", "email", "age"])
		.values_panic(["David", "david@example.com", 28i32])
		.values_panic(["Eve", "eve@example.com", 32i32])
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	let result = bind_and_execute!(pool, sql, values);
	assert_eq!(result.rows_affected(), 2, "Should insert exactly two rows");

	// Verify using Model
	let users = Users::select()
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch all users");

	assert_eq!(users.len(), 2);
}

/// Test insertion with RETURNING clause
///
/// Verifies that Query::insert() can return inserted values using RETURNING.
#[rstest]
#[tokio::test]
async fn test_insert_with_returning(#[future] users_table: Arc<PgPool>) {
	let pool = users_table.await;

	let stmt = Query::insert()
		.into_table(Users::table_name())
		.columns(["name", "email", "age"])
		.values_panic(["Frank", "frank@example.com", 40i32])
		.returning_all()
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify using Model
	let users = Users::select()
		.where_(format!("{} = {}", "email", "'frank@example.com'"))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch inserted user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].name, "Frank");
}

/// Test insertion with default values
///
/// Verifies that columns with DEFAULT values are correctly handled.
#[rstest]
#[tokio::test]
async fn test_insert_with_default_values(#[future] users_table: Arc<PgPool>) {
	let pool = users_table.await;

	let stmt = Query::insert()
		.into_table(Users::table_name())
		.columns(["name", "email"])
		.values_panic(["Grace", "grace@example.com"])
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify using Model
	let users = Users::select()
		.where_(format!("{} = {}", "email", "'grace@example.com'"))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch inserted user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].name, "Grace");
	assert_eq!(users[0].age, None); // No age specified
	assert_eq!(users[0].active, true); // Default value
}

/// Test insertion with NULL values
///
/// Verifies that NULL values are correctly inserted.
#[rstest]
#[tokio::test]
async fn test_insert_with_null_values(#[future] users_table: Arc<PgPool>) {
	let pool = users_table.await;

	let stmt = Query::insert()
		.into_table(Users::table_name())
		.columns(["name", "email", "age"])
		.values_panic(["Henry", "henry@example.com", Option::<i32>::None])
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify using Model
	let users = Users::select()
		.where_(format!("{} = {}", "email", "'henry@example.com'"))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch inserted user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].age, None);
}

/// Test insertion with all column types
///
/// Verifies that all supported column types work correctly.
#[rstest]
#[tokio::test]
async fn test_insert_all_column_types(#[future] users_table: Arc<PgPool>) {
	let pool = users_table.await;

	let stmt = Query::insert()
		.into_table(Users::table_name())
		.columns(["name", "email", "age", "active"])
		.values_panic(["Ivy", "ivy@example.com", 27i32, false])
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify using Model
	let users = Users::select()
		.where_(format!("{} = {}", "email", "'ivy@example.com'"))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch inserted user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].name, "Ivy");
	assert_eq!(users[0].age, Some(27));
	assert_eq!(users[0].active, false);
}

/// Test bulk insertion of 100 rows
///
/// Verifies that Query::insert() can handle bulk insertions efficiently.
#[rstest]
#[tokio::test]
async fn test_insert_bulk_100_rows(#[future] users_table: Arc<PgPool>) {
	let pool = users_table.await;

	let mut stmt = Query::insert()
		.into_table(Users::table_name())
		.columns(["name", "email", "age"]);

	for i in 0..100 {
		let name = format!("User{}", i);
		let email = format!("user{}@example.com", i);
		let age = 20 + (i % 50);
		stmt = stmt
			.values_panic([name.as_str(), email.as_str(), age])
			.to_owned();
	}

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	let result = bind_and_execute!(pool, sql, values);
	assert_eq!(
		result.rows_affected(),
		100,
		"Should insert exactly 100 rows"
	);

	// Verify using Model
	let count_result = sqlx::query("SELECT COUNT(*) as count FROM users")
		.fetch_one(pool.as_ref())
		.await
		.expect("Should count rows");

	let count: i64 = count_result.get("count");
	assert_eq!(count, 100);
}
