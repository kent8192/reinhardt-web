//! PostgreSQL execution coverage for typed expression grouping.

use std::sync::Arc;

use reinhardt_query::{
	ColumnDef, ColumnType, Condition, Expr, ExprTrait, Order, PostgresQueryBuilder, Query,
	QueryStatementBuilder, SimpleExpr, Value, Values,
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

#[fixture]
fn custom_publication_predicate(#[default(false)] with_values: bool) -> SimpleExpr {
	if with_values {
		Expr::cust_with_values("publish_until IS NULL OR state = ?", ["claimed"]).into()
	} else {
		Expr::cust("publish_until IS NULL OR publish_until <= CURRENT_TIMESTAMP").into()
	}
}

fn postgres_arguments(values: Values) -> sqlx::postgres::PgArguments {
	let mut arguments = sqlx::postgres::PgArguments::default();
	for value in values.0 {
		match value {
			Value::Int(Some(value)) => arguments.add(value).unwrap(),
			Value::String(Some(value)) => arguments.add(*value).unwrap(),
			value => panic!("Unexpected custom-predicate argument: {value:?}"),
		}
	}
	arguments
}

#[rstest]
#[case::raw_inlined(false, false)]
#[case::raw_parameterized(false, true)]
#[case::template_inlined(true, false)]
#[case::template_parameterized(true, true)]
#[tokio::test]
async fn postgres_custom_or_selects_only_targeted_obligations(
	#[future] obligations: (PgContainer, Arc<sqlx::PgPool>),
	#[case] _with_values: bool,
	#[case] parameterized: bool,
	#[with(_with_values)] custom_publication_predicate: SimpleExpr,
) {
	// Arrange
	let (_container, pool) = obligations.await;
	let query = Query::select()
		.column("id")
		.from("obligations")
		.and_where(Expr::col("state").ne("settled"))
		.and_where(custom_publication_predicate)
		.and_where(Expr::col("id").eq(2))
		.order_by("id", Order::Asc)
		.to_owned();
	let (sql, values) = if parameterized {
		query.build(PostgresQueryBuilder)
	} else {
		(query.to_string(PostgresQueryBuilder), Values::default())
	};

	// Act
	let ids = sqlx::query_scalar_with::<_, i32, _>(&sql, postgres_arguments(values))
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(ids, vec![2]);
}

#[rstest]
#[case::raw_inlined(false, false)]
#[case::raw_parameterized(false, true)]
#[case::template_inlined(true, false)]
#[case::template_parameterized(true, true)]
#[tokio::test]
async fn postgres_custom_line_comment_preserves_neighboring_filters(
	#[future] obligations: (PgContainer, Arc<sqlx::PgPool>),
	#[case] with_values: bool,
	#[case] parameterized: bool,
) {
	// Arrange
	let (_container, pool) = obligations.await;
	let predicate: SimpleExpr = if with_values {
		Expr::cust_with_values("state = ? OR publish_until IS NULL -- reason", ["pending"]).into()
	} else {
		Expr::cust("state = 'pending' OR publish_until IS NULL -- reason").into()
	};
	let query = Query::select()
		.column("id")
		.from("obligations")
		.and_where(predicate)
		.and_where(Expr::col("id").eq(2))
		.order_by("id", Order::Asc)
		.to_owned();
	let (sql, values) = if parameterized {
		query.build(PostgresQueryBuilder)
	} else {
		(query.to_string(PostgresQueryBuilder), Values::default())
	};

	// Act
	let ids = sqlx::query_scalar_with::<_, i32, _>(&sql, postgres_arguments(values))
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(ids, vec![2]);
}

#[rstest]
#[case::raw_inlined(false, false)]
#[case::raw_parameterized(false, true)]
#[case::template_inlined(true, false)]
#[case::template_parameterized(true, true)]
#[tokio::test]
async fn postgres_current_of_mutates_only_the_cursor_row(
	#[future] obligations: (PgContainer, Arc<sqlx::PgPool>),
	#[case] template: bool,
	#[case] parameterized: bool,
	#[values(false, true)] delete: bool,
) {
	// Arrange
	let (_container, pool) = obligations.await;
	let mut transaction = pool.begin().await.unwrap();
	let cursor_query = Query::select()
		.column("id")
		.from("obligations")
		.and_where(Expr::col("id").eq(2))
		.lock_exclusive()
		.to_string(PostgresQueryBuilder);
	// The AST has no cursor protocol nodes; issue DECLARE/FETCH directly while
	// building the cursor query and tested DML through Query.
	let declare = format!("DECLARE scoped_cursor CURSOR FOR {cursor_query}");
	let fetch = "FETCH NEXT FROM scoped_cursor";
	sqlx::query(&declare)
		.execute(&mut *transaction)
		.await
		.unwrap();
	let cursor_id = sqlx::query_scalar::<_, i32>(fetch)
		.fetch_one(&mut *transaction)
		.await
		.unwrap();
	let predicate: SimpleExpr = if template {
		SimpleExpr::CustomWithExpr(
			"CURRENT OF ?".into(),
			vec![Expr::cust("scoped_cursor").into()],
		)
	} else {
		Expr::cust("CURRENT OF scoped_cursor").into()
	};
	let (sql, values) = if delete {
		let query = Query::delete()
			.from_table("obligations")
			.and_where(predicate)
			.returning(["id"])
			.to_owned();
		if parameterized {
			query.build(PostgresQueryBuilder)
		} else {
			(query.to_string(PostgresQueryBuilder), Values::default())
		}
	} else {
		let query = Query::update()
			.table("obligations")
			.value("state", "processed")
			.and_where(predicate)
			.returning(["id"])
			.to_owned();
		if parameterized {
			query.build(PostgresQueryBuilder)
		} else {
			(query.to_string(PostgresQueryBuilder), Values::default())
		}
	};
	let remaining = Query::select()
		.columns(["id", "state"])
		.from("obligations")
		.order_by("id", Order::Asc)
		.to_string(PostgresQueryBuilder);

	// Act
	let ids = sqlx::query_scalar_with::<_, i32, _>(&sql, postgres_arguments(values))
		.fetch_all(&mut *transaction)
		.await
		.unwrap();
	let rows = sqlx::query_as::<_, (i32, String)>(&remaining)
		.fetch_all(&mut *transaction)
		.await
		.unwrap();

	// Assert
	assert_eq!(cursor_id, 2);
	assert_eq!(ids, vec![2]);
	let expected_rows = if delete {
		vec![(1, "pending".into()), (3, "settled".into())]
	} else {
		vec![
			(1, "pending".into()),
			(2, "processed".into()),
			(3, "settled".into()),
		]
	};
	assert_eq!(rows, expected_rows);
}

#[rstest]
#[case::raw_inlined(false, false)]
#[case::raw_parameterized(false, true)]
#[case::template_inlined(true, false)]
#[case::template_parameterized(true, true)]
#[tokio::test]
async fn postgres_custom_or_mutates_only_targeted_obligations(
	#[future] obligations: (PgContainer, Arc<sqlx::PgPool>),
	#[case] _with_values: bool,
	#[case] parameterized: bool,
	#[with(_with_values)] custom_publication_predicate: SimpleExpr,
	#[values(false, true)] delete: bool,
) {
	// Arrange
	let (_container, pool) = obligations.await;
	let (sql, values) = if delete {
		let query = Query::delete()
			.from_table("obligations")
			.and_where(Expr::col("state").ne("settled"))
			.and_where(custom_publication_predicate)
			.and_where(Expr::col("id").eq(2))
			.returning(["id"])
			.to_owned();
		if parameterized {
			query.build(PostgresQueryBuilder)
		} else {
			(query.to_string(PostgresQueryBuilder), Values::default())
		}
	} else {
		let query = Query::update()
			.table("obligations")
			.value("state", "processed")
			.and_where(Expr::col("state").ne("settled"))
			.and_where(custom_publication_predicate)
			.and_where(Expr::col("id").eq(2))
			.returning(["id"])
			.to_owned();
		if parameterized {
			query.build(PostgresQueryBuilder)
		} else {
			(query.to_string(PostgresQueryBuilder), Values::default())
		}
	};
	let remaining = Query::select()
		.columns(["id", "state"])
		.from("obligations")
		.order_by("id", Order::Asc)
		.to_string(PostgresQueryBuilder);

	// Act
	let ids = sqlx::query_scalar_with::<_, i32, _>(&sql, postgres_arguments(values))
		.fetch_all(pool.as_ref())
		.await
		.unwrap();
	let rows = sqlx::query_as::<_, (i32, String)>(&remaining)
		.fetch_all(pool.as_ref())
		.await
		.unwrap();

	// Assert
	assert_eq!(ids, vec![2]);
	let expected_rows = if delete {
		vec![(1, "pending".into()), (3, "settled".into())]
	} else {
		vec![
			(1, "pending".into()),
			(2, "processed".into()),
			(3, "settled".into()),
		]
	};
	assert_eq!(rows, expected_rows);
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
