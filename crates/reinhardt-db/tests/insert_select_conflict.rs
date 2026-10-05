#![cfg(all(feature = "backends", feature = "sqlite"))]

use std::sync::Arc;

use reinhardt_db::backends::{
	DatabaseBackend, QueryValue,
	dialect::SqliteBackend,
	error::DatabaseError,
	query_builder::{InsertBuilder, OnConflictClause},
};
use reinhardt_query::prelude::{
	ColumnDef, Expr, ExprTrait, Iden, IntoIden, Order, Query, QueryStatementBuilder,
	SelectStatement, SqliteQueryBuilder,
};
use rstest::{fixture, rstest};
use sqlx::sqlite::SqlitePoolOptions;

#[derive(Debug, Iden)]
enum ConflictRows {
	Table,
	Id,
	Revision,
}

#[derive(Debug, Iden)]
enum SourceRows {
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
				.not_null(true)
				.unique(true)
				.check(Expr::col(ConflictRows::Revision).gt(0)),
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

#[fixture]
async fn source_rows(#[future] conflict_rows: Arc<SqliteBackend>) -> Arc<SqliteBackend> {
	let backend = conflict_rows.await;
	let create = Query::create_table()
		.table(SourceRows::Table.into_iden())
		.col(ColumnDef::new(SourceRows::Id).integer())
		.col(ColumnDef::new(SourceRows::Revision).integer())
		.to_string(SqliteQueryBuilder);
	backend.execute(&create, Vec::new()).await.unwrap();
	let insert = Query::insert()
		.into_table(SourceRows::Table.into_iden())
		.columns([SourceRows::Id, SourceRows::Revision])
		.values_panic([4_i64, 2_i64])
		.values_panic([5_i64, 3_i64])
		.to_string(SqliteQueryBuilder);
	backend.execute(&insert, Vec::new()).await.unwrap();
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
#[case::targeted_do_nothing(OnConflictClause::columns(vec!["id"]).do_nothing(), 0, 1)]
#[tokio::test]
async fn test_insert_from_select_executes_fluent_conflict(
	#[future] conflict_rows: Arc<SqliteBackend>,
	#[case] clause: OnConflictClause,
	#[case] expected_affected: u64,
	#[case] expected_revision: i64,
	#[values(false, true)] from_table: bool,
) {
	// Arrange
	let backend = conflict_rows.await;
	let mut select = Query::select();
	select.expr(Expr::val(4_i64)).expr(Expr::val(2_i64));
	if from_table {
		select.from(ConflictRows::Table.into_iden());
	}
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
	#[values(false, true)] from_table: bool,
) {
	// Arrange
	let backend = conflict_rows.await;
	let mut select = Query::select();
	select.expr(Expr::val(4_i64)).expr(Expr::val(2_i64));
	if from_table {
		select.from(ConflictRows::Table.into_iden());
	}
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
#[case::plain(
	Query::select().columns([SourceRows::Id, SourceRows::Revision])
		.from(SourceRows::Table.into_iden()).to_owned(),
	vec![(4, 2), (5, 3)], 2
)]
#[case::filtered(
	Query::select().columns([SourceRows::Id, SourceRows::Revision])
		.from(SourceRows::Table.into_iden()).and_where(Expr::col(SourceRows::Id).eq(5))
		.to_owned(),
	vec![(4, 1), (5, 3)], 1
)]
#[case::ordered_limit_offset(
	Query::select().columns([SourceRows::Id, SourceRows::Revision])
		.from(SourceRows::Table.into_iden()).order_by(SourceRows::Id, Order::Asc)
		.limit(1).offset(1).to_owned(),
	vec![(4, 1), (5, 3)], 1
)]
#[case::compound(
	Query::select().columns([SourceRows::Id, SourceRows::Revision])
		.from(SourceRows::Table.into_iden()).and_where(Expr::col(SourceRows::Id).eq(4))
		.union(Query::select().columns([SourceRows::Id, SourceRows::Revision])
			.from(SourceRows::Table.into_iden()).to_owned()).to_owned(),
	vec![(4, 2), (5, 3)], 2
)]
#[tokio::test]
async fn test_insert_from_select_preserves_source_query(
	#[future] source_rows: Arc<SqliteBackend>,
	#[case] select: SelectStatement,
	#[case] expected_rows: Vec<(i64, i64)>,
	#[case] expected_affected: u64,
	#[values(false, true)] legacy: bool,
) {
	// Arrange
	let backend = source_rows.await;
	let builder = InsertBuilder::new(backend.clone(), "conflict_rows");
	let builder = if legacy {
		builder.on_conflict_do_update(Some(vec!["id".into()]), vec!["revision".into()])
	} else {
		builder.on_conflict(OnConflictClause::columns(vec!["id"]).do_update(vec!["revision"]))
	};
	let builder = builder.from_select(vec!["id", "revision"], select);

	// Act
	let result = builder
		.execute()
		.await
		.expect("table-source UPSERT must execute");

	// Assert
	assert_eq!(result.rows_affected, expected_affected);
	let select = Query::select()
		.columns([ConflictRows::Id, ConflictRows::Revision])
		.from(ConflictRows::Table.into_iden())
		.order_by(ConflictRows::Id, Order::Asc)
		.to_string(SqliteQueryBuilder);
	let rows = backend.fetch_all(&select, Vec::new()).await.unwrap();
	let actual_rows: Vec<_> = rows
		.iter()
		.map(|row| {
			(
				row.get::<i64>("id").unwrap(),
				row.get::<i64>("revision").unwrap(),
			)
		})
		.collect();
	assert_eq!(actual_rows, expected_rows);
}

