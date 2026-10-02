//! Regression coverage for conflict targets and targetless INSERT handling.

use reinhardt_query::types::{ColumnDef, ColumnType};
use reinhardt_query::{
	Alias, Expr, InsertStatement, MySqlQueryBuilder, OnConflict, Order, PostgresQueryBuilder,
	Query, QueryBuilderTrait, QueryStatementBuilder, SqliteQueryBuilder, Value,
};
use rstest::*;
use std::sync::Arc;

mod common;
use common::{PgContainer, postgres_container};

#[fixture]
fn registry_insert_select() -> InsertStatement {
	Query::insert()
		.into_table("registry")
		.columns(["id", "version", "kind", "metadata"])
		.from_subquery(
			Query::select()
				.expr(Expr::cust("$1"))
				.expr(Expr::cust("$2"))
				.expr(Expr::cust("$3"))
				.expr(Expr::cust("$4"))
				.to_owned(),
		)
		.to_owned()
}

#[rstest]
fn empty_conflict_target_in_insert_select(mut registry_insert_select: InsertStatement) {
	// Arrange
	registry_insert_select
		.on_conflict(OnConflict::columns(std::iter::empty::<Alias>()).do_nothing());

	// Act
	let sql = registry_insert_select.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		"INSERT INTO \"registry\" (\"id\", \"version\", \"kind\", \"metadata\") SELECT $1, $2, $3, $4 ON CONFLICT DO NOTHING"
	);
}

#[rstest]
#[case::postgres(
	PostgresQueryBuilder,
	"INSERT INTO \"registry\" (\"id\", \"version\") VALUES ($1, $2), ($3, $4) ON CONFLICT DO NOTHING"
)]
#[case::sqlite(
	SqliteQueryBuilder,
	"INSERT INTO \"registry\" (\"id\", \"version\") VALUES (?, ?), (?, ?) ON CONFLICT DO NOTHING"
)]
#[case::mysql(
	MySqlQueryBuilder,
	"INSERT INTO `registry` (`id`, `version`) VALUES (?, ?), (?, ?) ON DUPLICATE KEY UPDATE `id` = `id`"
)]
fn targetless_conflict_in_insert_values<B: QueryBuilderTrait>(
	#[case] builder: B,
	#[case] expected_sql: &str,
	#[values(
		OnConflict::new(),
		OnConflict::default(),
		OnConflict::columns(std::iter::empty::<Alias>()).do_nothing()
	)]
	conflict: OnConflict,
) {
	// Arrange
	let mut statement = Query::insert();
	statement
		.into_table("registry")
		.columns(["id", "version"])
		.values_panic([Value::from(1), Value::from("first")])
		.values_panic([Value::from(2), Value::from("second")])
		.on_conflict(conflict);

	// Act
	let (sql, values) = statement.build(builder);

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(
		values.0,
		vec![1.into(), "first".into(), 2.into(), "second".into()]
	);
}

#[rstest]
#[case::postgres(PostgresQueryBuilder)]
#[case::sqlite(SqliteQueryBuilder)]
fn targetless_conflict_preserves_returning<B: QueryBuilderTrait>(#[case] builder: B) {
	// Arrange
	let mut statement = Query::insert();
	statement
		.into_table("registry")
		.columns(["id"])
		.values_panic([1])
		.on_conflict(OnConflict::new().do_nothing())
		.returning_all();

	// Act
	let sql = statement.to_string(builder);

	// Assert
	assert_eq!(
		sql,
		"INSERT INTO \"registry\" (\"id\") VALUES (1) ON CONFLICT DO NOTHING RETURNING *"
	);
}

#[rstest]
#[case::postgres(
	PostgresQueryBuilder,
	"INSERT INTO \"registry\" (\"id\", \"version\") VALUES ($1, $2) ON CONFLICT (\"id\", \"version\") DO UPDATE SET \"version\" = EXCLUDED.\"version\""
)]
#[case::sqlite(
	SqliteQueryBuilder,
	"INSERT INTO \"registry\" (\"id\", \"version\") VALUES (?, ?) ON CONFLICT (\"id\", \"version\") DO UPDATE SET \"version\" = EXCLUDED.\"version\""
)]
#[case::mysql(
	MySqlQueryBuilder,
	"INSERT INTO `registry` (`id`, `version`) VALUES (?, ?) ON DUPLICATE KEY UPDATE `version` = VALUES(`version`)"
)]
fn nonempty_conflict_target_preserves_updates<B: QueryBuilderTrait>(
	#[case] builder: B,
	#[case] expected_sql: &str,
) {
	// Arrange
	let mut statement = Query::insert();
	statement
		.into_table("registry")
		.columns(["id", "version"])
		.values_panic([Value::from(1), Value::from("first")])
		.on_conflict(OnConflict::columns(["id", "version"]).update_columns(["version"]));

	// Act
	let (sql, values) = statement.build(builder);

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(values.0, vec![1.into(), "first".into()]);
}

