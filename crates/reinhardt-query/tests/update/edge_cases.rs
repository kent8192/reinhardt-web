//! Edge case tests for UPDATE statement

use crate::fixtures::{Users, users_with_data};
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

/// Test update to same value
///
/// Verifies that updating a column to its current value works correctly.
#[rstest]
#[tokio::test]
async fn test_update_to_same_value(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// First get current value
	let users = Users::select()
		.where_(format!("{} = {}", "email", "'alice@example.com'"))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch user");

	let original_name = users[0].name.clone();

	// Update to same value
	let stmt = Query::update()
		.table(Users::table_name())
		.values([("name", original_name.clone().into())])
		.and_where(Expr::col("email").eq("alice@example.com"))
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_update(&stmt);

	let result = bind_and_execute!(pool, sql, values);
	assert_eq!(result.rows_affected(), 1);

	// Verify value unchanged
	let users = Users::select()
		.where_(format!("{} = {}", "email", "'alice@example.com'"))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch user");

	assert_eq!(users[0].name, original_name);
}

/// Test update to NULL
///
/// Verifies that updating a column to NULL works correctly.
#[rstest]
#[tokio::test]
async fn test_update_to_null(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	let stmt = Query::update()
		.table(Users::table_name())
		.values([("age", Value::Int(None).into())])
		.and_where(Expr::col("email").eq("bob@example.com"))
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_update(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify using Model
	let users = Users::select()
		.where_(format!("{} = {}", "email", "'bob@example.com'"))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch updated user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].age, None);
}

/// Test update empty string to value
///
/// Verifies that updating from empty string to a value works correctly.
#[rstest]
#[tokio::test]
async fn test_update_empty_to_value(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// First set to empty string
	let _update_stmt = Query::update()
		.table(Users::table_name())
		.values([("name", "".into())])
		.and_where(Expr::col("email").eq("charlie@example.com"))
		.to_owned();

	// Then update to a value
	let stmt = Query::update()
		.table(Users::table_name())
		.values([("name", "Charlie Has Name".into())])
		.and_where(Expr::col("email").eq("charlie@example.com"))
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_update(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify using Model
	let users = Users::select()
		.where_(format!("{} = {}", "email", "'charlie@example.com'"))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch updated user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].name, "Charlie Has Name");
}

/// Test update value to empty string
///
/// Verifies that updating from a value to empty string works correctly.
#[rstest]
#[tokio::test]
async fn test_update_value_to_empty(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	let stmt = Query::update()
		.table(Users::table_name())
		.values([("name", "".into())])
		.and_where(Expr::col("email").eq("alice@example.com"))
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_update(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify using Model
	let users = Users::select()
		.where_(format!("{} = {}", "email", "'alice@example.com'"))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch updated user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].name, "");
}
