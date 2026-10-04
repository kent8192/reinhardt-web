//! Structural INSERT rows preserve binds, source replacement and checked traversal.
use reinhardt_query::QueryBuildError;
use reinhardt_query::prelude::*;
use rstest::rstest;

#[rstest]
#[case::postgres(PostgresQueryBuilder, "\"", ["$1", "$2", "$3", "$4", "$5"])]
#[case::mysql(MySqlQueryBuilder, "`", ["?", "?", "?", "?", "?"])]
#[case::sqlite(SqliteQueryBuilder, "\"", ["?", "?", "?", "?", "?"])]
fn mixed_rows_keep_call_order_and_omit_null_binds(
	#[case] builder: impl QueryBuilderTrait,
	#[case] quote: &str,
	#[case] placeholders: [&str; 5],
) {
	// Arrange
	let mut statement = Query::insert();
	statement
		.into_table("events")
		.columns(["name", "number"])
		.values_panic(vec!["original' ? $9".into(), Value::Int(None)]);
	statement
		.values_expr(vec![
			SimpleExpr::FunctionCall(
				Alias::new("LOWER").into_iden(),
				vec![Expr::val("UPPER").into()],
			),
			Expr::val(7_i32).into(),
		])
		.unwrap();
	statement.values(vec!["last".into(), 9_i32.into()]).unwrap();
	// Act
	let (sql, values) = statement.build(builder);
	// Assert
	let [a, b, c, d, e] = placeholders;
	assert_eq!(
		sql,
		format!(
			"INSERT INTO {quote}events{quote} ({quote}name{quote}, {quote}number{quote}) VALUES ({a}, NULL), (LOWER({b}), {c}), ({d}, {e})"
		)
	);
	assert_eq!(
		values,
		Values(vec![
			"original' ? $9".into(),
			"UPPER".into(),
			7_i32.into(),
			"last".into(),
			9_i32.into()
		])
	);
	assert!(statement.get_values().is_none());
}

#[rstest]
fn rejected_expression_row_does_not_change_the_previous_source() {
	// Arrange
	let mut statement = Query::insert();
	statement
		.into_table("events")
		.columns(["name", "number"])
		.values_panic([1_i32, 2]);
	let previous = statement.build(PostgresQueryBuilder);
	// Act
	let error = statement
		.values_expr(vec![Expr::val(3_i32).into()])
		.unwrap_err();
	// Assert
	assert_eq!(
		error,
		"Number of values (1) doesn't match number of columns (2)"
	);
	assert_eq!(statement.build(PostgresQueryBuilder), previous);
	assert!(statement.get_values().is_some());
}

#[rstest]
fn source_changes_and_take_discard_obsolete_expression_rows() {
	// Arrange
	let mut statement = Query::insert();
	statement.into_table("events").column("number");
	statement
		.values_expr(vec![Expr::val(1_i32).into()])
		.unwrap();
	let taken = statement.take();
	// Act / Assert: take transfers the expression row and leaves a reusable empty builder.
	assert_eq!(
		taken.build(PostgresQueryBuilder),
		(
			"INSERT INTO \"events\" (\"number\") VALUES ($1)".into(),
			Values(vec![1_i32.into()])
		)
	);
	assert_eq!(statement.get_values(), Some(&Vec::new()));
	statement
		.into_table("events")
		.column("number")
		.values_panic([2_i32]);
	assert_eq!(
		statement.build(PostgresQueryBuilder).1,
		Values(vec![2_i32.into()])
	);
	statement
		.values_expr(vec![Expr::val(3_i32).into()])
		.unwrap();
	statement.from_subquery(Query::select().expr(Expr::val(4_i32)).take());
	assert_eq!(
		statement.build(PostgresQueryBuilder),
		(
			"INSERT INTO \"events\" (\"number\") SELECT $1".into(),
			Values(vec![4_i32.into()])
		)
	);
	statement
		.values_expr(vec![Expr::val(5_i32).into()])
		.unwrap();
	assert_eq!(
		statement.build(PostgresQueryBuilder).1,
		Values(vec![5_i32.into()])
	);
	statement.default_values();
	assert_eq!(
		statement.build(PostgresQueryBuilder),
		(
			"INSERT INTO \"events\" DEFAULT VALUES".into(),
			Values::new()
		)
	);
}

#[rstest]
fn checked_builders_validate_vendor_expressions_in_insert_rows() {
	// Arrange
	let mut statement = Query::insert();
	statement.into_table("events").column("epoch");
	statement
		.values_expr(vec![Func::pg_extract_epoch(
			Expr::current_timestamp().into(),
		)])
		.unwrap();
	// Act / Assert
	assert!(
		PostgresQueryBuilder
			.build_insert_checked(&statement)
			.is_ok()
	);
	for result in [
		MySqlQueryBuilder.build_insert_checked(&statement),
		SqliteQueryBuilder.build_insert_checked(&statement),
		CockroachDBQueryBuilder::new().build_insert_checked(&statement),
	] {
		assert!(matches!(
			result,
			Err(QueryBuildError::UnsupportedBackendFeature { .. })
		));
	}
}

#[rstest]
fn checked_builders_traverse_locked_subqueries_in_insert_rows() {
	// Arrange
	let select = Query::select()
		.column("number")
		.from("events")
		.lock(LockType::Update)
		.take();
	let mut statement = Query::insert();
	statement.into_table("copied").column("number");
	statement
		.values_expr(vec![Expr::subquery(select).into()])
		.unwrap();
	// Act / Assert
	assert!(
		PostgresQueryBuilder
			.build_insert_checked(&statement)
			.is_ok()
	);
	assert_eq!(
		SqliteQueryBuilder.build_insert_checked(&statement),
		Err(QueryBuildError::UnsupportedBackendFeature {
			feature: "row locking",
			backend: "SQLite"
		})
	);
}

#[cfg(feature = "pgvector")]
#[rstest]
fn insert_feature_detection_traverses_expression_rows() {
	use reinhardt_query::error::{PgvectorFeature, insert_pgvector_feature};
	// Arrange
	let mut statement = Query::insert();
	statement.into_table("documents").column("embedding");
	statement
		.values_expr(vec![SimpleExpr::Value(Value::Vector(Some(Box::new(
			vec![1.0, 2.0],
		))))])
		.unwrap();
	// Act / Assert
	assert_eq!(
		insert_pgvector_feature(&statement),
		Some(PgvectorFeature::VectorValue)
	);
	assert_eq!(
		SqliteQueryBuilder.build_insert_checked(&statement),
		Err(QueryBuildError::UnsupportedBackendFeature {
			feature: "pgvector values",
			backend: "SQLite"
		})
	);
}
