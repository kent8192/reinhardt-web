//! PostgreSQL dialect implementation

use async_trait::async_trait;
use sqlx::{Column, PgPool, Postgres, Transaction, postgres::PgRow};
use std::sync::Arc;
use uuid::Uuid;

use crate::backends::{
	backend::DatabaseBackend,
	error::{DatabaseError, Result},
	types::{
		DatabaseType, IsolationLevel, QueryResult, QueryValue, Row, Savepoint, TransactionExecutor,
	},
};

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
	) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
		match value {
			QueryValue::Null => query.bind(None::<i32>),
			QueryValue::Bool(b) => query.bind(b),
			QueryValue::Int(i) => query.bind(i),
			QueryValue::Float(f) => query.bind(f),
			QueryValue::String(s) => query.bind(s),
			QueryValue::Bytes(b) => query.bind(b),
			QueryValue::Timestamp(dt) => query.bind(dt),
			QueryValue::Uuid(u) => query.bind(u),
			QueryValue::Now => {
				// PostgreSQL uses NOW() function, which should be part of SQL string
				// For binding, we use current UTC time
				query.bind(chrono::Utc::now())
			}
		}
	}

	fn convert_row(pg_row: PgRow) -> Result<Row> {
		Self::convert_row_internal(pg_row)
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
	async fn __execute_generated(
		&self,
		sql: &str,
		values: reinhardt_query::Values,
	) -> Result<QueryResult> {
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let mut connection = self.pool.acquire().await?;
		let query =
			uncached_postgres_query(&mut connection, sqlx::query_with(sql, arguments)).await?;
		let result = query.execute(&mut *connection).await?;
		Ok(QueryResult {
			rows_affected: result.rows_affected(),
		})
	}

	async fn __fetch_one_generated(
		&self,
		sql: &str,
		values: reinhardt_query::Values,
	) -> Result<Row> {
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let mut connection = self.pool.acquire().await?;
		let query =
			uncached_postgres_query(&mut connection, sqlx::query_with(sql, arguments)).await?;
		let row = query.fetch_one(&mut *connection).await?;
		Self::convert_row(row)
	}

	async fn __fetch_all_generated(
		&self,
		sql: &str,
		values: reinhardt_query::Values,
	) -> Result<Vec<Row>> {
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let mut connection = self.pool.acquire().await?;
		let query =
			uncached_postgres_query(&mut connection, sqlx::query_with(sql, arguments)).await?;
		let rows = query.fetch_all(&mut *connection).await?;
		rows.into_iter().map(Self::convert_row).collect()
	}
	fn database_type(&self) -> DatabaseType {
		DatabaseType::Postgres
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
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let mut connection = self.pool.acquire().await?;
		let query = uncached_postgres_query(&mut connection, query).await?;
		let result = query.execute(&mut *connection).await?;
		Ok(QueryResult {
			rows_affected: result.rows_affected(),
		})
	}

	async fn fetch_one(&self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let mut connection = self.pool.acquire().await?;
		let query = uncached_postgres_query(&mut connection, query).await?;
		let row = query.fetch_one(&mut *connection).await?;
		Self::convert_row(row)
	}

	async fn fetch_all(&self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let mut connection = self.pool.acquire().await?;
		let query = uncached_postgres_query(&mut connection, query).await?;
		let rows = query.fetch_all(&mut *connection).await?;
		rows.into_iter().map(Self::convert_row).collect()
	}

	async fn fetch_optional(&self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let mut connection = self.pool.acquire().await?;
		let query = uncached_postgres_query(&mut connection, query).await?;
		let row = query.fetch_optional(&mut *connection).await?;
		row.map(Self::convert_row).transpose()
	}

	async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
		let tx = self.pool.begin().await?;
		Ok(Box::new(PgTransactionExecutor::new(tx)))
	}

	async fn begin_with_isolation(
		&self,
		isolation_level: IsolationLevel,
	) -> Result<Box<dyn TransactionExecutor>> {
		// PostgreSQL supports setting isolation level at transaction start
		let mut tx = self.pool.begin().await?;

		// Set the isolation level using PostgreSQL's SET TRANSACTION command
		let sql = format!(
			"SET TRANSACTION ISOLATION LEVEL {}",
			isolation_level.to_sql(DatabaseType::Postgres)
		);
		sqlx::query(&sql).execute(&mut *tx).await?;

		Ok(Box::new(PgTransactionExecutor::new(tx)))
	}

	fn as_any(&self) -> &dyn std::any::Any {
		self
	}
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
	) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
		match value {
			QueryValue::Null => query.bind(None::<i32>),
			QueryValue::Bool(b) => query.bind(b),
			QueryValue::Int(i) => query.bind(i),
			QueryValue::Float(f) => query.bind(f),
			QueryValue::String(s) => query.bind(s),
			QueryValue::Bytes(b) => query.bind(b),
			QueryValue::Timestamp(dt) => query.bind(dt),
			QueryValue::Uuid(u) => query.bind(u),
			QueryValue::Now => query.bind(chrono::Utc::now()),
		}
	}

	fn convert_row(pg_row: PgRow) -> Result<Row> {
		PostgresBackend::convert_row_internal(pg_row)
	}
}

