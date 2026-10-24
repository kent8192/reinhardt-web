//! Generated PostgreSQL calls isolate changing native argument types (#6533).
#![cfg(all(feature = "backends", feature = "postgres"))]

use futures::StreamExt;
use reinhardt_db::backends::{DatabaseConnection, QueryValue};
use reinhardt_query::{PostgresQueryBuilder, Query, QueryStatementBuilder, Value, Values};
use rstest::rstest;
use sqlx::{Connection, Row};
use testcontainers::runners::AsyncRunner;

fn select(value: Value) -> (String, Values) {
	Query::select()
		.expr_as(value, "cached_type")
		.build(PostgresQueryBuilder)
}

#[rstest]
#[tokio::test]
async fn generated_pool_and_transaction_calls_bypass_incompatible_raw_cache_entries() {
	// Arrange: a single connection makes cache reuse deterministic.
	let container = testcontainers_modules::postgres::Postgres::default()
		.start()
		.await
		.unwrap();
	let url = format!(
		"postgres://postgres:postgres@{}:{}/postgres",
		container.get_host().await.unwrap(),
		container.get_host_port_ipv4(5432).await.unwrap()
	);
	let connection = DatabaseConnection::connect_postgres_with_pool_size(&url, Some(1))
		.await
		.unwrap();
	let sql = select(Value::Int(Some(7))).0;
	let seed = || async {
		// Reinhardt raw calls also bypass the cache; seed it through SQLx directly.
		let pool = connection.into_postgres().unwrap();
		let mut dedicated = pool.acquire().await.unwrap();
		dedicated.clear_cached_statements().await.unwrap();
		let row = sqlx::query(&sql)
			.bind(7_i32)
			.fetch_one(&mut *dedicated)
			.await
			.unwrap();
		assert_eq!(row.get::<i32, _>("cached_type"), 7);
		assert_eq!(dedicated.cached_statements_size(), 1);
	};
	// Act / Assert: each native pool operation clears a seeded INT4 signature.
	seed().await;
	let row = connection
		.fetch_one_generated(select(Value::BigInt(Some(8))), None)
		.await
		.unwrap();
	assert_eq!(row.get::<i64>("cached_type").unwrap(), 8);
	seed().await;
	let rows = connection
		.fetch_all_generated(select(Value::BigInt(Some(9))), None)
		.await
		.unwrap();
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0].get::<i64>("cached_type").unwrap(), 9);
	seed().await;
	let row = connection
		.fetch_optional_generated(select(Value::BigInt(Some(10))), None)
		.await
		.unwrap()
		.unwrap();
	assert_eq!(row.get::<i64>("cached_type").unwrap(), 10);
	seed().await;
	assert_eq!(
		connection
			.execute_generated(select(Value::BigInt(Some(11))), None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	seed().await;
	{
		let mut rows = connection
			.fetch_stream_generated(select(Value::BigInt(Some(12))), 1, None)
			.unwrap();
		assert_eq!(
			rows.next()
				.await
				.unwrap()
				.unwrap()
				.get::<i64>("cached_type")
				.unwrap(),
			12
		);
		// Leaving the scope drops a partial stream and releases its pool guard.
	}
	assert_eq!(
		connection
			.fetch_one_generated(select(Value::Int(Some(13))), None)
			.await
			.unwrap()
			.get::<i64>("cached_type")
			.unwrap(),
		13
	);
	{
		let pool = connection.into_postgres().unwrap();
		let dedicated = pool.acquire().await.unwrap();
		assert_eq!(dedicated.cached_statements_size(), 0);
	}
	// Dedicated transactions keep their connection while isolating each signature.
	seed().await;
	let mut transaction = connection.begin_write().await.unwrap();
	assert_eq!(
		transaction
			.fetch_one_generated(select(Value::BigInt(Some(14))), None)
			.await
			.unwrap()
			.get::<i64>("cached_type")
			.unwrap(),
		14
	);
	transaction
		.fetch_one(&sql, vec![QueryValue::Int32(7)])
		.await
		.unwrap();
	let rows = transaction
		.fetch_all_generated(select(Value::BigInt(Some(15))), None)
		.await
		.unwrap();
	assert_eq!(rows.len(), 1);
	assert_eq!(rows[0].get::<i64>("cached_type").unwrap(), 15);
	transaction
		.fetch_one(&sql, vec![QueryValue::Int32(7)])
		.await
		.unwrap();
	assert_eq!(
		transaction
			.fetch_optional_generated(select(Value::BigInt(Some(16))), None)
			.await
			.unwrap()
			.unwrap()
			.get::<i64>("cached_type")
			.unwrap(),
		16
	);
	transaction
		.fetch_one(&sql, vec![QueryValue::Int32(7)])
		.await
		.unwrap();
	assert_eq!(
		transaction
			.execute_generated(select(Value::BigInt(Some(17))), None)
			.await
			.unwrap()
			.rows_affected,
		1
	);
	transaction
		.fetch_one(&sql, vec![QueryValue::Int32(7)])
		.await
		.unwrap();
	{
		let mut rows = transaction
			.fetch_stream_generated(select(Value::BigInt(Some(18))), 1, None)
			.unwrap();
		assert_eq!(
			rows.next()
				.await
				.unwrap()
				.unwrap()
				.get::<i64>("cached_type")
				.unwrap(),
			18
		);
	}
	assert_eq!(
		transaction
			.fetch_one_generated(select(Value::Int(Some(19))), None)
			.await
			.unwrap()
			.get::<i64>("cached_type")
			.unwrap(),
		19
	);
	transaction.commit().await.unwrap();
	let pool = connection.into_postgres().unwrap();
	let dedicated = pool.acquire().await.unwrap();
	assert_eq!(dedicated.cached_statements_size(), 0);
}
