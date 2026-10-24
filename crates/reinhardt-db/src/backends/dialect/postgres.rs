//! PostgreSQL dialect implementation

use async_trait::async_trait;
use futures::StreamExt;
use sqlx::{Column, PgPool, Postgres, Transaction, TypeInfo, postgres::PgRow};
use std::sync::Arc;
use uuid::Uuid;

use crate::backends::{
	backend::DatabaseBackend,
	error::{
		DatabaseError, DatabaseErrorKind, PgvectorOperationKind, Result, map_sqlx_error,
		map_sqlx_error_with_pgvector_context,
	},
	types::{
		DatabaseType, IsolationLevel, QueryResult, QueryValue, Row, RowStream, Savepoint,
		TransactionExecutor, array_query_value,
	},
};
#[cfg(feature = "pgvector")]
use crate::orm::vector::PgVectorValue;

#[cfg(not(feature = "pgvector"))]
fn vector_support_disabled_error() -> DatabaseError {
	DatabaseError::new(
		DatabaseErrorKind::Type,
		"pgvector support is not enabled for PostgreSQL vector values",
	)
}

// A NULL has no native Rust payload type. OID 0 leaves its PostgreSQL type
// unspecified in Parse so the server can infer it from SQL context (#6631).
struct PgNull;

impl sqlx::Type<Postgres> for PgNull {
	fn type_info() -> sqlx::postgres::PgTypeInfo {
		sqlx::postgres::PgTypeInfo::with_oid(sqlx::postgres::types::Oid(0))
	}
}

impl<'q> sqlx::Encode<'q, Postgres> for PgNull {
	fn encode_by_ref(
		&self,
		_buf: &mut sqlx::postgres::PgArgumentBuffer,
	) -> std::result::Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
		Ok(sqlx::encode::IsNull::Yes)
	}

	fn size_hint(&self) -> usize {
		0
	}
}

/// PostgreSQL database backend
pub struct PostgresBackend {
	pool: Arc<PgPool>,
}

impl PostgresBackend {
	/// Creates a new instance.
	pub fn new(pool: PgPool) -> Self {
		Self {
			pool: Arc::new(pool),
		}
	}

	/// Performs the pool operation.
	pub fn pool(&self) -> &PgPool {
		&self.pool
	}

	fn bind_value<'q>(
		query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
		value: &'q QueryValue,
	) -> Result<sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>> {
		Ok(match value {
			QueryValue::Null => query.bind(PgNull),
			QueryValue::Bool(b) => query.bind(b),
			QueryValue::Int32(i) => query.bind(i),
			QueryValue::Int(i) => query.bind(i),
			QueryValue::Uint(i) => query.bind(crate::backends::types::checked_unsigned_integer(
				*i,
				"PostgreSQL",
			)?),
			QueryValue::Float(f) => query.bind(f),
			QueryValue::String(s) => query.bind(s),
			QueryValue::Bytes(b) => query.bind(b),
			QueryValue::Timestamp(dt) => query.bind(dt),
			QueryValue::NaiveTimestamp(dt) => query.bind(dt),
			QueryValue::Uuid(u) => query.bind(u),
			QueryValue::Json(value) => query.bind(value.as_deref().cloned().map(sqlx::types::Json)),
			#[cfg(feature = "pgvector")]
			QueryValue::Vector(values) => query.bind(
				values
					.as_ref()
					.map(|values| PgVectorValue::new(values.clone())),
			),
			QueryValue::StringArray(values) => query.bind(values),
			QueryValue::IntArray(values) => query.bind(values),
			QueryValue::BigIntArray(values) => query.bind(values),
			QueryValue::BoolArray(values) => query.bind(values),
			QueryValue::FloatArray(values) => query.bind(values),
			QueryValue::DoubleArray(values) => query.bind(values),
			QueryValue::UuidArray(values) => query.bind(values),
			QueryValue::NullableStringArray(values) => query.bind(values),
			QueryValue::NullableIntArray(values) => query.bind(values),
			QueryValue::NullableBigIntArray(values) => query.bind(values),
			QueryValue::NullableBoolArray(values) => query.bind(values),
			QueryValue::NullableFloatArray(values) => query.bind(values),
			QueryValue::NullableDoubleArray(values) => query.bind(values),
			QueryValue::NullableUuidArray(values) => query.bind(values),
			QueryValue::Now => {
				// PostgreSQL uses NOW() function, which should be part of SQL string
				// For binding, we use current UTC time
				query.bind(chrono::Utc::now())
			}
		})
	}

	fn convert_row(pg_row: PgRow) -> Result<Row> {
		Self::convert_row_internal(pg_row)
	}

	pub(crate) async fn execute_with_context(
		&self,
		sql: &str,
		params: Vec<QueryValue>,
		context: Option<PgvectorOperationKind>,
	) -> Result<QueryResult> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param)?;
		}
		let mut connection = self.pool.acquire().await.map_err(map_sqlx_error)?;
		let query = uncached_postgres_query(&mut connection, query)
			.await
			.map_err(map_sqlx_error)?;
		let result = query
			.execute(&mut *connection)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		Ok(QueryResult {
			rows_affected: result.rows_affected(),
			last_insert_id: None,
		})
	}

	pub(crate) async fn fetch_one_with_context(
		&self,
		sql: &str,
		params: Vec<QueryValue>,
		context: Option<PgvectorOperationKind>,
	) -> Result<Row> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param)?;
		}
		let mut connection = self.pool.acquire().await.map_err(map_sqlx_error)?;
		let query = uncached_postgres_query(&mut connection, query)
			.await
			.map_err(map_sqlx_error)?;
		let row = query
			.fetch_one(&mut *connection)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		Self::convert_row(row)
	}

	pub(crate) async fn fetch_all_with_context(
		&self,
		sql: &str,
		params: Vec<QueryValue>,
		context: Option<PgvectorOperationKind>,
	) -> Result<Vec<Row>> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param)?;
		}
		let mut connection = self.pool.acquire().await.map_err(map_sqlx_error)?;
		let query = uncached_postgres_query(&mut connection, query)
			.await
			.map_err(map_sqlx_error)?;
		let rows = query
			.fetch_all(&mut *connection)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		rows.into_iter().map(Self::convert_row).collect()
	}

	pub(crate) async fn fetch_optional_with_context(
		&self,
		sql: &str,
		params: Vec<QueryValue>,
		context: Option<PgvectorOperationKind>,
	) -> Result<Option<Row>> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param)?;
		}
		let mut connection = self.pool.acquire().await.map_err(map_sqlx_error)?;
		let query = uncached_postgres_query(&mut connection, query)
			.await
			.map_err(map_sqlx_error)?;
		let row = query
			.fetch_optional(&mut *connection)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		row.map(Self::convert_row).transpose()
	}
}

async fn uncached_postgres_query<'q>(
	connection: &mut sqlx::PgConnection,
	query: sqlx::query::Query<'q, Postgres, sqlx::postgres::PgArguments>,
) -> std::result::Result<sqlx::query::Query<'q, Postgres, sqlx::postgres::PgArguments>, sqlx::Error>
{
	use sqlx::Connection;

	// Workaround: https://github.com/kent8192/reinhardt-web/issues/6533.
	// SQLx 0.8.6 looks up statements by SQL before checking argument types or
	// persistence, so disabling persistence alone can reuse an incompatible entry.
	// Clear the cache on the executing connection while retaining its RAII guard.
	// Remove this bypass when the selected driver passes changing native signatures
	// with default persistence; the ideal implementation returns `query` unchanged.
	if connection.cached_statements_size() > 0 {
		connection.clear_cached_statements().await?;
	}
	Ok(query.persistent(false))
}