#[rstest]
#[case::other_unique(
	Some(1),
	"(code: 2067) UNIQUE constraint failed: conflict_rows.revision"
)]
#[case::not_null(
	None,
	"(code: 1299) NOT NULL constraint failed: conflict_rows.revision"
)]
#[case::check(Some(0), "(code: 275) CHECK constraint failed: revision")]
#[tokio::test]
async fn test_fluent_do_nothing_propagates_unhandled_constraints(
	#[future] conflict_rows: Arc<SqliteBackend>,
	#[case] revision: Option<i64>,
	#[case] expected_message: &str,
	#[values(false, true)] from_select: bool,
	#[values(false, true)] any_target: bool,
) {
	// Arrange
	let backend = conflict_rows.await;
	let clause = if any_target {
		OnConflictClause::any().do_nothing()
	} else {
		OnConflictClause::columns(vec!["id"]).do_nothing()
	};
	let select = Query::select()
		.expr(Expr::val(5_i64))
		.expr(Expr::val(revision))
		.to_owned();
	let builder = InsertBuilder::new(backend.clone(), "conflict_rows").on_conflict(clause);

	// Act
	let result = if from_select {
		builder
			.from_select(vec!["id", "revision"], select)
			.execute()
			.await
	} else {
		let value = revision.map(QueryValue::Int).unwrap_or(QueryValue::Null);
		builder
			.value("id", QueryValue::Int(5))
			.value("revision", value)
			.execute()
			.await
	};

	// Assert
	if any_target && revision == Some(1) {
		assert_eq!(result.unwrap().rows_affected, 0);
	} else {
		assert_eq!(
			result.unwrap_err(),
			DatabaseError::QueryError(expected_message.into())
		);
	}
	let select = Query::select()
		.columns([ConflictRows::Id, ConflictRows::Revision])
		.from(ConflictRows::Table.into_iden())
		.to_string(SqliteQueryBuilder);
	let rows = backend.fetch_all(&select, Vec::new()).await.unwrap();
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0].get::<i64>("id").unwrap(), 4);
	assert_eq!(rows[0].get::<i64>("revision").unwrap(), 1);
}

#[rstest]
#[case::existing(4, 2, 0, vec![(4, 1)])]
#[case::new(5, 2, 1, vec![(4, 1), (5, 2)])]
#[tokio::test]
async fn test_values_do_nothing_executes_with_returning(
	#[future] conflict_rows: Arc<SqliteBackend>,
	#[case] id: i64,
	#[case] revision: i64,
	#[case] expected_affected: u64,
	#[case] expected_rows: Vec<(i64, i64)>,
	#[values(false, true)] any_target: bool,
) {
	// Arrange
	let backend = conflict_rows.await;
	let clause = if any_target {
		OnConflictClause::any().do_nothing()
	} else {
		OnConflictClause::columns(vec!["id"]).do_nothing()
	};
	let builder = InsertBuilder::new(backend.clone(), "conflict_rows")
		.value("id", QueryValue::Int(id))
		.value("revision", QueryValue::Int(revision))
		.on_conflict(clause)
		.returning(vec!["id", "revision"]);

	// Act
	let result = builder
		.execute()
		.await
		.expect("DO NOTHING with RETURNING must execute");

	// Assert
	assert_eq!(result.rows_affected, expected_affected);
	let select = Query::select()
		.columns([ConflictRows::Id, ConflictRows::Revision])
		.from(ConflictRows::Table.into_iden())
		.order_by(ConflictRows::Id, Order::Asc)
		.to_string(SqliteQueryBuilder);
	let rows = backend.fetch_all(&select, Vec::new()).await.unwrap();
	let actual_rows: Vec<(i64, i64)> = rows
		.iter()
		.map(|row| {
			(
				row.get::<i64>("id").unwrap(),
				row.get::<i64>("revision").unwrap(),
			)
		})
		.collect();
	assert_eq!(actual_rows, expected_rows);
}

#[rstest]
#[tokio::test]
async fn test_values_do_nothing_fetches_with_returning(
	#[future] conflict_rows: Arc<SqliteBackend>,
	#[values(false, true)] any_target: bool,
) {
	// Arrange
	let backend = conflict_rows.await;
	let clause = if any_target {
		OnConflictClause::any().do_nothing()
	} else {
		OnConflictClause::columns(vec!["id"]).do_nothing()
	};
	let builder = InsertBuilder::new(backend, "conflict_rows")
		.value("id", QueryValue::Int(5))
		.value("revision", QueryValue::Int(2))
		.on_conflict(clause)
		.returning(vec!["id", "revision"]);

	// Act
	let row = builder
		.fetch_one()
		.await
		.expect("inserted row must be returned");

	// Assert
	assert_eq!(row.get::<i64>("id").unwrap(), 5);
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
#[case::empty_do_nothing_target(
	OnConflictClause::columns(Vec::<String>::new()).do_nothing(),
	DatabaseError::SyntaxError("SQLite ON CONFLICT requires non-empty conflict_columns for DO NOTHING".into())
)]
#[case::do_nothing_constraint(
	OnConflictClause::constraint("conflict_rows_pkey").do_nothing(),
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
