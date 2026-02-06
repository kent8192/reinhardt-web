// Combination tests for UPDATE statement
//
// These tests test UPDATE combined with other advanced SQL features.

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::{TestPool, users_with_data};
use reinhardt_query::prelude::*;
use rstest::*;
use sqlx::Row;

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

/// Test UPDATE with JOIN
///
/// Verifies that UPDATE can be combined with JOIN operations.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/49
#[rstest]
#[tokio::test]
#[ignore = "UPDATE with JOIN not yet implemented (Issue #49)"]
async fn test_update_with_join(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Add JOIN support when implemented
	let stmt = Query::update()
		.table("users")
		.values([(
			"name",
			Value::String(Some(Box::new("Joined Update".to_string()))),
		)])
		.and_where(Expr::col("email").eq("alice@example.com"))
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_update(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify the update happened
	let users = sqlx::query("SELECT * FROM users WHERE email = $1")
		.bind("alice@example.com")
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch updated user");

	assert_eq!(users.len(), 1);
}

/// Test UPDATE with CASE expression
///
/// Verifies that UPDATE can use CASE expressions in value assignments.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/50
#[rstest]
#[tokio::test]
#[ignore = "CASE expression support not yet implemented (Issue #50)"]
async fn test_update_with_case_expression(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Add CASE expression when implemented
	let stmt = Query::update()
		.table("users")
		.values([("active", Value::Bool(Some(false)))])
		.and_where(Expr::col("email").eq("bob@example.com"))
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_update(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify the update happened
	let users = sqlx::query("SELECT * FROM users WHERE email = $1")
		.bind("bob@example.com")
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch updated user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].get::<bool, _>("active"), false);
}

/// Test UPDATE with subquery
///
/// Verifies that UPDATE can use a subquery in the WHERE clause or value assignments.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/51
#[rstest]
#[tokio::test]
#[ignore = "UPDATE with subquery not yet implemented (Issue #51)"]
async fn test_update_with_subquery(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Add subquery support when implemented
	let stmt = Query::update()
		.table("users")
		.values([(
			"name",
			Value::String(Some(Box::new("Subquery Update".to_string()))),
		)])
		.and_where(Expr::col("email").eq("charlie@example.com"))
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_update(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify the update happened
	let users = sqlx::query("SELECT * FROM users WHERE email = $1")
		.bind("charlie@example.com")
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch updated user");

	assert_eq!(users.len(), 1);
}
