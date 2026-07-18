//! Database connection management
//!
//! This module provides the main `DatabaseConnection` type which wraps
//! the backend-specific connection implementations.

use async_trait::async_trait;
use reinhardt_core::exception::{DatabaseError, DatabaseErrorKind, Result};

/// Re-export backends types
pub use crate::backends::connection::DatabaseConnection as BackendsConnection;
pub use crate::backends::types::{IsolationLevel, QueryValue, Row, TransactionExecutor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Defines possible database backend values.
pub enum DatabaseBackend {
	/// Postgres variant.
	Postgres,
	/// MySql variant.
	MySql,
	/// Sqlite variant.
	Sqlite,
}

/// Query row wrapper for ORM compatibility
#[derive(serde::Serialize)]
pub struct QueryRow {
	/// The data.
	pub data: serde_json::Value,
	#[serde(skip)]
	json_null_fields: std::collections::HashSet<String>,
	#[serde(skip)]
	native_json_fields: std::collections::HashSet<String>,
	// Allow dead_code: field reserved for future connection metadata tracking
	#[allow(dead_code)]
	#[serde(skip)]
	inner: Option<Row>,
}

impl QueryRow {
	/// Creates a new instance.
	pub fn new(data: serde_json::Value) -> Self {
		Self {
			data,
			json_null_fields: std::collections::HashSet::new(),
			native_json_fields: std::collections::HashSet::new(),
			inner: None,
		}
	}

	/// Creates an instance from backend row.
	pub fn from_backend_row(row: Row) -> Self {
		// Convert Row to JSON for backward compatibility
		let mut map = serde_json::Map::new();
		let mut json_null_fields = std::collections::HashSet::new();
		let mut native_json_fields = std::collections::HashSet::new();
		for (key, value) in row.data.iter() {
			let json_value = match value.clone() {
				QueryValue::Null => serde_json::Value::Null,
				QueryValue::Bool(b) => serde_json::Value::Bool(b),
				QueryValue::Int(i) => serde_json::Value::Number(i.into()),
				QueryValue::Float(f) => serde_json::Number::from_f64(f)
					.map(serde_json::Value::Number)
					.unwrap_or(serde_json::Value::Null),
				QueryValue::String(s) => serde_json::Value::String(s),
				QueryValue::Bytes(b) => {
					// Encode bytes as base64 string
					use base64::Engine;
					serde_json::Value::String(base64::engine::general_purpose::STANDARD.encode(&b))
				}
				QueryValue::Timestamp(dt) => serde_json::Value::String(dt.to_rfc3339()),
				QueryValue::Uuid(u) => serde_json::Value::String(u.to_string()),
				QueryValue::Json(Some(value)) => {
					native_json_fields.insert(key.clone());
					if value.is_null() {
						json_null_fields.insert(key.clone());
					}
					value.as_ref().clone()
				}
				QueryValue::Json(None) => {
					native_json_fields.insert(key.clone());
					serde_json::Value::Null
				}
				QueryValue::StringArray(values) => serde_json::Value::Array(
					values.into_iter().map(serde_json::Value::String).collect(),
				),
				QueryValue::IntArray(values) => serde_json::Value::Array(
					values.into_iter().map(serde_json::Value::from).collect(),
				),
				QueryValue::BigIntArray(values) => serde_json::Value::Array(
					values.into_iter().map(serde_json::Value::from).collect(),
				),
				QueryValue::BoolArray(values) => serde_json::Value::Array(
					values.into_iter().map(serde_json::Value::from).collect(),
				),
				QueryValue::FloatArray(values) => serde_json::Value::Array(
					values.into_iter().map(serde_json::Value::from).collect(),
				),
				QueryValue::DoubleArray(values) => serde_json::Value::Array(
					values.into_iter().map(serde_json::Value::from).collect(),
				),
				QueryValue::UuidArray(values) => serde_json::Value::Array(
					values
						.into_iter()
						.map(|value| serde_json::Value::String(value.to_string()))
						.collect(),
				),
				// NOW() should never appear in Row data (it's resolved to actual timestamp in database)
				QueryValue::Now => panic!("QueryValue::Now should not appear in Row data"),
			};
			map.insert(key.clone(), json_value);
		}

		Self {
			data: serde_json::Value::Object(map),
			json_null_fields,
			native_json_fields,
			inner: Some(row),
		}
	}

	pub(crate) fn deserialize_model<M: super::Model>(
		&self,
	) -> std::result::Result<M, super::FieldCodecError> {
		super::json::deserialize_model_row::<M>(
			self.data.clone(),
			self.json_null_fields.clone(),
			self.native_json_fields.clone(),
		)
	}

	/// Database columns returned as native JSON values.
	pub(crate) fn native_json_fields(&self) -> &std::collections::HashSet<String> {
		&self.native_json_fields
	}

	/// Database columns returned as native JSON null values.
	pub(crate) fn json_null_fields(&self) -> &std::collections::HashSet<String> {
		&self.json_null_fields
	}

	/// Get a value from the row by column name
	///
	/// This method extracts a value from the row's JSON data by key.
	/// Supports common types like i64, f64, bool, and String.
	pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
		self.data
			.get(key)
			.and_then(|v| serde_json::from_value(v.clone()).ok())
	}
}

