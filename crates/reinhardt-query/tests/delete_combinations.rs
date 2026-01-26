// Combination tests for DELETE statement
//
// These tests test DELETE combined with other advanced SQL features.

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::users_with_data;
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

/// Test DELETE with subquery
///
/// Verifies that DELETE can use a subquery in the WHERE clause.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/52
#[rstest]
#[tokio::test]
#[ignore = "DELETE with subquery not yet implemented (Issue #52)"]
async fn test_delete_with_subquery(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Add subquery support when implemented
	let stmt = Query::delete()
		.from_table("users")
		.and_where(Expr::col("email").eq("charlie@example.com"))
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_delete(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify the deletion happened
	let users = sqlx::query("SELECT * FROM users WHERE email = $1")
		.bind("charlie@example.com")
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch users");

	assert_eq!(users.len(), 0);
}

/// Test DELETE with JOIN
///
/// Verifies that DELETE can be combined with JOIN operations.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/53
#[rstest]
#[tokio::test]
#[ignore = "DELETE with JOIN not yet implemented (Issue #53)"]
async fn test_delete_with_join(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Add JOIN support when implemented
	// For now, verify SQL structure
	let stmt = Query::delete()
		.from_table("users")
		.and_where(Expr::col("age").gt(30))
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_delete(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify some users were deleted
	let users = sqlx::query("SELECT * FROM users")
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch users");

	let old_users = users.iter().filter(|u| {
		u.get::<Option<i32>, _>("age")
			.map(|a| a > 30)
			.unwrap_or(false)
	});
	assert_eq!(old_users.count(), 0, "Should have no users with age > 30");
}
