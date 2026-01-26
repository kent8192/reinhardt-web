//! Happy path tests for UPDATE statement

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

/// Test single column update
///
/// Verifies that Query::update() can update a single column correctly.
#[rstest]
#[tokio::test]
async fn test_update_single_column(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	let stmt = Query::update()
		.table(Users::table_name())
		.values([("name", "Alice Updated".into())])
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
	assert_eq!(users[0].name, "Alice Updated");
}

/// Test multiple columns update
///
/// Verifies that Query::update() can update multiple columns in a single statement.
#[rstest]
#[tokio::test]
async fn test_update_multiple_columns(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	let stmt = Query::update()
		.table(Users::table_name())
		.values([("name", "Bob Updated".into()), ("age", 26i32.into())])
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
	assert_eq!(users[0].name, "Bob Updated");
	assert_eq!(users[0].age, Some(26));
}

/// Test update with RETURNING clause
///
/// Verifies that Query::update() can return updated values using RETURNING.
#[rstest]
#[tokio::test]
async fn test_update_with_returning(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	let stmt = Query::update()
		.table(Users::table_name())
		.values([("name", "Charlie Updated".into())])
		.and_where(Expr::col("email").eq("charlie@example.com"))
		.returning_all()
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
	assert_eq!(users[0].name, "Charlie Updated");
}

/// Test update with expression
///
/// Verifies that Query::update() can use expressions in value assignments.
#[rstest]
#[tokio::test]
async fn test_update_with_expression(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// Increment age by 1 using expression
	let stmt = Query::update()
		.table(Users::table_name())
		.values([("age", Expr::col("age").add(1).into())])
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
	assert_eq!(users[0].age, Some(31)); // Was 30, now 31
}

/// Test update multiple rows with condition
///
/// Verifies that Query::update() can update multiple rows matching a condition.
#[rstest]
#[tokio::test]
async fn test_update_multiple_rows_condition(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// Update all users with age < 30
	let stmt = Query::update()
		.table(Users::table_name())
		.values([("active", false.into())])
		.and_where(Expr::col("age").lt(30))
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_update(&stmt);

	let result = bind_and_execute!(pool, sql, values);
	assert!(result.rows_affected() > 0);

	// Verify using Model
	let users = Users::select()
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch all users");

	let inactive_count = users.iter().filter(|u| !u.active).count();
	assert!(inactive_count > 0, "Should have inactive users");
}
