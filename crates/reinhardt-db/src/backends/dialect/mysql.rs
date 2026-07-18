//! MySQL dialect implementation

use async_trait::async_trait;
use sqlx::{Column, MySql, MySqlPool, Row as SqlxRow, Transaction, TypeInfo, mysql::MySqlRow};
use std::sync::Arc;

use crate::backends::{
	backend::DatabaseBackend,
	error::{DatabaseError, DatabaseErrorKind, Result, map_sqlx_error},
	types::{
		DatabaseType, IsolationLevel, QueryResult, QueryValue, Row, Savepoint, TransactionExecutor,
	},
};

fn transaction_consumed_error() -> DatabaseError {
	DatabaseError::new(
		DatabaseErrorKind::Transaction,
		"Transaction already consumed",
	)
}

/// MySQL database backend
pub struct MySqlBackend {
	pool: Arc<MySqlPool>,
}

impl MySqlBackend {
	/// Creates a new MySQL backend with the given pool.
	pub fn new(pool: MySqlPool) -> Self {
		Self {
			pool: Arc::new(pool),
		}
	}

	/// Returns a reference to the underlying MySQL pool.
	pub fn pool(&self) -> &MySqlPool {
		&self.pool
	}

	fn bind_value<'q>(
		query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
		value: &'q QueryValue,
	) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
		match value {
			QueryValue::Null => query.bind(None::<i32>),
			QueryValue::Bool(b) => query.bind(b),
			QueryValue::Int(i) => query.bind(i),
			QueryValue::Float(f) => query.bind(f),
			QueryValue::String(s) => query.bind(s),
			QueryValue::Bytes(b) => query.bind(b),
			QueryValue::Timestamp(dt) => query.bind(dt),
			// MySQL stores UUIDs as BINARY(16) or CHAR(36); we bind as string
			QueryValue::Uuid(u) => query.bind(u.to_string()),
			QueryValue::Json(value) => query.bind(value.as_deref().cloned().map(sqlx::types::Json)),
			QueryValue::StringArray(values) => {
				query.bind(serde_json::to_string(values).expect("string arrays serialize"))
			}
			QueryValue::IntArray(values) => {
				query.bind(serde_json::to_string(values).expect("integer arrays serialize"))
			}
			QueryValue::BigIntArray(values) => {
				query.bind(serde_json::to_string(values).expect("big integer arrays serialize"))
			}
			QueryValue::BoolArray(values) => {
				query.bind(serde_json::to_string(values).expect("boolean arrays serialize"))
			}
			QueryValue::FloatArray(values) => {
				query.bind(serde_json::to_string(values).expect("float arrays serialize"))
			}
			QueryValue::DoubleArray(values) => {
				query.bind(serde_json::to_string(values).expect("double arrays serialize"))
			}
			QueryValue::UuidArray(values) => {
				query.bind(serde_json::to_string(values).expect("UUID arrays serialize"))
			}
			QueryValue::Now => {
				// MySQL uses NOW() function, which should be part of SQL string
				// For binding, we use current UTC time
				query.bind(chrono::Utc::now())
			}
		}
	}

	fn convert_row(mysql_row: MySqlRow) -> Result<Row> {
		let mut row = Row::new();
		for column in mysql_row.columns() {
			let column_name = column.name();
			let type_name = column.type_info().name().to_uppercase();
			if type_name == "JSON" {
				match mysql_row.try_get::<Option<serde_json::Value>, _>(column_name) {
					Ok(Some(value)) => row.insert(
						column_name.to_string(),
						QueryValue::Json(Some(Box::new(value))),
					),
					Ok(None) => row.insert(column_name.to_string(), QueryValue::Null),
					Err(error) => return Err(map_sqlx_error(error).into()),
				};
				continue;
			}
			if matches!(type_name.as_str(), "DECIMAL" | "NEWDECIMAL") {
				match mysql_row.try_get::<Option<rust_decimal::Decimal>, _>(column_name) {
					Ok(Some(value)) => {
						row.insert(
							column_name.to_string(),
							QueryValue::String(value.to_string()),
						);
					}
					Ok(None) => row.insert(column_name.to_string(), QueryValue::Null),
					Err(error) => return Err(map_sqlx_error(error).into()),
				};
				continue;
			}
			if let Ok(value) = mysql_row.try_get::<bool, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Bool(value));
			} else if let Ok(value) = mysql_row.try_get::<i64, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Int(value));
			} else if let Ok(value) = mysql_row.try_get::<i32, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Int(value as i64));
			} else if let Ok(value) = mysql_row.try_get::<f64, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::Float(value));
			} else if let Ok(value) = mysql_row.try_get::<chrono::NaiveDate, _>(column_name) {
				row.insert(
					column_name.to_string(),
					QueryValue::String(value.to_string()),
				);
			} else if let Ok(value) = mysql_row.try_get::<chrono::NaiveTime, _>(column_name) {
				row.insert(
					column_name.to_string(),
					QueryValue::String(value.to_string()),
				);
			} else if let Ok(value) = mysql_row.try_get::<String, _>(column_name) {
				row.insert(column_name.to_string(), QueryValue::String(value));
			} else if let Ok(value) = mysql_row.try_get::<Vec<u8>, _>(column_name) {
				// MySQL 8.0 information_schema returns binary collation columns that
				// sqlx reports as LONGBLOB. Attempt UTF-8 conversion to recover string data.
				match String::from_utf8(value.clone()) {
					Ok(s) => row.insert(column_name.to_string(), QueryValue::String(s)),
					Err(_) => row.insert(column_name.to_string(), QueryValue::Bytes(value)),
				};
			} else if let Ok(value) = mysql_row.try_get::<chrono::NaiveDateTime, _>(column_name) {
				// MySQL TIMESTAMP/DATETIME without timezone
				row.insert(
					column_name.to_string(),
					QueryValue::Timestamp(chrono::DateTime::from_naive_utc_and_offset(
						value,
						chrono::Utc,
					)),
				);
			} else if let Ok(value) =
				mysql_row.try_get::<chrono::DateTime<chrono::Utc>, _>(column_name)
			{
				row.insert(column_name.to_string(), QueryValue::Timestamp(value));
			} else if mysql_row.try_get::<Option<i32>, _>(column_name).is_ok() {
				row.insert(column_name.to_string(), QueryValue::Null);
			}
		}
		Ok(row)
	}
}

