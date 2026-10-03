//! Regression coverage for PostgreSQL expression grouping.

use reinhardt_query::{
	CockroachDBQueryBuilder, ColumnDef, ColumnType, Condition, Expr, ExprTrait,
	PostgresQueryBuilder, Query, QueryStatementBuilder, SimpleExpr, Values,
};
use rstest::*;

#[fixture]
fn publication_predicate() -> SimpleExpr {
	Expr::col("publish_until")
		.is_null()
		.or(Expr::col("publish_until").lte(Expr::cust("CURRENT_TIMESTAMP")))
}

#[rstest]
#[case::left_or(Expr::val(true).or(false).and(false), "SELECT ($1 OR $2) AND $3", "SELECT (TRUE OR FALSE) AND FALSE")]
#[case::right_or(Expr::val(false).and(Expr::val(true).or(false)), "SELECT $1 AND ($2 OR $3)", "SELECT FALSE AND (TRUE OR FALSE)")]
#[case::and_before_or(Expr::val(true).or(Expr::val(false).and(false)), "SELECT $1 OR $2 AND $3", "SELECT TRUE OR FALSE AND FALSE")]
#[case::not_or(Expr::val(true).or(false).not(), "SELECT NOT ($1 OR $2)", "SELECT NOT (TRUE OR FALSE)")]
#[case::not_and(Expr::val(true).and(false).not(), "SELECT NOT ($1 AND $2)", "SELECT NOT (TRUE AND FALSE)")]
#[case::not_comparison(Expr::val(true).not().eq(false), "SELECT (NOT $1) = $2", "SELECT (NOT TRUE) = FALSE")]
#[case::nested_comparison(Expr::val(true).eq(false).eq(true), "SELECT ($1 = $2) = $3", "SELECT (TRUE = FALSE) = TRUE")]
#[case::between_operand(Expr::val(1).eq(2).between(false, true), "SELECT ($1 = $2) BETWEEN $3 AND $4", "SELECT (1 = 2) BETWEEN FALSE AND TRUE")]
#[case::in_operand(Expr::val(true).or(false).is_in([true, false]), "SELECT ($1 OR $2) IN ($3, $4)", "SELECT (TRUE OR FALSE) IN (TRUE, FALSE)")]
#[case::postfix_cast(Expr::val(true).eq(false).as_enum("bool"), "SELECT ($1 = $2)::\"bool\"", "SELECT (TRUE = FALSE)::\"bool\"")]
#[case::escaped_like_cast(Expr::col("name").starts_with("prefix").as_enum("bool"), r#"SELECT ("name" LIKE $1 ESCAPE '\')::"bool""#, r#"SELECT ("name" LIKE 'prefix%' ESCAPE '\')::"bool""#)]
#[case::escaped_like_in(Expr::col("name").starts_with("prefix").is_in([true]), r#"SELECT ("name" LIKE $1 ESCAPE '\') IN ($2)"#, r#"SELECT ("name" LIKE 'prefix%' ESCAPE '\') IN (TRUE)"#)]
fn postgres_boolean_preserves_grouping(
	#[case] expression: SimpleExpr,
	#[case] expected_sql: &str,
	#[case] expected_inlined: &str,
) {
	// Arrange
	let query = Query::select().expr(expression).to_owned();

	// Act
	let sql = query.build(PostgresQueryBuilder).0;
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(inlined, expected_inlined);
}

#[rstest]
fn postgres_condition_all_preserves_typed_or_grouping(publication_predicate: SimpleExpr) {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("obligations")
		.and_where(
			Condition::all()
				.add(publication_predicate)
				.add(Expr::col("state").eq("claimed")),
		)
		.to_owned();

	// Act
	let sql = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		r#"SELECT "id" FROM "obligations" WHERE (("publish_until" IS NULL OR "publish_until" <= CURRENT_TIMESTAMP) AND "state" = 'claimed')"#
	);
}

