//! SQLite dialect implementation

use async_trait::async_trait;
use sqlx::{Column, Row as SqlxRow, Sqlite, SqlitePool, Transaction, sqlite::SqliteRow};
use std::sync::Arc;

use crate::{
	backend::DatabaseBackend,
	error::Result,
	types::{
		DatabaseType, IsolationLevel, QueryResult, QueryValue, Row, Savepoint, TransactionExecutor,
	},
};

/// SQLite database backend
pub struct SqliteBackend {
	pool: Arc<SqlitePool>,
}

impl SqliteBackend {
	pub fn new(pool: SqlitePool) -> Self {
		Self {
			pool: Arc::new(pool),
		}
	}

	fn bind_value<'q>(
		query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
		value: &'q QueryValue,
	) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
		match value {
			QueryValue::Null => query.bind(None::<i32>),
			QueryValue::Bool(b) => query.bind(b),
			QueryValue::Int(i) => query.bind(i),
			QueryValue::Float(f) => query.bind(f),
			QueryValue::String(s) => query.bind(s),
			QueryValue::Bytes(b) => query.bind(b),
			QueryValue::Timestamp(dt) => query.bind(dt),
			QueryValue::Now => {
				// SQLite uses datetime('now'), which should be part of SQL string
				// For binding, we use current UTC time
				query.bind(chrono::Utc::now())
			}
		}
	}

	fn convert_row(sqlite_row: SqliteRow) -> Result<Row> {
		let mut row = Row::new();
		for column in sqlite_row.columns() {
			let column_name = column.name();
			if let Ok(value) = sqlite_row.try_get::<bool, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Bool(value));
			} else if let Ok(value) = sqlite_row.try_get::<i64, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Int(value));
			} else if let Ok(value) = sqlite_row.try_get::<i32, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Int(value as i64));
			} else if let Ok(value) = sqlite_row.try_get::<f64, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Float(value));
			} else if let Ok(value) = sqlite_row.try_get::<String, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::String(value));
			} else if let Ok(value) = sqlite_row.try_get::<Vec<u8>, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Bytes(value));
			} else if let Ok(value) = sqlite_row.try_get::<chrono::NaiveDateTime, _>(column_name) {
				// SQLite stores timestamps as strings/integers, convert to DateTime<Utc>
				row.insert(
					column_name.to_string(),
					QueryValue::Timestamp(chrono::DateTime::from_naive_utc_and_offset(
						value,
						chrono::Utc,
					)),
				);
			} else if let Ok(value) =
				sqlite_row.try_get::<chrono::DateTime<chrono::Utc>, _>(column_name)
			{
				row.insert(column_name.to_string(), QueryValue::Timestamp(value));
			} else if sqlite_row.try_get::<Option<i32>, _>(column_name).is_ok() {
				row.insert(column_name.to_string(), QueryValue::Null);
			}
		}
		Ok(row)
	}
}

#[async_trait]
impl DatabaseBackend for SqliteBackend {
	fn database_type(&self) -> DatabaseType {
		DatabaseType::Sqlite
	}

	fn placeholder(&self, _index: usize) -> String {
		"?".to_string()
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
		Ok(Box::new(SqliteTransactionExecutor::new(tx)))
	}

	async fn begin_with_isolation(
		&self,
		isolation_level: IsolationLevel,
	) -> Result<Box<dyn TransactionExecutor>> {
		// SQLite has limited isolation level support
		// It only supports DEFERRED (default), IMMEDIATE, and EXCLUSIVE
		// Serializable maps to EXCLUSIVE, others use default DEFERRED
		let begin_sql = isolation_level.begin_transaction_sql(DatabaseType::Sqlite);

		// SQLite requires special handling - we can't use pool.begin() directly
		// when we want to specify a transaction type
		// For now, we use pool.begin() and document the limitation
		// SQLite in WAL mode provides serializable isolation by default anyway

		let tx = self.pool.begin().await?;
		if matches!(isolation_level, IsolationLevel::Serializable) {
			// Note: This doesn't actually change the transaction type after BEGIN
			// For true EXCLUSIVE behavior, users should use raw SQL
			// This is a known SQLite limitation with connection pooling
			let _ = begin_sql; // Acknowledge the generated SQL
		}
		Ok(Box::new(SqliteTransactionExecutor::new(tx)))
	}

