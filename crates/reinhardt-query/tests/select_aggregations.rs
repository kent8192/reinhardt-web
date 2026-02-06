// Aggregation tests for SELECT statement

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::{TestPool, users_with_data};
use reinhardt_query::prelude::*;
use rstest::*;

/// Test COUNT aggregation
///
/// Verifies that Query::select() can count rows correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/61
#[rstest]
#[tokio::test]
#[ignore = "COUNT aggregation not yet implemented (Issue #61)"]
async fn test_select_count(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (_pool, _ids) = users_with_data.await;

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
async fn test_select_sum(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (_pool, _ids) = users_with_data.await;

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
async fn test_select_avg(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (_pool, _ids) = users_with_data.await;

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
async fn test_select_min_max(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (_pool, _ids) = users_with_data.await;

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
async fn test_select_group_by(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (_pool, _ids) = users_with_data.await;

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
async fn test_select_having(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (_pool, _ids) = users_with_data.await;

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
async fn test_select_group_by_multiple_columns(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (_pool, _ids) = users_with_data.await;

	// TODO: Implement multiple column GROUP BY when supported
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, _values) = builder.build_select(&stmt);

	assert!(sql.contains("SELECT"), "SQL should be a SELECT statement");
}
