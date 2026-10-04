//! Portable escaped matching preserves syntax, grouping and bound parameters.
use reinhardt_query::query::{ExplainOptions, ExplainStatement};
use reinhardt_query::{
	CockroachDBQueryBuilder, Expr, ExprTrait, MySqlQueryBuilder, PostgresQueryBuilder, Query,
	QueryBuildError, QueryStatementBuilder, SelectStatement, SqliteQueryBuilder, Values,
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
#[case::postgres("postgres", "(\"quoted\"\"column`\" ILIKE $1 ESCAPE '\\')")]
#[case::mysql("mysql", "(LOWER(`quoted\"column```) LIKE LOWER(?) ESCAPE 0x5C)")]
#[case::sqlite("sqlite", "(LOWER(\"quoted\"\"column`\") LIKE LOWER(?) ESCAPE '\\')")]
#[case::cockroach("cockroach", "(\"quoted\"\"column`\" ILIKE $1 ESCAPE '\\')")]
fn escaped_patterns_and_identifiers_are_preserved(#[case] backend: &str, #[case] predicate: &str) {
	// Arrange: a deliberate wildcard surrounds already escaped literal data.
	let pattern = r"%BOUND' ? $42\%\_\\%";
	let statement = Query::select()
		.expr(Expr::col("quoted\"column`").ilike_with_escape(pattern))
		.to_owned();
	// Act
	let (sql, values) = checked(backend, &statement);
	// Assert
	assert_eq!(sql, format!("SELECT {predicate}"));
	assert_eq!(values.0, vec![pattern.into()]);
	assert!(!sql.contains("BOUND"));
}

#[rstest]
#[case::postgres(
	"postgres",
	"(\"name\" ILIKE $1 ESCAPE '\\')",
	"$2",
	"\"active\"",
	"$3"
)]
#[case::mysql(
	"mysql",
	"(LOWER(`name`) LIKE LOWER(?) ESCAPE 0x5C)",
	"?",
	"`active`",
	"?"
)]
#[case::sqlite(
	"sqlite",
	"(LOWER(\"name\") LIKE LOWER(?) ESCAPE '\\')",
	"?",
	"\"active\"",
	"?"
)]
#[case::cockroach(
	"cockroach",
	"(\"name\" ILIKE $1 ESCAPE '\\')",
	"$2",
	"\"active\"",
	"$3"
)]
fn predicate_grouping_survives_equality_and_negation(
	#[case] backend: &str,
	#[case] predicate: &str,
	#[case] equality_slot: &str,
	#[case] column: &str,
	#[case] final_slot: &str,
) {
	// Arrange
	let statement = Query::select()
		.expr(
			Expr::col("name")
				.ilike_with_escape("%a%")
				.eq(false)
				.and(Expr::col("active").eq(true)),
		)
		.expr(Expr::col("name").ilike_with_escape("%b%").not())
		.to_owned();
	// Act
	let (sql, values) = checked(backend, &statement);
	// Assert
	let negated = match backend {
		"postgres" | "cockroach" => "NOT (\"name\" ILIKE $4 ESCAPE '\\')",
		"mysql" => "NOT (LOWER(`name`) LIKE LOWER(?) ESCAPE 0x5C)",
		"sqlite" => "NOT (LOWER(\"name\") LIKE LOWER(?) ESCAPE '\\')",
		_ => unreachable!(),
	};
	assert_eq!(
		sql,
		format!("SELECT {predicate} = {equality_slot} AND {column} = {final_slot}, {negated}")
	);
	assert_eq!(
		values.0,
		vec!["%a%".into(), false.into(), true.into(), "%b%".into()]
	);
}

#[rstest]
fn parameter_mapping_visits_both_operands_without_changing_source() {
	// Arrange
	let source = Query::select()
		.expr(Expr::val("stored").ilike_with_escape("%STORED%"))
		.to_owned();
	let original = checked("mysql", &source);
	// Act
	let mapped = source
		.try_map_value_expressions(|value| {
			Ok::<_, std::convert::Infallible>(Expr::val(value.clone()).cast_as_text())
		})
		.unwrap();
	let (sql, values) = checked("mysql", &mapped);
	// Assert
	assert_eq!(
		sql,
		"SELECT (LOWER(CAST(? AS CHAR)) LIKE LOWER(CAST(? AS CHAR)) ESCAPE 0x5C)"
	);
	assert_eq!(values.0, vec!["stored".into(), "%STORED%".into()]);
	assert_eq!(checked("mysql", &source), original);
}

#[rstest]
#[case::left(true)]
#[case::right(false)]
fn checked_build_validates_both_operands(#[case] unsupported_left: bool) {
	// Arrange
	let unsupported = reinhardt_query::Func::mysql_last_insert_id(None);
	let expression = if unsupported_left {
		unsupported.ilike_with_escape("%x%")
	} else {
		Expr::col("name").ilike_with_escape(unsupported)
	};
	let statement = Query::select().expr(expression).to_owned();
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
#[case::left(true)]
#[case::right(false)]
fn safe_explain_rejects_unchecked_children(#[case] unchecked_left: bool) {
	// Arrange
	let expression = if unchecked_left {
		Expr::cust("unchecked()").ilike_with_escape("%x%")
	} else {
		Expr::col("name").ilike_with_escape(Expr::cust("unchecked()"))
	};
	let statement = ExplainStatement::new(
		Query::select().expr(expression).to_owned(),
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
fn safe_mysql_explain_accepts_portable_matching() {
	// Arrange
	let select = Query::select()
		.expr(Expr::col("name").ilike_with_escape("%Ada%"))
		.to_owned();
	// Act
	let (sql, values) = ExplainStatement::new(select, ExplainOptions::default())
		.build_mysql_checked()
		.unwrap();
	// Assert
	assert_eq!(
		sql,
		"EXPLAIN SELECT (LOWER(`name`) LIKE LOWER(?) ESCAPE 0x5C)"
	);
	assert_eq!(values.0, vec!["%Ada%".into()]);
}

#[rstest]
fn postgres_inline_rendering_retains_the_typed_predicate() {
	// Arrange
	let statement = Query::select()
		.expr(Expr::col("name").ilike_with_escape("A'_%"))
		.to_owned();
	// Act
	let sql = statement.to_string(PostgresQueryBuilder);
	// Assert
	assert_eq!(sql, "SELECT (\"name\" ILIKE 'A''_%' ESCAPE '\\')");
}