	fn as_any(&self) -> &dyn std::any::Any {
		self
	}
}

/// SQLite transaction executor
pub struct SqliteTransactionExecutor {
	tx: Option<Transaction<'static, Sqlite>>,
}

impl SqliteTransactionExecutor {
	pub fn new(tx: Transaction<'static, Sqlite>) -> Self {
		Self { tx: Some(tx) }
	}

	fn bind_value<'q>(
		query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
		value: &'q QueryValue,
	) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
		match value {
			QueryValue::Null => query.bind(None::<i32>),
			QueryValue::Bool(b) => query.bind(b),
			QueryValue::Int(i) => query.bind(i),
			QueryValue::Float(f) => query.bind(f),
			QueryValue::String(s) => query.bind(s),
			QueryValue::Bytes(b) => query.bind(b),
			QueryValue::Timestamp(dt) => query.bind(dt),
			QueryValue::Now => query.bind(chrono::Utc::now()),
		}
	}

	fn convert_row(sqlite_row: SqliteRow) -> Result<Row> {
		let mut row = Row::new();
		for column in sqlite_row.columns() {
			let column_name = column.name();
			if let Ok(value) = sqlite_row.try_get::<bool, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Bool(value));
			} else if let Ok(value) = sqlite_row.try_get::<i64, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Int(value));
			} else if let Ok(value) = sqlite_row.try_get::<i32, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Int(value as i64));
			} else if let Ok(value) = sqlite_row.try_get::<f64, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Float(value));
			} else if let Ok(value) = sqlite_row.try_get::<String, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::String(value));
			} else if let Ok(value) = sqlite_row.try_get::<Vec<u8>, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Bytes(value));
			} else if let Ok(value) = sqlite_row.try_get::<chrono::NaiveDateTime, _>(column_name) {
				row.insert(
					column_name.to_string(),
					QueryValue::Timestamp(chrono::DateTime::from_naive_utc_and_offset(
						value,
						chrono::Utc,
					)),
				);
			} else if let Ok(value) =
				sqlite_row.try_get::<chrono::DateTime<chrono::Utc>, _>(column_name)
			{
				row.insert(column_name.to_string(), QueryValue::Timestamp(value));
			} else if sqlite_row.try_get::<Option<i32>, _>(column_name).is_ok() {
				row.insert(column_name.to_string(), QueryValue::Null);
			}
		}
		Ok(row)
	}
}

#[async_trait]
impl TransactionExecutor for SqliteTransactionExecutor {
	async fn execute(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			crate::error::DatabaseError::TransactionError(
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
			crate::error::DatabaseError::TransactionError(
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
			crate::error::DatabaseError::TransactionError(
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
			crate::error::DatabaseError::TransactionError(
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
			crate::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;
		tx.commit().await?;
		Ok(())
	}

	async fn rollback(mut self: Box<Self>) -> Result<()> {
		let tx = self.tx.take().ok_or_else(|| {
			crate::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;
		tx.rollback().await?;
		Ok(())
	}

	async fn savepoint(&mut self, name: &str) -> Result<()> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			crate::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.to_sql()).execute(&mut **tx).await?;
		Ok(())
	}

	async fn release_savepoint(&mut self, name: &str) -> Result<()> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			crate::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.release_sql()).execute(&mut **tx).await?;
		Ok(())
	}

	async fn rollback_to_savepoint(&mut self, name: &str) -> Result<()> {
		let tx = self.tx.as_mut().ok_or_else(|| {
			crate::error::DatabaseError::TransactionError(
				"Transaction already consumed".to_string(),
			)
		})?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.rollback_sql()).execute(&mut **tx).await?;
		Ok(())
	}
}
