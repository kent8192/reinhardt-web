// Combination tests for INSERT statement
//
// These tests test INSERT combined with other advanced SQL features.

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::{Users, users_with_data};
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

/// Test INSERT ... ON CONFLICT DO UPDATE (UPSERT)
///
/// Verifies that upsert operations work correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/46
#[rstest]
#[tokio::test]
#[ignore = "ON CONFLICT DO UPDATE not yet implemented (Issue #46)"]
async fn test_insert_on_conflict_do_update(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// Try to upsert Alice with new age
	let stmt = Query::insert()
		.into_table("users")
		.columns(["name", "email", "age"])
		.values_panic([
			Value::String(Some(Box::new("Alice Updated".to_string()))),
			Value::String(Some(Box::new("alice@example.com".to_string()))),
			Value::Int(Some(35)),
		])
		.to_owned();

	// TODO: Add ON CONFLICT DO UPDATE clause when implemented
	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	let result = bind_and_execute!(pool, sql, values);
	assert!(result.rows_affected() > 0);

	// Verify the upsert happened
	let users = sqlx::query("SELECT * FROM users WHERE email = $1")
		.bind("alice@example.com")
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch user");

	assert_eq!(users.len(), 1);
	// After UPSERT, age should be updated
	// assert_eq!(users[0].get::<Option<i32>, _>("age"), Some(35));
}

/// Test INSERT with CTE (WITH clause)
///
/// Verifies that INSERT can be combined with Common Table Expressions.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/58
#[rstest]
#[tokio::test]
#[ignore = "CTE (WITH clause) support not yet implemented (Issue #58)"]
async fn test_insert_with_cte(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Add WITH clause when implemented
	let stmt = Query::insert()
		.into_table("users")
		.columns(["name", "email", "age"])
		.values_panic([
			Value::String(Some(Box::new("CTE User".to_string()))),
			Value::String(Some(Box::new("cte@example.com".to_string()))),
			Value::Int(Some(30)),
		])
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	let result = bind_and_execute!(pool, sql, values);
	assert_eq!(result.rows_affected(), 1);

	// Verify the insert happened
	let users = sqlx::query("SELECT * FROM users WHERE email = $1")
		.bind("cte@example.com")
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch user");

	assert_eq!(users.len(), 1);
}

/// Test INSERT from subquery
///
/// Verifies that INSERT can use a subquery as the data source.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/59
#[rstest]
#[tokio::test]
#[ignore = "INSERT from subquery not yet implemented (Issue #59)"]
async fn test_insert_with_subquery(#[future] users_with_data: (Arc<PgPool>, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement INSERT from subquery
	// For now, verify the basic INSERT structure
	let stmt = Query::insert()
		.into_table("users")
		.columns(["name", "email", "age"])
		.values_panic([
			Value::String(Some(Box::new("Subquery User".to_string()))),
			Value::String(Some(Box::new("subquery@example.com".to_string()))),
			Value::Int(Some(28)),
		])
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify the insert happened
	let users = sqlx::query("SELECT * FROM users WHERE email = $1")
		.bind("subquery@example.com")
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch user");

	assert_eq!(users.len(), 1);
}