#[async_trait]
impl DatabaseBackend for MySqlBackend {
	fn database_type(&self) -> DatabaseType {
		DatabaseType::Mysql
	}

	fn placeholder(&self, _index: usize) -> String {
		"?".to_string()
	}

	fn supports_returning(&self) -> bool {
		false
	}

	fn supports_on_conflict(&self) -> bool {
		false
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
		let mysql_row = query
			.fetch_one(self.pool.as_ref())
			.await
			.map_err(map_sqlx_error)?;
		Self::convert_row(mysql_row)
	}

	async fn fetch_all(&self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let mysql_rows = query
			.fetch_all(self.pool.as_ref())
			.await
			.map_err(map_sqlx_error)?;
		mysql_rows.into_iter().map(Self::convert_row).collect()
	}

	async fn fetch_optional(&self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let mysql_row = query
			.fetch_optional(self.pool.as_ref())
			.await
			.map_err(map_sqlx_error)?;
		mysql_row.map(Self::convert_row).transpose()
	}

	async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
		let tx = self.pool.begin().await.map_err(map_sqlx_error)?;
		Ok(Box::new(MySqlTransactionExecutor::new(tx)))
	}

	async fn begin_with_isolation(
		&self,
		isolation_level: IsolationLevel,
	) -> Result<Box<dyn TransactionExecutor>> {
		// MySQL requires SET TRANSACTION ISOLATION LEVEL before BEGIN.
		// We must set isolation and start the transaction on the same connection
		// to avoid a race condition where they run on different pool connections.
		//
		// Strategy: acquire a connection, set isolation level, send BEGIN manually,
		// then wrap the connection in a raw transaction executor that manages
		// COMMIT/ROLLBACK explicitly.
		let mut conn = self.pool.acquire().await.map_err(map_sqlx_error)?;

		let set_sql = format!(
			"SET TRANSACTION ISOLATION LEVEL {}",
			isolation_level.to_sql(DatabaseType::Mysql)
		);
		sqlx::query(&set_sql)
			.execute(&mut *conn)
			.await
			.map_err(map_sqlx_error)?;
		sqlx::query("BEGIN")
			.execute(&mut *conn)
			.await
			.map_err(map_sqlx_error)?;

		Ok(Box::new(MySqlRawTransactionExecutor::new(conn)))
	}

	fn as_any(&self) -> &dyn std::any::Any {
		self
	}
}