#[async_trait]
/// Trait defining database executor behavior.
pub trait DatabaseExecutor: Send + Sync {
	/// Executes a SQL statement and returns the number of affected rows.
	async fn execute(&self, sql: &str) -> Result<u64>;
	/// Executes a SQL query and returns the resulting rows.
	async fn query(&self, sql: &str) -> Result<Vec<QueryRow>>;
}

/// Database connection wrapper
#[derive(Clone)]
pub struct DatabaseConnection {
	backend: DatabaseBackend,
	inner: BackendsConnection,
}

impl DatabaseConnection {
	/// Creates a new instance.
	pub fn new(backend: DatabaseBackend, inner: BackendsConnection) -> Self {
		Self { backend, inner }
	}

	/// Connect to a database from a connection URL
	///
	/// Automatically detects the database type from the URL scheme:
	/// - `postgres://` or `postgresql://` → PostgreSQL
	/// - `mysql://` → MySQL
	/// - `sqlite://` or `sqlite:` → SQLite
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_db::orm::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect("postgres://localhost/mydb").await.unwrap();
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn connect(url: &str) -> Result<Self> {
		Self::connect_with_pool_size(url, None).await
	}

	/// Connect to a PostgreSQL database
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_db::orm::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect_postgres("postgres://localhost/mydb").await.unwrap();
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	#[cfg(feature = "postgres")]
	pub async fn connect_postgres(url: &str) -> Result<Self> {
		let inner = BackendsConnection::connect_postgres(url).await?;
		Ok(Self {
			backend: DatabaseBackend::Postgres,
			inner,
		})
	}

	/// Connect to a MySQL database
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_db::orm::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect_mysql("mysql://localhost/mydb").await.unwrap();
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	#[cfg(feature = "mysql")]
	pub async fn connect_mysql(url: &str) -> Result<Self> {
		let inner = BackendsConnection::connect_mysql(url).await?;
		Ok(Self {
			backend: DatabaseBackend::MySql,
			inner,
		})
	}

	/// Connect to a SQLite database
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_db::orm::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect_sqlite("sqlite::memory:").await.unwrap();
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	#[cfg(feature = "sqlite")]
	pub async fn connect_sqlite(url: &str) -> Result<Self> {
		let inner = BackendsConnection::connect_sqlite(url).await?;
		Ok(Self {
			backend: DatabaseBackend::Sqlite,
			inner,
		})
	}

