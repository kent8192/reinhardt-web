//! Parameter adaptation happens on the AST before final bind ordering.
use std::convert::Infallible;

use reinhardt_query::types::{JoinType, WindowStatement};
use reinhardt_query::{
	CockroachDBQueryBuilder, Cond, Expr, ExprTrait, MySqlQueryBuilder, OnConflict, Order,
	PostgresQueryBuilder, Query, SelectStatement, SimpleExpr, SqliteQueryBuilder, Value,
};
use rstest::rstest;

fn text(value: &Value) -> Result<SimpleExpr, Infallible> {
	Ok(Expr::val(value.clone()).cast_as_text())
}

fn checked(backend: &str, statement: &SelectStatement) -> (String, reinhardt_query::Values) {
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
#[case::postgres("postgres", "TEXT", "\"", ["$1", "$2", "$3", "$4", "$5", "$6", "$7"])]
#[case::mysql("mysql", "CHAR", "`", ["?", "?", "?", "?", "?", "?", "?"])]
#[case::sqlite("sqlite", "TEXT", "\"", ["?", "?", "?", "?", "?", "?", "?"])]
#[case::cockroach("cockroach", "TEXT", "\"", ["$1", "$2", "$3", "$4", "$5", "$6", "$7"])]
fn nested_selects_keep_exact_final_values(
	#[case] backend: &str,
	#[case] cast: &str,
	#[case] quote: &str,
	#[case] slots: [&str; 7],
) {
	// Arrange
	let source = Query::select()
		.expr(Expr::val("projection' $99"))
		.expr(Expr::subquery(
			Query::select().expr(Expr::val("scalar")).to_owned(),
		))
		.from_subquery(Query::select().expr(Expr::val("source")).to_owned(), "src")
		.and_where(Expr::col("name").eq("predicate"))
		.order_by_expr(Expr::val("order"), Order::Asc)
		.limit(4)
		.offset(2)
		.to_owned();
	let original = checked(backend, &source);
	// Act
	let adapted = source.try_map_value_expressions(text).unwrap();
	let (sql, values) = checked(backend, &adapted);
	// Assert: structural values are mapped; numeric pagination retains its grammar.
	assert_eq!(
		sql,
		format!(
			"SELECT CAST({} AS {cast}), (SELECT CAST({} AS {cast})) FROM (SELECT CAST({} AS {cast})) AS {quote}src{quote} WHERE {quote}name{quote} = CAST({} AS {cast}) ORDER BY CAST({} AS {cast}) ASC LIMIT {} OFFSET {}",
			slots[0], slots[1], slots[2], slots[3], slots[4], slots[5], slots[6]
		)
	);
	assert_eq!(
		values.0,
		vec![
			"projection' $99".into(),
			"scalar".into(),
			"source".into(),
			"predicate".into(),
			"order".into(),
			4_i32.into(),
			2_i32.into()
		]
	);
	assert_eq!(checked(backend, &source), original);
}

#[rstest]
fn cte_join_condition_union_and_window_values_are_visited() {
	// Arrange
	let window = WindowStatement {
		partition_by: vec![Expr::val("partition").into_simple_expr()],
		order_by: vec![],
		frame: None,
	};
	let source = Query::select()
		.with_cte("c", Query::select().expr(Expr::val("cte")).to_owned())
		.expr(SimpleExpr::Window {
			func: Box::new(Expr::val("function").into_simple_expr()),
			window: window.clone(),
		})
		.from("c")
		.join(
			JoinType::InnerJoin,
			"other",
			Cond::any().add(Expr::col("x").eq("join")),
		)
		.cond_where(Cond::all().add(Cond::any().add(Expr::col("y").eq("nested"))))
		.group_by_expr(Expr::val("group").into_simple_expr())
		.cond_having(Cond::all().add(Expr::col("z").eq("having")))
		.window_as("w", window)
		.union_all(Query::select().expr(Expr::val("union")).to_owned())
		.to_owned();
	// Act
	let mapped = source.try_map_value_expressions(text).unwrap();
	let (sql, values) = checked("postgres", &mapped);
	// Assert
	assert_eq!(
		sql,
		"WITH \"c\" AS (SELECT CAST($1 AS TEXT)) SELECT CAST($2 AS TEXT) OVER ( PARTITION BY CAST($3 AS TEXT) ) FROM \"c\" INNER JOIN \"other\" ON \"x\" = CAST($4 AS TEXT) WHERE \"y\" = CAST($5 AS TEXT) GROUP BY CAST($6 AS TEXT) HAVING \"z\" = CAST($7 AS TEXT) WINDOW \"w\" AS ( PARTITION BY CAST($8 AS TEXT) ) UNION ALL SELECT CAST($9 AS TEXT)"
	);
	assert_eq!(
		values.0,
		[
			"cte",
			"function",
			"partition",
			"join",
			"nested",
			"group",
			"having",
			"partition",
			"union"
		]
		.map(Value::from)
	);
}

#[rstest]
#[case::values(false)]
#[case::source(true)]
fn insert_conflict_and_returning_keep_argument_order(#[case] select_source: bool) {
	// Arrange
	let mut source = Query::insert();
	source.into_table("records").columns(["name"]);
	if select_source {
		source.from_subquery(Query::select().expr(Expr::val("row' $9")).to_owned());
	} else {
		source.values_panic(["row' $9"]);
	}
	source
		.on_conflict(
			OnConflict::column("name")
				.update_columns(["name"])
				.action_and_where(Expr::col("name").eq("conflict")),
		)
		.returning_exprs([Expr::val("return").into_simple_expr()]);
	let original = PostgresQueryBuilder.build_insert_checked(&source).unwrap();
	// Act
	let adapted = source.try_map_value_expressions(text).unwrap();
	let (sql, values) = PostgresQueryBuilder.build_insert_checked(&adapted).unwrap();
	// Assert
	let rows = if select_source {
		"SELECT CAST($1 AS TEXT)"
	} else {
		"VALUES (CAST($1 AS TEXT))"
	};
	assert_eq!(
		sql,
		format!(
			"INSERT INTO \"records\" (\"name\") {rows} ON CONFLICT (\"name\") DO UPDATE SET \"name\" = EXCLUDED.\"name\" WHERE \"name\" = CAST($2 AS TEXT) RETURNING CAST($3 AS TEXT)"
		)
	);
	assert_eq!(
		values.0,
		vec!["row' $9".into(), "conflict".into(), "return".into()]
	);
	assert_eq!(
		PostgresQueryBuilder.build_insert_checked(&source).unwrap(),
		original
	);
}

#[rstest]
fn update_and_delete_adapt_conditions_subqueries_and_returning() {
	// Arrange
	let subquery = Query::select().expr(Expr::val("key")).to_owned();
	let update = Query::update()
		.table("records")
		.value("name", "set")
		.and_where(Expr::col("id").in_subquery(subquery.clone()))
		.returning_exprs([Expr::val("return").into_simple_expr()])
		.to_owned();
	let delete = Query::delete()
		.from_table("records")
		.and_where(Expr::col("id").in_subquery(subquery))
		.returning_exprs([Expr::val("return").into_simple_expr()])
		.to_owned();
	// Act
	let update = update.try_map_value_expressions(text).unwrap();
	let delete = delete.try_map_value_expressions(text).unwrap();
	let (update_sql, update_values) = PostgresQueryBuilder.build_update_checked(&update).unwrap();
	let (delete_sql, delete_values) = PostgresQueryBuilder.build_delete_checked(&delete).unwrap();
	// Assert
	assert_eq!(
		update_sql,
		"UPDATE \"records\" SET \"name\" = CAST($1 AS TEXT) WHERE \"id\" IN (SELECT CAST($2 AS TEXT)) RETURNING CAST($3 AS TEXT)"
	);
	assert_eq!(
		delete_sql,
		"DELETE FROM \"records\" WHERE \"id\" IN (SELECT CAST($1 AS TEXT)) RETURNING CAST($2 AS TEXT)"
	);
	assert_eq!(
		update_values.0,
		vec!["set".into(), "key".into(), "return".into()]
	);
	assert_eq!(delete_values.0, vec!["key".into(), "return".into()]);
}

#[rstest]
fn callback_error_preserves_original_and_replacements_are_not_revisited() {
	// Arrange
	let statement = Query::select()
		.expr(Expr::val("ok"))
		.expr(Expr::val("fail"))
		.to_owned();
	let original = checked("postgres", &statement);
	let mut calls = 0;
	// Act
	let error = statement
		.try_map_value_expressions(|value| {
			calls += 1;
			if value == &Value::from("fail") {
				Err("codec rejected value")
			} else {
				Ok(Expr::val(value.clone()).cast_as_text())
			}
		})
		.unwrap_err();
	// Assert
	assert_eq!(error, "codec rejected value");
	assert_eq!(calls, 2);
	assert_eq!(checked("postgres", &statement), original);
}

#[rstest]
fn case_and_custom_arguments_keep_keywords_and_template_text() {
	// Arrange
	let case = reinhardt_query::expr::CaseStatement::new()
		.when(Expr::col("flag").eq(true), Expr::val("case"))
		.else_result(Expr::val(Value::String(None)));
	let statement = Query::select()
		.expr(SimpleExpr::Case(Box::new(case)))
		.expr(Expr::cust_with_values(
			"COALESCE(?, NULL)",
			["fallback' $9"],
		))
		.to_owned();
	// Act: preserve non-string values, particularly NULL and boolean conditions.
	let adapted = statement
		.try_map_value_expressions(|value| {
			Ok::<_, Infallible>(match value {
				Value::String(Some(_)) => Expr::val(value.clone()).cast_as_text(),
				_ => Expr::val(value.clone()).into_simple_expr(),
			})
		})
		.unwrap();
	let (sql, values) = checked("postgres", &adapted);
	// Assert
	assert_eq!(
		sql,
		"SELECT CASE WHEN \"flag\" = $1 THEN CAST($2 AS TEXT) ELSE NULL END, COALESCE(CAST($3 AS TEXT), NULL)"
	);
	assert_eq!(
		values.0,
		vec![true.into(), "case".into(), "fallback' $9".into()]
	);
}
