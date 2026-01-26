//! Edge case tests for INSERT statement

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

/// Test insertion with empty string
///
/// Verifies that empty strings are correctly inserted (not treated as NULL).
#[rstest]
#[tokio::test]
async fn test_insert_empty_string(#[future] users_table: Arc<PgPool>) {
	let pool = users_table.await;

	let stmt = Query::insert()
		.into_table(Users::table_name())
		.columns(["name", "email", "age"])
		.values_panic(["", "empty@example.com", 25i32])
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify using Model
	let users = Users::select()
		.where_(format!("{} = {}", "email", "'empty@example.com'"))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch inserted user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].name, "");
}

/// Test insertion with max length string
///
/// Verifies that strings at the maximum length limit are correctly handled.
#[rstest]
#[tokio::test]
async fn test_insert_max_length_string(#[future] users_table: Arc<PgPool>) {
	let pool = users_table.await;

	// Create a string with exactly 255 characters (max length for name field)
	let max_name = "a".repeat(255);
	let max_email = format!("{}@example.com", "b".repeat(244)); // 255 total

	let stmt = Query::insert()
		.into_table(Users::table_name())
		.columns(["name", "email", "age"])
		.values_panic([max_name.as_str(), max_email.as_str(), 30i32])
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	let result = bind_and_execute!(pool, sql, values);
	assert_eq!(result.rows_affected(), 1);

	// Verify using Model
	let users = Users::select()
		.where_(format!("{} = {}", "email", format!("'{}'", max_email)))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch inserted user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].name.len(), 255);
}

/// Test insertion with zero values
///
/// Verifies that zero values (0, 0.0) are correctly inserted (not treated as NULL).
#[rstest]
#[tokio::test]
async fn test_insert_zero_values(#[future] users_table: Arc<PgPool>) {
	let pool = users_table.await;

	let stmt = Query::insert()
		.into_table(Users::table_name())
		.columns(["name", "email", "age"])
		.values_panic(["Zero User", "zero@example.com", 0i32])
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify using Model
	let users = Users::select()
		.where_(format!("{} = {}", "email", "'zero@example.com'"))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch inserted user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].age, Some(0));
}

/// Test insertion with negative values
///
/// Verifies that negative values are correctly inserted.
#[rstest]
#[tokio::test]
async fn test_insert_negative_values(#[future] users_table: Arc<PgPool>) {
	let pool = users_table.await;

	// Note: Age doesn't make sense as negative, but we're testing the capability
	let stmt = Query::insert()
		.into_table(Users::table_name())
		.columns(["name", "email", "age"])
		.values_panic(["Negative User", "negative@example.com", -5i32])
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify using Model
	let users = Users::select()
		.where_(format!("{} = {}", "email", "'negative@example.com'"))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch inserted user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].age, Some(-5));
}

/// Test insertion with Unicode characters
///
/// Verifies that Unicode strings (emoji, multi-byte characters) are correctly handled.
#[rstest]
#[tokio::test]
async fn test_insert_unicode_chars(#[future] users_table: Arc<PgPool>) {
	let pool = users_table.await;

	// Test with emoji and multi-byte characters
	let unicode_name = "🎉 テスト User 👍";
	let unicode_email = "test+🚀@example.com";

	let stmt = Query::insert()
		.into_table(Users::table_name())
		.columns(["name", "email", "age"])
		.values_panic([unicode_name, unicode_email, 25i32])
		.to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_insert(&stmt);

	bind_and_execute!(pool, sql, values);

	// Verify using Model
	let users = Users::select()
		.where_(format!("{} = {}", "email", format!("'{}'", unicode_email)))
		.fetch_all(pool.as_ref())
		.await
		.expect("Should fetch inserted user");

	assert_eq!(users.len(), 1);
	assert_eq!(users[0].name, String::from(unicode_name));
}
