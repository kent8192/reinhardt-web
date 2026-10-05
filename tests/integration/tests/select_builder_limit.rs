//! SelectBuilder LIMIT arguments reach the native SQLite executor in order.
#![cfg(not(all(target_family = "wasm", target_os = "unknown")))]
#![cfg(feature = "sqlite")]

use std::sync::Arc;

use reinhardt_db::DatabaseConnection;
use reinhardt_db::backends::{InsertBuilder, SelectBuilder};
use reinhardt_db::migrations::{Migration, MigrationProvider};
use reinhardt_query::QueryStatementBuilder;
use reinhardt_query::prelude::{ColumnDef, Iden, IntoIden, Query, SqliteQueryBuilder};
use reinhardt_test::fixtures::sqlite_with_migrations_from;
use rstest::*;

struct NoMigrations;

impl MigrationProvider for NoMigrations {
	fn migrations() -> Vec<Migration> {
		Vec::new()
	}
}

#[derive(Debug, Iden)]
enum LimitRows {
	Table,
	Id,
	GroupId,
}

#[fixture]
async fn sqlite_limit_rows() -> Arc<DatabaseConnection> {
	let connection = sqlite_with_migrations_from::<NoMigrations>().await;
	let create = Query::create_table()
		.table(LimitRows::Table.into_iden())
		.col(
			ColumnDef::new(LimitRows::Id)
				.big_integer()
				.primary_key(true),
		)
		.col(
			ColumnDef::new(LimitRows::GroupId)
				.big_integer()
				.not_null(true),
		)
		.to_string(SqliteQueryBuilder);
	connection.inner().execute(&create, vec![]).await.unwrap();
	for (id, group_id) in [(1_i64, 0_i64), (2, 7), (3, 7), (4, 7)] {
		InsertBuilder::new(connection.inner().backend(), "limit_rows")
			.value("id", id)
			.value("group_id", group_id)
			.execute()
			.await
			.unwrap();
	}
	connection
}

#[rstest]
#[case::unlimited(None, 4)]
#[case::one(Some(1), 1)]
#[case::zero(Some(0), 0)]
#[case::negative(Some(-1), 4)]
#[case::largest(Some(i64::MAX), 4)]
#[tokio::test]
async fn test_select_limit_fetch_all_without_where(
	#[future] sqlite_limit_rows: Arc<DatabaseConnection>,
	#[case] limit: Option<i64>,
	#[case] expected_count: usize,
) {
	// Arrange
	let connection = sqlite_limit_rows.await;
	let mut builder = SelectBuilder::new(connection.inner().backend()).from("limit_rows");
	if let Some(limit) = limit {
		builder = builder.limit(limit);
	}

	// Act
	let rows = builder.fetch_all().await.unwrap();

	// Assert
	assert_eq!(rows.len(), expected_count);
}

#[rstest]
#[tokio::test]
async fn test_select_limit_fetch_all_after_where(
	#[future] sqlite_limit_rows: Arc<DatabaseConnection>,
) {
	// Arrange
	let connection = sqlite_limit_rows.await;
	let builder = SelectBuilder::new(connection.inner().backend())
		.from("limit_rows")
		.where_eq("group_id", 7_i64)
		.limit(1);

	// Act
	let rows = builder.fetch_all().await.unwrap();

	// Assert
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0].get::<i64>("group_id").unwrap(), 7);
}

#[rstest]
#[tokio::test]
async fn test_select_limit_fetch_one_after_multiple_wheres(
	#[future] sqlite_limit_rows: Arc<DatabaseConnection>,
) {
	// Arrange
	let connection = sqlite_limit_rows.await;
	let builder = SelectBuilder::new(connection.inner().backend())
		.from("limit_rows")
		.where_eq("group_id", 7_i64)
		.where_eq("id", 2_i64)
		.limit(1);

	// Act
	let row = builder.fetch_one().await.unwrap();

	// Assert
	assert_eq!(row.get::<i64>("id").unwrap(), 2);
	assert_eq!(row.get::<i64>("group_id").unwrap(), 7);
}