#[async_trait]
impl DatabaseBackend for PostgresBackend {
	fn fetch_stream_generated<'a>(
		&'a self,
		built: (String, reinhardt_query::Values),
		chunk_size: usize,
		context: Option<crate::backends::error::PgvectorOperationKind>,
	) -> Result<RowStream<'a>> {
		if chunk_size == 0 {
			return Err(DatabaseError::new(
				DatabaseErrorKind::Configuration,
				"Row stream chunk_size must be greater than zero",
			)
			.into());
		}
		let pool = Arc::clone(&self.pool);
		let (sql, values) = built;
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		Ok(Box::pin(async_stream::stream! {
			let mut connection = match pool.acquire().await {
				Ok(connection) => connection,
				Err(error) => {
					yield Err(map_sqlx_error_with_pgvector_context(error, context));
					return;
				}
			};
			let query = match crate::backends::generated::uncached_postgres_query(
				&mut connection, &sql, arguments,
			).await {
				Ok(query) => query,
				Err(error) => {
					yield Err(map_sqlx_error_with_pgvector_context(error, context));
					return;
				}
			};
			let rows = query.fetch(&mut *connection);
			futures::pin_mut!(rows);
			let rows = rows.ready_chunks(chunk_size);
			futures::pin_mut!(rows);
			while let Some(chunk) = rows.next().await {
				for row in chunk {
					yield row
						.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))
						.and_then(Self::convert_row);
				}
			}
		}))
	}

	async fn execute_generated(
		&self,
		built: (String, reinhardt_query::Values),
		context: Option<crate::backends::error::PgvectorOperationKind>,
	) -> Result<QueryResult> {
		let (sql, values) = built;
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let mut connection = self
			.pool
			.acquire()
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		let query =
			crate::backends::generated::uncached_postgres_query(&mut connection, &sql, arguments)
				.await
				.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		let result = query
			.execute(&mut *connection)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		Ok(QueryResult {
			rows_affected: result.rows_affected(),
			last_insert_id: None,
		})
	}

	async fn fetch_one_generated(
		&self,
		built: (String, reinhardt_query::Values),
		context: Option<crate::backends::error::PgvectorOperationKind>,
	) -> Result<Row> {
		let (sql, values) = built;
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let mut connection = self
			.pool
			.acquire()
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		let query =
			crate::backends::generated::uncached_postgres_query(&mut connection, &sql, arguments)
				.await
				.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		let result = query
			.fetch_one(&mut *connection)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		Self::convert_row(result)
	}

	async fn fetch_all_generated(
		&self,
		built: (String, reinhardt_query::Values),
		context: Option<crate::backends::error::PgvectorOperationKind>,
	) -> Result<Vec<Row>> {
		let (sql, values) = built;
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let mut connection = self
			.pool
			.acquire()
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		let query =
			crate::backends::generated::uncached_postgres_query(&mut connection, &sql, arguments)
				.await
				.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		let result = query
			.fetch_all(&mut *connection)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		result.into_iter().map(Self::convert_row).collect()
	}

	async fn fetch_optional_generated(
		&self,
		built: (String, reinhardt_query::Values),
		context: Option<crate::backends::error::PgvectorOperationKind>,
	) -> Result<Option<Row>> {
		let (sql, values) = built;
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let mut connection = self
			.pool
			.acquire()
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		let query =
			crate::backends::generated::uncached_postgres_query(&mut connection, &sql, arguments)
				.await
				.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		let result = query
			.fetch_optional(&mut *connection)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		result.map(Self::convert_row).transpose()
	}

	async fn __execute_generated(
		&self,
		sql: &str,
		values: reinhardt_query::Values,
	) -> Result<QueryResult> {
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let mut connection = self.pool.acquire().await.map_err(map_sqlx_error)?;
		let query = uncached_postgres_query(&mut connection, sqlx::query_with(sql, arguments))
			.await
			.map_err(map_sqlx_error)?;
		let result = query
			.execute(&mut *connection)
			.await
			.map_err(map_sqlx_error)?;
		Ok(QueryResult {
			rows_affected: result.rows_affected(),
			last_insert_id: None,
		})
	}

	async fn __fetch_one_generated(
		&self,
		sql: &str,
		values: reinhardt_query::Values,
	) -> Result<Row> {
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let mut connection = self.pool.acquire().await.map_err(map_sqlx_error)?;
		let query = uncached_postgres_query(&mut connection, sqlx::query_with(sql, arguments))
			.await
			.map_err(map_sqlx_error)?;
		let row = query
			.fetch_one(&mut *connection)
			.await
			.map_err(map_sqlx_error)?;
		Self::convert_row(row)
	}

	async fn __fetch_all_generated(
		&self,
		sql: &str,
		values: reinhardt_query::Values,
	) -> Result<Vec<Row>> {
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let mut connection = self.pool.acquire().await.map_err(map_sqlx_error)?;
		let query = uncached_postgres_query(&mut connection, sqlx::query_with(sql, arguments))
			.await
			.map_err(map_sqlx_error)?;
		let rows = query
			.fetch_all(&mut *connection)
			.await
			.map_err(map_sqlx_error)?;
		rows.into_iter().map(Self::convert_row).collect()
	}
	fn database_type(&self) -> DatabaseType {
		DatabaseType::Postgres
	}

	fn supports_pgvector_error_hints(&self) -> bool {
		true
	}

	fn supports_row_streaming(&self) -> bool {
		true
	}

	fn placeholder(&self, index: usize) -> String {
		format!("${}", index)
	}

	fn supports_returning(&self) -> bool {
		true
	}

	fn supports_on_conflict(&self) -> bool {
		true
	}

	async fn execute(&self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
		self.execute_with_context(sql, params, None).await
	}

	async fn execute_with_context(
		&self,
		sql: &str,
		params: Vec<QueryValue>,
		context: Option<PgvectorOperationKind>,
	) -> Result<QueryResult> {
		PostgresBackend::execute_with_context(self, sql, params, context).await
	}

	async fn fetch_one(&self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
		self.fetch_one_with_context(sql, params, None).await
	}

	async fn fetch_one_with_context(
		&self,
		sql: &str,
		params: Vec<QueryValue>,
		context: Option<PgvectorOperationKind>,
	) -> Result<Row> {
		PostgresBackend::fetch_one_with_context(self, sql, params, context).await
	}

	async fn fetch_all(&self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
		self.fetch_all_with_context(sql, params, None).await
	}

	async fn fetch_all_with_context(
		&self,
		sql: &str,
		params: Vec<QueryValue>,
		context: Option<PgvectorOperationKind>,
	) -> Result<Vec<Row>> {
		PostgresBackend::fetch_all_with_context(self, sql, params, context).await
	}

	fn fetch_stream<'a>(
		&'a self,
		sql: String,
		params: Vec<QueryValue>,
		chunk_size: usize,
	) -> Result<RowStream<'a>> {
		self.fetch_stream_with_context(sql, params, chunk_size, None)
	}

	fn fetch_stream_with_context<'a>(
		&'a self,
		sql: String,
		params: Vec<QueryValue>,
		chunk_size: usize,
		context: Option<PgvectorOperationKind>,
	) -> Result<RowStream<'a>> {
		if chunk_size == 0 {
			return Err(DatabaseError::new(
				DatabaseErrorKind::Configuration,
				"Row stream chunk_size must be greater than zero",
			)
			.into());
		}
		let pool = Arc::clone(&self.pool);
		Ok(Box::pin(async_stream::stream! {
			let mut query = sqlx::query(&sql);
			for param in &params {
				query = match Self::bind_value(query, param) {
					Ok(query) => query,
					Err(error) => {
						yield Err(error);
						return;
					}
				};
			}
			let mut connection = match pool.acquire().await {
				Ok(connection) => connection,
				Err(error) => { yield Err(map_sqlx_error(error).into()); return; }
			};
			let query = match uncached_postgres_query(&mut connection, query).await {
				Ok(query) => query,
				Err(error) => { yield Err(map_sqlx_error(error).into()); return; }
			};
			let rows = query.fetch(&mut *connection);
			futures::pin_mut!(rows);
			let rows = rows.ready_chunks(chunk_size);
			futures::pin_mut!(rows);
			while let Some(chunk) = rows.next().await {
				for row in chunk {
					yield row
						.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))
						.and_then(Self::convert_row);
				}
			}
		}))
	}

	async fn fetch_optional(&self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
		self.fetch_optional_with_context(sql, params, None).await
	}

	async fn fetch_optional_with_context(
		&self,
		sql: &str,
		params: Vec<QueryValue>,
		context: Option<PgvectorOperationKind>,
	) -> Result<Option<Row>> {
		PostgresBackend::fetch_optional_with_context(self, sql, params, context).await
	}

	async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
		let tx = self.pool.begin().await.map_err(map_sqlx_error)?;
		Ok(Box::new(PgTransactionExecutor::new(tx)))
	}

	async fn begin_write(&self) -> Result<Box<dyn TransactionExecutor>> {
		self.begin_with_isolation(IsolationLevel::ReadCommitted)
			.await
	}

	async fn begin_with_isolation(
		&self,
		isolation_level: IsolationLevel,
	) -> Result<Box<dyn TransactionExecutor>> {
		// PostgreSQL supports setting isolation level at transaction start
		let mut tx = self.pool.begin().await.map_err(map_sqlx_error)?;

		// Set the isolation level using PostgreSQL's SET TRANSACTION command
		let sql = format!(
			"SET TRANSACTION ISOLATION LEVEL {}",
			isolation_level.to_sql(DatabaseType::Postgres)
		);
		sqlx::query(&sql)
			.execute(&mut *tx)
			.await
			.map_err(map_sqlx_error)?;

		Ok(Box::new(PgTransactionExecutor::new(tx)))
	}

	fn as_any(&self) -> &dyn std::any::Any {
		self
	}
}

