//! PostgreSQL dialect implementation

use async_trait::async_trait;
use sqlx::{Column, PgPool, Postgres, Transaction, TypeInfo, postgres::PgRow};
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
			QueryValue::Json(value) => query.bind(value.as_deref().cloned().map(sqlx::types::Json)),
			QueryValue::StringArray(values) => query.bind(values),
			QueryValue::IntArray(values) => query.bind(values),
			QueryValue::BigIntArray(values) => query.bind(values),
			QueryValue::BoolArray(values) => query.bind(values),
			QueryValue::FloatArray(values) => query.bind(values),
			QueryValue::DoubleArray(values) => query.bind(values),
			QueryValue::UuidArray(values) => query.bind(values),
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

#[async_trait]
impl DatabaseBackend for PostgresBackend {
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
		let result = query.execute(self.pool.as_ref()).await?;
		Ok(QueryResult {
			rows_affected: result.rows_affected(),
		})
	}

	async fn fetch_one(&self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let row = query.fetch_one(self.pool.as_ref()).await?;
		Self::convert_row(row)
	}

	async fn fetch_all(&self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let rows = query.fetch_all(self.pool.as_ref()).await?;
		rows.into_iter().map(Self::convert_row).collect()
	}

	async fn fetch_optional(&self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let row = query.fetch_optional(self.pool.as_ref()).await?;
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
			QueryValue::Json(value) => query.bind(value.as_deref().cloned().map(sqlx::types::Json)),
			QueryValue::StringArray(values) => query.bind(values),
			QueryValue::IntArray(values) => query.bind(values),
			QueryValue::BigIntArray(values) => query.bind(values),
			QueryValue::BoolArray(values) => query.bind(values),
			QueryValue::FloatArray(values) => query.bind(values),
			QueryValue::DoubleArray(values) => query.bind(values),
			QueryValue::UuidArray(values) => query.bind(values),
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
			let type_name = column.type_info().name().to_uppercase();
			if matches!(type_name.as_str(), "JSON" | "JSONB") {
				match pg_row.try_get::<Option<serde_json::Value>, _>(column_name) {
					Ok(Some(value)) => row.insert(
						column_name.to_string(),
						QueryValue::Json(Some(Box::new(value))),
					),
					Ok(None) => row.insert(column_name.to_string(), QueryValue::Null),
					Err(error) => return Err(error.into()),
				};
				continue;
			}

			match type_name.as_str() {
				"TEXT[]" | "VARCHAR[]" | "BPCHAR[]" => {
					match pg_row.try_get::<Option<Vec<String>>, _>(column_name)? {
						Some(values) => {
							row.insert(column_name.to_string(), QueryValue::StringArray(values))
						}
						None => row.insert(column_name.to_string(), QueryValue::Null),
					};
					continue;
				}
				"INT4[]" => {
					match pg_row.try_get::<Option<Vec<i32>>, _>(column_name)? {
						Some(values) => {
							row.insert(column_name.to_string(), QueryValue::IntArray(values))
						}
						None => row.insert(column_name.to_string(), QueryValue::Null),
					};
					continue;
				}
				"INT8[]" => {
					match pg_row.try_get::<Option<Vec<i64>>, _>(column_name)? {
						Some(values) => {
							row.insert(column_name.to_string(), QueryValue::BigIntArray(values))
						}
						None => row.insert(column_name.to_string(), QueryValue::Null),
					};
					continue;
				}
				"BOOL[]" => {
					match pg_row.try_get::<Option<Vec<bool>>, _>(column_name)? {
						Some(values) => {
							row.insert(column_name.to_string(), QueryValue::BoolArray(values))
						}
						None => row.insert(column_name.to_string(), QueryValue::Null),
					};
					continue;
				}
				"FLOAT4[]" => {
					match pg_row.try_get::<Option<Vec<f32>>, _>(column_name)? {
						Some(values) => {
							row.insert(column_name.to_string(), QueryValue::FloatArray(values))
						}
						None => row.insert(column_name.to_string(), QueryValue::Null),
					};
					continue;
				}
				"FLOAT8[]" => {
					match pg_row.try_get::<Option<Vec<f64>>, _>(column_name)? {
						Some(values) => {
							row.insert(column_name.to_string(), QueryValue::DoubleArray(values))
						}
						None => row.insert(column_name.to_string(), QueryValue::Null),
					};
					continue;
				}
				"UUID[]" => {
					match pg_row.try_get::<Option<Vec<Uuid>>, _>(column_name)? {
						Some(values) => {
							row.insert(column_name.to_string(), QueryValue::UuidArray(values))
						}
						None => row.insert(column_name.to_string(), QueryValue::Null),
					};
					continue;
				}
				_ => {}
			}

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
	use rstest::rstest;
	use rust_decimal::prelude::ToPrimitive;

	#[tokio::test]
	async fn convert_row_preserves_postgres_arrays() {
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
				ARRAY['00000000-0000-0000-0000-000000000000']::uuid[] AS uuid_values",
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

		// Arrange
		let value = rust_decimal::Decimal::new(12345, 2);
		let column_name = "price_column";

		// Act
		let error = super::PostgresBackend::decimal_conversion_error(&value, column_name);

		// Assert
		assert!(matches!(error, DatabaseError::TypeError(_)));
		let error_msg = error.to_string();
		assert!(
			error_msg.contains("price_column"),
			"Error message should contain the column name"
		);
		assert!(
			error_msg.contains("123.45"),
			"Error message should contain the decimal value"
		);
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
