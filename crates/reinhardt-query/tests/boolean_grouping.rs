//! MySQL and SQLite regression coverage for nested boolean grouping.
//!
//! A mandatory predicate followed by a typed alternative must keep the
//! alternative grouped, otherwise `guard AND (left OR right)` is rendered as
//! `guard AND left OR right` and `right` bypasses `guard`.

use std::sync::Arc;

use reinhardt_query::{
	ColumnDef, Condition, ConditionExpression, CreateTableStatement, Expr, ExprTrait,
	InsertStatement, JoinType, MySqlQueryBuilder, Query, QueryStatementBuilder, SelectStatement,
	SimpleExpr, SqliteQueryBuilder, Value, Values,
};
use rstest::*;
use sqlx::{Arguments, Connection, Row};

mod common;
use common::{MySqlContainer, mysql_container};

#[derive(Debug, Clone, Copy)]
enum Backend {
	MySql,
	Sqlite,
}

impl Backend {
	fn build<S: QueryStatementBuilder>(self, statement: &S) -> (String, Values) {
		match self {
			Self::MySql => statement.build(MySqlQueryBuilder),
			Self::Sqlite => statement.build(SqliteQueryBuilder),
		}
	}

	fn to_string<S: QueryStatementBuilder>(self, statement: &S) -> String {
		match self {
			Self::MySql => statement.to_string(MySqlQueryBuilder),
			Self::Sqlite => statement.to_string(SqliteQueryBuilder),
		}
	}

	/// Quote identifiers in an expected SQL template written with backticks.
	fn quote(self, template: &str) -> String {
		match self {
			Self::MySql => template.to_owned(),
			Self::Sqlite => template.replace('`', "\""),
		}
	}
}

#[fixture]
fn publication_predicate() -> SimpleExpr {
	Expr::col("publish_until")
		.is_null()
		.or(Expr::col("publish_until").lte(Expr::cust("CURRENT_TIMESTAMP")))
}

#[rstest]
#[case::left_or(Expr::val(true).or(false).and(false), "SELECT (? OR ?) AND ?")]
#[case::right_or(Expr::val(false).and(Expr::val(true).or(false)), "SELECT ? AND (? OR ?)")]
#[case::and_before_or(Expr::val(true).or(Expr::val(false).and(false)), "SELECT ? OR ? AND ?")]
#[case::associative_and(Expr::val(true).and(Expr::val(false).and(true)), "SELECT ? AND ? AND ?")]
#[case::not_or(Expr::val(true).or(false).not(), "SELECT NOT (? OR ?)")]
#[case::not_and(Expr::val(true).and(false).not(), "SELECT NOT (? AND ?)")]
#[case::not_comparison(Expr::val(true).not().eq(false), "SELECT (NOT ?) = ?")]
#[case::nested_comparison(Expr::val(true).eq(false).eq(true), "SELECT (? = ?) = ?")]
#[case::between_operand(Expr::val(1).eq(2).between(false, true), "SELECT (? = ?) BETWEEN ? AND ?")]
#[case::in_operand(Expr::val(true).or(false).is_in([true, false]), "SELECT (? OR ?) IN (?, ?)")]
#[case::bitwise_right(Expr::val(1).bit_or(Expr::val(2).bit_and(4)), "SELECT ? | (? & ?)")]
#[case::bitwise_left(Expr::val(1).bit_or(2).bit_and(4), "SELECT (? | ?) & ?")]
fn boolean_operands_preserve_grouping(
	#[case] expression: SimpleExpr,
	#[case] expected_sql: &str,
	#[values(Backend::MySql, Backend::Sqlite)] backend: Backend,
) {
	// Arrange
	let query = Query::select().expr(expression).to_owned();

	// Act
	let (sql, _) = backend.build(&query);

	// Assert
	assert_eq!(sql, expected_sql);
}

#[rstest]
fn where_chain_keeps_mandatory_guard(#[values(Backend::MySql, Backend::Sqlite)] backend: Backend) {
	// Arrange
	let query = Query::select()
		.expr(Expr::val(7_i32))
		.and_where(Expr::val(false))
		.and_where(Expr::val(false).or(Expr::val(true)))
		.to_owned();

	// Act
	let (sql, values) = backend.build(&query);

	// Assert
	assert_eq!(sql, "SELECT ? WHERE ? AND (? OR ?)");
	assert_eq!(
		values,
		Values(vec![7_i32.into(), false.into(), false.into(), true.into()])
	);
}