#[rstest]
fn postgres_having_preserves_typed_or_grouping(publication_predicate: SimpleExpr) {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("obligations")
		.and_having(publication_predicate)
		.and_having(Expr::col("state").eq("claimed"))
		.to_owned();

	// Act
	let sql = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		r#"SELECT "id" FROM "obligations" HAVING ("publish_until" IS NULL OR "publish_until" <= CURRENT_TIMESTAMP) AND "state" = 'claimed'"#
	);
}

#[rstest]
fn postgres_update_preserves_typed_or_grouping(publication_predicate: SimpleExpr) {
	// Arrange
	let query = Query::update()
		.table("obligations")
		.value("state", "settled")
		.and_where(publication_predicate)
		.and_where(Expr::col("state").eq("claimed"))
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		r#"UPDATE "obligations" SET "state" = $1 WHERE ("publish_until" IS NULL OR "publish_until" <= CURRENT_TIMESTAMP) AND "state" = $2"#
	);
	assert_eq!(values, Values(vec!["settled".into(), "claimed".into()]));
	assert_eq!(
		inlined,
		r#"UPDATE "obligations" SET "state" = 'settled' WHERE ("publish_until" IS NULL OR "publish_until" <= CURRENT_TIMESTAMP) AND "state" = 'claimed'"#
	);
}

#[rstest]
fn postgres_delete_preserves_typed_or_grouping(publication_predicate: SimpleExpr) {
	// Arrange
	let query = Query::delete()
		.from_table("obligations")
		.and_where(publication_predicate)
		.and_where(Expr::col("state").eq("claimed"))
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		r#"DELETE FROM "obligations" WHERE ("publish_until" IS NULL OR "publish_until" <= CURRENT_TIMESTAMP) AND "state" = $1"#
	);
	assert_eq!(values, Values(vec!["claimed".into()]));
	assert_eq!(
		inlined,
		r#"DELETE FROM "obligations" WHERE ("publish_until" IS NULL OR "publish_until" <= CURRENT_TIMESTAMP) AND "state" = 'claimed'"#
	);
}

#[rstest]
fn postgres_check_preserves_expression_grouping() {
	// Arrange
	let query = Query::create_table()
		.table("probe")
		.col(
			ColumnDef::new("value")
				.column_type(ColumnType::Integer)
				.check(Expr::val(2).add(3).mul(4).eq(20)),
		)
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		r#"CREATE TABLE "probe" ("value" INTEGER CHECK ((2 + 3) * 4 = 20))"#
	);
	assert_eq!(values, Values::new());
}

#[rstest]
fn cockroachdb_preserves_postgres_grouping() {
	// Arrange
	let query = Query::select().expr(Expr::val(2).add(3).mul(4)).to_owned();

	// Act
	let (sql, values) = reinhardt_query::backend::QueryBuilder::build_select(
		&CockroachDBQueryBuilder::new(),
		&query,
	);

	// Assert
	assert_eq!(sql, "SELECT ($1 + $2) * $3");
	assert_eq!(values, Values(vec![2.into(), 3.into(), 4.into()]));
}

#[rstest]
fn postgres_where_preserves_typed_or_grouping(publication_predicate: SimpleExpr) {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("obligations")
		.and_where(publication_predicate)
		.and_where(Expr::col("state").eq("claimed"))
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		r#"SELECT "id" FROM "obligations" WHERE ("publish_until" IS NULL OR "publish_until" <= CURRENT_TIMESTAMP) AND "state" = $1"#
	);
	assert_eq!(values, Values(vec!["claimed".into()]));
	assert_eq!(
		inlined,
		r#"SELECT "id" FROM "obligations" WHERE ("publish_until" IS NULL OR "publish_until" <= CURRENT_TIMESTAMP) AND "state" = 'claimed'"#
	);
}

