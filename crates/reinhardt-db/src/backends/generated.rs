//! Owned generated arguments and checked compatibility for custom backends.

use super::error::{DatabaseError, DatabaseErrorKind, Result};
use super::types::{DatabaseType, QueryValue};
use reinhardt_query::{Value, Values};

fn compatibility_error(
	backend: DatabaseType,
	index: usize,
	kind: &str,
	reason: &str,
) -> DatabaseError {
	let backend = match backend {
		DatabaseType::Postgres => "postgres/custom",
		DatabaseType::Mysql => "mysql/custom",
		DatabaseType::Sqlite => "sqlite/custom",
	};
	DatabaseError::new(
		DatabaseErrorKind::Type,
		format!("cannot encode {kind} argument {index} for {backend}: {reason}"),
	)
}

/// Bridge generated values into an existing custom backend's raw parameter API.
/// Native SQLx implementations bypass this bridge and preserve their full codecs.
pub(crate) fn compatibility_arguments(
	built: (String, Values),
	backend: DatabaseType,
) -> Result<(String, Vec<QueryValue>)> {
	let (sql, values) = built;
	let mut arguments = Vec::with_capacity(values.0.len());
	for (offset, value) in values.into_iter().enumerate() {
		let index = offset + 1;
		let unsupported = |kind| {
			compatibility_error(
				backend,
				index,
				kind,
				"custom backend must implement generated execution for this value type",
			)
		};
		let argument = match value {
			Value::Json(value) => QueryValue::Json(value),
			#[cfg(feature = "pgvector")]
			Value::Vector(value) => QueryValue::Vector(value.map(|value| *value)),
			Value::Array(..) => return Err(unsupported("Array").into()),
			Value::ChronoDate(..) => return Err(unsupported("ChronoDate").into()),
			Value::ChronoTime(..) => return Err(unsupported("ChronoTime").into()),
			Value::Decimal(..) => return Err(unsupported("Decimal").into()),
			Value::BigDecimal(..) => return Err(unsupported("BigDecimal").into()),
			value if value.is_null() => QueryValue::Null,
			Value::Bool(Some(value)) => QueryValue::Bool(value),
			Value::TinyInt(Some(value)) => QueryValue::Int(i64::from(value)),
			Value::SmallInt(Some(value)) => QueryValue::Int(i64::from(value)),
			Value::Int(Some(value)) => QueryValue::Int32(value),
			Value::BigInt(Some(value)) => QueryValue::Int(value),
			Value::TinyUnsigned(Some(value)) => QueryValue::Int(i64::from(value)),
			Value::SmallUnsigned(Some(value)) => QueryValue::Int(i64::from(value)),
			Value::Unsigned(Some(value)) => QueryValue::Int(i64::from(value)),
			Value::BigUnsigned(Some(value)) => {
				QueryValue::Int(i64::try_from(value).map_err(|_| {
					compatibility_error(
						backend,
						index,
						"BigUnsigned",
						"unsigned integer exceeds signed 64-bit range",
					)
				})?)
			}
			Value::Float(Some(value)) => QueryValue::Float(f64::from(value)),
			Value::Double(Some(value)) => QueryValue::Float(value),
			Value::Char(Some(value)) => QueryValue::String(value.to_string()),
			Value::String(Some(value)) => QueryValue::String(*value),
			Value::Bytes(Some(value)) => QueryValue::Bytes(*value),
			Value::ChronoDateTimeUtc(Some(value)) => QueryValue::Timestamp(*value),
			Value::ChronoDateTimeLocal(Some(value)) => {
				QueryValue::Timestamp(value.with_timezone(&chrono::Utc))
			}
			Value::ChronoDateTimeWithTimeZone(Some(value)) => {
				QueryValue::Timestamp(value.with_timezone(&chrono::Utc))
			}
			Value::ChronoDateTime(Some(value)) => QueryValue::NaiveTimestamp(*value),
			Value::Uuid(Some(value)) => QueryValue::Uuid(*value),
			// NULL variants are handled by is_null() above. Keeping this catch-all
			// avoids feature-unified variants becoming a debug/string fallback.
			_ => return Err(unsupported("Value").into()),
		};
		arguments.push(argument);
	}
	Ok((sql, arguments))
}

pub(crate) fn binding_error(error: reinhardt_query_sqlx::BindError) -> DatabaseError {
	DatabaseError::new(DatabaseErrorKind::Type, error.to_string()).with_source(error)
}