#[rstest]
fn single_where_predicate_is_not_wrapped(
	#[values(Backend::MySql, Backend::Sqlite)] backend: Backend,
	publication_predicate: SimpleExpr,
) {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("obligations")
		.and_where(publication_predicate)
		.to_owned();

	// Act
	let sql = backend.to_string(&query);

	// Assert
	assert_eq!(
		sql,
		backend.quote(
			"SELECT `id` FROM `obligations` WHERE `publish_until` IS NULL OR `publish_until` <= CURRENT_TIMESTAMP"
		)
	);
}

#[rstest]
fn where_chain_preserves_typed_or_grouping(
	#[values(Backend::MySql, Backend::Sqlite)] backend: Backend,
	publication_predicate: SimpleExpr,
) {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("obligations")
		.and_where(Expr::col("tenant_id").eq(1_i64))
		.and_where(publication_predicate)
		.to_owned();

	// Act
	let (sql, values) = backend.build(&query);
	let inlined = backend.to_string(&query);

	// Assert
	assert_eq!(
		sql,
		backend.quote(
			"SELECT `id` FROM `obligations` WHERE `tenant_id` = ? AND (`publish_until` IS NULL OR `publish_until` <= CURRENT_TIMESTAMP)"
		)
	);
	assert_eq!(values, Values(vec![1_i64.into()]));
	assert_eq!(
		inlined,
		backend.quote(
			"SELECT `id` FROM `obligations` WHERE `tenant_id` = 1 AND (`publish_until` IS NULL OR `publish_until` <= CURRENT_TIMESTAMP)"
		)
	);
}

#[rstest]
fn condition_all_preserves_typed_or_grouping(
	#[values(Backend::MySql, Backend::Sqlite)] backend: Backend,
	publication_predicate: SimpleExpr,
) {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("obligations")
		.cond_where(
			Condition::all()
				.add(Expr::col("tenant_id").eq(1_i64))
				.add(publication_predicate),
		)
		.to_owned();

	// Act
	let sql = backend.to_string(&query);

	// Assert
	assert_eq!(
		sql,
		backend.quote(
			"SELECT `id` FROM `obligations` WHERE (`tenant_id` = 1 AND (`publish_until` IS NULL OR `publish_until` <= CURRENT_TIMESTAMP))"
		)
	);
}

#[rstest]
fn single_condition_preserves_typed_or_grouping(
	#[values(Backend::MySql, Backend::Sqlite)] backend: Backend,
	publication_predicate: SimpleExpr,
) {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("obligations")
		.and_where(Expr::col("tenant_id").eq(1_i64))
		.and_where(Condition::any().add(publication_predicate))
		.to_owned();

	// Act
	let sql = backend.to_string(&query);

	// Assert
	assert_eq!(
		sql,
		backend.quote(
			"SELECT `id` FROM `obligations` WHERE `tenant_id` = 1 AND (`publish_until` IS NULL OR `publish_until` <= CURRENT_TIMESTAMP)"
		)
	);
}

#[rstest]
#[case::negated_condition(Condition::any().add(Expr::col("id").eq(1_i64).or(Expr::col("is_public").eq(true))).not())]
#[case::unary_not(Condition::all().add(Expr::col("id").eq(1_i64).or(Expr::col("is_public").eq(true)).not()))]
fn negated_alternatives_stay_grouped(
	#[case] negated: Condition,
	#[values(Backend::MySql, Backend::Sqlite)] backend: Backend,
) {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("documents")
		.and_where(Expr::col("tenant_id").eq(1_i64))
		.and_where(negated)
		.to_owned();

	// Act
	let (sql, values) = backend.build(&query);

	// Assert
	assert_eq!(
		sql,
		backend.quote(
			"SELECT `id` FROM `documents` WHERE `tenant_id` = ? AND NOT (`id` = ? OR `is_public` = ?)"
		)
	);
	assert_eq!(
		values,
		Values(vec![1_i64.into(), 1_i64.into(), true.into()])
	);
}

