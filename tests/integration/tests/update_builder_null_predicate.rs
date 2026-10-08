//! NULL equality predicates match only the intended rows during native execution.
#![cfg(all(
	not(all(target_family = "wasm", target_os = "unknown")),
	feature = "sqlite"
))]

use reinhardt_db::backends::connection::DatabaseConnection;
use reinhardt_db::backends::{InsertBuilder, QueryValue, SelectBuilder, UpdateBuilder};
use reinhardt_query::QueryStatementBuilder;
use reinhardt_query::prelude::{ColumnDef, Iden, IntoIden, Query, SqliteQueryBuilder};
use rstest::*;

#[derive(Debug, Iden)]
enum NullPredicateRows {
	Table,
	Id,
	Name,
}

#[fixture]
async fn sqlite_null_predicate_rows() -> DatabaseConnection {
	let connection = DatabaseConnection::connect("sqlite::memory:")
		.await
		.unwrap();
	let create = Query::create_table()
		.table(NullPredicateRows::Table.into_iden())
		.col(
			ColumnDef::new(NullPredicateRows::Id)
				.big_integer()
				.primary_key(true),
		)
		.col(ColumnDef::new(NullPredicateRows::Name).text())
		.to_string(SqliteQueryBuilder);
	connection.execute(&create, vec![]).await.unwrap();
	for (id, name) in [
		(1_i64, QueryValue::Null),
		(2, QueryValue::Null),
		(3, QueryValue::from("three")),
	] {
		InsertBuilder::new(connection.backend(), "null_predicate_rows")
			.value("id", id)
			.value("name", name)
			.execute()
			.await
			.unwrap();
	}
	connection
}

#[rstest]
#[case::null_first(true)]
#[case::null_last(false)]
#[tokio::test]
async fn test_update_null_predicate_matches_only_the_selected_row(
	#[future] sqlite_null_predicate_rows: DatabaseConnection,
	#[case] null_first: bool,
) {
	// Arrange
	let connection = sqlite_null_predicate_rows.await;
	let builder =
		UpdateBuilder::new(connection.backend(), "null_predicate_rows").set("name", "updated");
	let builder = if null_first {
		builder
			.where_eq("name", QueryValue::Null)
			.where_eq("id", 2_i64)
	} else {
		builder
			.where_eq("id", 2_i64)
			.where_eq("name", QueryValue::Null)
	};

	// Act
	let result = builder.execute().await.unwrap();

	// Assert
	assert_eq!(result.rows_affected, 1);
	let rows = SelectBuilder::new(connection.backend())
		.from("null_predicate_rows")
		.fetch_all()
		.await
		.unwrap();
	let mut values: Vec<_> = rows
		.iter()
		.map(|row| {
			(
				row.get::<i64>("id").unwrap(),
				row.data.get("name").unwrap().clone(),
			)
		})
		.collect();
	values.sort_by_key(|(id, _)| *id);
	assert_eq!(
		values,
		vec![
			(1, QueryValue::Null),
			(2, QueryValue::from("updated")),
			(3, QueryValue::from("three")),
		]
	);
}