/// Build an unnamed query after removing potentially incompatible named statements.
#[cfg(feature = "postgres")]
pub(crate) async fn uncached_postgres_query<'q>(
	connection: &mut sqlx::PgConnection,
	sql: &'q str,
	arguments: sqlx::postgres::PgArguments,
) -> std::result::Result<
	sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
	sqlx::Error,
> {
	use sqlx::Connection;
	if connection.cached_statements_size() > 0 {
		connection.clear_cached_statements().await?;
	}
	// Workaround: https://github.com/kent8192/reinhardt-web/issues/6533
	// SQLx 0.8 reuses SQL-only cache entries before checking parameter types or
	// persistence. Clear existing entries above and keep generated queries unnamed.
	// Remove this bypass after a validated driver keys or invalidates by argument type.
	// The replacement is sqlx::query_with(sql, arguments) with its default persistence.
	Ok(sqlx::query_with(sql, arguments).persistent(false))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::backends::backend::DatabaseBackend;
	use crate::backends::types::{QueryResult, Row, RowStream, TransactionExecutor};
	use futures::StreamExt;
	use rstest::rstest;
	use std::sync::{Arc, Mutex};

	type CapturedCalls = Arc<Mutex<Vec<(String, Vec<QueryValue>)>>>;

	struct CustomBackend {
		calls: CapturedCalls,
	}
	struct CustomTransaction(CustomBackend);

	#[async_trait::async_trait]
	impl TransactionExecutor for CustomTransaction {
		async fn execute(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
			self.0.execute(sql, params).await
		}
		async fn fetch_one(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
			self.0.fetch_one(sql, params).await
		}
		async fn fetch_all(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
			self.0.fetch_all(sql, params).await
		}
		async fn fetch_optional(
			&mut self,
			sql: &str,
			params: Vec<QueryValue>,
		) -> Result<Option<Row>> {
			self.0.fetch_optional(sql, params).await
		}
		fn fetch_stream<'a>(
			&'a mut self,
			sql: String,
			params: Vec<QueryValue>,
			chunk_size: usize,
		) -> Result<RowStream<'a>> {
			self.0.fetch_stream(sql, params, chunk_size)
		}
		async fn commit(self: Box<Self>) -> Result<()> {
			Ok(())
		}
		async fn rollback(self: Box<Self>) -> Result<()> {
			Ok(())
		}
	}

	#[async_trait::async_trait]
	impl DatabaseBackend for CustomBackend {
		fn database_type(&self) -> DatabaseType {
			DatabaseType::Postgres
		}
		fn placeholder(&self, index: usize) -> String {
			format!("${index}")
		}
		fn supports_returning(&self) -> bool {
			true
		}
		fn supports_on_conflict(&self) -> bool {
			true
		}
		async fn execute(&self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
			self.calls.lock().unwrap().push((sql.to_owned(), params));
			Ok(QueryResult {
				rows_affected: 1,
				last_insert_id: Some(7),
			})
		}
		async fn fetch_one(&self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
			self.calls.lock().unwrap().push((sql.to_owned(), params));
			let mut row = Row::new();
			row.insert("id".to_owned(), QueryValue::Int(7));
			Ok(row)
		}
		async fn fetch_all(&self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
			Ok(vec![self.fetch_one(sql, params).await?])
		}
		async fn fetch_optional(&self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
			Ok(Some(self.fetch_one(sql, params).await?))
		}
		async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
			Ok(Box::new(CustomTransaction(CustomBackend {
				calls: self.calls.clone(),
			})))
		}
		fn fetch_stream<'a>(
			&'a self,
			sql: String,
			params: Vec<QueryValue>,
			_chunk_size: usize,
		) -> Result<RowStream<'a>> {
			self.calls.lock().unwrap().push((sql, params));
			let mut row = Row::new();
			row.insert("id".to_owned(), QueryValue::Int(7));
			Ok(Box::pin(futures::stream::once(async move { Ok(row) })))
		}
		fn as_any(&self) -> &dyn std::any::Any {
			self
		}
	}

	#[rstest]
	#[tokio::test]
	async fn existing_custom_backend_works_without_implementing_new_generated_methods() {
		// Arrange: this provider implements only the pre-existing required raw methods.
		let calls: CapturedCalls = Arc::new(Mutex::new(Vec::new()));
		let connection = crate::backends::DatabaseConnection::new(Arc::new(CustomBackend {
			calls: calls.clone(),
		}));
		let sql = "SELECT $1 AS id, $2 AS name";
		let built = || {
			(
				sql.to_owned(),
				Values(vec![
					Value::Int(Some(7)),
					Value::String(Some(Box::new("quoted' payload".to_owned()))),
				]),
			)
		};
		// Act
		let result = connection.execute_generated(built(), None).await.unwrap();
		let one = connection.fetch_one_generated(built(), None).await.unwrap();
		let all = connection.fetch_all_generated(built(), None).await.unwrap();
		let optional = connection
			.fetch_optional_generated(built(), None)
			.await
			.unwrap();
		let failed = connection
			.execute_generated(
				(
					"private SQL".to_owned(),
					Values(vec![Value::BigUnsigned(Some(u64::MAX))]),
				),
				None,
			)
			.await;
		// Assert: metadata, row shapes and exact pairs reach the original raw seam.
		assert_eq!(result.rows_affected, 1);
		assert_eq!(result.last_insert_id, Some(7));
		assert_eq!(one.get::<i64>("id").unwrap(), 7);
		assert_eq!(all, [one.clone()]);
		assert_eq!(optional, Some(one));
		assert_eq!(
			failed.unwrap_err().database_kind(),
			Some(DatabaseErrorKind::Type)
		);
		assert_eq!(
			*calls.lock().unwrap(),
			vec![
				(
					sql.to_owned(),
					vec![
						QueryValue::Int32(7),
						QueryValue::String("quoted' payload".to_owned())
					]
				);
				4
			]
		);
	}

	#[rstest]
	#[tokio::test]
	async fn existing_custom_transaction_and_stream_defaults_preserve_raw_parameters() {
		// Arrange: no generated methods are implemented by this legacy provider.
		let calls: CapturedCalls = Arc::new(Mutex::new(Vec::new()));
		let connection = crate::backends::DatabaseConnection::new(Arc::new(CustomBackend {
			calls: calls.clone(),
		}));
		let sql = "SELECT $1 AS id";
		let built = || (sql.to_owned(), Values(vec![Value::Int(Some(7))]));
		let mut transaction = connection.begin().await.unwrap();
		// Act
		let result = transaction.execute_generated(built(), None).await.unwrap();
		let row = transaction
			.fetch_one_generated(built(), None)
			.await
			.unwrap();
		assert_eq!(
			transaction
				.fetch_all_generated(built(), None)
				.await
				.unwrap(),
			vec![row.clone()]
		);
		assert_eq!(
			transaction
				.fetch_optional_generated(built(), None)
				.await
				.unwrap(),
			Some(row.clone())
		);
		{
			let mut stream = transaction
				.fetch_stream_generated(built(), 1, None)
				.unwrap();
			assert_eq!(stream.next().await.unwrap().unwrap(), row);
			assert!(stream.next().await.is_none());
		}
		let error = transaction
			.execute_generated(
				(
					"private SQL".to_owned(),
					Values(vec![Value::BigUnsigned(Some(u64::MAX))]),
				),
				None,
			)
			.await
			.unwrap_err();
		transaction.rollback().await.unwrap();
		{
			let mut stream = connection.fetch_stream_generated(built(), 1, None).unwrap();
			assert_eq!(
				stream
					.next()
					.await
					.unwrap()
					.unwrap()
					.get::<i64>("id")
					.unwrap(),
				7
			);
		}
		// Assert: failed encoding never reaches the raw provider.
		assert_eq!(result.rows_affected, 1);
		assert_eq!(result.last_insert_id, Some(7));
		assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Type));
		assert_eq!(
			*calls.lock().unwrap(),
			vec![(sql.to_owned(), vec![QueryValue::Int32(7)]); 6]
		);
	}

	#[rstest]
	fn custom_bridge_preserves_pair_order_and_rejects_unsigned_overflow() {
		// Arrange
		let sql = "SELECT ? AS amount, ? AS name";
		let built = (
			sql.to_owned(),
			Values(vec![
				Value::Int(Some(7)),
				Value::String(Some(Box::new("quoted' payload".to_owned()))),
			]),
		);
		// Act
		let (actual, values) = compatibility_arguments(built, DatabaseType::Sqlite).unwrap();
		// Assert
		assert_eq!(actual, sql);
		assert_eq!(
			values,
			[
				QueryValue::Int32(7),
				QueryValue::String("quoted' payload".to_owned())
			]
		);
		let error = compatibility_arguments(
			(
				"private SQL".to_owned(),
				Values(vec![
					Value::String(Some(Box::new("private payload".to_owned()))),
					Value::BigUnsigned(Some(u64::MAX)),
				]),
			),
			DatabaseType::Mysql,
		)
		.unwrap_err();
		let message = error.to_string();
		assert!(message.contains("BigUnsigned argument 2 for mysql/custom"));
		assert!(!message.contains("private"));
		assert!(!message.contains(&u64::MAX.to_string()));
	}

	#[rstest]
	#[case(
		Value::Decimal(Some(Box::new(rust_decimal::Decimal::new(12345, 4)))),
		"Decimal"
	)]
	#[case(Value::Array(reinhardt_query::ArrayType::Int, Some(Box::new(vec![Value::Int(Some(1))]))), "Array")]
	fn custom_bridge_rejects_unrepresentable_types_without_stringifying(
		#[case] value: Value,
		#[case] kind: &str,
	) {
		// Act
		let error = compatibility_arguments(
			("private SQL".to_owned(), Values(vec![value])),
			DatabaseType::Postgres,
		)
		.unwrap_err();
		// Assert
		assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Type));
		let message = error.to_string();
		assert!(message.contains(&format!("{kind} argument 1 for postgres/custom")));
		assert!(!message.contains("private SQL"));
		assert!(!message.contains("12345"));
	}
}