#[rstest]
fn having_preserves_typed_or_grouping(
	#[values(Backend::MySql, Backend::Sqlite)] backend: Backend,
	publication_predicate: SimpleExpr,
) {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("obligations")
		.and_having(Expr::col("tenant_id").eq(1_i64))
		.and_having(publication_predicate)
		.to_owned();

	// Act
	let sql = backend.to_string(&query);

	// Assert
	assert_eq!(
		sql,
		backend.quote(
			"SELECT `id` FROM `obligations` HAVING `tenant_id` = 1 AND (`publish_until` IS NULL OR `publish_until` <= CURRENT_TIMESTAMP)"
		)
	);
}

#[rstest]
fn join_on_preserves_typed_or_grouping(
	#[values(Backend::MySql, Backend::Sqlite)] backend: Backend,
	publication_predicate: SimpleExpr,
) {
	// Arrange
	let query = Query::select()
		.column(("obligations", "id"))
		.from("obligations")
		.join(
			JoinType::InnerJoin,
			"tenants",
			Condition::all()
				.add(Expr::col(("tenants", "id")).equals(("obligations", "tenant_id")))
				.add(publication_predicate),
		)
		.to_owned();

	// Act
	let sql = backend.to_string(&query);

	// Assert
	assert_eq!(
		sql,
		backend.quote(
			"SELECT `obligations`.`id` FROM `obligations` INNER JOIN `tenants` ON (`tenants`.`id` = `obligations`.`tenant_id` AND (`publish_until` IS NULL OR `publish_until` <= CURRENT_TIMESTAMP))"
		)
	);
}

#[rstest]
fn update_preserves_typed_or_grouping(
	#[values(Backend::MySql, Backend::Sqlite)] backend: Backend,
	publication_predicate: SimpleExpr,
) {
	// Arrange
	let query = Query::update()
		.table("obligations")
		.value("state", "settled")
		.and_where(Expr::col("tenant_id").eq(1_i64))
		.and_where(publication_predicate)
		.to_owned();

	// Act
	let (sql, values) = backend.build(&query);

	// Assert
	assert_eq!(
		sql,
		backend.quote(
			"UPDATE `obligations` SET `state` = ? WHERE `tenant_id` = ? AND (`publish_until` IS NULL OR `publish_until` <= CURRENT_TIMESTAMP)"
		)
	);
	assert_eq!(values, Values(vec!["settled".into(), 1_i64.into()]));
}

#[rstest]
fn delete_preserves_typed_or_grouping(
	#[values(Backend::MySql, Backend::Sqlite)] backend: Backend,
	publication_predicate: SimpleExpr,
) {
	// Arrange
	let query = Query::delete()
		.from_table("obligations")
		.and_where(Expr::col("tenant_id").eq(1_i64))
		.and_where(publication_predicate)
		.to_owned();

	// Act
	let (sql, values) = backend.build(&query);

	// Assert
	assert_eq!(
		sql,
		backend.quote(
			"DELETE FROM `obligations` WHERE `tenant_id` = ? AND (`publish_until` IS NULL OR `publish_until` <= CURRENT_TIMESTAMP)"
		)
	);
	assert_eq!(values, Values(vec![1_i64.into()]));
}

#[rstest]
fn custom_or_retains_neighboring_filters(
	#[values(Backend::MySql, Backend::Sqlite)] backend: Backend,
) {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("runs")
		.and_where(Expr::col("control").ne("PAUSED"))
		.and_where(Expr::cust(
			"lease_until IS NULL OR lease_until <= CURRENT_TIMESTAMP",
		))
		.and_where(Expr::col("id").eq(2_i64))
		.to_owned();

	// Act
	let (sql, values) = backend.build(&query);

	// Assert
	assert_eq!(
		sql,
		backend.quote(
			"SELECT `id` FROM `runs` WHERE `control` <> ? AND (lease_until IS NULL OR lease_until <= CURRENT_TIMESTAMP) AND `id` = ?"
		)
	);
	assert_eq!(values, Values(vec!["PAUSED".into(), 2_i64.into()]));
}

