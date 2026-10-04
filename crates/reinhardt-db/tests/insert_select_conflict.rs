#![cfg(all(feature = "backends", feature = "sqlite"))]

use std::sync::Arc;

use reinhardt_db::backends::{
	DatabaseBackend,
	dialect::SqliteBackend,
	error::DatabaseError,
	query_builder::{InsertBuilder, OnConflictClause},
};
use reinhardt_query::prelude::{
	ColumnDef, Expr, Iden, IntoIden, Query, QueryStatementBuilder, SqliteQueryBuilder,
};
use rstest::{fixture, rstest};
use sqlx::sqlite::SqlitePoolOptions;

#[derive(Debug, Iden)]
enum ConflictRows {
	Table,
	Id,
	Revision,
}

// The pool owns an isolated memory database and releases it on drop.
#[fixture]
async fn conflict_rows() -> Arc<SqliteBackend> {
	let pool = SqlitePoolOptions::new()
		.max_connections(1)
		.connect("sqlite::memory:")
		.await
		.expect("in-memory SQLite must connect");
	let backend = Arc::new(SqliteBackend::new(pool));
	let create = Query::create_table()
		.table(ConflictRows::Table.into_iden())
		.col(
			ColumnDef::new(ConflictRows::Id)
				.integer()
				.not_null(true)
				.primary_key(true),
		)
		.col(
			ColumnDef::new(ConflictRows::Revision)
				.integer()
				.not_null(true),
		)
		.to_string(SqliteQueryBuilder);
	backend
		.execute(&create, Vec::new())
		.await
		.expect("conflict table must be created");
	let insert = Query::insert()
		.into_table(ConflictRows::Table.into_iden())
		.columns([ConflictRows::Id, ConflictRows::Revision])
		.values_panic([4_i64, 1_i64])
		.to_string(SqliteQueryBuilder);
	backend
		.execute(&insert, Vec::new())
		.await
		.expect("existing row must be inserted");
	backend
}

#[rstest]
#[case::update(
	OnConflictClause::columns(vec!["id"]).do_update(vec!["revision"]),
	1, 2
)]
#[case::conditional_update(
	OnConflictClause::columns(vec!["id"]).do_update(vec!["revision"])
		.where_clause("conflict_rows.revision < excluded.revision"),
	1, 2
)]
#[case::condition_skips_update(
	OnConflictClause::columns(vec!["id"]).do_update(vec!["revision"])
		.where_clause("conflict_rows.revision > excluded.revision"),
	0, 1
)]
#[case::do_nothing(OnConflictClause::any().do_nothing(), 0, 1)]
#[tokio::test]
async fn test_insert_from_select_executes_fluent_conflict(
	#[future] conflict_rows: Arc<SqliteBackend>,
	#[case] clause: OnConflictClause,
	#[case] expected_affected: u64,
	#[case] expected_revision: i64,
) {
	// Arrange
	let backend = conflict_rows.await;
	let select = Query::select()
		.expr(Expr::val(4_i64))
		.expr(Expr::val(2_i64))
		.to_owned();
	let builder = InsertBuilder::new(backend.clone(), "conflict_rows")
		.on_conflict(clause)
		.from_select(vec!["id", "revision"], select);

	// Act
	let result = builder
		.execute()
		.await
		.expect("conflict action must execute");

	// Assert
	assert_eq!(result.rows_affected, expected_affected);
	let select = Query::select()
		.columns([ConflictRows::Id, ConflictRows::Revision])
		.from(ConflictRows::Table.into_iden())
		.to_string(SqliteQueryBuilder);
	let rows = backend.fetch_all(&select, Vec::new()).await.unwrap();
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0].get::<i64>("id").unwrap(), 4);
	assert_eq!(rows[0].get::<i64>("revision").unwrap(), expected_revision);
}

#[rstest]
#[tokio::test]
async fn test_insert_from_select_fetches_fluent_conflict_returning(
	#[future] conflict_rows: Arc<SqliteBackend>,
) {
	// Arrange
	let backend = conflict_rows.await;
	let select = Query::select()
		.expr(Expr::val(4_i64))
		.expr(Expr::val(2_i64))
		.to_owned();
	let builder = InsertBuilder::new(backend, "conflict_rows")
		.on_conflict(OnConflictClause::columns(vec!["id"]).do_update(vec!["revision"]))
		.returning(vec!["id", "revision"])
		.from_select(vec!["id", "revision"], select);

	// Act
	let row = builder
		.fetch_one()
		.await
		.expect("updated row must be returned");

	// Assert
	assert_eq!(row.get::<i64>("id").unwrap(), 4);
	assert_eq!(row.get::<i64>("revision").unwrap(), 2);
}

#[rstest]
#[case::empty_target(
	OnConflictClause::columns(Vec::<String>::new()).do_update(vec!["revision"]),
	DatabaseError::SyntaxError("SQLite ON CONFLICT requires non-empty conflict_columns for DO UPDATE".into())
)]
#[case::empty_updates(
	OnConflictClause::columns(vec!["id"]).do_update(Vec::<String>::new()),
	DatabaseError::SyntaxError("update_columns cannot be empty for OnConflictClauseAction::DoUpdate".into())
)]
#[case::constraint(
	OnConflictClause::constraint("conflict_rows_pkey").do_update(vec!["revision"]),
	DatabaseError::NotSupported("SQLite does not support ON CONFLICT ON CONSTRAINT syntax".into())
)]
#[tokio::test]
async fn test_insert_from_select_reports_invalid_fluent_conflict(
	#[future] conflict_rows: Arc<SqliteBackend>,
	#[case] clause: OnConflictClause,
	#[case] expected_error: DatabaseError,
	#[values(false, true)] fetch: bool,
) {
	// Arrange
	let backend = conflict_rows.await;
	let select = Query::select()
		.expr(Expr::val(4_i64))
		.expr(Expr::val(2_i64))
		.to_owned();
	let builder = InsertBuilder::new(backend.clone(), "conflict_rows")
		.on_conflict(clause)
		.from_select(vec!["id", "revision"], select);

	// Act
	let result = if fetch {
		builder.fetch_one().await.map(|_| ())
	} else {
		builder.execute().await.map(|_| ())
	};

	// Assert
	assert_eq!(result.unwrap_err(), expected_error);
	let select = Query::select()
		.column(ConflictRows::Revision)
		.from(ConflictRows::Table.into_iden())
		.to_string(SqliteQueryBuilder);
	let rows = backend.fetch_all(&select, Vec::new()).await.unwrap();
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0].get::<i64>("revision").unwrap(), 1);
}