	/// Connect to a database with a specific connection pool size
	///
	/// # Arguments
	///
	/// * `url` - Database connection URL
	/// * `pool_size` - Maximum number of connections in the pool (None = use default)
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_db::orm::connection::DatabaseConnection;
	///
	/// // Use larger pool for high-concurrency scenarios
	/// let conn = DatabaseConnection::connect_with_pool_size(
	///     "postgres://localhost/mydb",
	///     Some(50)
	/// ).await.unwrap();
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	// Allow unused_variables because pool_size is only used with Postgres backend.
	// MySQL and SQLite backends don't support pool size configuration yet.
	#[allow(unused_variables)]
	pub async fn connect_with_pool_size(url: &str, pool_size: Option<u32>) -> Result<Self> {
		let backend_type = if url.starts_with("postgres://") || url.starts_with("postgresql://") {
			DatabaseBackend::Postgres
		} else if url.starts_with("mysql://") {
			DatabaseBackend::MySql
		} else if url.starts_with("sqlite://") || url.starts_with("sqlite:") {
			DatabaseBackend::Sqlite
		} else {
			return Err(DatabaseError::new(
				DatabaseErrorKind::Configuration,
				format!("Unsupported database URL scheme: {url}"),
			)
			.into());
		};

		#[cfg(feature = "postgres")]
		if backend_type == DatabaseBackend::Postgres {
			let inner = BackendsConnection::connect_postgres_with_pool_size(url, pool_size).await?;
			return Ok(Self {
				backend: backend_type,
				inner,
			});
		}

		#[cfg(feature = "mysql")]
		if backend_type == DatabaseBackend::MySql {
			let inner = BackendsConnection::connect_mysql(url).await?;
			return Ok(Self {
				backend: backend_type,
				inner,
			});
		}

		#[cfg(feature = "sqlite")]
		if backend_type == DatabaseBackend::Sqlite {
			let inner = BackendsConnection::connect_sqlite(url).await?;
			return Ok(Self {
				backend: backend_type,
				inner,
			});
		}

		Err(DatabaseError::new(
			DatabaseErrorKind::Configuration,
			format!(
				"Database backend not compiled in. Enable the '{}' feature.",
				match backend_type {
					DatabaseBackend::Postgres => "postgres",
					DatabaseBackend::MySql => "mysql",
					DatabaseBackend::Sqlite => "sqlite",
				}
			),
		)
		.into())
	}

	/// Performs the backend operation.
	pub fn backend(&self) -> DatabaseBackend {
		self.backend
	}

	/// Get a reference to the inner backends connection
	///
	/// This provides access to the low-level connection for operations
	/// that require direct database access.
	pub fn inner(&self) -> &BackendsConnection {
		&self.inner
	}

	/// Consume self and return the inner backends connection
	///
	/// This is useful when you need to pass ownership of the connection
	/// to functions that expect a `BackendsConnection`.
	pub fn into_inner(self) -> BackendsConnection {
		self.inner
	}

	/// Execute a SQL query and return a single row
	pub async fn query_one(&self, sql: &str, params: Vec<QueryValue>) -> Result<QueryRow> {
		let row = self.inner.fetch_one(sql, params).await?;
		Ok(QueryRow::from_backend_row(row))
	}

	/// Execute a SQL query and return an optional row
	pub async fn query_optional(
		&self,
		sql: &str,
		params: Vec<QueryValue>,
	) -> Result<Option<QueryRow>> {
		match self.inner.fetch_one(sql, params).await {
			Ok(row) => Ok(Some(QueryRow::from_backend_row(row))),
			Err(_) => Ok(None),
		}
	}

	/// Execute a SQL statement (INSERT, UPDATE, DELETE, etc.)
	pub async fn execute(&self, sql: &str, params: Vec<QueryValue>) -> Result<u64> {
		let result = self.inner.execute(sql, params).await?;
		Ok(result.rows_affected)
	}

	/// Execute a SQL query and return all rows
	pub async fn query(&self, sql: &str, params: Vec<QueryValue>) -> Result<Vec<QueryRow>> {
		let rows = self.inner.fetch_all(sql, params).await?;
		Ok(rows.into_iter().map(QueryRow::from_backend_row).collect())
	}

	/// Begin a database transaction
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_db::orm::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect("sqlite::memory:").await.unwrap();
	/// let result = conn.begin_transaction().await;
	/// assert!(result.is_ok());
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn begin_transaction(&self) -> Result<()> {
		self.execute("BEGIN TRANSACTION", vec![]).await?;
		Ok(())
	}