#[rstest]
#[case::mysql_hash(
	Backend::MySql,
	"flag = 1 # note",
	"SELECT ? WHERE (flag = 1 # note\n) AND ?"
)]
#[case::mysql_dashes(
	Backend::MySql,
	"flag = 1 -- note",
	"SELECT ? WHERE (flag = 1 -- note\n) AND ?"
)]
#[case::sqlite_dashes(
	Backend::Sqlite,
	"flag = 1 -- note",
	"SELECT ? WHERE (flag = 1 -- note\n) AND ?"
)]
fn grouped_custom_line_comments_keep_closing_parentheses(
	#[case] backend: Backend,
	#[case] predicate: &str,
	#[case] expected_sql: &str,
) {
	// Arrange
	let query = Query::select()
		.expr(Expr::val(1_i32))
		.and_where(Expr::cust(predicate))
		.and_where(Expr::val(true))
		.to_owned();

	// Act
	let (sql, _) = backend.build(&query);

	// Assert
	assert_eq!(sql, expected_sql);
}

/// Tenant-scoped filters whose final alternative matches only another tenant's row.
fn tenant_guarded_queries() -> Vec<(&'static str, Condition)> {
	let alternative = || {
		Expr::col("id")
			.eq(99_i64)
			.or(Expr::col("is_public").eq(true))
	};
	vec![
		(
			"where chain",
			Condition::all()
				.add(Expr::col("tenant_id").eq(1_i64))
				.add(alternative()),
		),
		(
			"nested expression",
			Condition::all().add(Expr::col("tenant_id").eq(1_i64).and(alternative())),
		),
		(
			"negated condition",
			Condition::all().add(Expr::col("tenant_id").eq(1_i64)).add(
				Condition::any()
					.add(
						Expr::col("id")
							.eq(1_i64)
							.or(Expr::col("is_public").eq(false)),
					)
					.not(),
			),
		),
		(
			"unary not",
			Condition::all().add(Expr::col("tenant_id").eq(1_i64)).add(
				Expr::col("id")
					.eq(1_i64)
					.or(Expr::col("is_public").eq(false))
					.not(),
			),
		),
	]
}

fn tenant_query(condition: Condition) -> SelectStatement {
	let mut query = Query::select();
	query.column("id").from("documents");
	// Spread top-level conjuncts over separate `and_where` calls, as callers do.
	for conjunct in condition.conditions {
		match conjunct {
			ConditionExpression::Condition(condition) => query.and_where(condition),
			ConditionExpression::SimpleExpr(expr) => query.and_where(expr),
		};
	}
	query
}

fn create_documents() -> CreateTableStatement {
	Query::create_table()
		.table("documents")
		.col(ColumnDef::new("id").big_integer().primary_key(true))
		.col(ColumnDef::new("tenant_id").big_integer().not_null(true))
		.col(ColumnDef::new("is_public").boolean().not_null(true))
		.to_owned()
}

fn insert_documents() -> InsertStatement {
	// Document 1 is private to tenant 1; document 2 is public in tenant 2.
	Query::insert()
		.into_table("documents")
		.columns(["id", "tenant_id", "is_public"])
		.values_panic([Value::from(1_i64), 1_i64.into(), false.into()])
		.values_panic([Value::from(2_i64), 2_i64.into(), true.into()])
		.to_owned()
}

fn sqlite_arguments(values: Values) -> sqlx::sqlite::SqliteArguments<'static> {
	let mut arguments = sqlx::sqlite::SqliteArguments::default();
	for value in values.0 {
		match value {
			Value::Bool(Some(value)) => arguments.add(value).unwrap(),
			Value::Int(Some(value)) => arguments.add(value).unwrap(),
			Value::BigInt(Some(value)) => arguments.add(value).unwrap(),
			value => panic!("unexpected argument: {value:?}"),
		}
	}
	arguments
}

fn mysql_arguments(values: Values) -> sqlx::mysql::MySqlArguments {
	let mut arguments = sqlx::mysql::MySqlArguments::default();
	for value in values.0 {
		match value {
			Value::Bool(Some(value)) => arguments.add(value).unwrap(),
			Value::Int(Some(value)) => arguments.add(value).unwrap(),
			Value::BigInt(Some(value)) => arguments.add(value).unwrap(),
			value => panic!("unexpected argument: {value:?}"),
		}
	}
	arguments
}