#[cfg(all(test, feature = "postgres"))]
mod postgres_cache_tests {
	use super::uncached_postgres_query;
	use futures::StreamExt;
	use rstest::{fixture, rstest};
	use sqlx::{Arguments, Connection, Row};
	use testcontainers::{ContainerAsync, runners::AsyncRunner};
	use testcontainers_modules::postgres::Postgres;

	struct PostgresFixture {
		// Drop the connection before the container that owns its database.
		connection: sqlx::PgConnection,
		_container: ContainerAsync<Postgres>,
	}

	#[fixture]
	async fn postgres_fixture() -> PostgresFixture {
		let container = Postgres::default().start().await.unwrap();
		let url = format!(
			"postgres://postgres:postgres@{}:{}/postgres",
			container.get_host().await.unwrap(),
			container.get_host_port_ipv4(5432).await.unwrap()
		);
		PostgresFixture {
			connection: sqlx::PgConnection::connect(&url).await.unwrap(),
			_container: container,
		}
	}

	#[rstest]
	#[case::int4_to_int8(true, 1_i64 << 40, "bigint")]
	#[case::int8_to_int4(false, 8, "integer")]
	#[tokio::test]
	async fn uncached_query_preserves_native_signature(
		#[future] postgres_fixture: PostgresFixture,
		#[case] wide: bool,
		#[case] expected: i64,
		#[case] native_type: &str,
	) {
		// Arrange: the same SQL already has the opposite integer signature cached.
		let mut fixture = postgres_fixture.await;
		let connection = &mut fixture.connection;
		let sql = "SELECT $1::BIGINT AS cached_width, pg_typeof($1)::TEXT AS native_type";
		let seed = if wide {
			sqlx::query(sql).bind(7_i32)
		} else {
			sqlx::query(sql).bind(7_i64)
		};
		let row = seed.fetch_one(&mut *connection).await.unwrap();
		assert_eq!(row.get::<i64, _>("cached_width"), 7);
		assert_eq!(
			row.get::<String, _>("native_type"),
			if wide { "integer" } else { "bigint" }
		);
		assert!(connection.cached_statements_size() > 0);
		if wide {
			let error = sqlx::query(sql)
				.bind(expected)
				.persistent(false)
				.fetch_one(&mut *connection)
				.await
				.unwrap_err();
			assert_eq!(
				error.as_database_error().unwrap().code().as_deref(),
				Some("22P03")
			);
		}
		let mut arguments = sqlx::postgres::PgArguments::default();
		if wide {
			arguments.add(expected).unwrap();
		} else {
			arguments.add(i32::try_from(expected).unwrap()).unwrap();
		}

		// Act: clear the incompatible statement on the same dedicated connection.
		let row = uncached_postgres_query(connection, sql, arguments)
			.await
			.unwrap()
			.fetch_one(&mut *connection)
			.await
			.unwrap();

		// Assert: retain the native width and full value without caching the query.
		assert_eq!(row.get::<i64, _>("cached_width"), expected);
		assert_eq!(row.get::<String, _>("native_type"), native_type);
		assert_eq!(connection.cached_statements_size(), 0);
		let row = sqlx::query(sql)
			.bind(9_i32)
			.fetch_one(&mut *connection)
			.await
			.unwrap();
		assert_eq!(row.get::<i64, _>("cached_width"), 9);
		assert_eq!(row.get::<String, _>("native_type"), "integer");
		assert!(connection.cached_statements_size() > 0);
	}

