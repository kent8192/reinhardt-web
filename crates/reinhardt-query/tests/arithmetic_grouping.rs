//! MySQL and SQLite regression coverage for nested typed arithmetic.

use std::sync::Arc;

use reinhardt_query::{
	Expr, ExprTrait, MySqlQueryBuilder, Query, QueryStatementBuilder, SimpleExpr,
	SqliteQueryBuilder, Value, Values,
};
use rstest::*;
use sqlx::{Arguments, Column, Connection, Row, TypeInfo};

mod common;
use common::{MySqlContainer, mysql_container};

struct ArithmeticCase {
	expr: SimpleExpr,
	sql: &'static str,
	control: &'static str,
	operands: Vec<i64>,
	result: i64,
}

#[fixture]
fn arithmetic_cases() -> Vec<ArithmeticCase> {
	vec![
		ArithmeticCase {
			expr: Expr::val(3_i64).add(5_i64).mul(2_i64),
			sql: "(? + ?) * ?",
			control: "(? + ?) * ?",
			operands: vec![3, 5, 2],
			result: 16,
		},
		ArithmeticCase {
			expr: Expr::val(3_i64).mul(Expr::val(5_i64).add(2_i64)),
			sql: "? * (? + ?)",
			control: "? * (? + ?)",
			operands: vec![3, 5, 2],
			result: 21,
		},
		ArithmeticCase {
			expr: Expr::val(12_i64).sub(Expr::val(5_i64).sub(2_i64)),
			sql: "? - (? - ?)",
			control: "? - (? - ?)",
			operands: vec![12, 5, 2],
			result: 9,
		},
		ArithmeticCase {
			expr: Expr::val(24_i64).div(Expr::val(6_i64).div(2_i64)),
			sql: "? / (? / ?)",
			control: "? / (? / ?)",
			operands: vec![24, 6, 2],
			result: 8,
		},
		ArithmeticCase {
			expr: Expr::val(64_i64).div(Expr::val(8_i64).mul(2_i64)),
			sql: "? / (? * ?)",
			control: "? / (? * ?)",
			operands: vec![64, 8, 2],
			result: 4,
		},
		ArithmeticCase {
			expr: Expr::val(12_i64).sub(Expr::val(5_i64).add(2_i64)),
			sql: "? - (? + ?)",
			control: "? - (? + ?)",
			operands: vec![12, 5, 2],
			result: 5,
		},
		ArithmeticCase {
			expr: Expr::val(20_i64).modulo(Expr::val(7_i64).modulo(4_i64)),
			sql: "? % (? % ?)",
			control: "? % (? % ?)",
			operands: vec![20, 7, 4],
			result: 2,
		},
		ArithmeticCase {
			expr: Expr::val(12_i64).sub(5_i64).sub(2_i64),
			sql: "? - ? - ?",
			control: "(? - ?) - ?",
			operands: vec![12, 5, 2],
			result: 5,
		},
		ArithmeticCase {
			expr: Expr::val(3_i64).add(Expr::val(5_i64).add(2_i64)),
			sql: "? + (? + ?)",
			control: "? + (? + ?)",
			operands: vec![3, 5, 2],
			result: 10,
		},
		ArithmeticCase {
			expr: Expr::val(3_i64).mul(Expr::val(5_i64).mul(2_i64)),
			sql: "? * (? * ?)",
			control: "? * (? * ?)",
			operands: vec![3, 5, 2],
			result: 30,
		},
		ArithmeticCase {
			expr: Expr::val(3_i64).add(Expr::val(5_i64).mul(2_i64)),
			sql: "? + ? * ?",
			control: "? + (? * ?)",
			operands: vec![3, 5, 2],
			result: 13,
		},
		ArithmeticCase {
			expr: Expr::val(20_i64).div(5_i64).modulo(3_i64),
			sql: "? / ? % ?",
			control: "(? / ?) % ?",
			operands: vec![20, 5, 3],
			result: 1,
		},
		ArithmeticCase {
			expr: Expr::val(3_i64).add(5_i64).mul(Expr::val(9_i64).sub(7_i64)),
			sql: "(? + ?) * (? - ?)",
			control: "(? + ?) * (? - ?)",
			operands: vec![3, 5, 9, 7],
			result: 16,
		},
		ArithmeticCase {
			expr: Expr::val(1_i64).add(Expr::val(2_i64).add(Expr::cust("3 -- note"))),
			sql: "? + (? + 3 -- note\n)",
			control: "? + (? + 3)",
			operands: vec![1, 2],
			result: 6,
		},
		ArithmeticCase {
			expr: Expr::val(1_i64).add(Expr::cust("2 -- note")).mul(3_i64),
			sql: "(? + 2 -- note\n) * ?",
			control: "(? + 2) * ?",
			operands: vec![1, 3],
			result: 9,
		},
		ArithmeticCase {
			expr: Expr::val(1_i64)
				.add(Expr::val(2_i64).add(Expr::cust_with_values("? -- note", [3_i64]))),
			sql: "? + (? + ? -- note\n)",
			control: "? + (? + ?)",
			operands: vec![1, 2, 3],
			result: 6,
		},
		ArithmeticCase {
			expr: Expr::val(1_i64)
				.add(Expr::val(2_i64).add(Expr::val(3_i64).add(Expr::cust("4 -- note")))),
			sql: "? + (? + (? + 4 -- note\n)\n)",
			control: "? + (? + (? + 4))",
			operands: vec![1, 2, 3],
			result: 10,
		},
	]
}

