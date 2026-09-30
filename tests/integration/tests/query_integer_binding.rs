//! Native execution must preserve PostgreSQL integer function argument types.

use std::sync::Arc;

use futures::TryStreamExt;
use reinhardt_db::backends::{DatabaseConnection, QueryValue};
use reinhardt_db::orm::execution::convert_values;
use reinhardt_query::prelude::{
	Alias, Expr, ExprTrait, IntoIden, MySqlQueryBuilder, PostgresQueryBuilder, Query,
	QueryStatementBuilder, SimpleExpr, SqliteQueryBuilder, Values,
};
use reinhardt_test::fixtures::{mysql_container, postgres_container};
use rstest::rstest;
use sqlx::{MySqlPool, PgPool};
use testcontainers::{ContainerAsync, GenericImage};

#[rstest]
#[case::query_values(false)]
#[case::direct_values(true)]
#[tokio::test]
async fn postgres_integer_function_overload_executes_on_all_native_paths(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
	#[case] direct_values: bool,
) {
	// Arrange
	let (_container, _pool, _port, url) = postgres_container.await;
	let db = DatabaseConnection::connect_postgres(&url).await.unwrap();
	let (sql, values) = Query::select()
		.expr_as(
			SimpleExpr::FunctionCall(
				"right".into_iden(),
				vec![Expr::value("abcdef").into(), Expr::value(3_i32).into()],
			),
			Alias::new("suffix"),
		)
		.build(PostgresQueryBuilder);
	let params = if direct_values {
		vec![QueryValue::from("abcdef"), QueryValue::from(3_i32)]
	} else {
		convert_values(values)
	};
	assert_eq!(sql, r#"SELECT right($1, $2) AS "suffix""#);

	// Act
	let row = db.fetch_one(&sql, params.clone()).await.unwrap();
	let rows = db.fetch_all(&sql, params.clone()).await.unwrap();
	let optional = db.fetch_optional(&sql, params.clone()).await.unwrap();
	let streamed = db
		.fetch_stream(sql.clone(), params.clone(), 1)
		.unwrap()
		.try_collect::<Vec<_>>()
		.await
		.unwrap();
	let result = db.execute(&sql, params.clone()).await.unwrap();

	let mut transaction = db.begin().await.unwrap();
	let transaction_row = transaction.fetch_one(&sql, params.clone()).await.unwrap();
	let transaction_rows = transaction.fetch_all(&sql, params.clone()).await.unwrap();
	let transaction_optional = transaction
		.fetch_optional(&sql, params.clone())
		.await
		.unwrap();
	let transaction_streamed = transaction
		.fetch_stream(sql.clone(), params.clone(), 1)
		.unwrap()
		.try_collect::<Vec<_>>()
		.await
		.unwrap();
	let transaction_result = transaction.execute(&sql, params).await.unwrap();
	transaction.rollback().await.unwrap();

	// Assert
	assert_eq!(row.get::<String>("suffix").unwrap(), "def");
	assert_eq!(rows, vec![row.clone()]);
	assert_eq!(optional, Some(row.clone()));
	assert_eq!(streamed, vec![row.clone()]);
	assert_eq!(result.rows_affected, 1);
	assert_eq!(transaction_row, row);
	assert_eq!(transaction_rows, vec![row.clone()]);
	assert_eq!(transaction_optional, Some(row.clone()));
	assert_eq!(transaction_streamed, vec![row]);
	assert_eq!(transaction_result.rows_affected, 1);
}

#[rstest]
#[tokio::test]
async fn postgres_preserves_integer_widths_and_values(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	// Arrange
	let (_container, _pool, _port, url) = postgres_container.await;
	let db = DatabaseConnection::connect_postgres(&url).await.unwrap();
	for (narrow, wide) in [(i32::MIN, i64::MIN), (0, 0), (3, 3), (i32::MAX, i64::MAX)] {
		let (sql, values) = Query::select()
			.expr_as(
				SimpleExpr::FunctionCall("pg_typeof".into_iden(), vec![Expr::value(narrow).into()])
					.cast_as("text"),
				Alias::new("narrow_type"),
			)
			.expr_as(
				SimpleExpr::FunctionCall("pg_typeof".into_iden(), vec![Expr::value(wide).into()])
					.cast_as("text"),
				Alias::new("wide_type"),
			)
			.expr_as(Expr::value(narrow), Alias::new("narrow"))
			.expr_as(Expr::value(wide), Alias::new("wide"))
			.build(PostgresQueryBuilder);

		// Act
		let params = convert_values(values);
		let row = db.fetch_one(&sql, params.clone()).await.unwrap();
		let mut transaction = db.begin().await.unwrap();
		let transaction_row = transaction.fetch_one(&sql, params).await.unwrap();
		transaction.rollback().await.unwrap();

		// Assert
		assert_eq!(row.get::<String>("narrow_type").unwrap(), "integer");
		assert_eq!(row.get::<String>("wide_type").unwrap(), "bigint");
		assert_eq!(row.get::<i32>("narrow").unwrap(), narrow);
		assert_eq!(row.get::<i64>("wide").unwrap(), wide);
		assert_eq!(transaction_row, row);
	}
}

#[rstest]
#[tokio::test]
async fn postgres_null_integer_still_resolves_function_overloads(
	#[future] postgres_container: (ContainerAsync<GenericImage>, Arc<PgPool>, u16, String),
) {
	// Arrange
	let (_container, _pool, _port, url) = postgres_container.await;
	let db = DatabaseConnection::connect_postgres(&url).await.unwrap();
	let (sql, values) = Query::select()
		.expr_as(
			SimpleExpr::FunctionCall(
				"right".into_iden(),
				vec![
					Expr::value("abcdef").into(),
					Expr::value(None::<i32>).into(),
				],
			),
			Alias::new("suffix"),
		)
		.build(PostgresQueryBuilder);

	// Act
	let params = convert_values(values);
	let row = db.fetch_one(&sql, params.clone()).await.unwrap();
	let mut transaction = db.begin().await.unwrap();
	let transaction_row = transaction.fetch_one(&sql, params).await.unwrap();
	transaction.rollback().await.unwrap();

	// Assert
	assert_eq!(row.data.get("suffix"), Some(&QueryValue::Null));
	assert_eq!(transaction_row, row);
}

async fn assert_integer_roundtrip(db: &DatabaseConnection, sql: &str, values: Values) {
	// Act
	let params = convert_values(values);
	let row = db.fetch_one(sql, params.clone()).await.unwrap();
	let mut transaction = db.begin().await.unwrap();
	let transaction_row = transaction.fetch_one(sql, params).await.unwrap();
	transaction.rollback().await.unwrap();

	// Assert
	assert_eq!(row.get::<i32>("narrow").unwrap(), i32::MIN);
	assert_eq!(row.get::<i64>("wide").unwrap(), i64::MAX);
	assert_eq!(transaction_row, row);
}

#[rstest]
#[tokio::test]
async fn mysql_preserves_integer_values(
	#[future] mysql_container: (ContainerAsync<GenericImage>, Arc<MySqlPool>, u16, String),
) {
	// Arrange
	let (_container, _pool, _port, url) = mysql_container.await;
	let db = DatabaseConnection::connect_mysql(&url).await.unwrap();
	let (sql, values) = Query::select()
		.expr_as(Expr::value(i32::MIN), Alias::new("narrow"))
		.expr_as(Expr::value(i64::MAX), Alias::new("wide"))
		.build(MySqlQueryBuilder);

	assert_integer_roundtrip(&db, &sql, values).await;
}

#[rstest]
#[tokio::test]
async fn sqlite_preserves_integer_values() {
	// Arrange
	let db = DatabaseConnection::connect_sqlite("sqlite::memory:")
		.await
		.unwrap();
	let (sql, values) = Query::select()
		.expr_as(Expr::value(i32::MIN), Alias::new("narrow"))
		.expr_as(Expr::value(i64::MAX), Alias::new("wide"))
		.build(SqliteQueryBuilder);

	assert_integer_roundtrip(&db, &sql, values).await;
}
