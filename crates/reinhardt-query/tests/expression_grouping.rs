//! Regression coverage for PostgreSQL expression grouping.

use reinhardt_query::{
	CockroachDBQueryBuilder, ColumnDef, ColumnType, Condition, Expr, ExprTrait,
	PostgresQueryBuilder, Query, QueryBuilder, QueryStatementBuilder, SimpleExpr, Values,
};
use rstest::*;

#[fixture]
fn custom_lease_predicate(#[default(false)] with_values: bool) -> SimpleExpr {
	if with_values {
		Expr::cust_with_values("lease_until IS NULL OR control = ?", ["ACTIVE"]).into()
	} else {
		Expr::cust("lease_until IS NULL OR control = 'ACTIVE'").into()
	}
}

#[rstest]
fn postgres_custom_or_retains_neighboring_filters(#[values(false, true)] cockroachdb: bool) {
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
	let (sql, values) = if cockroachdb {
		CockroachDBQueryBuilder::new().build_select(&query)
	} else {
		query.build(PostgresQueryBuilder)
	};
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		r#"SELECT "id" FROM "runs" WHERE "control" <> $1 AND (lease_until IS NULL OR lease_until <= CURRENT_TIMESTAMP) AND "id" = $2"#
	);
	assert_eq!(values, Values(vec!["PAUSED".into(), 2_i64.into()]));
	assert_eq!(
		inlined,
		r#"SELECT "id" FROM "runs" WHERE "control" <> 'PAUSED' AND (lease_until IS NULL OR lease_until <= CURRENT_TIMESTAMP) AND "id" = 2"#
	);
}

#[rstest]
#[case::raw(false)]
#[case::with_values(true)]
fn postgres_custom_select_preserves_grouping_and_bind_order(
	#[case] with_values: bool,
	#[with(with_values)] custom_lease_predicate: SimpleExpr,
) {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("runs")
		.and_where(Expr::col("control").ne("PAUSED"))
		.and_where(custom_lease_predicate)
		.and_where(Expr::col("id").eq(2_i64))
		.to_owned();
	let (expected_sql, expected_values) = if with_values {
		(
			r#"SELECT "id" FROM "runs" WHERE "control" <> $1 AND (lease_until IS NULL OR control = $2) AND "id" = $3"#,
			Values(vec!["PAUSED".into(), "ACTIVE".into(), 2_i64.into()]),
		)
	} else {
		(
			r#"SELECT "id" FROM "runs" WHERE "control" <> $1 AND (lease_until IS NULL OR control = 'ACTIVE') AND "id" = $2"#,
			Values(vec!["PAUSED".into(), 2_i64.into()]),
		)
	};

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(values, expected_values);
	assert_eq!(
		inlined,
		r#"SELECT "id" FROM "runs" WHERE "control" <> 'PAUSED' AND (lease_until IS NULL OR control = 'ACTIVE') AND "id" = 2"#
	);
}

#[rstest]
#[case::raw(false)]
#[case::with_values(true)]
fn postgres_custom_update_preserves_grouping_and_bind_order(
	#[case] with_values: bool,
	#[with(with_values)] custom_lease_predicate: SimpleExpr,
) {
	// Arrange
	let query = Query::update()
		.table("runs")
		.value("control", "CLAIMED")
		.and_where(Expr::col("control").ne("PAUSED"))
		.and_where(custom_lease_predicate)
		.and_where(Expr::col("id").eq(2_i64))
		.to_owned();
	let (expected_sql, expected_values) = if with_values {
		(
			r#"UPDATE "runs" SET "control" = $1 WHERE "control" <> $2 AND (lease_until IS NULL OR control = $3) AND "id" = $4"#,
			Values(vec![
				"CLAIMED".into(),
				"PAUSED".into(),
				"ACTIVE".into(),
				2_i64.into(),
			]),
		)
	} else {
		(
			r#"UPDATE "runs" SET "control" = $1 WHERE "control" <> $2 AND (lease_until IS NULL OR control = 'ACTIVE') AND "id" = $3"#,
			Values(vec!["CLAIMED".into(), "PAUSED".into(), 2_i64.into()]),
		)
	};

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(values, expected_values);
	assert_eq!(
		inlined,
		r#"UPDATE "runs" SET "control" = 'CLAIMED' WHERE "control" <> 'PAUSED' AND (lease_until IS NULL OR control = 'ACTIVE') AND "id" = 2"#
	);
}

