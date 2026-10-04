//! SQL aggregate function builders.
//!
//! This module provides the [`Func`] struct with static methods for
//! constructing common SQL aggregate function calls.

use super::simple_expr::{SimpleExpr, TemporalTimeZone, TemporalTruncKind, TemporalTruncOutput};
use crate::types::IntoIden;

/// SQL aggregate function builder.
///
/// Provides static methods for building common aggregate function expressions
/// such as COUNT, SUM, AVG, MIN, MAX, and COALESCE.
///
/// # Examples
///
/// ```rust
/// use reinhardt_query::prelude::*;
///
/// // COUNT(*)
/// let count_all = Func::count(Expr::asterisk().into_simple_expr());
///
/// // SUM(price)
/// let total = Func::sum(Expr::col("price").into_simple_expr());
///
/// // COALESCE(name, 'Unknown')
/// let name = Func::coalesce(vec![
///     Expr::col("name").into_simple_expr(),
///     Expr::val("Unknown").into_simple_expr(),
/// ]);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Func;

impl Func {
	/// Create a COUNT(expr) function call.
	pub fn count(expr: SimpleExpr) -> SimpleExpr {
		SimpleExpr::FunctionCall("COUNT".into_iden(), vec![expr])
	}

	/// Create a SUM(expr) function call.
	pub fn sum(expr: SimpleExpr) -> SimpleExpr {
		SimpleExpr::FunctionCall("SUM".into_iden(), vec![expr])
	}

	/// Create an AVG(expr) function call.
	pub fn avg(expr: SimpleExpr) -> SimpleExpr {
		SimpleExpr::FunctionCall("AVG".into_iden(), vec![expr])
	}

	/// Create a MIN(expr) function call.
	pub fn min(expr: SimpleExpr) -> SimpleExpr {
		SimpleExpr::FunctionCall("MIN".into_iden(), vec![expr])
	}

	/// Create a MAX(expr) function call.
	pub fn max(expr: SimpleExpr) -> SimpleExpr {
		SimpleExpr::FunctionCall("MAX".into_iden(), vec![expr])
	}

	/// Create a COALESCE(expr1, expr2, ...) function call.
	pub fn coalesce(exprs: Vec<SimpleExpr>) -> SimpleExpr {
		SimpleExpr::FunctionCall("COALESCE".into_iden(), exprs)
	}

	/// Extract numeric epoch seconds using PostgreSQL's `EXTRACT` (P2).
	///
	/// This preserves the numeric result and fractional seconds. Cast to the
	/// desired database type explicitly. Checked non-PostgreSQL builders reject
	/// this expression. The API is available on native and WASM targets.
	///
	/// ```
	/// use reinhardt_query::{Expr, ExprTrait, Func, PostgresQueryBuilder, Query, QueryStatementBuilder};
	/// let sql = Query::select()
	///     .expr(Func::pg_extract_epoch(Expr::current_timestamp().into_simple_expr()).cast_as("int8"))
	///     .to_string(PostgresQueryBuilder);
	/// assert_eq!(sql, "SELECT CAST(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP) AS \"int8\")");
	/// ```
	pub fn pg_extract_epoch(expr: SimpleExpr) -> SimpleExpr {
		SimpleExpr::PgExtractEpoch(Box::new(expr))
	}