#[rstest]
#[case::parameterized(false)]
#[case::inline(true)]
#[should_panic(expected = "PostgreSQL ON CONFLICT DO UPDATE requires a conflict target")]
fn postgres_rejects_targetless_updates(
	#[case] inline: bool,
	#[values(OnConflict::new(), OnConflict::columns(std::iter::empty::<Alias>()))]
	conflict: OnConflict,
) {
	// Arrange
	let mut statement = Query::insert();
	statement
		.into_table("registry")
		.columns(["id"])
		.values_panic([1])
		.on_conflict(conflict.update_columns(["id"]));

	// Act
	if inline {
		statement.to_string(PostgresQueryBuilder);
	} else {
		statement.build(PostgresQueryBuilder);
	}
}

#[rstest]
fn sqlite_supports_targetless_updates() {
	// Arrange
	let mut statement = Query::insert();
	statement
		.into_table("registry")
		.columns(["id"])
		.values_panic([1])
		.on_conflict(OnConflict::new().update_columns(["id"]));

	// Act
	let (sql, values) = statement.build(SqliteQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		"INSERT INTO \"registry\" (\"id\") VALUES (?) ON CONFLICT DO UPDATE SET \"id\" = EXCLUDED.\"id\""
	);
	assert_eq!(values.0, vec![1.into()]);
}

#[rstest]
#[case::constructor(OnConflict::new().do_nothing())]
#[case::empty_columns(OnConflict::columns(std::iter::empty::<Alias>()).do_nothing())]
#[tokio::test]
async fn postgres_targetless_insert_select_skips_unique_conflicts(
	#[future] postgres_container: (PgContainer, Arc<sqlx::PgPool>, u16, String),
	mut registry_insert_select: InsertStatement,
	#[case] conflict: OnConflict,
) {
	// Arrange
	let (_container, pool, _port, _url) = postgres_container.await;
	let table_sql = Query::create_table()
		.table("registry")
		.col(
			ColumnDef::new("id")
				.column_type(ColumnType::Integer)
				.primary_key(true),
		)
		.col(ColumnDef::new("version").column_type(ColumnType::String(None)))
		.col(ColumnDef::new("kind").column_type(ColumnType::String(None)))
		.col(ColumnDef::new("metadata").column_type(ColumnType::String(None)))
		.unique(["version", "kind"])
		.to_string(PostgresQueryBuilder);
	sqlx::query(&table_sql)
		.execute(pool.as_ref())
		.await
		.expect("create registry table");
	registry_insert_select.on_conflict(conflict);
	let (sql, values) = registry_insert_select.build(PostgresQueryBuilder);
	let mut affected_rows = Vec::new();

	// Act
	for (id, version, kind, metadata) in [
		(1, "1.0", "widget", "original"),
		(1, "2.0", "widget", "primary-key collision"),
		(2, "1.0", "widget", "natural-key collision"),
		(3, "2.0", "widget", "new"),
	] {
		let result = sqlx::query(&sql)
			.bind(id)
			.bind(version)
			.bind(kind)
			.bind(metadata)
			.execute(pool.as_ref())
			.await
			.expect("execute targetless INSERT SELECT");
		affected_rows.push(result.rows_affected());
	}
	let select_sql = Query::select()
		.columns(["id", "version", "kind", "metadata"])
		.from("registry")
		.order_by("id", Order::Asc)
		.to_string(PostgresQueryBuilder);
	let rows: Vec<(i32, String, String, String)> = sqlx::query_as(&select_sql)
		.fetch_all(pool.as_ref())
		.await
		.expect("read registry rows");

	// Assert
	assert_eq!(values.0, vec![]);
	assert_eq!(affected_rows, vec![1, 0, 0, 1]);
	assert_eq!(
		rows,
		vec![
			(1, "1.0".into(), "widget".into(), "original".into()),
			(3, "2.0".into(), "widget".into(), "new".into()),
		]
	);
}