#[fixture]
fn mysql_hash_comment_cases() -> Vec<ArithmeticCase> {
	vec![
		ArithmeticCase {
			expr: Expr::val(1_i64).add(Expr::val(2_i64).add(Expr::cust("3 # note"))),
			sql: "? + (? + 3 # note\n)",
			control: "? + (? + 3)",
			operands: vec![1, 2],
			result: 6,
		},
		ArithmeticCase {
			expr: Expr::val(1_i64).add(Expr::cust("2 # note")).mul(3_i64),
			sql: "(? + 2 # note\n) * ?",
			control: "(? + 2) * ?",
			operands: vec![1, 3],
			result: 9,
		},
		ArithmeticCase {
			expr: Expr::val(1_i64)
				.add(Expr::val(2_i64).add(Expr::cust_with_values("? # note", [3_i64]))),
			sql: "? + (? + ? # note\n)",
			control: "? + (? + ?)",
			operands: vec![1, 2, 3],
			result: 6,
		},
		ArithmeticCase {
			expr: Expr::val(1_i64)
				.add(Expr::val(2_i64).add(Expr::val(3_i64).add(Expr::cust("4 # note")))),
			sql: "? + (? + (? + 4 # note\n)\n)",
			control: "? + (? + (? + 4))",
			operands: vec![1, 2, 3],
			result: 10,
		},
	]
}

#[rstest]
fn arithmetic_sql_retains_grouping_and_bind_order(
	mut arithmetic_cases: Vec<ArithmeticCase>,
	mysql_hash_comment_cases: Vec<ArithmeticCase>,
	#[values(false, true)] mysql: bool,
) {
	if mysql {
		arithmetic_cases.extend(mysql_hash_comment_cases);
	}
	for case in arithmetic_cases {
		// Arrange
		let query = Query::select().expr_as(case.expr, "computed").to_owned();
		let alias = if mysql { "`computed`" } else { "\"computed\"" };
		let expected_values = Values(case.operands.iter().copied().map(Value::from).collect());

		// Act
		let (sql, values) = if mysql {
			query.build(MySqlQueryBuilder)
		} else {
			query.build(SqliteQueryBuilder)
		};
		let inlined = if mysql {
			query.to_string(MySqlQueryBuilder)
		} else {
			query.to_string(SqliteQueryBuilder)
		};

		// Assert
		assert_eq!(sql, format!("SELECT {} AS {alias}", case.sql));
		assert_eq!(values, expected_values, "{}", case.sql);
		let mut operands = case.operands.iter();
		let expected_inlined = case
			.sql
			.chars()
			.map(|character| {
				if character == '?' {
					operands.next().unwrap().to_string()
				} else {
					character.to_string()
				}
			})
			.collect::<String>();
		assert_eq!(inlined, format!("SELECT {expected_inlined} AS {alias}"));
	}
}

#[rstest]
fn sqlite_hash_in_literal_keeps_group_format() {
	// Arrange
	let query = Query::select()
		.expr(Expr::val(1_i64).add(Expr::val(2_i64).add(Expr::cust("'3 # literal'"))))
		.to_owned();

	// Act
	let (sql, values) = query.build(SqliteQueryBuilder);
	let inlined = query.to_string(SqliteQueryBuilder);

	// Assert: SQLite does not treat a hash as a line-comment marker.
	assert_eq!(sql, "SELECT ? + (? + '3 # literal')");
	assert_eq!(values, Values(vec![1_i64.into(), 2_i64.into()]));
	assert_eq!(inlined, "SELECT 1 + (2 + '3 # literal')");
}

