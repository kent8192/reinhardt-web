//! Database connection management
//!
//! This module provides the main `DatabaseConnection` type which wraps
//! the backend-specific connection implementations.

use async_trait::async_trait;

/// Re-export backends types
pub use backends::connection::DatabaseConnection as BackendsConnection;
pub use backends::types::{QueryValue, Row};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseBackend {
	Postgres,
	MySql,
	Sqlite,
}

/// Query row wrapper for ORM compatibility
pub struct QueryRow {
	pub data: serde_json::Value,
	#[allow(dead_code)]
	inner: Option<Row>,
}

impl QueryRow {
	pub fn new(data: serde_json::Value) -> Self {
		Self { data, inner: None }
	}

	pub fn from_backend_row(row: Row) -> Self {
		// Convert Row to JSON for backward compatibility
		let mut map = serde_json::Map::new();
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
			};
			map.insert(key.clone(), json_value);
		}

		Self {
			data: serde_json::Value::Object(map),
			inner: Some(row),
		}
	}

	pub fn get<T>(&self, _key: &str) -> Option<T> {
		None
	}
}

#[async_trait]
pub trait DatabaseExecutor: Send + Sync {
	async fn execute(&self, sql: &str) -> Result<u64, anyhow::Error>;
	async fn query(&self, sql: &str) -> Result<Vec<QueryRow>, anyhow::Error>;
}

/// Database connection wrapper
#[derive(Clone)]
pub struct DatabaseConnection {
	backend: DatabaseBackend,
	inner: BackendsConnection,
}

impl DatabaseConnection {
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
	/// use reinhardt_orm::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect("postgres://localhost/mydb").await.unwrap();
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn connect(url: &str) -> Result<Self, anyhow::Error> {
		let backend_type = if url.starts_with("postgres://") || url.starts_with("postgresql://") {
			DatabaseBackend::Postgres
		} else if url.starts_with("mysql://") {
			DatabaseBackend::MySql
		} else if url.starts_with("sqlite://") || url.starts_with("sqlite:") {
			DatabaseBackend::Sqlite
		} else {
			return Err(anyhow::anyhow!("Unsupported database URL scheme: {}", url));
		};

		#[cfg(feature = "postgres")]
		if backend_type == DatabaseBackend::Postgres {
			let inner = BackendsConnection::connect_postgres(url).await?;
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

		Err(anyhow::anyhow!(
			"Database backend not compiled in. Enable the '{}' feature.",
			match backend_type {
				DatabaseBackend::Postgres => "postgres",
				DatabaseBackend::MySql => "mysql",
				DatabaseBackend::Sqlite => "sqlite",
			}
		))
	}

	pub fn backend(&self) -> DatabaseBackend {
		self.backend
	}

	/// Execute a SQL query and return a single row
	pub async fn query_one(&self, sql: &str) -> Result<QueryRow, anyhow::Error> {
		let row = self.inner.fetch_one(sql, vec![]).await?;
		Ok(QueryRow::from_backend_row(row))
	}

	/// Execute a SQL query and return an optional row
	pub async fn query_optional(&self, sql: &str) -> Result<Option<QueryRow>, anyhow::Error> {
		match self.inner.fetch_one(sql, vec![]).await {
			Ok(row) => Ok(Some(QueryRow::from_backend_row(row))),
			Err(_) => Ok(None),
		}
	}

	/// Execute a SQL statement (INSERT, UPDATE, DELETE, etc.)
	pub async fn execute(&self, sql: &str) -> Result<u64, anyhow::Error> {
		let result = self.inner.execute(sql, vec![]).await?;
		Ok(result.rows_affected)
	}

	/// Execute a SQL query and return all rows
	pub async fn query(&self, sql: &str) -> Result<Vec<QueryRow>, anyhow::Error> {
		let rows = self.inner.fetch_all(sql, vec![]).await?;
		Ok(rows.into_iter().map(QueryRow::from_backend_row).collect())
	}

	/// Begin a database transaction
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_orm::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect("sqlite::memory:").await.unwrap();
	/// let result = conn.begin_transaction().await;
	/// assert!(result.is_ok());
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn begin_transaction(&self) -> Result<(), anyhow::Error> {
		self.execute("BEGIN TRANSACTION").await?;
		Ok(())
	}

	/// Begin a transaction with a specific isolation level
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_orm::connection::DatabaseConnection;
	/// use reinhardt_orm::transaction::IsolationLevel;
	///
	/// let conn = DatabaseConnection::connect("sqlite::memory:").await.unwrap();
	/// let result = conn.begin_transaction_with_isolation(IsolationLevel::Serializable).await;
	/// assert!(result.is_ok());
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn begin_transaction_with_isolation(
		&self,
		level: crate::transaction::IsolationLevel,
	) -> Result<(), anyhow::Error> {
		let sql = format!("BEGIN TRANSACTION ISOLATION LEVEL {}", level.to_sql());
		self.execute(&sql).await?;
		Ok(())
	}

	/// Commit the current transaction
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_orm::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect("sqlite::memory:").await.unwrap();
	/// conn.begin_transaction().await.unwrap();
	/// // ... perform operations ...
	/// let result = conn.commit_transaction().await;
	/// assert!(result.is_ok());
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn commit_transaction(&self) -> Result<(), anyhow::Error> {
		self.execute("COMMIT").await?;
		Ok(())
	}

	/// Rollback the current transaction
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_orm::connection::DatabaseConnection;
	///
	/// let conn = DatabaseConnection::connect("sqlite::memory:").await.unwrap();
	/// conn.begin_transaction().await.unwrap();
	/// // ... error occurs ...
	/// let result = conn.rollback_transaction().await;
	/// assert!(result.is_ok());
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn rollback_transaction(&self) -> Result<(), anyhow::Error> {
		self.execute("ROLLBACK").await?;
		Ok(())
	}

	/// Create a savepoint for nested transactions
	///
	/// # Examples
	///
	/// ```no_run
	/// # async fn example() {
	/// use reinhardt_orm::connection::DatabaseConnection;
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
	pub async fn savepoint(&self, name: &str) -> Result<(), anyhow::Error> {
		let sql = format!("SAVEPOINT {}", name);
		self.execute(&sql).await?;
		Ok(())
	}

	/// Release a savepoint
	pub async fn release_savepoint(&self, name: &str) -> Result<(), anyhow::Error> {
		let sql = format!("RELEASE SAVEPOINT {}", name);
		self.execute(&sql).await?;
		Ok(())
	}

	/// Rollback to a savepoint
	pub async fn rollback_to_savepoint(&self, name: &str) -> Result<(), anyhow::Error> {
		let sql = format!("ROLLBACK TO SAVEPOINT {}", name);
		self.execute(&sql).await?;
		Ok(())
	}
}

#[async_trait]
impl DatabaseExecutor for DatabaseConnection {
	async fn execute(&self, sql: &str) -> Result<u64, anyhow::Error> {
		self.execute(sql).await
	}

	async fn query(&self, sql: &str) -> Result<Vec<QueryRow>, anyhow::Error> {
		self.query(sql).await
	}
}
