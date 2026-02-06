// JOIN tests for SELECT statement

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

/// Test INNER JOIN
///
/// Verifies that Query::select() can perform inner joins correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/54
#[rstest]
#[tokio::test]
#[ignore = "INNER JOIN not yet implemented (Issue #54)"]
async fn test_select_inner_join(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// Create orders table
	use sea_query::{ColumnDef, ForeignKey, ForeignKeyAction, Table};

	let create_table = Table::create()
		.table("orders")
		.if_not_exists()
		.col(
			ColumnDef::new("id")
				.integer()
				.not_null()
				.auto_increment()
				.primary_key(),
		)
		.col(ColumnDef::new("user_id").integer().not_null())
		.col(ColumnDef::new("total_amount").big_integer().not_null())
		.col(ColumnDef::new("status").string_len(50).not_null())
		.foreign_key(
			ForeignKey::create()
				.name("fk_orders_user_id")
				.from("orders", "user_id")
				.to("users", "id")
				.on_delete(ForeignKeyAction::Cascade)
				.on_update(ForeignKeyAction::Cascade),
		)
		.to_owned();

	let create_sql = create_table.to_string(sea_query::PostgresQueryBuilder);
	sqlx::query(&create_sql)
		.execute(pool.as_ref())
		.await
		.expect("Failed to create orders table");

	// TODO: Implement INNER JOIN when supported
	// For now, verify basic SELECT works
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_select(&stmt);

	let rows = bind_and_execute_query!(pool, sql, values);
	assert!(rows.len() >= 3, "Should have at least 3 users");
}

/// Test LEFT JOIN
///
/// Verifies that Query::select() can perform left outer joins correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/55
#[rstest]
#[tokio::test]
#[ignore = "LEFT JOIN not yet implemented (Issue #55)"]
async fn test_select_left_join(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement LEFT JOIN when supported
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_select(&stmt);

	let rows = bind_and_execute_query!(pool, sql, values);
	assert!(rows.len() >= 3, "Should have at least 3 users");
}

/// Test RIGHT JOIN
///
/// Verifies that Query::select() can perform right outer joins correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/56
#[rstest]
#[tokio::test]
#[ignore = "RIGHT JOIN not yet implemented (Issue #56)"]
async fn test_select_right_join(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement RIGHT JOIN when supported
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_select(&stmt);

	let rows = bind_and_execute_query!(pool, sql, values);
	assert!(rows.len() >= 3, "Should have at least 3 users");
}

/// Test FULL OUTER JOIN
///
/// Verifies that Query::select() can perform full outer joins correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/57
#[rstest]
#[tokio::test]
#[ignore = "FULL OUTER JOIN not yet implemented (Issue #57)"]
async fn test_select_full_outer_join(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement FULL OUTER JOIN when supported
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_select(&stmt);

	let rows = bind_and_execute_query!(pool, sql, values);
	assert!(rows.len() >= 3, "Should have at least 3 users");
}

/// Test CROSS JOIN
///
/// Verifies that Query::select() can perform cross joins correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/58
#[rstest]
#[tokio::test]
#[ignore = "CROSS JOIN not yet implemented (Issue #58)"]
async fn test_select_cross_join(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement CROSS JOIN when supported
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_select(&stmt);

	let rows = bind_and_execute_query!(pool, sql, values);
	assert!(rows.len() >= 3, "Should have at least 3 users");
}

/// Test SELF JOIN
///
/// Verifies that Query::select() can perform self joins correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/59
#[rstest]
#[tokio::test]
#[ignore = "SELF JOIN not yet implemented (Issue #59)"]
async fn test_select_self_join(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement SELF JOIN when supported
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_select(&stmt);

	let rows = bind_and_execute_query!(pool, sql, values);
	assert!(rows.len() >= 3, "Should have at least 3 users");
}

/// Test multiple JOINs
///
/// Verifies that Query::select() can join multiple tables correctly.
///
/// **IGNORED**: This feature is not yet implemented in reinhardt-query.
/// See: https://github.com/kent8192/reinhardt-web/issues/60
#[rstest]
#[tokio::test]
#[ignore = "Multiple JOINs not yet implemented (Issue #60)"]
async fn test_select_multiple_joins(#[future] users_with_data: (TestPool, Vec<i32>)) {
	let (pool, _ids) = users_with_data.await;

	// TODO: Implement multiple JOINs when supported
	let stmt = Query::select().from("users").to_owned();

	let builder = PostgresQueryBuilder;
	let (sql, values) = builder.build_select(&stmt);

	let rows = bind_and_execute_query!(pool, sql, values);
	assert!(rows.len() >= 3, "Should have at least 3 users");
}
