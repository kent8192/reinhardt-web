//! Stateful MySQL ID operations preserve values and checked capabilities.
use reinhardt_query::query::{ExplainOptions, ExplainStatement};
use reinhardt_query::{
	CockroachDBQueryBuilder, Expr, Func, MySqlQueryBuilder, PostgresQueryBuilder, Query,
	QueryBuildError, SqliteQueryBuilder, Values,
};
use rstest::rstest;

#[rstest]
fn mysql_read_last_insert_id_has_no_arguments() {
	// Arrange
	let statement = Query::select()
		.expr_as(Func::mysql_last_insert_id(None), "generated_id")
		.to_owned();
	// Act
	let (sql, values) = MySqlQueryBuilder.build_select_checked(&statement).unwrap();
	// Assert
	assert_eq!(
		sql,
		"SELECT CAST(LAST_INSERT_ID() AS SIGNED) AS `generated_id`"
	);
	assert!(values.0.is_empty());
}

#[rstest]
fn mysql_assign_last_insert_id_retains_argument_order() {
	// Arrange
	let statement = Query::select()
		.expr(Expr::val("bound' ? $42"))
		.expr_as(
			Func::mysql_last_insert_id(Some(Expr::val(0_i64).into_simple_expr())),
			"generated`id",
		)
		.expr(Expr::val(7_i64))
		.to_owned();
	// Act
	let (sql, values) = MySqlQueryBuilder.build_select_checked(&statement).unwrap();
	// Assert
	assert_eq!(
		sql,
		"SELECT ?, CAST(LAST_INSERT_ID(?) AS SIGNED) AS `generated``id`, ?"
	);
	assert_eq!(
		values,
		Values(vec!["bound' ? $42".into(), 0_i64.into(), 7_i64.into()])
	);
}

#[rstest]
#[case::postgres("PostgreSQL")]
#[case::sqlite("SQLite")]
#[case::cockroach("CockroachDB")]
fn unsupported_backend_rejects_last_insert_id(#[case] backend: &'static str) {
	// Arrange
	let statement = Query::select()
		.expr(Func::mysql_last_insert_id(None))
		.to_owned();
	// Act
	let error = match backend {
		"PostgreSQL" => PostgresQueryBuilder.build_select_checked(&statement),
		"SQLite" => SqliteQueryBuilder.build_select_checked(&statement),
		"CockroachDB" => CockroachDBQueryBuilder::new().build_select_checked(&statement),
		_ => panic!("unsupported test backend"),
	}
	.unwrap_err();
	// Assert
	assert_eq!(
		error,
		QueryBuildError::UnsupportedBackendFeature {
			feature: "MySQL last insert ID",
			backend,
		}
	);
}

#[rstest]
fn last_insert_id_validates_its_nested_expression() {
	// Arrange
	let statement = Query::select()
		.expr(Func::mysql_last_insert_id(Some(Func::pg_extract_epoch(
			Expr::current_timestamp().into_simple_expr(),
		))))
		.to_owned();
	// Act
	let error = MySqlQueryBuilder
		.build_select_checked(&statement)
		.unwrap_err();
	// Assert
	assert_eq!(
		error,
		QueryBuildError::UnsupportedBackendFeature {
			feature: "PostgreSQL numeric epoch extraction",
			backend: "MySQL",
		}
	);
}

#[rstest]
#[case::read(None)]
#[case::assign(Some(Expr::val(0_i64).into_simple_expr()))]
fn mysql_explain_rejects_connection_state_expressions(
	#[case] value: Option<reinhardt_query::SimpleExpr>,
) {
	// Arrange
	let statement = ExplainStatement::new(
		Query::select()
			.expr(Func::mysql_last_insert_id(value))
			.to_owned(),
		ExplainOptions::default(),
	);
	// Act
	let error = statement.build_mysql_checked().unwrap_err();
	// Assert
	assert_eq!(
		error,
		QueryBuildError::UnsupportedBackendFeature {
			feature: "plan-only EXPLAIN for subqueries or unchecked expressions",
			backend: "MySQL",
		}
	);
}
