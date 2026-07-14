//! PostgreSQL dialect implementation

use async_trait::async_trait;
use sqlx::{Column, PgPool, Postgres, Transaction, TypeInfo, postgres::PgRow};
use std::sync::Arc;
use uuid::Uuid;

use crate::backends::{
	backend::DatabaseBackend,
	error::{DatabaseError, DatabaseErrorKind, Result, map_sqlx_error},
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
		let result = query
			.execute(self.pool.as_ref())
			.await
			.map_err(map_sqlx_error)?;
		Ok(QueryResult {
			rows_affected: result.rows_affected(),
		})
	}

	async fn fetch_one(&self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let row = query
			.fetch_one(self.pool.as_ref())
			.await
			.map_err(map_sqlx_error)?;
		Self::convert_row(row)
	}

	async fn fetch_all(&self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let rows = query
			.fetch_all(self.pool.as_ref())
			.await
			.map_err(map_sqlx_error)?;
		rows.into_iter().map(Self::convert_row).collect()
	}

	async fn fetch_optional(&self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let row = query
			.fetch_optional(self.pool.as_ref())
			.await
			.map_err(map_sqlx_error)?;
		row.map(Self::convert_row).transpose()
	}

	async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
		let tx = self.pool.begin().await.map_err(map_sqlx_error)?;
		Ok(Box::new(PgTransactionExecutor::new(tx)))
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
			} else if let Ok(value) = pg_row.try_get::<rust_decimal::Decimal, _>(column_name) {
				// Convert DECIMAL/NUMERIC to f64 for Float storage
				if let Some(f) = value.to_f64() {
					row.insert(column_name.to_string(), QueryValue::Float(f));
				} else {
					return Err(Self::decimal_conversion_error(&value, column_name).into());
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
		DatabaseError::new(
			DatabaseErrorKind::Type,
			format!(
				"Failed to convert Decimal value '{}' to f64 for column '{}'",
				value, column_name
			),
		)
	}
}

#[async_trait]
impl TransactionExecutor for PgTransactionExecutor {
	async fn execute(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let result = query.execute(&mut **tx).await.map_err(map_sqlx_error)?;
		Ok(QueryResult {
			rows_affected: result.rows_affected(),
		})
	}

	async fn fetch_one(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let row = query.fetch_one(&mut **tx).await.map_err(map_sqlx_error)?;
		Self::convert_row(row)
	}

	async fn fetch_all(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let rows = query.fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
		rows.into_iter().map(Self::convert_row).collect()
	}

	async fn fetch_optional(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			DatabaseError::new(
				DatabaseErrorKind::Transaction,
				"Transaction already consumed",
			)
		})?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let row = query
			.fetch_optional(&mut **tx)
			.await
			.map_err(map_sqlx_error)?;
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
	use rstest::rstest;
	use rust_decimal::prelude::ToPrimitive;

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
		use crate::backends::error::DatabaseErrorKind;

		// Arrange
		let value = rust_decimal::Decimal::new(12345, 2);
		let column_name = "price_column";

		// Act
		let error = super::PostgresBackend::decimal_conversion_error(&value, column_name);

		// Assert
		assert_eq!(error.kind(), DatabaseErrorKind::Type);
		assert_eq!(
			error.to_string(),
			"Failed to convert Decimal value '123.45' to f64 for column 'price_column'"
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