fn transaction_consumed_error() -> DatabaseError {
	DatabaseError::new(
		DatabaseErrorKind::Transaction,
		"Transaction already consumed",
	)
}

/// PostgreSQL transaction executor
///
/// This struct wraps a SQLx `Transaction` to ensure all queries
/// within a transaction run on the same physical database connection.
pub struct PgTransactionExecutor {
	tx: Option<Transaction<'static, Postgres>>,
}

impl PgTransactionExecutor {
	/// Creates a new instance.
	pub fn new(tx: Transaction<'static, Postgres>) -> Self {
		Self { tx: Some(tx) }
	}

	fn bind_value<'q>(
		query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
		value: &'q QueryValue,
	) -> Result<sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>> {
		Ok(match value {
			QueryValue::Null => query.bind(PgNull),
			QueryValue::Bool(b) => query.bind(b),
			QueryValue::Int32(i) => query.bind(i),
			QueryValue::Int(i) => query.bind(i),
			QueryValue::Uint(i) => query.bind(crate::backends::types::checked_unsigned_integer(
				*i,
				"PostgreSQL",
			)?),
			QueryValue::Float(f) => query.bind(f),
			QueryValue::String(s) => query.bind(s),
			QueryValue::Bytes(b) => query.bind(b),
			QueryValue::Timestamp(dt) => query.bind(dt),
			QueryValue::NaiveTimestamp(dt) => query.bind(dt),
			QueryValue::Uuid(u) => query.bind(u),
			QueryValue::Json(value) => query.bind(value.as_deref().cloned().map(sqlx::types::Json)),
			#[cfg(feature = "pgvector")]
			QueryValue::Vector(values) => query.bind(
				values
					.as_ref()
					.map(|values| PgVectorValue::new(values.clone())),
			),
			QueryValue::StringArray(values) => query.bind(values),
			QueryValue::IntArray(values) => query.bind(values),
			QueryValue::BigIntArray(values) => query.bind(values),
			QueryValue::BoolArray(values) => query.bind(values),
			QueryValue::FloatArray(values) => query.bind(values),
			QueryValue::DoubleArray(values) => query.bind(values),
			QueryValue::UuidArray(values) => query.bind(values),
			QueryValue::NullableStringArray(values) => query.bind(values),
			QueryValue::NullableIntArray(values) => query.bind(values),
			QueryValue::NullableBigIntArray(values) => query.bind(values),
			QueryValue::NullableBoolArray(values) => query.bind(values),
			QueryValue::NullableFloatArray(values) => query.bind(values),
			QueryValue::NullableDoubleArray(values) => query.bind(values),
			QueryValue::NullableUuidArray(values) => query.bind(values),
			QueryValue::Now => query.bind(chrono::Utc::now()),
		})
	}

	fn convert_row(pg_row: PgRow) -> Result<Row> {
		PostgresBackend::convert_row_internal(pg_row)
	}
}