/// MySQL transaction executor
pub struct MySqlTransactionExecutor {
	tx: Option<Transaction<'static, MySql>>,
}

impl MySqlTransactionExecutor {
	/// Creates a new MySQL transaction executor.
	pub fn new(tx: Transaction<'static, MySql>) -> Self {
		Self { tx: Some(tx) }
	}

	fn bind_value<'q>(
		query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
		value: &'q QueryValue,
	) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
		match value {
			QueryValue::Null => query.bind(None::<i32>),
			QueryValue::Bool(b) => query.bind(b),
			QueryValue::Int(i) => query.bind(i),
			QueryValue::Float(f) => query.bind(f),
			QueryValue::String(s) => query.bind(s),
			QueryValue::Bytes(b) => query.bind(b),
			QueryValue::Timestamp(dt) => query.bind(dt),
			// MySQL stores UUIDs as BINARY(16) or CHAR(36); we bind as string
			QueryValue::Uuid(u) => query.bind(u.to_string()),
			QueryValue::Json(value) => query.bind(value.as_deref().cloned().map(sqlx::types::Json)),
			QueryValue::StringArray(values) => {
				query.bind(serde_json::to_string(values).expect("string arrays serialize"))
			}
			QueryValue::IntArray(values) => {
				query.bind(serde_json::to_string(values).expect("integer arrays serialize"))
			}
			QueryValue::BigIntArray(values) => {
				query.bind(serde_json::to_string(values).expect("big integer arrays serialize"))
			}
			QueryValue::BoolArray(values) => {
				query.bind(serde_json::to_string(values).expect("boolean arrays serialize"))
			}
			QueryValue::FloatArray(values) => {
				query.bind(serde_json::to_string(values).expect("float arrays serialize"))
			}
			QueryValue::DoubleArray(values) => {
				query.bind(serde_json::to_string(values).expect("double arrays serialize"))
			}
			QueryValue::UuidArray(values) => {
				query.bind(serde_json::to_string(values).expect("UUID arrays serialize"))
			}
			QueryValue::Now => query.bind(chrono::Utc::now()),
		}
	}

	fn convert_row(mysql_row: MySqlRow) -> Result<Row> {
		MySqlBackend::convert_row(mysql_row)
	}
}

#[async_trait]
impl TransactionExecutor for MySqlTransactionExecutor {
	async fn execute(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
		let tx = self.tx.as_mut().ok_or_else(transaction_consumed_error)?;

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
		let tx = self.tx.as_mut().ok_or_else(transaction_consumed_error)?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let row = query.fetch_one(&mut **tx).await.map_err(map_sqlx_error)?;
		Self::convert_row(row)
	}

	async fn fetch_all(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
		let tx = self.tx.as_mut().ok_or_else(transaction_consumed_error)?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let rows = query.fetch_all(&mut **tx).await.map_err(map_sqlx_error)?;
		rows.into_iter().map(Self::convert_row).collect()
	}

	async fn fetch_optional(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
		let tx = self.tx.as_mut().ok_or_else(transaction_consumed_error)?;

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
		let tx = self.tx.take().ok_or_else(transaction_consumed_error)?;
		tx.commit().await.map_err(map_sqlx_error)?;
		Ok(())
	}

	async fn rollback(mut self: Box<Self>) -> Result<()> {
		let tx = self.tx.take().ok_or_else(transaction_consumed_error)?;
		tx.rollback().await.map_err(map_sqlx_error)?;
		Ok(())
	}

