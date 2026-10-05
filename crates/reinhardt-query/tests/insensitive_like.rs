//! Portable escaped matching preserves syntax, grouping and bound parameters.
use reinhardt_query::{
	CockroachDBQueryBuilder, Expr, ExprTrait, MySqlQueryBuilder, PostgresQueryBuilder, Query,
	QueryBuilder, QueryStatementBuilder, SelectStatement, SqliteQueryBuilder, Values,
};
use rstest::rstest;

fn render(backend: &str, statement: &SelectStatement) -> (String, Values) {
	match backend {
		"postgres" => PostgresQueryBuilder.build_select(statement),
		"mysql" => MySqlQueryBuilder.build_select(statement),
		"sqlite" => SqliteQueryBuilder.build_select(statement),
		"cockroach" => CockroachDBQueryBuilder::new().build_select(statement),
		_ => panic!("unknown test backend"),
	}
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
	let (sql, values) = render(backend, &statement);
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
	let (sql, values) = render(backend, &statement);
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