impl PostgresBackend {
	/// Internal row conversion method shared between backend and transaction executor
	pub(crate) fn convert_row_internal(pg_row: PgRow) -> Result<Row> {
		use sqlx::{Row as SqlxRow, ValueRef};

		let mut row = Row::new();
		for column in pg_row.columns() {
			let column_name = column.name();
			let type_name = column.type_info().name().to_uppercase();
			if matches!(type_name.as_str(), "JSON" | "JSONB") {
				match pg_row.try_get::<Option<serde_json::Value>, _>(column_name) {
					Ok(Some(value)) => row.insert(
						column_name.to_string(),
						QueryValue::Json(Some(Box::new(value))),
					),
					Ok(None) => row.insert(column_name.to_string(), QueryValue::Null),
					Err(error) => return Err(map_sqlx_error(error).into()),
				};
				continue;
			}
			if type_name == "XML" {
				let value = pg_row.try_get_raw(column_name).map_err(map_sqlx_error)?;
				if value.is_null() {
					row.insert(column_name.to_string(), QueryValue::Null);
				} else {
					let value = std::str::from_utf8(value.as_bytes().map_err(|error| {
						DatabaseError::new(DatabaseErrorKind::Serialization, error.to_string())
					})?)
					.map_err(|error| {
						DatabaseError::new(DatabaseErrorKind::Serialization, error.to_string())
					})?;
					row.insert(
						column_name.to_string(),
						QueryValue::String(value.to_string()),
					);
				}
				continue;
			}
			if type_name == "VECTOR" {
				#[cfg(not(feature = "pgvector"))]
				return Err(vector_support_disabled_error().into());
				#[cfg(feature = "pgvector")]
				match pg_row
					.try_get::<Option<PgVectorValue>, _>(column_name)
					.map_err(map_sqlx_error)?
				{
					Some(value) => row.insert(
						column_name.to_string(),
						QueryValue::Vector(Some(value.into_vec())),
					),
					None => row.insert(column_name.to_string(), QueryValue::Null),
				};
				#[cfg(feature = "pgvector")]
				continue;
			}

			let array_value = match type_name.as_str() {
				// SQLx reports PostgreSQL bpchar arrays as CHAR[].
				"TEXT[]" | "VARCHAR[]" | "CHAR[]" | "BPCHAR[]" => Some(array_query_value(
					pg_row
						.try_get::<Option<Vec<Option<String>>>, _>(column_name)
						.map_err(map_sqlx_error)?,
					QueryValue::StringArray,
					QueryValue::NullableStringArray,
				)),
				"INT4[]" => Some(array_query_value(
					pg_row
						.try_get::<Option<Vec<Option<i32>>>, _>(column_name)
						.map_err(map_sqlx_error)?,
					QueryValue::IntArray,
					QueryValue::NullableIntArray,
				)),
				"INT8[]" => Some(array_query_value(
					pg_row
						.try_get::<Option<Vec<Option<i64>>>, _>(column_name)
						.map_err(map_sqlx_error)?,
					QueryValue::BigIntArray,
					QueryValue::NullableBigIntArray,
				)),
				"BOOL[]" => Some(array_query_value(
					pg_row
						.try_get::<Option<Vec<Option<bool>>>, _>(column_name)
						.map_err(map_sqlx_error)?,
					QueryValue::BoolArray,
					QueryValue::NullableBoolArray,
				)),
				"FLOAT4[]" => Some(array_query_value(
					pg_row
						.try_get::<Option<Vec<Option<f32>>>, _>(column_name)
						.map_err(map_sqlx_error)?,
					QueryValue::FloatArray,
					QueryValue::NullableFloatArray,
				)),
				"FLOAT8[]" => Some(array_query_value(
					pg_row
						.try_get::<Option<Vec<Option<f64>>>, _>(column_name)
						.map_err(map_sqlx_error)?,
					QueryValue::DoubleArray,
					QueryValue::NullableDoubleArray,
				)),
				"UUID[]" => Some(array_query_value(
					pg_row
						.try_get::<Option<Vec<Option<Uuid>>>, _>(column_name)
						.map_err(map_sqlx_error)?,
					QueryValue::UuidArray,
					QueryValue::NullableUuidArray,
				)),
				_ => None,
			};
			if let Some(value) = array_value {
				row.insert(column_name.to_string(), value);
				continue;
			}
			if matches!(type_name.as_str(), "NUMERIC" | "DECIMAL") {
				match pg_row.try_get::<Option<sqlx::types::BigDecimal>, _>(column_name) {
					Ok(Some(value)) => {
						row.insert(
							column_name.to_string(),
							QueryValue::String(value.normalized().to_string()),
						);
					}
					Ok(None) => row.insert(column_name.to_string(), QueryValue::Null),
					Err(error) => return Err(map_sqlx_error(error).into()),
				};
				continue;
			}

			if let Ok(value) = pg_row.try_get::<Uuid, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Uuid(value));
			} else if let Ok(value) = pg_row.try_get::<bool, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Bool(value));
			} else if let Ok(value) = pg_row.try_get::<i64, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Int(value));
			} else if let Ok(value) = pg_row.try_get::<i32, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Int(value as i64));
			} else if let Ok(value) = pg_row.try_get::<i16, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Int(value as i64));
			} else if let Ok(value) = pg_row.try_get::<f64, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Float(value));
			} else if let Ok(value) = pg_row.try_get::<f32, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Float(value as f64));
			} else if let Ok(value) = pg_row.try_get::<chrono::NaiveDate, _>(column_name) {
				row.insert(
					column_name.to_string(),
					QueryValue::String(value.to_string()),
				);
			} else if let Ok(value) = pg_row.try_get::<chrono::NaiveTime, _>(column_name) {
				row.insert(
					column_name.to_string(),
					QueryValue::String(value.to_string()),
				);
			} else if let Ok(value) = pg_row.try_get::<String, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::String(value));
			} else if let Ok(value) = pg_row.try_get::<Vec<u8>, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Bytes(value));
			} else if let Ok(value) = pg_row.try_get::<chrono::NaiveDateTime, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::NaiveTimestamp(value));
			} else if let Ok(value) =
				pg_row.try_get::<chrono::DateTime<chrono::Utc>, _>(column_name)
			{
				row.insert(column_name.to_string(), QueryValue::Timestamp(value));
			} else if pg_row.try_get::<Option<i32>, _>(column_name).is_ok() {
				row.insert(column_name.to_string(), QueryValue::Null);
			}
		}
		Ok(row)
	}
}