#[rstest]
#[case::raw(false)]
#[case::with_values(true)]
fn postgres_custom_delete_preserves_grouping_and_bind_order(
	#[case] with_values: bool,
	#[with(with_values)] custom_lease_predicate: SimpleExpr,
) {
	// Arrange
	let query = Query::delete()
		.from_table("runs")
		.and_where(Expr::col("control").ne("PAUSED"))
		.and_where(custom_lease_predicate)
		.and_where(Expr::col("id").eq(2_i64))
		.to_owned();
	let (expected_sql, expected_values) = if with_values {
		(
			r#"DELETE FROM "runs" WHERE "control" <> $1 AND (lease_until IS NULL OR control = $2) AND "id" = $3"#,
			Values(vec!["PAUSED".into(), "ACTIVE".into(), 2_i64.into()]),
		)
	} else {
		(
			r#"DELETE FROM "runs" WHERE "control" <> $1 AND (lease_until IS NULL OR control = 'ACTIVE') AND "id" = $2"#,
			Values(vec!["PAUSED".into(), 2_i64.into()]),
		)
	};

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(values, expected_values);
	assert_eq!(
		inlined,
		r#"DELETE FROM "runs" WHERE "control" <> 'PAUSED' AND (lease_until IS NULL OR control = 'ACTIVE') AND "id" = 2"#
	);
}

