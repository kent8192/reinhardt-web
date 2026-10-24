//! Explicit AST grouping preserves operands, boolean scope and value traversal.
use reinhardt_query::query::{ExplainOptions, ExplainStatement};
use reinhardt_query::{
	CockroachDBQueryBuilder, Expr, ExprTrait, MySqlQueryBuilder, PostgresQueryBuilder, Query,
	QueryBuildError, QueryStatementBuilder, SelectStatement, SimpleExpr, SqliteQueryBuilder,
	Values,
};
use rstest::rstest;

fn checked(backend: &str, statement: &SelectStatement) -> (String, Values) {
	match backend {
		"postgres" => PostgresQueryBuilder.build_select_checked(statement),
		"mysql" => MySqlQueryBuilder.build_select_checked(statement),
		"sqlite" => SqliteQueryBuilder.build_select_checked(statement),
		"cockroach" => CockroachDBQueryBuilder::new().build_select_checked(statement),
		_ => panic!("unknown test backend"),
	}
	.unwrap()
}

#[rstest]
#[case::postgres("postgres", ["$1", "$2", "$3", "$4", "$5"], "\"quoted\"\"id`\"", "\"enabled\"", "\"locked\"")]
#[case::mysql("mysql", ["?", "?", "?", "?", "?"], "`quoted\"id```", "`enabled`", "`locked`")]
#[case::sqlite("sqlite", ["?", "?", "?", "?", "?"], "\"quoted\"\"id`\"", "\"enabled\"", "\"locked\"")]
#[case::cockroach("cockroach", ["$1", "$2", "$3", "$4", "$5"], "\"quoted\"\"id`\"", "\"enabled\"", "\"locked\"")]
fn explicit_groups_preserve_arithmetic_boolean_and_scalar_structure(
	#[case] backend: &str,
	#[case] slots: [&str; 5],
	#[case] field: &str,
	#[case] enabled: &str,
	#[case] locked: &str,
) {
	// Arrange: every group is an AST node, including a grouped scalar parameter.
	let payload = "bound' ? $77";
	let statement = Query::select()
		.expr(Expr::val(payload).grouped())
		.expr(
			Expr::col("quoted\"id`")
				.mul(Expr::val(7_i64).sub(3_i64).grouped())
				.grouped(),
		)
		.expr(
			Expr::col("enabled")
				.eq(false)
				.or(Expr::col("locked").eq(true))
				.grouped()
				.not(),
		)
		.to_owned();
	// Act
	let (sql, values) = checked(backend, &statement);
	// Assert
	assert_eq!(
		sql,
		format!(
			"SELECT ({}), ({field} * ({} - {})), NOT ({enabled} = {} OR {locked} = {})",
			slots[0], slots[1], slots[2], slots[3], slots[4]
		)
	);
	assert_eq!(
		values.0,
		vec![
			payload.into(),
			7_i64.into(),
			3_i64.into(),
			false.into(),
			true.into()
		]
	);
	assert!(!sql.contains(payload));
}

#[rstest]
fn grouping_retains_mapping_and_input_ownership() {
	// Arrange
	let statement = Query::select()
		.expr(Expr::val("first").grouped())
		.expr(Expr::val("second").grouped().grouped())
		.to_owned();
	let original = checked("postgres", &statement);
	// Act
	let mapped = statement
		.try_map_value_expressions(|value| {
			Ok::<_, std::convert::Infallible>(Expr::val(value.clone()).cast_as_text())
		})
		.unwrap();
	let (sql, values) = checked("postgres", &mapped);
	// Assert
	assert_eq!(sql, "SELECT (CAST($1 AS TEXT)), ((CAST($2 AS TEXT)))");
	assert_eq!(values.0, vec!["first".into(), "second".into()]);
	assert_eq!(checked("postgres", &statement), original);
}

#[rstest]
fn checked_build_inspects_the_grouped_child() {
	// Arrange
	let statement = Query::select()
		.expr(reinhardt_query::Func::mysql_last_insert_id(None).grouped())
		.to_owned();
	// Act
	let result = PostgresQueryBuilder.build_select_checked(&statement);
	// Assert
	assert_eq!(
		result,
		Err(QueryBuildError::UnsupportedBackendFeature {
			feature: "MySQL last insert ID",
			backend: "PostgreSQL",
		})
	);
}

#[rstest]
fn safe_explain_inspects_the_grouped_child() {
	// Arrange
	let statement = ExplainStatement::new(
		Query::select()
			.expr(Expr::cust("unchecked()").grouped())
			.to_owned(),
		ExplainOptions::default(),
	);
	// Act
	let result = statement.build_mysql_checked();
	// Assert
	assert_eq!(
		result,
		Err(QueryBuildError::UnsupportedBackendFeature {
			feature: "plan-only EXPLAIN for subqueries or unchecked expressions",
			backend: "MySQL",
		})
	);
}

#[rstest]
fn postgres_inline_rendering_preserves_a_grouped_right_operand() {
	// Arrange
	let statement = Query::select()
		.expr(Expr::val(8_i64).div(Expr::val(3_i64).sub(1_i64).grouped()))
		.expr(SimpleExpr::Grouped(Box::new(
			Expr::val("a' b").into_simple_expr(),
		)))
		.to_owned();
	// Act
	let sql = statement.to_string(PostgresQueryBuilder);
	// Assert
	assert_eq!(sql, "SELECT 8 / (3 - 1), ('a'' b')");
}
