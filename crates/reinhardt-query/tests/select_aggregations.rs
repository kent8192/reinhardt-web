// Aggregation tests for SELECT statement

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::{Users, users_with_data};
use reinhardt_query::prelude::*;
use rstest::*;
use sqlx::{PgPool, Row};
use std::sync::Arc;

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

/// Test COUNT aggregation
///
/// Verifies that Query::select() can count rows correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/61
#[rstest]
#[tokio::test]
#[ignore = "COUNT aggregation not yet implemented (Issue #61)"]
async fn test_select_count(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement COUNT when supported
	// For now, verify SQL structure
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, _values) = builder.build_select(&stmt);

	assert!(sql.contains("SELECT"), "SQL should be a SELECT statement");
	assert!(sql.contains("users"), "SQL should specify users table");
}

/// Test SUM aggregation
///
/// Verifies that Query::select() can sum values correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/62
#[rstest]
#[tokio::test]
#[ignore = "SUM aggregation not yet implemented (Issue #62)"]
async fn test_select_sum(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement SUM when supported
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, _values) = builder.build_select(&stmt);

	assert!(sql.contains("SELECT"), "SQL should be a SELECT statement");
}

/// Test AVG aggregation
///
/// Verifies that Query::select() can calculate average correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/63
#[rstest]
#[tokio::test]
#[ignore = "AVG aggregation not yet implemented (Issue #63)"]
async fn test_select_avg(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement AVG when supported
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, _values) = builder.build_select(&stmt);

	assert!(sql.contains("SELECT"), "SQL should be a SELECT statement");
}

/// Test MIN/MAX aggregation
///
/// Verifies that Query::select() can find min/max values correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/64
#[rstest]
#[tokio::test]
#[ignore = "MIN/MAX aggregation not yet implemented (Issue #64)"]
async fn test_select_min_max(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement MIN/MAX when supported
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, _values) = builder.build_select(&stmt);

	assert!(sql.contains("SELECT"), "SQL should be a SELECT statement");
}

/// Test GROUP BY
///
/// Verifies that Query::select() can group results correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/65
#[rstest]
#[tokio::test]
#[ignore = "GROUP BY not yet implemented (Issue #65)"]
async fn test_select_group_by(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement GROUP BY when supported
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, _values) = builder.build_select(&stmt);

	assert!(sql.contains("SELECT"), "SQL should be a SELECT statement");
}

/// Test HAVING
///
/// Verifies that Query::select() can filter groups using HAVING correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/66
#[rstest]
#[tokio::test]
#[ignore = "HAVING not yet implemented (Issue #66)"]
async fn test_select_having(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement HAVING when supported
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, _values) = builder.build_select(&stmt);

	assert!(sql.contains("SELECT"), "SQL should be a SELECT statement");
}

/// Test GROUP BY with multiple columns
///
/// Verifies that Query::select() can group by multiple columns correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/67
#[rstest]
#[tokio::test]
#[ignore = "Multiple column GROUP BY not yet implemented (Issue #67)"]
async fn test_select_group_by_multiple_columns(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement multiple column GROUP BY when supported
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, _values) = builder.build_select(&stmt);

	assert!(sql.contains("SELECT"), "SQL should be a SELECT statement");
}
