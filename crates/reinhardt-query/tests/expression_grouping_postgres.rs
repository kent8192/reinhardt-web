//! PostgreSQL execution coverage for typed expression grouping.

use std::sync::Arc;

use reinhardt_query::{
	ColumnDef, ColumnType, Condition, Expr, ExprTrait, Order, PostgresQueryBuilder, Query,
	QueryStatementBuilder, Value,
};
use rstest::*;
use sqlx::Arguments;

mod common;
use common::{PgContainer, postgres_container};

#[fixture]
async fn obligations(
	#[future] postgres_container: (PgContainer, Arc<sqlx::PgPool>, u16, String),
) -> (PgContainer, Arc<sqlx::PgPool>) {
	let (container, pool, _, _) = postgres_container.await;
	let schema = Query::create_table()
		.table("obligations")
		.col(ColumnDef::new("id").column_type(ColumnType::Integer))
		.col(ColumnDef::new("state").column_type(ColumnType::Text))
		.col(ColumnDef::new("publish_until").column_type(ColumnType::TimestampWithTimeZone))
		.to_string(PostgresQueryBuilder);
	sqlx::query(&schema).execute(pool.as_ref()).await.unwrap();

	for (id, state) in [(1, "pending"), (2, "claimed"), (3, "settled")] {
		let insert = Query::insert()
			.into_table("obligations")
			.columns(["id", "state", "publish_until"])
			.values(vec![id.into(), state.into(), Value::String(None)])
			.unwrap()
			.to_string(PostgresQueryBuilder);
		sqlx::query(&insert).execute(pool.as_ref()).await.unwrap();
	}

	(container, pool)
}

#[rstest]
#[case::typed_inlined(false, false)]
#[case::typed_parameterized(true, false)]
#[case::condition_inlined(false, true)]
#[case::condition_parameterized(true, true)]
#[tokio::test]
async fn postgres_typed_or_selects_only_claimed_obligations(
	#[future] obligations: (PgContainer, Arc<sqlx::PgPool>),
	#[case] parameterized: bool,
	#[case] use_condition: bool,
) {
	// Arrange
	let (_container, pool) = obligations.await;
	let mut query = Query::select();
	query.column("id").from("obligations");
	let no_deadline = Expr::col("publish_until").is_null();
	let expired = Expr::col("publish_until").lte(Expr::cust("CURRENT_TIMESTAMP"));
	if use_condition {
		query.and_where(Condition::any().add(no_deadline).add(expired));
	} else {
		query.and_where(no_deadline.or(expired));
	}
	query
		.and_where(Expr::col("state").eq("claimed"))
		.order_by("id", Order::Asc);
	let sql = if parameterized {
		query.build(PostgresQueryBuilder).0
	} else {
		query.to_string(PostgresQueryBuilder)
	};
	let execution = sqlx::query_scalar::<_, i32>(&sql);
	let execution = if parameterized {
		execution.bind("claimed")
	} else {
		execution
	};

	// Act
	let ids = execution.fetch_all(pool.as_ref()).await.unwrap();

	// Assert
	assert_eq!(ids, vec![2]);
}

#[rstest]
#[case::inlined(false)]
#[case::parameterized(true)]
#[tokio::test]
async fn postgres_arithmetic_evaluates_ast_grouping(
	#[future] postgres_container: (PgContainer, Arc<sqlx::PgPool>, u16, String),
	#[case] parameterized: bool,
) {
	// Arrange
	let (_container, pool, _, _) = postgres_container.await;
	let query = Query::select()
		.expr(Expr::val(2).add(3).mul(4))
		.expr(Expr::val(8).sub(Expr::val(3).sub(1)))
		.expr(Expr::val(2).mul(Expr::val(5).div(2)))
		.expr(Expr::val(64).div(Expr::val(8).mul(2)))
		.to_owned();
	let (sql, values) = if parameterized {
		query.build(PostgresQueryBuilder)
	} else {
		(query.to_string(PostgresQueryBuilder), Default::default())
	};
	let mut arguments = sqlx::postgres::PgArguments::default();
	for value in values.0 {
		let Value::Int(Some(value)) = value else {
			panic!("Expected an integer operand");
		};
		arguments.add(value).unwrap();
	}

	// Act
	let result: (i32, i32, i32, i32) = sqlx::query_as_with(&sql, arguments)
		.fetch_one(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(result, (20, 6, 4, 4));
}
