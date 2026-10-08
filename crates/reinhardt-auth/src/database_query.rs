//! PostgreSQL arguments for native database-backed authentication state (P0).
use reinhardt_query::{PostgresQueryBuilder, QueryStatementBuilder};

pub(crate) type Prepared = (String, sqlx::postgres::PgArguments);

pub(crate) fn prepare(statement: impl QueryStatementBuilder) -> Result<Prepared, String> {
	reinhardt_query_sqlx::prepare_postgres(statement.build_any(&PostgresQueryBuilder))
		.map(|prepared| prepared.into_parts())
		.map_err(|error| error.to_string())
}

/// Render a built-in schema operation while preserving its migration boundary.
#[cfg(feature = "oauth")]
pub(crate) fn schema_operation(
	forward: impl QueryStatementBuilder,
	reverse: impl QueryStatementBuilder,
) -> reinhardt_db::migrations::Operation {
	reinhardt_db::migrations::Operation::RunSQL {
		sql: forward.to_string(PostgresQueryBuilder),
		reverse_sql: Some(reverse.to_string(PostgresQueryBuilder)),
	}
}

/// Extract a JSONB text field using typed PostgreSQL operators and bound keys.
#[cfg(feature = "oauth")]
pub(crate) fn json_text_path(keys: &[&str]) -> reinhardt_query::SimpleExpr {
	use reinhardt_query::types::PgBinOper;
	use reinhardt_query::{BinOper, Expr, SimpleExpr};
	let mut expression = Expr::col("payload").into_simple_expr();
	for (index, key) in keys.iter().enumerate() {
		let operator = if index + 1 == keys.len() {
			PgBinOper::JsonGetAsText
		} else {
			PgBinOper::JsonGetByIndex
		};
		expression = SimpleExpr::Binary(
			Box::new(expression),
			BinOper::PgOperator(operator),
			Box::new(Expr::val(*key).into_simple_expr()),
		);
	}
	expression
}

#[cfg(all(test, feature = "oauth"))]
mod tests {
	use super::*;
	use reinhardt_query::{ExprTrait, Query, Value, Values};
	use rstest::rstest;

	#[rstest]
	fn json_paths_and_runtime_values_keep_their_generated_positions() {
		// Arrange: JSON keys are bound ahead of runtime predicates and LIMIT.
		let statement = Query::select()
			.column("payload")
			.from("requests")
			.and_where(json_text_path(&["request", "client_id"]).eq("client' ? $1"))
			.limit(1)
			.take();
		// Act
		let (sql, values) = statement.build(PostgresQueryBuilder);
		// Assert
		assert_eq!(
			sql,
			"SELECT \"payload\" FROM \"requests\" WHERE \"payload\" -> $1 ->> $2 = $3 LIMIT $4"
		);
		assert_eq!(
			values,
			Values(vec![
				Value::String(Some(Box::new("request".into()))),
				Value::String(Some(Box::new("client_id".into()))),
				Value::String(Some(Box::new("client' ? $1".into()))),
				Value::Int(Some(1))
			])
		);
		use sqlx::Arguments;
		assert_eq!(prepare(statement).unwrap().1.len(), 4);
	}
}