	#[rstest]
	#[tokio::test]
	async fn dropping_partial_unnamed_stream_allows_cached_connection_reuse(
		#[future] postgres_fixture: PostgresFixture,
	) {
		// Arrange: three rows ensure dropping after the first row leaves unread rows.
		let mut fixture = postgres_fixture.await;
		let connection = &mut fixture.connection;
		let sql = "SELECT $1::BIGINT AS cached_width FROM generate_series(1, 3)";
		let seeded = sqlx::query(sql)
			.bind(7_i32)
			.fetch_all(&mut *connection)
			.await
			.unwrap();
		assert_eq!(seeded.len(), 3);
		assert!(connection.cached_statements_size() > 0);
		let mut arguments = sqlx::postgres::PgArguments::default();
		arguments.add(1_i64 << 40).unwrap();

		// Act: drop a partially consumed stream while retaining its connection guard.
		{
			let mut rows = uncached_postgres_query(connection, sql, arguments)
				.await
				.unwrap()
				.fetch(&mut *connection);
			let row = rows.next().await.unwrap().unwrap();
			assert_eq!(row.get::<i64, _>("cached_width"), 1_i64 << 40);
		}

		// Assert: the original INT4 query can drain pending messages and cache again.
		assert_eq!(connection.cached_statements_size(), 0);
		let rows = sqlx::query(sql)
			.bind(8_i32)
			.fetch_all(&mut *connection)
			.await
			.unwrap();
		assert_eq!(rows.len(), 3);
		for row in rows {
			assert_eq!(row.get::<i64, _>("cached_width"), 8);
		}
		assert!(connection.cached_statements_size() > 0);
	}