#[async_trait]
impl TransactionExecutor for PgTransactionExecutor {
	fn fetch_stream_generated<'a>(
		&'a mut self,
		built: (String, reinhardt_query::Values),
		chunk_size: usize,
		context: Option<crate::backends::error::PgvectorOperationKind>,
	) -> Result<RowStream<'a>> {
		if chunk_size == 0 {
			return Err(DatabaseError::new(
				DatabaseErrorKind::Configuration,
				"Row stream chunk_size must be greater than zero",
			)
			.into());
		}
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;
		let (sql, values) = built;
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		Ok(Box::pin(async_stream::stream! {
			let query = match crate::backends::generated::uncached_postgres_query(
				tx, &sql, arguments,
			).await {
				Ok(query) => query,
				Err(error) => {
					yield Err(map_sqlx_error_with_pgvector_context(error, context));
					return;
				}
			};
			let rows = query.fetch(&mut **tx);
			futures::pin_mut!(rows);
			let rows = rows.ready_chunks(chunk_size);
			futures::pin_mut!(rows);
			while let Some(chunk) = rows.next().await {
				for row in chunk {
					yield row
						.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))
						.and_then(Self::convert_row);
				}
			}
		}))
	}

	async fn execute_generated(
		&mut self,
		built: (String, reinhardt_query::Values),
		context: Option<crate::backends::error::PgvectorOperationKind>,
	) -> Result<QueryResult> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;
		let (sql, values) = built;
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let query = crate::backends::generated::uncached_postgres_query(tx, &sql, arguments)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		let result = query
			.execute(&mut **tx)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		Ok(QueryResult {
			rows_affected: result.rows_affected(),
			last_insert_id: None,
		})
	}

	async fn fetch_one_generated(
		&mut self,
		built: (String, reinhardt_query::Values),
		context: Option<crate::backends::error::PgvectorOperationKind>,
	) -> Result<Row> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;
		let (sql, values) = built;
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let query = crate::backends::generated::uncached_postgres_query(tx, &sql, arguments)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		let result = query
			.fetch_one(&mut **tx)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		Self::convert_row(result)
	}

	async fn fetch_all_generated(
		&mut self,
		built: (String, reinhardt_query::Values),
		context: Option<crate::backends::error::PgvectorOperationKind>,
	) -> Result<Vec<Row>> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;
		let (sql, values) = built;
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let query = crate::backends::generated::uncached_postgres_query(tx, &sql, arguments)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		let result = query
			.fetch_all(&mut **tx)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		result.into_iter().map(Self::convert_row).collect()
	}

	async fn fetch_optional_generated(
		&mut self,
		built: (String, reinhardt_query::Values),
		context: Option<crate::backends::error::PgvectorOperationKind>,
	) -> Result<Option<Row>> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;
		let (sql, values) = built;
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let query = crate::backends::generated::uncached_postgres_query(tx, &sql, arguments)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		let result = query
			.fetch_optional(&mut **tx)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		result.map(Self::convert_row).transpose()
	}

	async fn __fetch_one_generated(
		&mut self,
		sql: &str,
		values: reinhardt_query::Values,
		_backend: DatabaseType,
		context: Option<crate::backends::error::PgvectorOperationKind>,
	) -> Result<Row> {
		let result: Result<Row> = async {
			let arguments = crate::backends::generated::postgres::arguments(values)?;
			let connection = self.tx.as_mut().ok_or_else(transaction_consumed_error)?;
			let query = uncached_postgres_query(connection, sqlx::query_with(sql, arguments))
				.await
				.map_err(map_sqlx_error)?;
			let row = query
				.fetch_one(&mut **connection)
				.await
				.map_err(map_sqlx_error)?;
			Self::convert_row(row)
		}
		.await;
		result.map_err(|error| {
			crate::backends::error::decorate_error_with_pgvector_context(error, context)
		})
	}

	async fn __fetch_all_generated(
		&mut self,
		sql: &str,
		values: reinhardt_query::Values,
		_backend: DatabaseType,
		context: Option<crate::backends::error::PgvectorOperationKind>,
	) -> Result<Vec<Row>> {
		let result: Result<Vec<Row>> = async {
			let arguments = crate::backends::generated::postgres::arguments(values)?;
			let connection = self.tx.as_mut().ok_or_else(transaction_consumed_error)?;
			let query = uncached_postgres_query(connection, sqlx::query_with(sql, arguments))
				.await
				.map_err(map_sqlx_error)?;
			let rows = query
				.fetch_all(&mut **connection)
				.await
				.map_err(map_sqlx_error)?;
			rows.into_iter().map(Self::convert_row).collect()
		}
		.await;
		result.map_err(|error| {
			crate::backends::error::decorate_error_with_pgvector_context(error, context)
		})
	}

	fn backend(&self) -> DatabaseType {
		DatabaseType::Postgres
	}

	fn supports_pgvector_error_hints(&self) -> bool {
		true
	}

	async fn __execute_generated(
		&mut self,
		sql: &str,
		values: reinhardt_query::Values,
		_backend: DatabaseType,
		context: Option<crate::backends::error::PgvectorOperationKind>,
	) -> Result<QueryResult> {
		let result: Result<QueryResult> = async {
			let arguments = crate::backends::generated::postgres::arguments(values)?;
			let connection = self.tx.as_mut().ok_or_else(transaction_consumed_error)?;
			let query = uncached_postgres_query(connection, sqlx::query_with(sql, arguments))
				.await
				.map_err(map_sqlx_error)?;
			let result = query
				.execute(&mut **connection)
				.await
				.map_err(map_sqlx_error)?;
			Ok(QueryResult {
				rows_affected: result.rows_affected(),
				last_insert_id: None,
			})
		}
		.await;
		result.map_err(|error| {
			crate::backends::error::decorate_error_with_pgvector_context(error, context)
		})
	}
	async fn execute(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
		self.execute_with_context(sql, params, None).await
	}

	async fn execute_with_context(
		&mut self,
		sql: &str,
		params: Vec<QueryValue>,
		context: Option<PgvectorOperationKind>,
	) -> Result<QueryResult> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param)?;
		}
		let query = uncached_postgres_query(tx, query)
			.await
			.map_err(map_sqlx_error)?;
		let result = query
			.execute(&mut **tx)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		Ok(QueryResult {
			rows_affected: result.rows_affected(),
			last_insert_id: None,
		})
	}

	async fn fetch_one(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
		self.fetch_one_with_context(sql, params, None).await
	}

	async fn fetch_one_with_context(
		&mut self,
		sql: &str,
		params: Vec<QueryValue>,
		context: Option<PgvectorOperationKind>,
	) -> Result<Row> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param)?;
		}
		let query = uncached_postgres_query(tx, query)
			.await
			.map_err(map_sqlx_error)?;
		let row = query
			.fetch_one(&mut **tx)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		Self::convert_row(row)
	}

	async fn fetch_all(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
		self.fetch_all_with_context(sql, params, None).await
	}

	async fn fetch_all_with_context(
		&mut self,
		sql: &str,
		params: Vec<QueryValue>,
		context: Option<PgvectorOperationKind>,
	) -> Result<Vec<Row>> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param)?;
		}
		let query = uncached_postgres_query(tx, query)
			.await
			.map_err(map_sqlx_error)?;
		let rows = query
			.fetch_all(&mut **tx)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		rows.into_iter().map(Self::convert_row).collect()
	}

	fn fetch_stream<'a>(
		&'a mut self,
		sql: String,
		params: Vec<QueryValue>,
		chunk_size: usize,
	) -> Result<RowStream<'a>> {
		self.fetch_stream_with_context(sql, params, chunk_size, None)
	}

	fn fetch_stream_with_context<'a>(
		&'a mut self,
		sql: String,
		params: Vec<QueryValue>,
		chunk_size: usize,
		context: Option<PgvectorOperationKind>,
	) -> Result<RowStream<'a>> {
		if chunk_size == 0 {
			return Err(DatabaseError::new(
				DatabaseErrorKind::Configuration,
				"Row stream chunk_size must be greater than zero",
			)
			.into());
		}
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;
		Ok(Box::pin(async_stream::stream! {
			let mut query = sqlx::query(&sql);
			for param in &params {
				query = match Self::bind_value(query, param) {
					Ok(query) => query,
					Err(error) => {
						yield Err(error);
						return;
					}
				};
			}
			let rows = query.fetch(&mut **tx);
			futures::pin_mut!(rows);
			let rows = rows.ready_chunks(chunk_size);
			futures::pin_mut!(rows);
			while let Some(chunk) = rows.next().await {
				for row in chunk {
					yield row
						.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))
						.and_then(Self::convert_row);
				}
			}
		}))
	}

	async fn fetch_optional(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
		self.fetch_optional_with_context(sql, params, None).await
	}

	async fn fetch_optional_with_context(
		&mut self,
		sql: &str,
		params: Vec<QueryValue>,
		context: Option<PgvectorOperationKind>,
	) -> Result<Option<Row>> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param)?;
		}
		let query = uncached_postgres_query(tx, query)
			.await
			.map_err(map_sqlx_error)?;
		let row = query
			.fetch_optional(&mut **tx)
			.await
			.map_err(|error| map_sqlx_error_with_pgvector_context(error, context))?;
		row.map(Self::convert_row).transpose()
	}

	async fn commit(mut self: Box<Self>) -> Result<()> {
		let tx = self.tx.take().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;
		tx.commit().await.map_err(map_sqlx_error)?;
		Ok(())
	}

	async fn rollback(mut self: Box<Self>) -> Result<()> {
		let tx = self.tx.take().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;
		tx.rollback().await.map_err(map_sqlx_error)?;
		Ok(())
	}

	async fn savepoint(&mut self, name: &str) -> Result<()> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.to_sql())
			.execute(&mut **tx)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
	}

	async fn release_savepoint(&mut self, name: &str) -> Result<()> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.release_sql())
			.execute(&mut **tx)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
	}

	async fn rollback_to_savepoint(&mut self, name: &str) -> Result<()> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.rollback_sql())
			.execute(&mut **tx)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::PgTransactionExecutor;
	use crate::backends::{
		backend::DatabaseBackend,
		types::{DatabaseType, TransactionExecutor},
	};
	use rstest::rstest;
	use rust_decimal::prelude::ToPrimitive;
	use sqlx::postgres::PgPoolOptions;

	#[tokio::test]
	async fn test_postgres_backend_capabilities_without_connecting() {
		// Arrange
		let postgres_pool = PgPoolOptions::new()
			.connect_lazy("postgresql://localhost/reinhardt_coverage")
			.expect("PostgreSQL URL must be valid");
		let postgres = super::PostgresBackend::new(postgres_pool);

		// Act and assert
		assert_eq!(postgres.database_type(), DatabaseType::Postgres);
		assert_eq!(postgres.placeholder(3), "$3");
		assert!(postgres.supports_returning());
		assert!(postgres.supports_on_conflict());
		assert!(postgres.supports_transactional_ddl());
		assert!(postgres.as_any().is::<super::PostgresBackend>());
	}

	#[test]
	fn test_transaction_executor_reports_postgres_backend() {
		let executor = PgTransactionExecutor { tx: None };

		assert_eq!(executor.backend(), DatabaseType::Postgres);
	}

	/// Verify that normal Decimal values succeed to_f64() conversion
	#[rstest]
	#[case::positive(rust_decimal::Decimal::new(12345, 2), 123.45)]
	#[case::zero(rust_decimal::Decimal::ZERO, 0.0)]
	#[case::negative(rust_decimal::Decimal::new(-999, 1), -99.9)]
	#[case::max(rust_decimal::Decimal::MAX, 7.922816251426434e28)]
	fn test_decimal_to_f64_conversion_succeeds(
		#[case] decimal: rust_decimal::Decimal,
		#[case] expected: f64,
	) {
		// Act
		let result = decimal.to_f64();
		assert!(result.is_some());
		assert!((result.unwrap() - expected).abs() < f64::EPSILON * expected.abs().max(1.0));
	}

	#[tokio::test]
	async fn convert_row_preserves_postgres_arrays_and_decimals() {
		use sqlx::postgres::PgPoolOptions;
		use testcontainers::{GenericImage, ImageExt, core::WaitFor, runners::AsyncRunner};

		let container = GenericImage::new("postgres", "17-alpine")
			.with_wait_for(WaitFor::message_on_stderr(
				"database system is ready to accept connections",
			))
			.with_env_var("POSTGRES_HOST_AUTH_METHOD", "trust")
			.start()
			.await
			.expect("PostgreSQL test container should start");
		let port = container
			.get_host_port_ipv4(5432)
			.await
			.expect("PostgreSQL test container should expose port 5432");
		let pool = PgPoolOptions::new()
			.max_connections(1)
			.connect(format!("postgres://postgres@localhost:{port}/postgres").as_str())
			.await
			.expect("test pool should connect to PostgreSQL");

		let row = sqlx::query(
			"SELECT \
				ARRAY['alpha', 'beta']::text[] AS string_values, \
				ARRAY[1, 2]::integer[] AS int_values, \
				ARRAY[3, 4]::bigint[] AS bigint_values, \
				ARRAY[TRUE, FALSE]::boolean[] AS bool_values, \
				ARRAY[1.5, 2.5]::real[] AS float_values, \
				ARRAY[3.5, 4.5]::double precision[] AS double_values, \
				ARRAY['00000000-0000-0000-0000-000000000000']::uuid[] AS uuid_values, \
				9007199254740993.01::numeric AS rust_decimal_value, \
				123456789012345678901234567890.123456789::numeric AS decimal_value, \
				7::smallint AS small_integer_value, \
				1.25::real AS real_value",
		)
		.fetch_one(&pool)
		.await
		.expect("PostgreSQL should return native arrays");
		let converted = super::PostgresBackend::convert_row_internal(row)
			.expect("backend row conversion should preserve arrays");

		assert_eq!(
			converted.data.get("string_values"),
			Some(&super::QueryValue::StringArray(vec![
				"alpha".to_string(),
				"beta".to_string(),
			]))
		);
		assert_eq!(
			converted.data.get("int_values"),
			Some(&super::QueryValue::IntArray(vec![1, 2]))
		);
		assert_eq!(
			converted.data.get("bigint_values"),
			Some(&super::QueryValue::BigIntArray(vec![3, 4]))
		);
		assert_eq!(
			converted.data.get("bool_values"),
			Some(&super::QueryValue::BoolArray(vec![true, false]))
		);
		assert_eq!(
			converted.data.get("float_values"),
			Some(&super::QueryValue::FloatArray(vec![1.5, 2.5]))
		);
		assert_eq!(
			converted.data.get("double_values"),
			Some(&super::QueryValue::DoubleArray(vec![3.5, 4.5]))
		);
		assert_eq!(
			converted.data.get("uuid_values"),
			Some(&super::QueryValue::UuidArray(vec![uuid::Uuid::nil()]))
		);
		assert_eq!(
			converted.data.get("decimal_value"),
			Some(&super::QueryValue::String(
				"123456789012345678901234567890.123456789".to_string()
			))
		);
		assert_eq!(
			converted.data.get("small_integer_value"),
			Some(&super::QueryValue::Int(7))
		);
		assert_eq!(
			converted.data.get("real_value"),
			Some(&super::QueryValue::Float(1.25))
		);
		let query_row = crate::orm::connection::QueryRow::from_backend_row(converted);
		assert_eq!(
			query_row.get::<rust_decimal::Decimal>("rust_decimal_value"),
			Some(rust_decimal::Decimal::new(900_719_925_474_099_301, 2))
		);
	}

	/// Verify TypeError is the correct variant for type conversion failures
	#[rstest]
	fn test_type_error_variant_distinction() {
		use crate::backends::error::{DatabaseError, DatabaseErrorKind};

		// Arrange & Act
		let type_error = DatabaseError::new(DatabaseErrorKind::Type, "conversion failed");
		let query_error = DatabaseError::new(DatabaseErrorKind::Query, "query failed");

		// Assert
		assert_eq!(type_error.kind(), DatabaseErrorKind::Type);
		assert_eq!(query_error.kind(), DatabaseErrorKind::Query);
	}
}