	async fn savepoint(&mut self, name: &str) -> Result<()> {
		let tx = self.tx.as_mut().ok_or_else(transaction_consumed_error)?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.to_sql())
			.execute(&mut **tx)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
	}

	async fn release_savepoint(&mut self, name: &str) -> Result<()> {
		let tx = self.tx.as_mut().ok_or_else(transaction_consumed_error)?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.release_sql())
			.execute(&mut **tx)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
	}

	async fn rollback_to_savepoint(&mut self, name: &str) -> Result<()> {
		let tx = self.tx.as_mut().ok_or_else(transaction_consumed_error)?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.rollback_sql())
			.execute(&mut **tx)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
	}
}

/// MySQL raw transaction executor for isolation-level-aware transactions.
///
/// Unlike `MySqlTransactionExecutor` which wraps sqlx's `Transaction`, this type
/// manages a manually-started transaction on a pool connection. This is necessary
/// when we need to execute `SET TRANSACTION ISOLATION LEVEL` before `BEGIN` on
/// the same connection to avoid the race condition with connection pools.
struct MySqlRawTransactionExecutor {
	conn: Option<sqlx::pool::PoolConnection<MySql>>,
}

impl MySqlRawTransactionExecutor {
	/// Creates a new raw transaction executor wrapping a pool connection
	/// that already has an active transaction (BEGIN was sent manually).
	fn new(conn: sqlx::pool::PoolConnection<MySql>) -> Self {
		Self { conn: Some(conn) }
	}

	fn bind_value<'q>(
		query: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>,
		value: &'q QueryValue,
	) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
		MySqlTransactionExecutor::bind_value(query, value)
	}

	fn convert_row(mysql_row: MySqlRow) -> Result<Row> {
		MySqlTransactionExecutor::convert_row(mysql_row)
	}
}

#[async_trait]
impl TransactionExecutor for MySqlRawTransactionExecutor {
	async fn execute(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<QueryResult> {
		let conn = self.conn.as_mut().ok_or_else(transaction_consumed_error)?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let result = query.execute(&mut **conn).await.map_err(map_sqlx_error)?;
		Ok(QueryResult {
			rows_affected: result.rows_affected(),
		})
	}

	async fn fetch_one(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Row> {
		let conn = self.conn.as_mut().ok_or_else(transaction_consumed_error)?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let row = query.fetch_one(&mut **conn).await.map_err(map_sqlx_error)?;
		Self::convert_row(row)
	}

	async fn fetch_all(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<Row>> {
		let conn = self.conn.as_mut().ok_or_else(transaction_consumed_error)?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let rows = query.fetch_all(&mut **conn).await.map_err(map_sqlx_error)?;
		rows.into_iter().map(Self::convert_row).collect()
	}

	async fn fetch_optional(&mut self, sql: &str, params: Vec<QueryValue>) -> Result<Option<Row>> {
		let conn = self.conn.as_mut().ok_or_else(transaction_consumed_error)?;

		let mut query = sqlx::query(sql);
		for param in &params {
			query = Self::bind_value(query, param);
		}
		let row = query
			.fetch_optional(&mut **conn)
			.await
			.map_err(map_sqlx_error)?;
		row.map(Self::convert_row).transpose()
	}

	async fn commit(mut self: Box<Self>) -> Result<()> {
		let mut conn = self.conn.take().ok_or_else(transaction_consumed_error)?;
		sqlx::query("COMMIT")
			.execute(&mut *conn)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
	}

	async fn rollback(mut self: Box<Self>) -> Result<()> {
		let mut conn = self.conn.take().ok_or_else(transaction_consumed_error)?;
		sqlx::query("ROLLBACK")
			.execute(&mut *conn)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
	}

	async fn savepoint(&mut self, name: &str) -> Result<()> {
		let conn = self.conn.as_mut().ok_or_else(transaction_consumed_error)?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.to_sql())
			.execute(&mut **conn)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
	}

	async fn release_savepoint(&mut self, name: &str) -> Result<()> {
		let conn = self.conn.as_mut().ok_or_else(transaction_consumed_error)?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.release_sql())
			.execute(&mut **conn)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
	}

	async fn rollback_to_savepoint(&mut self, name: &str) -> Result<()> {
		let conn = self.conn.as_mut().ok_or_else(transaction_consumed_error)?;

		let sp = Savepoint::new(name);
		sqlx::query(&sp.rollback_sql())
			.execute(&mut **conn)
			.await
			.map_err(map_sqlx_error)?;
		Ok(())
	}
}