#[rstest]
#[tokio::test]
async fn sqlite_guard_excludes_other_tenant_rows() {
	// Arrange
	let mut connection = sqlx::SqliteConnection::connect("sqlite::memory:")
		.await
		.unwrap();
	for statement in [
		create_documents().to_string(SqliteQueryBuilder),
		insert_documents().to_string(SqliteQueryBuilder),
	] {
		sqlx::query(&statement)
			.execute(&mut connection)
			.await
			.unwrap();
	}
	let (literal_sql, literal_values) = Query::select()
		.expr(Expr::val(7_i32))
		.and_where(Expr::val(false))
		.and_where(Expr::val(false).or(Expr::val(true)))
		.build(SqliteQueryBuilder);

	// Act
	let literal_rows = sqlx::query_with(&literal_sql, sqlite_arguments(literal_values))
		.fetch_all(&mut connection)
		.await
		.unwrap();
	let mut tenant_rows = Vec::new();
	for (label, condition) in tenant_guarded_queries() {
		let (sql, values) = tenant_query(condition).build(SqliteQueryBuilder);
		let ids = sqlx::query_with(&sql, sqlite_arguments(values))
			.fetch_all(&mut connection)
			.await
			.unwrap()
			.into_iter()
			.map(|row| row.get::<i64, _>(0))
			.collect::<Vec<_>>();
		tenant_rows.push((label, sql, ids));
	}
	let (delete_sql, delete_values) = Query::delete()
		.from_table("documents")
		.and_where(Expr::col("tenant_id").eq(1_i64))
		.and_where(
			Expr::col("id")
				.eq(99_i64)
				.or(Expr::col("is_public").eq(true)),
		)
		.build(SqliteQueryBuilder);
	let deleted = sqlx::query_with(&delete_sql, sqlite_arguments(delete_values))
		.execute(&mut connection)
		.await
		.unwrap()
		.rows_affected();

	// Assert
	assert_eq!(literal_rows.len(), 0, "{literal_sql}");
	for (label, sql, ids) in tenant_rows {
		assert_eq!(ids, Vec::<i64>::new(), "{label}: {sql}");
	}
	assert_eq!(deleted, 0, "{delete_sql}");
}

#[rstest]
#[tokio::test]
async fn mysql_guard_excludes_other_tenant_rows(
	#[future] mysql_container: (MySqlContainer, Arc<sqlx::MySqlPool>, u16, String),
) {
	// Arrange: the container guard owns cleanup, including assertion failures.
	let (_container, pool, _, _) = mysql_container.await;
	for statement in [
		create_documents().to_string(MySqlQueryBuilder),
		insert_documents().to_string(MySqlQueryBuilder),
	] {
		sqlx::query(&statement)
			.execute(pool.as_ref())
			.await
			.unwrap();
	}
	let (literal_sql, literal_values) = Query::select()
		.expr(Expr::val(7_i32))
		.and_where(Expr::val(false))
		.and_where(Expr::val(false).or(Expr::val(true)))
		.build(MySqlQueryBuilder);

	// Act
	let literal_rows = sqlx::query_with(&literal_sql, mysql_arguments(literal_values))
		.fetch_all(pool.as_ref())
		.await
		.unwrap();
	let mut tenant_rows = Vec::new();
	for (label, condition) in tenant_guarded_queries() {
		let (sql, values) = tenant_query(condition).build(MySqlQueryBuilder);
		let ids = sqlx::query_with(&sql, mysql_arguments(values))
			.fetch_all(pool.as_ref())
			.await
			.unwrap()
			.into_iter()
			.map(|row| row.get::<i64, _>(0))
			.collect::<Vec<_>>();
		tenant_rows.push((label, sql, ids));
	}
	let (delete_sql, delete_values) = Query::delete()
		.from_table("documents")
		.and_where(Expr::col("tenant_id").eq(1_i64))
		.and_where(
			Expr::col("id")
				.eq(99_i64)
				.or(Expr::col("is_public").eq(true)),
		)
		.build(MySqlQueryBuilder);
	let deleted = sqlx::query_with(&delete_sql, mysql_arguments(delete_values))
		.execute(pool.as_ref())
		.await
		.unwrap()
		.rows_affected();

	// Assert
	assert_eq!(literal_rows.len(), 0, "{literal_sql}");
	for (label, sql, ids) in tenant_rows {
		assert_eq!(ids, Vec::<i64>::new(), "{label}: {sql}");
	}
	assert_eq!(deleted, 0, "{delete_sql}");
}