#[cfg(test)]
mod statement_cache_tests {
	use super::{PostgresBackend, uncached_postgres_query};
	use crate::backends::{backend::DatabaseBackend, types::QueryValue};
	use futures::{StreamExt, TryStreamExt};
	use rstest::{fixture, rstest};
	use sqlx::postgres::PgPoolOptions;
	use sqlx::{Arguments, Connection, Row};
	use testcontainers::{ContainerAsync, ImageExt, runners::AsyncRunner};
	use testcontainers_modules::postgres::Postgres;

	struct PostgresFixture {
		// Release the pool before the container that owns its database.
		pool: sqlx::PgPool,
		_container: ContainerAsync<Postgres>,
	}

	#[fixture]
	async fn postgres_fixture() -> PostgresFixture {
		let container = Postgres::default()
			.with_tag("17-alpine")
			.start()
			.await
			.unwrap();
		let url = format!(
			"postgres://postgres:postgres@{}:{}/postgres",
			container.get_host().await.unwrap(),
			container.get_host_port_ipv4(5432).await.unwrap()
		);
		PostgresFixture {
			pool: PgPoolOptions::new()
				.max_connections(1)
				.connect(&url)
				.await
				.unwrap(),
			_container: container,
		}
	}

	#[rstest]
	#[case::select("SELECT $1, $2, $3, $4")]
	#[case::values("VALUES ($1, $2, $3, $4)")]
	#[tokio::test]
	async fn pool_null_parameters_infer_insert_types(
		#[future] postgres_fixture: PostgresFixture,
		#[case] values_clause: &str,
	) {
		// Arrange
		let fixture = postgres_fixture.await;
		let backend = PostgresBackend::new(fixture.pool.clone());
		backend
			.execute(
				"CREATE TABLE nullable_parameters (expires_at TIMESTAMPTZ, request_id UUID, marker BIGINT, label TEXT)",
				vec![],
			)
			.await
			.unwrap();
		let timestamp = chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap();
		let request_id = uuid::Uuid::from_u128(42);
		let label = "literal $1, 'quotes', and SQL NULL";
		let sql = format!(
			"INSERT INTO nullable_parameters (expires_at, request_id, marker, label) {values_clause}"
		);

		// Act: reuse identical SQL with NULL, native values, and NULL again.
		for (marker, expires_at, request_id) in [
			(1, QueryValue::Null, QueryValue::Null),
			(
				2,
				QueryValue::Timestamp(timestamp),
				QueryValue::Uuid(request_id),
			),
			(3, QueryValue::Null, QueryValue::Null),
		] {
			let result = backend
				.execute(
					&sql,
					vec![
						expires_at,
						request_id,
						QueryValue::Int(marker),
						label.into(),
					],
				)
				.await
				.unwrap();
			assert_eq!(result.rows_affected, 1);
		}

		// Assert
		let rows = backend
			.fetch_all("SELECT * FROM nullable_parameters ORDER BY marker", vec![])
			.await
			.unwrap();
		assert_eq!(rows.len(), 3);
		for (index, row) in rows.iter().enumerate() {
			assert_eq!(row.get::<i64>("marker").unwrap(), index as i64 + 1);
			assert_eq!(row.get::<String>("label").unwrap(), label);
			let (expires_at, request_id) = if index == 1 {
				(
					QueryValue::Timestamp(timestamp),
					QueryValue::Uuid(request_id),
				)
			} else {
				(QueryValue::Null, QueryValue::Null)
			};
			assert_eq!(row.data.get("expires_at"), Some(&expires_at));
			assert_eq!(row.data.get("request_id"), Some(&request_id));
		}
	}