#[rstest]
#[case::left_sum(Expr::val(2).add(3).mul(4), "($1 + $2) * $3", "(2 + 3) * 4", [2, 3, 4])]
#[case::right_sum(Expr::val(2).mul(Expr::val(3).add(4)), "$1 * ($2 + $3)", "2 * (3 + 4)", [2, 3, 4])]
#[case::right_subtraction(Expr::val(8).sub(Expr::val(3).sub(1)), "$1 - ($2 - $3)", "8 - (3 - 1)", [8, 3, 1])]
#[case::right_division(Expr::val(2).mul(Expr::val(5).div(2)), "$1 * ($2 / $3)", "2 * (5 / 2)", [2, 5, 2])]
#[case::right_multiplication(Expr::val(64).div(Expr::val(8).mul(2)), "$1 / ($2 * $3)", "64 / (8 * 2)", [64, 8, 2])]
#[case::right_modulo(Expr::val(17).modulo(Expr::val(5).modulo(3)), "$1 % ($2 % $3)", "17 % (5 % 3)", [17, 5, 3])]
#[case::left_associative(Expr::val(8).sub(3).sub(1), "$1 - $2 - $3", "8 - 3 - 1", [8, 3, 1])]
#[case::higher_precedence(Expr::val(2).add(Expr::val(3).mul(4)), "$1 + $2 * $3", "2 + 3 * 4", [2, 3, 4])]
fn postgres_arithmetic_preserves_grouping(
	#[case] expression: SimpleExpr,
	#[case] expected_sql: &str,
	#[case] expected_inlined: &str,
	#[case] operands: [i32; 3],
) {
	// Arrange
	let query = Query::select().expr(expression).to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(sql, format!("SELECT {expected_sql}"));
	assert_eq!(
		values,
		Values(operands.into_iter().map(Into::into).collect())
	);
	assert_eq!(inlined, format!("SELECT {expected_inlined}"));
}

#[rstest]
fn postgres_condition_any_keeps_existing_grouping() {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("obligations")
		.and_where(
			Condition::any()
				.add(Expr::col("publish_until").is_null())
				.add(Expr::col("publish_until").lte(Expr::cust("CURRENT_TIMESTAMP"))),
		)
		.and_where(Expr::col("state").eq("claimed"))
		.to_owned();

	// Act
	let sql = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		r#"SELECT "id" FROM "obligations" WHERE ("publish_until" IS NULL OR "publish_until" <= CURRENT_TIMESTAMP) AND "state" = 'claimed'"#
	);
}

#[rstest]
fn postgres_check_preserves_boolean_grouping() {
	// Arrange
	let query = Query::create_table()
		.table("probe")
		.col(
			ColumnDef::new("value")
				.column_type(ColumnType::Integer)
				.check(Expr::val(true).or(false).not()),
		)
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		r#"CREATE TABLE "probe" ("value" INTEGER CHECK (NOT (TRUE OR FALSE)))"#
	);
	assert_eq!(values, Values::new());
}

#[rstest]
fn postgres_negated_single_condition_preserves_typed_or_grouping() {
	// Arrange
	let query = Query::select()
		.expr(1)
		.and_where(Condition::any().add(Expr::val(true).or(false)).not())
		.to_owned();

	// Act
	let sql = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(sql, "SELECT 1 WHERE NOT (TRUE OR FALSE)");
}

#[rstest]
fn postgres_single_condition_preserves_typed_or_grouping(publication_predicate: SimpleExpr) {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("obligations")
		.and_where(Condition::any().add(publication_predicate))
		.and_where(Expr::col("state").eq("claimed"))
		.to_owned();

	// Act
	let sql = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		r#"SELECT "id" FROM "obligations" WHERE ("publish_until" IS NULL OR "publish_until" <= CURRENT_TIMESTAMP) AND "state" = 'claimed'"#
	);
}

#[rstest]
#[case::right_nested(Expr::val(1).bit_or(Expr::val(2).bit_and(4)), "SELECT $1 | ($2 & $3)", "SELECT 1 | (2 & 4)")]
#[case::left_nested(Expr::val(1).bit_or(2).bit_and(4), "SELECT $1 | $2 & $3", "SELECT 1 | 2 & 4")]
fn postgres_bitwise_operators_use_postgres_precedence(
	#[case] expression: SimpleExpr,
	#[case] expected_sql: &str,
	#[case] expected_inlined: &str,
) {
	// Arrange
	let query = Query::select().expr(expression).to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(values, Values(vec![1.into(), 2.into(), 4.into()]));
	assert_eq!(inlined, expected_inlined);
}
