//! Structural cardinality limits preserve parameter order and row-lock placement.

use reinhardt_query::prelude::*;
use rstest::rstest;

#[derive(Clone, Copy)]
enum Backend {
	Postgres,
	MySql,
	Sqlite,
	Cockroach,
}

fn build(backend: Backend, statement: &SelectStatement) -> (String, Values) {
	match backend {
		Backend::Postgres => PostgresQueryBuilder
			.build_select_checked(statement)
			.unwrap(),
		Backend::MySql => MySqlQueryBuilder.build_select_checked(statement).unwrap(),
		Backend::Sqlite => SqliteQueryBuilder.build_select_checked(statement).unwrap(),
		Backend::Cockroach => CockroachDBQueryBuilder::new()
			.build_select_checked(statement)
			.unwrap(),
	}
}

#[rstest]
#[case(
	Backend::Postgres,
	"SELECT \"id\" FROM \"items\" WHERE \"id\" = $1 AND \"absent\" = NULL LIMIT 2 OFFSET $2 FOR UPDATE"
)]
#[case(
	Backend::MySql,
	"SELECT `id` FROM `items` WHERE `id` = ? AND `absent` = NULL LIMIT 2 OFFSET ? FOR UPDATE"
)]
#[case(
	Backend::Sqlite,
	"SELECT \"id\" FROM \"items\" WHERE \"id\" = ? AND \"absent\" = NULL LIMIT 2 OFFSET ?"
)]
#[case(
	Backend::Cockroach,
	"SELECT \"id\" FROM \"items\" WHERE \"id\" = $1 AND \"absent\" = NULL LIMIT 2 OFFSET $2 FOR UPDATE"
)]
fn literal_limit_keeps_exact_values_and_backend_lock_order(
	#[case] backend: Backend,
	#[case] expected: &str,
) {
	// Arrange
	let mut statement = Query::select();
	statement
		.column("id")
		.from("items")
		.and_where(Expr::col("id").eq(7_i64))
		.and_where(Expr::col("absent").eq(Value::String(None)))
		.limit_literal(2)
		.offset(3_i64);
	if !matches!(backend, Backend::Sqlite) {
		statement.lock_exclusive();
	}
	// Act
	let built = build(backend, &statement);
	// Assert: NULL and the literal limit add no slots; offset follows the predicate.
	assert_eq!(built.0, expected);
	assert_eq!(
		built.1.0,
		vec![Value::BigInt(Some(7)), Value::BigInt(Some(3))]
	);
}

#[rstest]
#[case(Backend::Postgres)]
#[case(Backend::MySql)]
#[case(Backend::Sqlite)]
#[case(Backend::Cockroach)]
fn limit_modes_replace_each_other_and_take_preserves_ownership(#[case] backend: Backend) {
	// Arrange
	let mut statement = Query::select();
	statement.column("id").from("items").limit_literal(2);
	// Act / Assert: changing back to bound mode creates exactly one parameter.
	statement.limit(5_i64);
	let bound = build(backend, &statement);
	assert_eq!(bound.1.0, vec![Value::BigInt(Some(5))]);
	assert!(!bound.0.ends_with("LIMIT 2"));
	statement.limit_literal(2);
	let moved = statement.take();
	let literal = build(backend, &moved);
	assert!(literal.0.ends_with("LIMIT 2"));
	assert!(literal.1.0.is_empty());
	statement.column("name").from("other");
	let reset = build(backend, &statement);
	assert!(!reset.0.contains("LIMIT"));
	assert!(reset.1.0.is_empty());
}