	/// Begin a transaction with a specific isolation level
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_db::orm::connection::DatabaseConnection;
	/// use reinhardt_db::orm::transaction::IsolationLevel;
	///
	/// let conn = DatabaseConnection::connect("sqlite::memory:").await.unwrap();
	/// let result = conn.begin_transaction_with_isolation(IsolationLevel::Serializable).await;
	/// assert!(result.is_ok());
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn begin_transaction_with_isolation(
		&self,
		level: super::transaction::IsolationLevel,
	) -> Result<()> {
		let sql = format!("BEGIN TRANSACTION ISOLATION LEVEL {}", level.to_sql());
		self.execute(&sql, vec![]).await?;
		Ok(())
	}

	/// Commit the current transaction
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_db::orm::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect("sqlite::memory:").await.unwrap();
	/// conn.begin_transaction().await.unwrap();
	/// // ... perform operations ...
	/// let result = conn.commit_transaction().await;
	/// assert!(result.is_ok());
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn commit_transaction(&self) -> Result<()> {
		self.execute("COMMIT", vec![]).await?;
		Ok(())
	}

	/// Rollback the current transaction
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_db::orm::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect("sqlite::memory:").await.unwrap();
	/// conn.begin_transaction().await.unwrap();
	/// // ... error occurs ...
	/// let result = conn.rollback_transaction().await;
	/// assert!(result.is_ok());
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn rollback_transaction(&self) -> Result<()> {
		self.execute("ROLLBACK", vec![]).await?;
		Ok(())
	}

	/// Create a savepoint for nested transactions
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_db::orm::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect("sqlite::memory:").await.unwrap();
	/// conn.begin_transaction().await.unwrap();
	/// let result = conn.savepoint("sp1").await;
	/// assert!(result.is_ok());
	/// // ... nested operations ...
	/// conn.release_savepoint("sp1").await.unwrap();
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn savepoint(&self, name: &str) -> Result<()> {
		let sql = format!("SAVEPOINT {}", name);
		self.execute(&sql, vec![]).await?;
		Ok(())
	}

	/// Release a savepoint
	pub async fn release_savepoint(&self, name: &str) -> Result<()> {
		let sql = format!("RELEASE SAVEPOINT {}", name);
		self.execute(&sql, vec![]).await?;
		Ok(())
	}

	/// Rollback to a savepoint
	pub async fn rollback_to_savepoint(&self, name: &str) -> Result<()> {
		let sql = format!("ROLLBACK TO SAVEPOINT {}", name);
		self.execute(&sql, vec![]).await?;
		Ok(())
	}

	/// Begin a database transaction and return a dedicated executor
	///
	/// This method acquires a dedicated database connection and begins a
	/// transaction on it. All queries executed through the returned
	/// `TransactionExecutor` are guaranteed to run on the same physical
	/// connection, ensuring proper transaction isolation.
	///
	/// # Returns
	///
	/// A boxed `TransactionExecutor` that holds the dedicated connection
	/// and provides methods for executing queries within the transaction.
	///
	/// # Example
	///
	/// ```no_run
	/// # async fn example() -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_db::orm::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect("postgres://localhost/mydb").await?;
	/// let mut tx = conn.begin().await?;
	///
	/// tx.execute("INSERT INTO users (name) VALUES ($1)", vec!["Alice".into()]).await?;
	/// tx.commit().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
		self.inner.begin().await
	}

	/// Begin a transaction with a specific isolation level using TransactionExecutor
	///
	/// This method returns a `TransactionExecutor` that provides dedicated connection
	/// semantics with the specified isolation level. All queries executed through
	/// the returned executor are guaranteed to run on the same physical connection.
	///
	/// # Arguments
	///
	/// * `level` - The desired isolation level for the transaction
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() -> reinhardt_core::exception::Result<()> {
	/// use reinhardt_db::orm::connection::{DatabaseConnection, IsolationLevel};
	///
	/// let conn = DatabaseConnection::connect("postgres://localhost/mydb").await?;
	/// let mut tx = conn.begin_with_isolation(IsolationLevel::Serializable).await?;
	///
	/// tx.execute("INSERT INTO users (name) VALUES ($1)", vec!["Alice".into()]).await?;
	/// tx.commit().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn begin_with_isolation(
		&self,
		level: IsolationLevel,
	) -> Result<Box<dyn TransactionExecutor>> {
		self.inner.begin_with_isolation(level).await
	}
}

#[async_trait]
impl DatabaseExecutor for DatabaseConnection {
	async fn execute(&self, sql: &str) -> Result<u64> {
		self.execute(sql, vec![]).await
	}