impl PostgresBackend {
	/// Internal row conversion method shared between backend and transaction executor
	pub(crate) fn convert_row_internal(pg_row: PgRow) -> Result<Row> {
		use rust_decimal::prelude::ToPrimitive;
		use sqlx::Row as SqlxRow;

		let mut row = Row::new();
		for column in pg_row.columns() {
			let column_name = column.name();

			if let Ok(value) = pg_row.try_get::<Uuid, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Uuid(value));
			} else if let Ok(value) = pg_row.try_get::<bool, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Bool(value));
			} else if let Ok(value) = pg_row.try_get::<i64, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Int(value));
			} else if let Ok(value) = pg_row.try_get::<i32, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Int(value as i64));
			} else if let Ok(value) = pg_row.try_get::<rust_decimal::Decimal, _>(column_name) {
				// Convert DECIMAL/NUMERIC to f64 for Float storage
				if let Some(f) = value.to_f64() {
					row.insert(column_name.to_string(), QueryValue::Float(f));
				} else {
					return Err(Self::decimal_conversion_error(&value, column_name));
				}
			} else if let Ok(value) = pg_row.try_get::<f64, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Float(value));
			} else if let Ok(value) = pg_row.try_get::<String, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::String(value));
			} else if let Ok(value) = pg_row.try_get::<Vec<u8>, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Bytes(value));
			} else if let Ok(value) = pg_row.try_get::<chrono::NaiveDateTime, _>(column_name) {
				row.insert(
					column_name.to_string(),
					QueryValue::Timestamp(chrono::DateTime::from_naive_utc_and_offset(
						value,
						chrono::Utc,
					)),
				);
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

	/// Build a TypeError for failed Decimal-to-f64 conversion
	fn decimal_conversion_error(value: &rust_decimal::Decimal, column_name: &str) -> DatabaseError {
		DatabaseError::TypeError(format!(
			"Failed to convert Decimal value '{}' to f64 for column '{}'",
			value, column_name
		))
	}
}

