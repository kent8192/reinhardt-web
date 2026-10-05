#![cfg(all(feature = "backends", feature = "sqlite"))]

use reinhardt_db::backends::DatabaseConnection;
use reinhardt_db::backends::query_builder::{InsertBuilder, InsertFromSelectBuilder};
use reinhardt_query::prelude::{
	ColumnDef, ColumnType, Expr, ExprTrait, Iden, IntoIden, Order, Query, QueryStatementBuilder,
	SelectStatement, SqliteQueryBuilder,
};
use rstest::{fixture, rstest};

#[derive(Debug, Iden)]
enum CopyTable {
	Source,
	Target,
}

#[derive(Debug, Iden)]
enum CopyColumn {
	Id,
	Name,
}

#[fixture]
async fn sqlite_copy_source() -> DatabaseConnection {
	// The connection owns the isolated in-memory database; dropping it cleans up
	// even if execution or an assertion fails.
	let connection = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.unwrap();
	for table in [CopyTable::Source, CopyTable::Target] {
		let sql = Query::create_table()
			.table(table.into_iden())
			.col(
				ColumnDef::new(CopyColumn::Id)
					.column_type(ColumnType::Integer)
					.primary_key(true),
			)
			.col(ColumnDef::new(CopyColumn::Name).column_type(ColumnType::Text))
			.to_string(SqliteQueryBuilder);
		connection.execute(&sql, vec![]).await.unwrap();
	}
	for (table, rows) in [
		(CopyTable::Source, [(1_i64, "one' ? $19"), (2, "two")]),
		(CopyTable::Target, [(1_i64, "old"), (3, "untouched")]),
	] {
		let mut insert = Query::insert()
			.into_table(table.into_iden())
			.columns([CopyColumn::Id, CopyColumn::Name])
			.to_owned();
		for (id, name) in rows {
			insert.values(vec![id.into(), name.into()]).unwrap();
		}
		connection
			.execute(&insert.to_string(SqliteQueryBuilder), vec![])
			.await
			.unwrap();
	}
	connection
}

#[derive(Clone, Copy)]
enum SourceShape {
	Unfiltered,
	Filtered,
	Limited,
	Compound,
}

impl SourceShape {
	fn select(self) -> SelectStatement {
		let mut source = Query::select()
			.columns([CopyColumn::Id, CopyColumn::Name])
			.from(CopyTable::Source.into_iden())
			.to_owned();
		match self {
			Self::Unfiltered => {}
			Self::Filtered => {
				source.and_where(Expr::col(CopyColumn::Id).eq(1_i64));
			}
			Self::Limited => {
				source.order_by(CopyColumn::Id, Order::Desc).limit(1_u64);
			}
			Self::Compound => {
				// A WHERE on the first branch cannot disambiguate the final FROM.
				let mut first = source.clone();
				first.and_where(Expr::col(CopyColumn::Id).eq(1_i64));
				first.union_all(source);
				source = first;
			}
		}
		source
	}
}

#[rstest]
#[case::unfiltered(SourceShape::Unfiltered, 2, vec![(1, "one' ? $19"), (2, "two"), (3, "untouched")])]
#[case::filtered(SourceShape::Filtered, 1, vec![(1, "one' ? $19"), (3, "untouched")])]
#[case::limited(SourceShape::Limited, 1, vec![(1, "old"), (2, "two"), (3, "untouched")])]
#[case::compound(SourceShape::Compound, 3, vec![(1, "one' ? $19"), (2, "two"), (3, "untouched")])]
#[tokio::test]
async fn sqlite_select_upsert_preserves_source_rows(
	#[future] sqlite_copy_source: DatabaseConnection,
	#[case] shape: SourceShape,
	#[case] affected_rows: u64,
	#[case] expected: Vec<(i64, &str)>,
	#[values(false, true)] from_insert_builder: bool,
	#[values(None, Some(vec![]), Some(vec!["id".into()]))] conflict_columns: Option<Vec<String>>,
) {
	// Arrange
	let connection = sqlite_copy_source.await;
	let source = shape.select();
	let builder = if from_insert_builder {
		InsertBuilder::new(connection.backend(), "target")
			.on_conflict_do_update(conflict_columns, vec!["name".into()])
			.from_select(vec!["id", "name"], source)
	} else {
		InsertFromSelectBuilder::new(connection.backend(), "target", vec!["id", "name"], source)
			.on_conflict_do_update(conflict_columns, vec!["name".into()])
	};

	// Act
	let result = builder.execute().await.unwrap();
	let sql = Query::select()
		.columns([CopyColumn::Id, CopyColumn::Name])
		.from(CopyTable::Target.into_iden())
		.order_by(CopyColumn::Id, Order::Asc)
		.to_string(SqliteQueryBuilder);
	let rows = connection.fetch_all(&sql, vec![]).await.unwrap();
	let actual: Vec<(i64, String)> = rows
		.iter()
		.map(|row| (row.get("id").unwrap(), row.get("name").unwrap()))
		.collect();

	// Assert
	assert_eq!(result.rows_affected, affected_rows);
	assert_eq!(
		actual,
		expected
			.into_iter()
			.map(|(id, name)| (id, name.to_owned()))
			.collect::<Vec<_>>()
	);
}