	#[rstest]
	#[case::commit(true)]
	#[case::rollback(false)]
	#[tokio::test]
	async fn uncached_query_keeps_transaction_changes_on_its_connection(
		#[future] postgres_fixture: PostgresFixture,
		#[case] commit: bool,
	) {
		// Arrange: seed an INT4 INSERT inside a transaction on a connection-local table.
		let mut fixture = postgres_fixture.await;
		let connection = &mut fixture.connection;
		sqlx::query("CREATE TEMP TABLE cached_rows (value BIGINT)")
			.execute(&mut *connection)
			.await
			.unwrap();
		let mut transaction = connection.begin().await.unwrap();
		let sql = "INSERT INTO cached_rows (value) VALUES ($1)";
		assert_eq!(
			sqlx::query(sql)
				.bind(7_i32)
				.execute(&mut *transaction)
				.await
				.unwrap()
				.rows_affected(),
			1
		);
		assert!(transaction.cached_statements_size() > 0);
		let mut arguments = sqlx::postgres::PgArguments::default();
		arguments.add(1_i64 << 40).unwrap();

		// Act: execute the INT8 INSERT without replacing or ending the transaction.
		assert_eq!(
			uncached_postgres_query(&mut transaction, sql, arguments)
				.await
				.unwrap()
				.execute(&mut *transaction)
				.await
				.unwrap()
				.rows_affected(),
			1
		);
		let select = "SELECT value FROM cached_rows ORDER BY value";
		let values: Vec<i64> = sqlx::query_scalar(select)
			.fetch_all(&mut *transaction)
			.await
			.unwrap();
		assert_eq!(values, [7, 1_i64 << 40]);
		if commit {
			transaction.commit().await.unwrap();
		} else {
			transaction.rollback().await.unwrap();
		}

		// Assert: both INSERTs share the enclosing transaction's commit/rollback result.
		let values: Vec<i64> = sqlx::query_scalar(select)
			.fetch_all(&mut *connection)
			.await
			.unwrap();
		if commit {
			assert_eq!(values, [7, 1_i64 << 40]);
		} else {
			assert!(values.is_empty());
		}
	}
}
