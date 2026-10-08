//! Backend-checked analysis preserves identifiers and explicit target capabilities.
use reinhardt_query::query::AnalyzeStatement;
use reinhardt_query::{
	CockroachDBQueryBuilder, MySqlQueryBuilder, PostgresQueryBuilder, Query, QueryBuildError,
	SqliteQueryBuilder, Values,
};
use rstest::rstest;

fn checked(
	backend: &str,
	statement: &AnalyzeStatement,
) -> Result<(String, Values), QueryBuildError> {
	match backend {
		"PostgreSQL" => PostgresQueryBuilder.build_analyze_checked(statement),
		"MySQL" => MySqlQueryBuilder.build_analyze_checked(statement),
		"SQLite" => SqliteQueryBuilder.build_analyze_checked(statement),
		"CockroachDB" => CockroachDBQueryBuilder::new().build_analyze_checked(statement),
		_ => panic!("unsupported test backend"),
	}
}

#[rstest]
#[case::postgres("PostgreSQL", "ANALYZE \"quoted\"\"table`\"")]
#[case::mysql("MySQL", "ANALYZE TABLE `quoted\"table```")]
#[case::sqlite("SQLite", "ANALYZE \"quoted\"\"table`\"")]
#[case::cockroach("CockroachDB", "ANALYZE \"quoted\"\"table`\"")]
fn analyze_identifier_escaping(#[case] backend: &str, #[case] expected: &str) {
	// Arrange
	let statement = Query::analyze().table("quoted\"table`").to_owned();
	// Act
	let (sql, values) = checked(backend, &statement).unwrap();
	// Assert
	assert_eq!(sql, expected);
	assert!(values.0.is_empty());
}

#[rstest]
#[case::postgres("PostgreSQL", "ANALYZE \"one\", \"two\"")]
#[case::mysql("MySQL", "ANALYZE TABLE `one`, `two`")]
fn analyze_table_list(#[case] backend: &str, #[case] expected: &str) {
	// Arrange
	let statement = Query::analyze().table("one").table("two").to_owned();
	// Act
	let (sql, values) = checked(backend, &statement).unwrap();
	// Assert
	assert_eq!(sql, expected);
	assert!(values.0.is_empty());
}

#[rstest]
fn postgres_analyze_columns_and_verbose() {
	// Arrange
	let statement = Query::analyze()
		.verbose()
		.table_columns("quoted\"table", ["quoted\"column"])
		.to_owned();
	// Act
	let (sql, values) = checked("PostgreSQL", &statement).unwrap();
	// Assert
	assert_eq!(
		sql,
		"ANALYZE VERBOSE \"quoted\"\"table\" (\"quoted\"\"column\")"
	);
	assert!(values.0.is_empty());
}

#[rstest]
#[case::postgres("PostgreSQL")]
#[case::sqlite("SQLite")]
fn analyze_entire_database(#[case] backend: &str) {
	// Arrange
	let statement = Query::analyze();
	// Act
	let (sql, values) = checked(backend, &statement).unwrap();
	// Assert
	assert_eq!(sql, "ANALYZE");
	assert!(values.0.is_empty());
}

#[rstest]
#[case::mysql_missing("MySQL", Query::analyze(), "tableless ANALYZE")]
#[case::cockroach_missing("CockroachDB", Query::analyze(), "tableless ANALYZE")]
#[case::sqlite_multiple("SQLite", Query::analyze().table("one").table("two").to_owned(), "multi-table ANALYZE")]
#[case::cockroach_multiple("CockroachDB", Query::analyze().table("one").table("two").to_owned(), "multi-table ANALYZE")]
#[case::mysql_verbose("MySQL", Query::analyze().table("one").verbose().to_owned(), "ANALYZE VERBOSE")]
#[case::sqlite_verbose("SQLite", Query::analyze().table("one").verbose().to_owned(), "ANALYZE VERBOSE")]
#[case::cockroach_verbose("CockroachDB", Query::analyze().table("one").verbose().to_owned(), "ANALYZE VERBOSE")]
#[case::mysql_columns("MySQL", Query::analyze().table_columns("one", ["id"]).to_owned(), "column-level ANALYZE")]
#[case::sqlite_columns("SQLite", Query::analyze().table_columns("one", ["id"]).to_owned(), "column-level ANALYZE")]
#[case::cockroach_columns("CockroachDB", Query::analyze().table_columns("one", ["id"]).to_owned(), "column-level ANALYZE")]
fn analyze_unsupported_configuration_returns_error(
	#[case] backend: &'static str,
	#[case] statement: AnalyzeStatement,
	#[case] feature: &'static str,
) {
	// Act
	let error = checked(backend, &statement).unwrap_err();
	// Assert
	assert_eq!(
		error,
		QueryBuildError::UnsupportedBackendFeature { feature, backend }
	);
}
