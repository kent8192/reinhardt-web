// Combination tests for SELECT statement
//
// These tests test SELECT combined with other advanced SQL features.

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::{TestPool, users_with_data};
use reinhardt_query::prelude::*;
use rstest::*;

/// Macro to bind values and execute query
macro_rules! bind_and_execute_query {
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
			.fetch_all($pool.as_ref())
			.await
			.expect("Query execution failed")
	}};
}

/// Test SELECT with CTE (WITH clause)
///
/// Verifies that SELECT can use Common Table Expressions.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/68
#[rstest]
#[tokio::test]
#[ignore = "CTE (WITH clause) support not yet implemented (Issue #68)"]
async fn test_select_with_cte(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Add WITH clause when implemented
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_select(&stmt);

	let rows = bind_and_execute_query!(pool, sql, values);
	assert!(rows.len() >= 3, "Should have at least 3 users");
}

/// Test SELECT with UNION
///
/// Verifies that SELECT can combine results using UNION.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/69
#[rstest]
#[tokio::test]
#[ignore = "UNION support not yet implemented (Issue #69)"]
async fn test_select_with_union(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Add UNION when implemented
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_select(&stmt);

	let rows = bind_and_execute_query!(pool, sql, values);
	assert!(rows.len() >= 3, "Should have at least 3 users");
}

/// Test SELECT with DISTINCT ON
///
/// Verifies that SELECT can use DISTINCT ON clause correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/70
#[rstest]
#[tokio::test]
#[ignore = "DISTINCT ON not yet implemented (Issue #70)"]
async fn test_select_with_distinct_on(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Add DISTINCT ON when implemented
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_select(&stmt);

	let rows = bind_and_execute_query!(pool, sql, values);
	assert!(rows.len() >= 3, "Should have at least 3 users");
}

/// Test SELECT with window function
///
/// Verifies that SELECT can use window functions correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/71
#[rstest]
#[tokio::test]
#[ignore = "Window functions not yet implemented (Issue #71)"]
async fn test_select_with_window_function(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Add window function support when implemented
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_select(&stmt);

	let rows = bind_and_execute_query!(pool, sql, values);
	assert!(rows.len() >= 3, "Should have at least 3 users");
}