	#[rstest]
	#[tokio::test]
	async fn transaction_null_parameters_infer_temp_table_types(
		#[future] postgres_fixture: PostgresFixture,
	) {
		// Arrange
		let fixture = postgres_fixture.await;
		let backend = PostgresBackend::new(fixture.pool.clone());
		let mut tx = backend.begin().await.unwrap();
		tx.execute(
			"CREATE TEMP TABLE nullable_parameter_repro (expires_at TIMESTAMPTZ, request_id UUID) ON COMMIT DROP",
			vec![],
		)
		.await
		.unwrap();

		// Act: no casts or SQL NULL literals are needed for either parameter.
		let result = tx
			.execute(
				"INSERT INTO nullable_parameter_repro (expires_at, request_id) SELECT $1, $2",
				vec![QueryValue::Null, QueryValue::Null],
			)
			.await
			.unwrap();

		// Assert: the values belong to this transaction's physical connection.
		assert_eq!(result.rows_affected, 1);
		let row = tx
			.fetch_one("SELECT * FROM nullable_parameter_repro", vec![])
			.await
			.unwrap();
		assert_eq!(row.data.get("expires_at"), Some(&QueryValue::Null));
		assert_eq!(row.data.get("request_id"), Some(&QueryValue::Null));
		tx.commit().await.unwrap();
		let row = backend
			.fetch_one(
				"SELECT to_regclass('pg_temp.nullable_parameter_repro') IS NULL AS dropped",
				vec![],
			)
			.await
			.unwrap();
		assert!(row.get::<bool>("dropped").unwrap());
	}

	#[rstest]
	#[case::commit(true)]
	#[case::rollback(false)]
	#[tokio::test]
	async fn transaction_null_parameters_preserve_atomic_writes(
		#[future] postgres_fixture: PostgresFixture,
		#[case] commit: bool,
	) {
		// Arrange
		let fixture = postgres_fixture.await;
		let backend = PostgresBackend::new(fixture.pool.clone());
		backend
			.execute(
				"CREATE TABLE nullable_parameters (expires_at TIMESTAMPTZ, request_id UUID)",
				vec![],
			)
			.await
			.unwrap();
		let mut tx = backend.begin().await.unwrap();

		// Act
		let result = tx
			.execute(
				"INSERT INTO nullable_parameters (expires_at, request_id) SELECT $1, $2",
				vec![QueryValue::Null, QueryValue::Null],
			)
			.await
			.unwrap();
		assert_eq!(result.rows_affected, 1);
		if commit {
			tx.commit().await.unwrap();
		} else {
			tx.rollback().await.unwrap();
		}

		// Assert
		let row = backend
			.fetch_one("SELECT COUNT(*) AS total FROM nullable_parameters", vec![])
			.await
			.unwrap();
		assert_eq!(row.get::<i64>("total").unwrap(), i64::from(commit));
	}

	#[rstest]
	#[case::pool_one(false, "one")]
	#[case::pool_all(false, "all")]
	#[case::pool_optional(false, "optional")]
	#[case::pool_stream(false, "stream")]
	#[case::transaction_one(true, "one")]
	#[case::transaction_all(true, "all")]
	#[case::transaction_optional(true, "optional")]
	#[case::transaction_stream(true, "stream")]
	#[tokio::test]
	async fn null_parameters_work_in_all_fetch_methods(
		#[future] postgres_fixture: PostgresFixture,
		#[case] transactional: bool,
		#[case] method: &str,
	) {
		// Arrange
		let fixture = postgres_fixture.await;
		let backend = PostgresBackend::new(fixture.pool.clone());
		let sql = "SELECT $1::TIMESTAMPTZ AS expires_at, $2::UUID AS request_id, $3::BOOLEAN AS enabled, $4::TEXT AS label, $5::BYTEA AS payload, $6::NUMERIC AS amount, $7::BIGINT AS marker";
		let params = vec![
			QueryValue::Null,
			QueryValue::Null,
			QueryValue::Null,
			QueryValue::Null,
			QueryValue::Null,
			QueryValue::Null,
			QueryValue::Int(1_i64 << 40),
		];

		// Act
		let rows = if transactional {
			let mut tx = backend.begin().await.unwrap();
			let rows = match method {
				"one" => vec![tx.fetch_one(sql, params).await.unwrap()],
				"all" => tx.fetch_all(sql, params).await.unwrap(),
				"stream" => tx
					.fetch_stream(sql.into(), params, 1)
					.unwrap()
					.try_collect::<Vec<_>>()
					.await
					.unwrap(),
				"optional" => tx
					.fetch_optional(sql, params)
					.await
					.unwrap()
					.into_iter()
					.collect(),
				_ => unreachable!("unknown fetch method"),
			};
			tx.rollback().await.unwrap();
			rows
		} else {
			match method {
				"one" => vec![backend.fetch_one(sql, params).await.unwrap()],
				"all" => backend.fetch_all(sql, params).await.unwrap(),
				"stream" => backend
					.fetch_stream(sql.into(), params, 1)
					.unwrap()
					.try_collect::<Vec<_>>()
					.await
					.unwrap(),
				"optional" => backend
					.fetch_optional(sql, params)
					.await
					.unwrap()
					.into_iter()
					.collect(),
				_ => unreachable!("unknown fetch method"),
			}
		};

		// Assert
		assert_eq!(rows.len(), 1);
		assert_eq!(rows[0].data.len(), 7);
		for column in [
			"expires_at",
			"request_id",
			"enabled",
			"label",
			"payload",
			"amount",
		] {
			assert_eq!(rows[0].data.get(column), Some(&QueryValue::Null));
		}
		assert_eq!(rows[0].get::<i64>("marker").unwrap(), 1_i64 << 40);
	}