	async fn query(&self, sql: &str) -> Result<Vec<QueryRow>> {
		self.query(sql, vec![]).await
	}
}

/// Injectable implementation for DatabaseConnection
///
/// DatabaseConnection must be explicitly registered in the DI context using
/// `InjectionContextBuilder::singleton()`. It cannot be auto-injected because
/// it requires runtime configuration (connection URL, pool settings, etc.).
///
/// # Example
///
/// ```rust,no_run
/// use reinhardt_db::orm::DatabaseConnection;
/// use reinhardt_di::InjectionContext;
///
/// # async fn example() {
/// // First, establish a database connection
/// let db = DatabaseConnection::connect("postgres://localhost/mydb").await.unwrap();
///
/// // Then register it in the DI context as a singleton
/// let singleton_scope = reinhardt_di::SingletonScope::new();
/// let ctx = InjectionContext::builder(singleton_scope)
///     .singleton(db)
///     .build();
/// # }
/// ```
#[cfg(feature = "di")]
#[async_trait]
impl reinhardt_di::Injectable for DatabaseConnection {
	async fn inject(ctx: &reinhardt_di::InjectionContext) -> reinhardt_di::DiResult<Self> {
		// Try singleton scope first (primary expected location)
		if let Some(conn) = ctx.get_singleton::<Self>() {
			return Ok(std::sync::Arc::try_unwrap(conn).unwrap_or_else(|arc| (*arc).clone()));
		}

		// Try request scope as fallback
		if let Some(conn) = ctx.get_request::<Self>() {
			return Ok(std::sync::Arc::try_unwrap(conn).unwrap_or_else(|arc| (*arc).clone()));
		}

		// Not registered - provide helpful error
		Err(reinhardt_di::DiError::NotRegistered {
			type_name: std::any::type_name::<Self>().to_string(),
			hint: "Use InjectionContextBuilder::singleton(db_connection) to register a \
			       DatabaseConnection. Create it with DatabaseConnection::connect(), \
			       connect_postgres(), connect_sqlite(), or connect_mysql()."
				.to_string(),
		})
	}

	async fn inject_uncached(ctx: &reinhardt_di::InjectionContext) -> reinhardt_di::DiResult<Self> {
		// For DatabaseConnection, inject_uncached behaves the same as inject
		// since database connections are typically shared (singleton or request-scoped)
		Self::inject(ctx).await
	}
}

#[cfg(test)]
mod tests {
	use super::DatabaseConnection;
	use crate::backends::DatabaseErrorKind;

	#[tokio::test]
	async fn test_error_kind_for_unsupported_url_scheme() {
		let Err(error) = DatabaseConnection::connect("unsupported://database").await else {
			panic!("an unsupported URL scheme must fail");
		};

		assert_eq!(
			error.database_kind(),
			Some(DatabaseErrorKind::Configuration)
		);
	}

	#[cfg(feature = "postgres")]
	#[tokio::test]
	async fn test_error_kind_for_refused_postgres_connection() {
		let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
			.expect("a local ephemeral port must be available");
		let address = listener
			.local_addr()
			.expect("the bound listener must have a local address");
		drop(listener);
		let url = format!(
			"postgres://postgres@{}:{}/postgres?connect_timeout=1",
			address.ip(),
			address.port()
		);

		let Err(error) = DatabaseConnection::connect_postgres(&url).await else {
			panic!("a closed local endpoint must refuse the connection");
		};

		assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Connection));
	}

	#[cfg(feature = "sqlite")]
	#[tokio::test]
	async fn test_error_kind_for_missing_sqlite_column() {
		let connection = DatabaseConnection::connect_sqlite("sqlite::memory:")
			.await
			.expect("the in-memory SQLite database must connect");
		connection
			.execute("CREATE TABLE records (id INTEGER PRIMARY KEY)", vec![])
			.await
			.expect("the fixture table must be created");

		let Err(error) = connection
			.query("SELECT missing_column FROM records", vec![])
			.await
		else {
			panic!("querying a missing column must fail");
		};

		assert_eq!(error.database_kind(), Some(DatabaseErrorKind::Query));
	}
}