fn sqlite_arguments(values: Values) -> sqlx::sqlite::SqliteArguments<'static> {
	let mut arguments = sqlx::sqlite::SqliteArguments::default();
	for value in values.0 {
		let Value::BigInt(Some(value)) = value else {
			panic!("unexpected arithmetic argument: {value:?}");
		};
		arguments.add(value).unwrap();
	}
	arguments
}

fn mysql_arguments(values: Values) -> sqlx::mysql::MySqlArguments {
	let mut arguments = sqlx::mysql::MySqlArguments::default();
	for value in values.0 {
		let Value::BigInt(Some(value)) = value else {
			panic!("unexpected arithmetic argument: {value:?}");
		};
		arguments.add(value).unwrap();
	}
	arguments
}

fn mysql_arithmetic_result(row: sqlx::mysql::MySqlRow) -> f64 {
	// Literal operands can make MySQL report BIGINT instead of DOUBLE.
	match row.column(0).type_info().name() {
		"BIGINT" => row.get::<i64, _>(0) as f64,
		"DOUBLE" => row.get(0),
		data_type => panic!("unexpected arithmetic result type: {data_type}"),
	}
}

#[rstest]
#[tokio::test]
async fn sqlite_arithmetic_matches_explicitly_grouped_control(
	arithmetic_cases: Vec<ArithmeticCase>,
) {
	// Arrange
	let mut connection = sqlx::SqliteConnection::connect("sqlite::memory:")
		.await
		.unwrap();
	for case in arithmetic_cases {
		let query = Query::select().expr_as(case.expr, "computed").to_owned();
		let (sql, values) = query.build(SqliteQueryBuilder);
		let control_sql = format!("SELECT {} AS computed", case.control);
		assert_eq!(
			values,
			Values(case.operands.into_iter().map(Value::from).collect())
		);

		// Act
		let result = sqlx::query_scalar_with::<_, i64, _>(&sql, sqlite_arguments(values.clone()))
			.fetch_one(&mut connection)
			.await
			.unwrap();
		let control = sqlx::query_scalar_with::<_, i64, _>(&control_sql, sqlite_arguments(values))
			.fetch_one(&mut connection)
			.await
			.unwrap();

		// Assert
		assert_eq!(result, case.result, "{sql}");
		assert_eq!(control, case.result, "{control_sql}");
	}
}

#[rstest]
#[tokio::test]
async fn mysql_arithmetic_matches_explicitly_grouped_control(
	#[future] mysql_container: (MySqlContainer, Arc<sqlx::MySqlPool>, u16, String),
	mut arithmetic_cases: Vec<ArithmeticCase>,
	mysql_hash_comment_cases: Vec<ArithmeticCase>,
) {
	// Arrange: the container guard owns cleanup, including assertion failures.
	let (_container, pool, _, _) = mysql_container.await;
	arithmetic_cases.extend(mysql_hash_comment_cases);
	for case in arithmetic_cases {
		let query = Query::select().expr_as(case.expr, "computed").to_owned();
		let (sql, values) = query.build(MySqlQueryBuilder);
		let control_sql = format!("SELECT {} AS computed", case.control);
		assert_eq!(
			values,
			Values(case.operands.into_iter().map(Value::from).collect())
		);

		// Act
		let result = sqlx::query_with(&sql, mysql_arguments(values.clone()))
			.fetch_one(pool.as_ref())
			.await
			.unwrap();
		let control = sqlx::query_with(&control_sql, mysql_arguments(values))
			.fetch_one(pool.as_ref())
			.await
			.unwrap();

		// Assert: every expected value is a small, exactly representable integer.
		assert_eq!(mysql_arithmetic_result(result), case.result as f64, "{sql}");
		assert_eq!(
			mysql_arithmetic_result(control),
			case.result as f64,
			"{control_sql}"
		);
		if case.sql.contains('#') {
			// Act: execute the hash-comment forms with literal values as well.
			let inlined_sql = query.to_string(MySqlQueryBuilder);
			let inlined_result = sqlx::query(&inlined_sql)
				.fetch_one(pool.as_ref())
				.await
				.unwrap();

			// Assert
			assert_eq!(
				mysql_arithmetic_result(inlined_result),
				case.result as f64,
				"{inlined_sql}"
			);
		}
	}
}