	#[rstest]
	#[case::pool(false)]
	#[case::transaction(true)]
	#[tokio::test]
	async fn null_parameters_require_type_context(
		#[future] postgres_fixture: PostgresFixture,
		#[case] transactional: bool,
	) {
		// Arrange
		let fixture = postgres_fixture.await;
		let backend = PostgresBackend::new(fixture.pool.clone());
		let sql = "SELECT $1 IS NULL AS missing";

		// Act
		let error = if transactional {
			let mut tx = backend.begin().await.unwrap();
			let error = tx.fetch_one(sql, vec![QueryValue::Null]).await.unwrap_err();
			tx.rollback().await.unwrap();
			error
		} else {
			backend
				.fetch_one(sql, vec![QueryValue::Null])
				.await
				.unwrap_err()
		};

		// Assert: the API does not silently choose an integer type for ambiguous NULL.
		let error = error.database_error().unwrap();
		assert_eq!(error.code(), Some("42P18"));
		assert_eq!(
			error.message(),
			"could not determine data type of parameter $1"
		);
		let row = backend
			.fetch_one(
				"SELECT $1::INTEGER IS NULL AS missing",
				vec![QueryValue::Null],
			)
			.await
			.unwrap();
		assert!(row.get::<bool>("missing").unwrap());
	}

	#[rstest]
	#[tokio::test]
	async fn pool_query_accepts_int8_after_null(#[future] postgres_fixture: PostgresFixture) {
		// Arrange: NULL infers BIGINT from SQL, while integer values use native INT8.
		let fixture = postgres_fixture.await;
		let backend = PostgresBackend::new(fixture.pool.clone());
		let sql = "SELECT $1::BIGINT AS cached_width";
		let row = backend
			.fetch_one(sql, vec![QueryValue::Null])
			.await
			.unwrap();
		assert!(matches!(
			row.data.get("cached_width"),
			Some(QueryValue::Null)
		));

		// Act: reuse the SQL with a value that cannot be represented as INT4.
		let row = backend
			.fetch_one(sql, vec![QueryValue::Int(1_i64 << 40)])
			.await
			.unwrap();

		// Assert: the full native value survives the previous NULL signature.
		assert_eq!(row.get::<i64>("cached_width").unwrap(), 1_i64 << 40);
	}

	#[derive(Clone, Copy)]
	enum Operation {
		Execute,
		FetchOne,
		FetchAll,
		FetchOptional,
	}

	#[rstest]
	#[tokio::test]
	async fn backend_operations_bypass_incompatible_raw_cache_entries(
		#[future] postgres_fixture: PostgresFixture,
		#[values(
			Operation::Execute,
			Operation::FetchOne,
			Operation::FetchAll,
			Operation::FetchOptional
		)]
		operation: Operation,
		#[values(false, true)] transactional: bool,
	) {
		// Arrange: one pooled connection retains a raw INT4 statement for the same SQL.
		let fixture = postgres_fixture.await;
		let backend = PostgresBackend::new(fixture.pool.clone());
		let sql = "SELECT $1::BIGINT AS cached_width, pg_typeof($1)::TEXT AS native_type";
		{
			let mut connection = fixture.pool.acquire().await.unwrap();
			let row = sqlx::query(sql)
				.bind(7_i32)
				.fetch_one(&mut *connection)
				.await
				.unwrap();
			assert_eq!(row.get::<i64, _>("cached_width"), 7);
			assert_eq!(row.get::<String, _>("native_type"), "integer");
			assert!(connection.cached_statements_size() > 0);
		}
		let mut transaction = if transactional {
			Some(backend.begin().await.unwrap())
		} else {
			None
		};
		let params = vec![QueryValue::Int(1_i64 << 40)];

		// Act: each public backend operation must isolate its INT8 signature.
		let row = match operation {
			Operation::Execute => {
				let result = if let Some(transaction) = transaction.as_mut() {
					transaction.execute(sql, params).await.unwrap()
				} else {
					backend.execute(sql, params).await.unwrap()
				};
				assert_eq!(result.rows_affected, 1);
				None
			}
			Operation::FetchOne => Some(if let Some(transaction) = transaction.as_mut() {
				transaction.fetch_one(sql, params).await.unwrap()
			} else {
				backend.fetch_one(sql, params).await.unwrap()
			}),
			Operation::FetchAll => {
				let mut rows = if let Some(transaction) = transaction.as_mut() {
					transaction.fetch_all(sql, params).await.unwrap()
				} else {
					backend.fetch_all(sql, params).await.unwrap()
				};
				assert_eq!(rows.len(), 1);
				rows.pop()
			}
			Operation::FetchOptional => {
				let row = if let Some(transaction) = transaction.as_mut() {
					transaction.fetch_optional(sql, params).await.unwrap()
				} else {
					backend.fetch_optional(sql, params).await.unwrap()
				};
				Some(row.unwrap())
			}
		};
		if let Some(transaction) = transaction {
			transaction.commit().await.unwrap();
		}

		// Assert: values keep their native type and raw SQLx calls can cache again.
		if let Some(row) = row {
			assert_eq!(row.get::<i64>("cached_width").unwrap(), 1_i64 << 40);
			assert_eq!(row.get::<String>("native_type").unwrap(), "bigint");
		}
		let mut connection = fixture.pool.acquire().await.unwrap();
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
		let fixture = postgres_fixture.await;
		let mut dedicated = fixture.pool.acquire().await.unwrap();
		let connection = &mut *dedicated;
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
		let row = uncached_postgres_query(connection, sqlx::query_with(sql, arguments))
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
		let fixture = postgres_fixture.await;
		let mut dedicated = fixture.pool.acquire().await.unwrap();
		let connection = &mut *dedicated;
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
			let mut rows = uncached_postgres_query(connection, sqlx::query_with(sql, arguments))
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
		let fixture = postgres_fixture.await;
		let mut dedicated = fixture.pool.acquire().await.unwrap();
		let connection = &mut *dedicated;
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
			uncached_postgres_query(&mut transaction, sqlx::query_with(sql, arguments))
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
