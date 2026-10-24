//! Closed built-in casts preserve backend grammar and bound values.
use reinhardt_query::query::{ExplainOptions, ExplainStatement};
use reinhardt_query::{
	CockroachDBQueryBuilder, Expr, ExprTrait, MySqlQueryBuilder, PostgresQueryBuilder, Query,
	QueryBuildError, SelectStatement, SimpleExpr, SqliteQueryBuilder,
};
use rstest::rstest;

fn checked(backend: &str, statement: &SelectStatement) -> (String, reinhardt_query::Values) {
	match backend {
		"postgres" => PostgresQueryBuilder.build_select_checked(statement),
		"mysql" => MySqlQueryBuilder.build_select_checked(statement),
		"sqlite" => SqliteQueryBuilder.build_select_checked(statement),
		"cockroach" => CockroachDBQueryBuilder::new().build_select_checked(statement),
		_ => panic!("unsupported test backend"),
	}
	.unwrap()
}

#[rstest]
#[case::postgres("postgres", "TEXT", "BIGINT", "$1", "$2", "\"quoted\"\"name`\"")]
#[case::mysql("mysql", "CHAR", "SIGNED", "?", "?", "`quoted\"name```")]
#[case::sqlite("sqlite", "TEXT", "INTEGER", "?", "?", "\"quoted\"\"name`\"")]
#[case::cockroach("cockroach", "TEXT", "BIGINT", "$1", "$2", "\"quoted\"\"name`\"")]
fn casts_preserve_backend_types_and_values(
	#[case] backend: &str,
	#[case] text_type: &str,
	#[case] integer_type: &str,
	#[case] first: &str,
	#[case] second: &str,
	#[case] alias: &str,
) {
	// Arrange
	let payload = "bound' ? $42";
	let statement = Query::select()
		.expr_as(Expr::val(payload).cast_as_text(), "quoted\"name`")
		.expr(Expr::val(7_i64).cast_as_signed_integer())
		.to_owned();
	// Act
	let (sql, values) = checked(backend, &statement);
	// Assert
	assert_eq!(
		sql,
		format!("SELECT CAST({first} AS {text_type}) AS {alias}, CAST({second} AS {integer_type})")
	);
	assert_eq!(values.0, vec![payload.into(), 7_i64.into()]);
	assert!(!sql.contains(payload));
}

#[rstest]
#[case::text(Expr::val(7_i64).cast_as_text())]
#[case::signed(Expr::val(7_i64).cast_as_signed_integer())]
fn mysql_explain_retains_cast_restriction(#[case] expression: SimpleExpr) {
	// Arrange
	let statement = ExplainStatement::new(
		Query::select().expr(expression).to_owned(),
		ExplainOptions::default(),
	);
	// Act
	let error = statement.build_mysql_checked().unwrap_err();
	// Assert
	assert_eq!(
		error,
		QueryBuildError::UnsupportedBackendFeature {
			feature: "CAST expressions",
			backend: "MySQL",
		}
	);
}

#[rstest]
#[case::text(Expr::current_timestamp().into_simple_expr().cast_as_text())]
#[case::signed(Expr::current_timestamp().into_simple_expr().cast_as_signed_integer())]
fn null_and_nested_values_remain_structural(#[case] expression: SimpleExpr) {
	// Arrange
	let statement = Query::select()
		.expr(Expr::val(reinhardt_query::Value::String(None)).cast_as_text())
		.expr(expression)
		.to_owned();
	// Act
	let (sql, values) = checked("postgres", &statement);
	// Assert
	assert!(sql.starts_with("SELECT CAST(NULL AS TEXT), CAST(CURRENT_TIMESTAMP AS "));
	assert!(values.0.is_empty());
}
