//! Error path tests for SELECT statement

use crate::fixtures::{Users, users_with_data};
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

/// Test select invalid column
///
/// Verifies that selecting a non-existent column results in a database error.
#[rstest]
#[tokio::test]
async fn test_select_invalid_column(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// Try to select a non-existent column
	let stmt = Query::select()
		.column("invalid_column")
		.from(Users::table_name())
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_select(&stmt);

	let result = sqlx::query(&sql).fetch_all(pool.as_ref()).await;

	assert!(result.is_err(), "Should fail with invalid column error");
	let err = result.unwrap_err();
	let err_msg = err.to_string();
	assert!(
		err_msg.contains("column") || err_msg.contains("does not exist"),
		"Error should mention invalid column: {}",
		err_msg
	);
}

/// Test select invalid table
///
/// Verifies that selecting from a non-existent table results in a database error.
#[rstest]
#[tokio::test]
async fn test_select_invalid_table(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// Try to select from a non-existent table
	let stmt = Query::select().from("invalid_table").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_select(&stmt);

	let result = sqlx::query(&sql).fetch_all(pool.as_ref()).await;

	assert!(result.is_err(), "Should fail with invalid table error");
	let err = result.unwrap_err();
	let err_msg = err.to_string();
	assert!(
		err_msg.contains("table")
			|| err_msg.contains("does not exist")
			|| err_msg.contains("relation"),
		"Error should mention invalid table: {}",
		err_msg
	);
}