	/// Create a typed temporal truncation expression.
	///
	/// For PostgreSQL [`TemporalTruncOutput::DateTime`] output, `expr` must
	/// produce `TIMESTAMP WITH TIME ZONE`. Convert a `TIMESTAMP WITHOUT TIME
	/// ZONE` source explicitly before constructing the expression.
	pub fn temporal_trunc(
		expr: SimpleExpr,
		kind: TemporalTruncKind,
		time_zone: Option<TemporalTimeZone>,
		output: TemporalTruncOutput,
	) -> Result<SimpleExpr, crate::QueryBuildError> {
		if output == TemporalTruncOutput::Date
			&& matches!(
				kind,
				TemporalTruncKind::Hour | TemporalTruncKind::Minute | TemporalTruncKind::Second
			) {
			return Err(crate::QueryBuildError::InvalidTemporalTruncation {
				kind: kind.as_str(),
				output: "date",
			});
		}

		Ok(SimpleExpr::TemporalTrunc {
			expr: Box::new(expr),
			kind,
			time_zone,
			output,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::expr::Expr;
	use crate::value::Value;
	use rstest::rstest;

	#[rstest]
	fn test_temporal_trunc_rejects_time_units_for_date_output() {
		// Arrange
		let expr = Expr::col("occurred_at").into_simple_expr();

		// Act
		let result = Func::temporal_trunc(
			expr,
			TemporalTruncKind::Hour,
			None,
			TemporalTruncOutput::Date,
		);

		// Assert
		assert!(matches!(
			result,
			Err(crate::QueryBuildError::InvalidTemporalTruncation {
				kind: "hour",
				output: "date"
			})
		));
	}

	#[rstest]
	fn test_func_count_creates_function_call() {
		// Arrange
		let expr = Expr::asterisk().into_simple_expr();

		// Act
		let result = Func::count(expr);

		// Assert
		if let SimpleExpr::FunctionCall(name, args) = result {
			assert_eq!(name.to_string(), "COUNT");
			assert_eq!(args.len(), 1);
			assert!(matches!(args[0], SimpleExpr::Asterisk));
		} else {
			panic!("Expected FunctionCall variant");
		}
	}

	#[rstest]
	fn test_func_sum_creates_function_call() {
		// Arrange
		let expr = Expr::col("price").into_simple_expr();

		// Act
		let result = Func::sum(expr);

		// Assert
		if let SimpleExpr::FunctionCall(name, args) = result {
			assert_eq!(name.to_string(), "SUM");
			assert_eq!(args.len(), 1);
			assert!(matches!(args[0], SimpleExpr::Column(_)));
		} else {
			panic!("Expected FunctionCall variant");
		}
	}

	#[rstest]
	fn test_func_avg_creates_function_call() {
		// Arrange
		let expr = Expr::col("score").into_simple_expr();

		// Act
		let result = Func::avg(expr);

		// Assert
		if let SimpleExpr::FunctionCall(name, args) = result {
			assert_eq!(name.to_string(), "AVG");
			assert_eq!(args.len(), 1);
			assert!(matches!(args[0], SimpleExpr::Column(_)));
		} else {
			panic!("Expected FunctionCall variant");
		}
	}

	#[rstest]
	fn test_func_min_creates_function_call() {
		// Arrange
		let expr = Expr::col("age").into_simple_expr();

		// Act
		let result = Func::min(expr);

		// Assert
		if let SimpleExpr::FunctionCall(name, args) = result {
			assert_eq!(name.to_string(), "MIN");
			assert_eq!(args.len(), 1);
			assert!(matches!(args[0], SimpleExpr::Column(_)));
		} else {
			panic!("Expected FunctionCall variant");
		}
	}

	#[rstest]
	fn test_func_max_creates_function_call() {
		// Arrange
		let expr = Expr::col("salary").into_simple_expr();

		// Act
		let result = Func::max(expr);

		// Assert
		if let SimpleExpr::FunctionCall(name, args) = result {
			assert_eq!(name.to_string(), "MAX");
			assert_eq!(args.len(), 1);
			assert!(matches!(args[0], SimpleExpr::Column(_)));
		} else {
			panic!("Expected FunctionCall variant");
		}
	}

	#[rstest]
	fn test_func_coalesce_creates_function_call() {
		// Arrange
		let exprs = vec![
			Expr::col("name").into_simple_expr(),
			Expr::val("Unknown").into_simple_expr(),
		];

		// Act
		let result = Func::coalesce(exprs);

		// Assert
		if let SimpleExpr::FunctionCall(name, args) = result {
			assert_eq!(name.to_string(), "COALESCE");
			assert_eq!(args.len(), 2);
			assert!(matches!(args[0], SimpleExpr::Column(_)));
			assert!(matches!(args[1], SimpleExpr::Value(Value::String(Some(_)))));
		} else {
			panic!("Expected FunctionCall variant");
		}
	}

	#[rstest]
	fn test_func_count_with_column() {
		// Arrange
		let expr = Expr::col("id").into_simple_expr();

		// Act
		let result = Func::count(expr);

		// Assert
		if let SimpleExpr::FunctionCall(name, args) = result {
			assert_eq!(name.to_string(), "COUNT");
			assert_eq!(args.len(), 1);
			assert!(matches!(args[0], SimpleExpr::Column(_)));
		} else {
			panic!("Expected FunctionCall variant");
		}
	}
}

#[cfg(test)]
mod epoch_tests {
	use crate::{
		CockroachDBQueryBuilder, ColumnDef, Expr, ExprTrait, Func, MySqlQueryBuilder,
		PostgresQueryBuilder, Query, QueryBuildError, SqliteQueryBuilder, Value, Values,
	};
	use rstest::rstest;

	#[rstest]
	fn epoch_extraction_keeps_numeric_syntax_and_argument_order() {
		// Arrange: extraction wraps the first bind, and the next projection follows it.
		let statement = Query::select()
			.expr(Func::pg_extract_epoch(
				Expr::val("2000-01-01T00:00:00.125Z").cast_as("timestamptz"),
			))
			.expr(Expr::val(9_i64))
			.take();
		// Act
		let (sql, values) = PostgresQueryBuilder
			.build_select_checked(&statement)
			.unwrap();
		// Assert
		assert_eq!(
			sql,
			"SELECT EXTRACT(EPOCH FROM CAST($1 AS \"timestamptz\")), $2"
		);
		assert_eq!(
			values,
			Values(vec![
				Value::String(Some(Box::new("2000-01-01T00:00:00.125Z".into()))),
				Value::BigInt(Some(9))
			])
		);
		for (backend, result) in [
			("MySQL", MySqlQueryBuilder.build_select_checked(&statement)),
			(
				"SQLite",
				SqliteQueryBuilder.build_select_checked(&statement),
			),
			(
				"CockroachDB",
				CockroachDBQueryBuilder::new().build_select_checked(&statement),
			),
		] {
			assert_eq!(
				result,
				Err(QueryBuildError::UnsupportedBackendFeature {
					feature: "PostgreSQL numeric epoch extraction",
					backend
				})
			);
		}
	}

	#[rstest]
	fn schema_defaults_and_nested_index_expressions_use_checked_backends() {
		// Arrange
		let epoch = Func::pg_extract_epoch(Expr::current_timestamp().into_simple_expr());
		let table = Query::create_table()
			.table("records")
			.col(
				ColumnDef::new("created_at")
					.big_integer()
					.default(epoch.clone().cast_as("int8")),
			)
			.take();
		let index = Query::create_index()
			.table("records")
			.name("epoch_index")
			.expr(epoch)
			.take();
		// Act / Assert: schema defaults are constants; SQLite rejects the nested PG operation.
		let (sql, values) = PostgresQueryBuilder
			.build_create_table_checked(&table)
			.unwrap();
		assert_eq!(
			sql,
			"CREATE TABLE \"records\" (\"created_at\" BIGINT DEFAULT CAST(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP) AS \"int8\"))"
		);
		assert!(values.is_empty());
		let expected = Err(QueryBuildError::UnsupportedBackendFeature {
			feature: "PostgreSQL numeric epoch extraction",
			backend: "SQLite",
		});
		assert_eq!(
			SqliteQueryBuilder.build_create_table_checked(&table),
			expected
		);
		assert_eq!(
			SqliteQueryBuilder.build_create_index_checked(&index),
			expected
		);
	}
}