#[rstest]
#[case::left_and(Expr::cust("FALSE OR TRUE").and(false), "SELECT (FALSE OR TRUE) AND $1", "SELECT (FALSE OR TRUE) AND FALSE")]
#[case::right_and(Expr::val(false).and(Expr::cust("FALSE OR TRUE")), "SELECT $1 AND (FALSE OR TRUE)", "SELECT FALSE AND (FALSE OR TRUE)")]
#[case::left_or(Expr::cust("TRUE AND FALSE").or(true), "SELECT (TRUE AND FALSE) OR $1", "SELECT (TRUE AND FALSE) OR TRUE")]
#[case::right_or(Expr::val(true).or(Expr::cust("TRUE AND FALSE")), "SELECT $1 OR (TRUE AND FALSE)", "SELECT TRUE OR (TRUE AND FALSE)")]
#[case::template(Expr::cust_with_values("? OR ?", [false, true]).and(false), "SELECT ($1 OR $2) AND $3", "SELECT (FALSE OR TRUE) AND FALSE")]
#[case::not(Expr::cust("FALSE OR TRUE").not(), "SELECT NOT (FALSE OR TRUE)", "SELECT NOT (FALSE OR TRUE)")]
#[case::not_template(Expr::cust_with_values("? OR ?", [false, true]).not(), "SELECT NOT ($1 OR $2)", "SELECT NOT (FALSE OR TRUE)")]
fn postgres_custom_boolean_operands_retain_grouping(
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
#[case::left_and(Expr::cust("TRUE -- reason").and(false), "SELECT (TRUE -- reason\n) AND $1", "SELECT (TRUE -- reason\n) AND FALSE")]
#[case::right_and(Expr::val(false).and(Expr::cust("TRUE -- reason")), "SELECT $1 AND (TRUE -- reason\n)", "SELECT FALSE AND (TRUE -- reason\n)")]
#[case::left_or(Expr::cust("TRUE -- reason").or(false), "SELECT (TRUE -- reason\n) OR $1", "SELECT (TRUE -- reason\n) OR FALSE")]
#[case::right_or(Expr::val(false).or(Expr::cust("TRUE -- reason")), "SELECT $1 OR (TRUE -- reason\n)", "SELECT FALSE OR (TRUE -- reason\n)")]
#[case::template(Expr::cust_with_values("? -- reason", [true]).and(false), "SELECT ($1 -- reason\n) AND $2", "SELECT (TRUE -- reason\n) AND FALSE")]
#[case::not(Expr::cust("TRUE -- reason").not(), "SELECT NOT (TRUE -- reason\n)", "SELECT NOT (TRUE -- reason\n)")]
#[case::not_template(Expr::cust_with_values("? -- reason", [true]).not(), "SELECT NOT ($1 -- reason\n)", "SELECT NOT (TRUE -- reason\n)")]
#[case::nested_template(SimpleExpr::CustomWithExpr("?".into(), vec![Expr::cust("TRUE -- reason").into()]).not(), "SELECT NOT (TRUE -- reason\n)", "SELECT NOT (TRUE -- reason\n)")]
#[case::quoted_dashes(Expr::cust("'--' = '--'").not(), "SELECT NOT ('--' = '--'\n)", "SELECT NOT ('--' = '--'\n)")]
fn postgres_custom_line_comments_keep_closing_parentheses(
	#[case] expression: SimpleExpr,
	#[case] expected_sql: &str,
	#[case] expected_inlined: &str,
	#[values(false, true)] cockroachdb: bool,
) {
	// Arrange
	let query = Query::select().expr(expression).to_owned();

	// Act
	let sql = if cockroachdb {
		CockroachDBQueryBuilder::new().build_select(&query).0
	} else {
		query.build(PostgresQueryBuilder).0
	};
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(inlined, expected_inlined);
}

#[rstest]
#[case::raw(false)]
#[case::template(true)]
fn postgres_standalone_where_preserves_custom_line_comment(#[case] with_values: bool) {
	// Arrange
	let predicate: SimpleExpr = if with_values {
		Expr::cust_with_values("? -- reason", [true]).into()
	} else {
		Expr::cust("TRUE -- reason").into()
	};
	let query = Query::select()
		.column("id")
		.from("runs")
		.and_where(predicate)
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	let (expected_sql, expected_values) = if with_values {
		(
			"SELECT \"id\" FROM \"runs\" WHERE $1 -- reason",
			Values(vec![true.into()]),
		)
	} else {
		(
			"SELECT \"id\" FROM \"runs\" WHERE TRUE -- reason",
			Values::default(),
		)
	};
	assert_eq!(sql, expected_sql);
	assert_eq!(values, expected_values);
	assert_eq!(inlined, "SELECT \"id\" FROM \"runs\" WHERE TRUE -- reason");
}

#[rstest]
fn postgres_current_of_preserves_standalone_where(
	#[values(false, true)] delete: bool,
	#[values(false, true)] template: bool,
	#[values(false, true)] condition_group: bool,
	#[values(false, true)] cockroachdb: bool,
) {
	// Arrange
	let predicate = if template {
		SimpleExpr::CustomWithExpr(
			"CURRENT OF ?".into(),
			vec![Expr::cust("scoped_cursor").into()],
		)
	} else {
		Expr::cust("CURRENT OF scoped_cursor").into()
	};
	// Act
	let (sql, values, inlined) = if delete {
		let mut query = Query::delete().from_table("runs").to_owned();
		if condition_group {
			query.and_where(Condition::all().add(Condition::any().add(predicate)));
		} else {
			query.and_where(predicate);
		}
		let (sql, values) = if cockroachdb {
			CockroachDBQueryBuilder::new().build_delete(&query)
		} else {
			query.build(PostgresQueryBuilder)
		};
		(sql, values, query.to_string(PostgresQueryBuilder))
	} else {
		let mut query = Query::update()
			.table("runs")
			.value("control", "CLAIMED")
			.to_owned();
		if condition_group {
			query.and_where(Condition::all().add(Condition::any().add(predicate)));
		} else {
			query.and_where(predicate);
		}
		let (sql, values) = if cockroachdb {
			CockroachDBQueryBuilder::new().build_update(&query)
		} else {
			query.build(PostgresQueryBuilder)
		};
		(sql, values, query.to_string(PostgresQueryBuilder))
	};

	// Assert
	if delete {
		assert_eq!(sql, "DELETE FROM \"runs\" WHERE CURRENT OF scoped_cursor");
		assert_eq!(inlined, sql);
		assert_eq!(values, Values::default());
	} else {
		assert_eq!(
			sql,
			"UPDATE \"runs\" SET \"control\" = $1 WHERE CURRENT OF scoped_cursor"
		);
		assert_eq!(
			inlined,
			"UPDATE \"runs\" SET \"control\" = 'CLAIMED' WHERE CURRENT OF scoped_cursor"
		);
		assert_eq!(values, Values(vec!["CLAIMED".into()]));
	}
}

#[rstest]
fn postgres_single_custom_condition_keeps_outer_composition_grouping() {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("runs")
		.and_where(Condition::all().add(Condition::any().add(Expr::cust("FALSE OR TRUE"))))
		.and_where(Expr::val(false))
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		"SELECT \"id\" FROM \"runs\" WHERE (FALSE OR TRUE) AND $1"
	);
	assert_eq!(values, Values(vec![false.into()]));
}