#[async_trait]
impl TransactionExecutor for PgTransactionExecutor {
	async fn __execute_generated(
		&mut self,
		sql: &str,
		values: reinhardt_query::Values,
		_backend: DatabaseType,
	) -> Result<QueryResult> {
		let arguments = crate::backends::generated::postgres::arguments(values)?;
		let connection = self.tx.as_mut().ok_or_else(|| {
			crate::backends::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;
		let query = uncached_postgres_query(connection, sqlx::query_with(sql, arguments)).await?;
		let result = query.execute(&mut **connection).await?;
		Ok(QueryResult {
			rows_affected: result.rows_affected(),
		})
	}
	async fn execute(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			crate::backends::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let query = uncached_postgres_query(tx, query).await?;
		let result = query.execute(&mut **tx).await?;
		Ok(QueryResult {
			rows_affected: result.rows_affected(),
		})
	}

	async fn fetch_one(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			crate::backends::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let query = uncached_postgres_query(tx, query).await?;
		let row = query.fetch_one(&mut **tx).await?;
		Self::convert_row(row)
	}

	async fn fetch_all(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			crate::backends::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let query = uncached_postgres_query(tx, query).await?;
		let rows = query.fetch_all(&mut **tx).await?;
		rows.into_iter().map(Self::convert_row).collect()
	}

	async fn fetch_optional(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			crate::backends::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let query = uncached_postgres_query(tx, query).await?;
		let row = query.fetch_optional(&mut **tx).await?;
		row.map(Self::convert_row).transpose()
	}

	async fn commit(mut self: Box<Self>) -> Result<()> {
		let tx = self.tx.take().ok_or_else(|| {
			crate::backends::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;
		tx.commit().await?;
		Ok(())
	}

	async fn rollback(mut self: Box<Self>) -> Result<()> {
		let tx = self.tx.take().ok_or_else(|| {
			crate::backends::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;
		tx.rollback().await?;
		Ok(())
	}

	async fn savepoint(&mut self, name: &str) -> Result<()> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			crate::backends::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.to_sql()).execute(&mut **tx).await?;
		Ok(())
	}

	async fn release_savepoint(&mut self, name: &str) -> Result<()> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			crate::backends::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.release_sql()).execute(&mut **tx).await?;
		Ok(())
	}

	async fn rollback_to_savepoint(&mut self, name: &str) -> Result<()> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			crate::backends::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.rollback_sql()).execute(&mut **tx).await?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use crate::backends::{backend::DatabaseBackend, types::DatabaseType};
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

		// Assert
		assert!(
			result.is_some(),
			"Decimal '{}' should convert to f64",
			decimal
		);
		let f = result.unwrap();

		// Use combined relative and absolute tolerance for float comparison
		let diff = (f - expected).abs();
		let rel_tol = 1e-12;
		let abs_tol = 1e-12;
		let tol = expected.abs() * rel_tol + abs_tol;

		assert!(
			diff <= tol,
			"Expected approximately {} (tolerance {}, diff {}), got {}",
			expected,
			tol,
			diff,
			f
		);
	}

	/// Verify the TypeError is constructed correctly for conversion failures
	#[rstest]
	fn test_decimal_conversion_error_message_format() {
		use crate::backends::error::DatabaseError;

		assert!(matches!(
			super::PostgresBackend::decimal_conversion_error(
				&rust_decimal::Decimal::new(12345, 2),
				"price_column"
			),
			DatabaseError::TypeError(message)
				if message == "Failed to convert Decimal value '123.45' to f64 for column 'price_column'"
		));
	}

	/// Verify TypeError is the correct variant for type conversion failures
	#[rstest]
	fn test_type_error_variant_distinction() {
		use crate::backends::error::DatabaseError;

		// Arrange & Act
		let type_error = DatabaseError::TypeError("conversion failed".to_string());
		let query_error = DatabaseError::QueryError("query failed".to_string());

		// Assert
		assert!(matches!(type_error, DatabaseError::TypeError(_)));
		assert!(!matches!(type_error, DatabaseError::QueryError(_)));
		assert!(matches!(query_error, DatabaseError::QueryError(_)));
		assert!(!matches!(query_error, DatabaseError::TypeError(_)));
	}
}

#[cfg(test)]
mod statement_cache_tests {
	use super::{PostgresBackend, uncached_postgres_query};
	use crate::backends::{backend::DatabaseBackend, types::QueryValue};
	use futures::StreamExt;
	use rstest::{fixture, rstest};
	use sqlx::postgres::PgPoolOptions;
	use sqlx::{Arguments, Connection, Row};
	use testcontainers::{ContainerAsync, runners::AsyncRunner};
	use testcontainers_modules::postgres::Postgres;

	struct PostgresFixture {
		// Release the pool before the container that owns its database.
		pool: sqlx::PgPool,
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
			pool: PgPoolOptions::new()
				.max_connections(1)
				.connect(&url)
				.await
				.unwrap(),
			_container: container,
		}
	}

	#[rstest]
	#[tokio::test]
	async fn pool_query_accepts_int8_after_null(#[future] postgres_fixture: PostgresFixture) {
		// Arrange: NULL is encoded with INT4, whereas integer values use INT8.
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
