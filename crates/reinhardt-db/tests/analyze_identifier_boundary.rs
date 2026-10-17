//! `AnalyzeBuilder` keeps runtime table names inside one quoted identifier.
#![cfg(all(feature = "backends", feature = "sqlite"))]

use reinhardt_db::backends::{AnalyzeBuilder, DatabaseConnection};
use reinhardt_query::prelude::{
	ColumnDef, ColumnType, Query, QueryStatementBuilder, SqliteQueryBuilder,
};
use rstest::{fixture, rstest};

const QUOTED_TABLE: &str = "analysis\"target`";

#[fixture]
async fn sqlite_connection() -> DatabaseConnection {
	// The connection owns the isolated in-memory database; dropping it cleans up
	// even if execution or an assertion fails.
	let connection = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.unwrap();
	for table in ["safe", "audit_probe", QUOTED_TABLE] {
		let sql = Query::create_table()
			.table(table)
			.col(
				ColumnDef::new("id")
					.column_type(ColumnType::BigInteger)
					.primary_key(true),
			)
			.to_string(SqliteQueryBuilder);
		connection.execute(&sql, vec![]).await.unwrap();
	}
	connection
}

async fn audit_probe_count(connection: &DatabaseConnection) -> i64 {
	let row = connection
		.fetch_one("SELECT COUNT(*) AS count FROM audit_probe", vec![])
		.await
		.unwrap();
	row.get("count").unwrap()
}

#[rstest]
#[tokio::test]
async fn sqlite_analyze_identifier_cannot_execute_another_statement(
	#[future] sqlite_connection: DatabaseConnection,
) {
	// Arrange
	let connection = sqlite_connection.await;
	let identifier = "safe\"; INSERT INTO audit_probe (id) VALUES (7); --";

	// Act
	let result = AnalyzeBuilder::new(connection.backend())
		.table(identifier)
		.execute()
		.await;

	// Assert: the whole input names one (missing) table and writes nothing else.
	assert_eq!(audit_probe_count(&connection).await, 0, "{result:?}");
	assert!(result.is_err(), "{result:?}");
}

#[rstest]
#[tokio::test]
async fn sqlite_analyze_accepts_table_name_with_embedded_quotes(
	#[future] sqlite_connection: DatabaseConnection,
) {
	// Arrange
	let connection = sqlite_connection.await;
	let insert = Query::insert()
		.into_table(QUOTED_TABLE)
		.columns(["id"])
		.values_panic([1_i64])
		.to_string(SqliteQueryBuilder);
	connection.execute(&insert, vec![]).await.unwrap();

	// Act
	let result = AnalyzeBuilder::new(connection.backend())
		.table(QUOTED_TABLE)
		.execute()
		.await;

	// Assert: SQLite resolved the escaped identifier to the real table.
	assert!(result.is_ok(), "{result:?}");
	let row = connection
		.fetch_one(
			"SELECT COUNT(*) AS count FROM sqlite_stat1 WHERE tbl = ?",
			vec![QUOTED_TABLE.into()],
		)
		.await
		.unwrap();
	assert_eq!(row.get::<i64>("count").unwrap(), 1);
}