#[rstest]
fn postgres_nested_conditions_preserve_custom_predicates() {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("runs")
		.and_where(
			Condition::all()
				.add(
					Condition::any()
						.add(Expr::cust("lease_until IS NULL OR control = 'ACTIVE'"))
						.add(Expr::col("control").eq("RESUMED")),
				)
				.add(Expr::col("id").eq(2_i64)),
		)
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		r#"SELECT "id" FROM "runs" WHERE (((lease_until IS NULL OR control = 'ACTIVE') OR "control" = $1) AND "id" = $2)"#
	);
	assert_eq!(values, Values(vec!["RESUMED".into(), 2_i64.into()]));
	assert_eq!(
		inlined,
		r#"SELECT "id" FROM "runs" WHERE (((lease_until IS NULL OR control = 'ACTIVE') OR "control" = 'RESUMED') AND "id" = 2)"#
	);
}

#[rstest]
fn postgres_custom_scalars_keep_their_rendering() {
	// Arrange
	let query = Query::select()
		.expr(Expr::cust("CURRENT_TIMESTAMP"))
		.expr(Expr::col("lease_until").lte(Expr::cust("CURRENT_TIMESTAMP")))
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		r#"SELECT CURRENT_TIMESTAMP, "lease_until" <= CURRENT_TIMESTAMP"#
	);
	assert_eq!(values, Values::default());
	assert_eq!(inlined, sql);
}

#[rstest]
#[case::raw(false)]
#[case::with_values(true)]
fn postgres_custom_having_preserves_grouping(
	#[case] with_values: bool,
	#[with(with_values)] custom_lease_predicate: SimpleExpr,
) {
	// Arrange
	let query = Query::select()
		.column("id")
		.from("runs")
		.and_having(custom_lease_predicate)
		.and_having(Expr::col("id").eq(2_i64))
		.to_owned();
	let (expected_sql, expected_values) = if with_values {
		(
			r#"SELECT "id" FROM "runs" HAVING (lease_until IS NULL OR control = $1) AND "id" = $2"#,
			Values(vec!["ACTIVE".into(), 2_i64.into()]),
		)
	} else {
		(
			r#"SELECT "id" FROM "runs" HAVING (lease_until IS NULL OR control = 'ACTIVE') AND "id" = $1"#,
			Values(vec![2_i64.into()]),
		)
	};

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);
	let inlined = query.to_string(PostgresQueryBuilder);

	// Assert
	assert_eq!(sql, expected_sql);
	assert_eq!(values, expected_values);
	assert_eq!(
		inlined,
		r#"SELECT "id" FROM "runs" HAVING (lease_until IS NULL OR control = 'ACTIVE') AND "id" = 2"#
	);
}

#[rstest]
#[case::and(Expr::cust("FALSE OR TRUE").and(false), "(FALSE OR TRUE) AND FALSE")]
#[case::not(Expr::cust("FALSE OR TRUE").not(), "NOT (FALSE OR TRUE)")]
#[case::not_template(Expr::cust_with_values("? OR ?", [false, true]).not(), "NOT (FALSE OR TRUE)")]
fn postgres_custom_check_preserves_unquoted_grouping(
	#[case] expression: SimpleExpr,
	#[case] expected_predicate: &str,
) {
	// Arrange
	let query = Query::create_table()
		.table("probe")
		.col(
			ColumnDef::new("value")
				.column_type(ColumnType::Boolean)
				.check(expression),
		)
		.to_owned();

	// Act
	let (sql, values) = query.build(PostgresQueryBuilder);

	// Assert
	assert_eq!(
		sql,
		format!(r#"CREATE TABLE "probe" ("value" BOOLEAN CHECK ({expected_predicate}))"#)
	);
	assert_eq!(values, Values::default());
}

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
