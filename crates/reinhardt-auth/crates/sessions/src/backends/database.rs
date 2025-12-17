//! Database-backed session storage
//!
//! This module provides session storage using a database backend (PostgreSQL, MySQL, SQLite).
//! Sessions are persisted to a database table, making them survive application restarts.
//!
//! ## Features
//!
//! - Persistent session storage
//! - Automatic session expiration cleanup
//! - Support for multiple database backends
//!
//! ## Example
//!
//! ```rust,no_run
//! use reinhardt_sessions::backends::{DatabaseSessionBackend, SessionBackend};
//! use serde_json::json;
//!
//! # async fn example() {
//! // Create a database session backend
//! // Note: For actual usage, any database URL is supported (postgres://, mysql://, sqlite:)
//! let backend = DatabaseSessionBackend::new("sqlite::memory:").await.unwrap();
//! backend.create_table().await.unwrap();
//!
//! // Store user session
//! let session_data = json!({
//!     "user_id": 42,
//!     "username": "alice",
//!     "authenticated": true,
//! });
//!
//! backend.save("session_key_123", &session_data, Some(3600)).await.unwrap();
//!
//! // Retrieve session
//! let retrieved: Option<serde_json::Value> = backend.load("session_key_123").await.unwrap();
//! assert!(retrieved.is_some());
//! assert_eq!(retrieved.unwrap()["user_id"], 42);
//!
//! // Clean up expired sessions
//! let deleted_count = backend.cleanup_expired().await.unwrap();
//! assert_eq!(deleted_count, 0); // No expired sessions
//! # }
//! # tokio::runtime::Runtime::new().unwrap().block_on(example());
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{AnyPool, Row};
use std::sync::Arc;

use crate::{CleanupableBackend, SessionMetadata};

use super::cache::{SessionBackend, SessionError};

#[cfg(feature = "database")]
use sea_query::{
	Alias, ColumnDef, Expr, ExprTrait, Index, OnConflict, PostgresQueryBuilder, Query,
	SqliteQueryBuilder, Table,
};

/// Database session model
///
/// Represents a session stored in the database with expiration information.
///
/// ## Example
///
/// ```rust
/// use reinhardt_sessions::backends::database::SessionModel;
/// use chrono::Utc;
///
/// let session = SessionModel {
///     session_key: "abc123".to_string(),
///     session_data: serde_json::json!({"user_id": 42}),
///     expire_date: Utc::now() + chrono::Duration::hours(1),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionModel {
	/// Unique session key (primary key)
	pub session_key: String,
	/// Session data stored as JSON
	pub session_data: serde_json::Value,
	/// Session expiration timestamp
	pub expire_date: DateTime<Utc>,
}

/// Database-backed session storage
///
/// Stores sessions in a database table with automatic expiration handling.
/// Supports PostgreSQL, MySQL, and SQLite through sqlx's `Any` driver.
///
/// ## Database Schema
///
/// The backend expects a table with the following structure:
///
/// ```sql
/// CREATE TABLE sessions (
///     session_key VARCHAR(255) PRIMARY KEY,
///     session_data TEXT NOT NULL,
///     expire_date BIGINT NOT NULL,
///     created_at BIGINT NOT NULL,
///     last_accessed BIGINT
/// );
/// CREATE INDEX idx_sessions_expire_date ON sessions(expire_date);
/// ```
///
/// Note: Timestamps are stored as Unix timestamps (milliseconds since epoch) in BIGINT columns
/// for compatibility with sqlx's `Any` driver across different database backends.
///
/// ## Example
///
/// ```rust,no_run
/// use reinhardt_sessions::backends::{DatabaseSessionBackend, SessionBackend};
/// use serde_json::json;
///
/// # async fn example() {
/// // Initialize backend with database URL
/// let backend = DatabaseSessionBackend::new("sqlite::memory:").await.unwrap();
///
/// // Create the sessions table
/// backend.create_table().await.unwrap();
///
/// // Store session with 1 hour TTL
/// let data = json!({"cart_total": 99.99});
/// backend.save("cart_xyz", &data, Some(3600)).await.unwrap();
///
/// // Check if session exists
/// let exists = backend.exists("cart_xyz").await.unwrap();
/// assert!(exists);
/// # }
/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
/// ```
#[derive(Clone)]
pub struct DatabaseSessionBackend {
	pool: Arc<AnyPool>,
}

impl DatabaseSessionBackend {
	/// Create a new database session backend
	///
	/// Initializes a connection pool to the specified database URL.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_sessions::backends::DatabaseSessionBackend;
	///
	/// # async fn example() {
	/// // Supports multiple database backends:
	/// // - PostgreSQL: "postgres://localhost/mydb"
	/// // - MySQL: "mysql://localhost/mydb"
	/// // - SQLite (in-memory): "sqlite::memory:"
	/// // - SQLite (file): "sqlite://sessions.db"
	///
	/// let backend = DatabaseSessionBackend::new("sqlite::memory:").await.unwrap();
	/// // Backend created successfully
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn new(database_url: &str) -> Result<Self, SessionError> {
		// Install default drivers for AnyPool (required for sqlx v0.8)
		sqlx::any::install_default_drivers();

		let pool = AnyPool::connect(database_url)
			.await
			.map_err(|e| SessionError::CacheError(format!("Database connection error: {}", e)))?;

		Ok(Self {
			pool: Arc::new(pool),
		})
	}

	/// Create a new backend from an existing pool
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_sessions::backends::DatabaseSessionBackend;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// # async fn example() {
	/// let pool = AnyPool::connect("sqlite::memory:").await.unwrap();
	/// let backend = DatabaseSessionBackend::from_pool(Arc::new(pool));
	/// // Backend created from existing pool
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub fn from_pool(pool: Arc<AnyPool>) -> Self {
		Self { pool }
	}

	/// Create the sessions table if it doesn't exist
	///
	/// Creates the required database table for session storage.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_sessions::backends::DatabaseSessionBackend;
	///
	/// # async fn example() {
	/// let backend = DatabaseSessionBackend::new("sqlite::memory:").await.unwrap();
	/// backend.create_table().await.unwrap();
	/// // Table created successfully
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn create_table(&self) -> Result<(), SessionError> {
		// Use a transaction to ensure both CREATE TABLE and CREATE INDEX
		// execute on the same connection (important for SQLite in-memory databases)
		let mut tx =
			self.pool.begin().await.map_err(|e| {
				SessionError::CacheError(format!("Failed to begin transaction: {}", e))
			})?;

		// Use sea-query for CREATE TABLE
		// Note: Using INTEGER for timestamps to store Unix timestamp (compatible with AnyPool)
		let create_table_stmt = Table::create()
			.table(Alias::new("sessions"))
			.if_not_exists()
			.col(
				ColumnDef::new(Alias::new("session_key"))
					.text()
					.not_null()
					.primary_key(),
			)
			.col(ColumnDef::new(Alias::new("session_data")).text().not_null())
			.col(
				ColumnDef::new(Alias::new("expire_date"))
					.big_integer()
					.not_null(),
			)
			.col(
				ColumnDef::new(Alias::new("created_at"))
					.big_integer()
					.not_null(),
			)
			.col(ColumnDef::new(Alias::new("last_accessed")).big_integer())
			.to_owned();
		let create_table_sql = create_table_stmt.to_string(PostgresQueryBuilder);

		sqlx::query(&create_table_sql)
			.execute(&mut *tx)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to create table: {}", e)))?;

		// Create index on expire_date for efficient cleanup
		let create_index_stmt = Index::create()
			.if_not_exists()
			.name("idx_sessions_expire_date")
			.table(Alias::new("sessions"))
			.col(Alias::new("expire_date"))
			.to_owned();
		let create_index_sql = create_index_stmt.to_string(PostgresQueryBuilder);

		sqlx::query(&create_index_sql)
			.execute(&mut *tx)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to create index: {}", e)))?;

		tx.commit().await.map_err(|e| {
			SessionError::CacheError(format!("Failed to commit transaction: {}", e))
		})?;

		Ok(())
	}

	/// Clean up expired sessions
	///
	/// Deletes all sessions that have passed their expiration time.
	/// This should be called periodically to prevent database bloat.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_sessions::backends::DatabaseSessionBackend;
	///
	/// # async fn example() {
	/// let backend = DatabaseSessionBackend::new("sqlite::memory:").await.unwrap();
	/// backend.create_table().await.unwrap();
	///
	/// // Clean up expired sessions
	/// let deleted_count = backend.cleanup_expired().await.unwrap();
	/// assert!(deleted_count >= 0); // Returns number of deleted sessions
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn cleanup_expired(&self) -> Result<u64, SessionError> {
		// Use UNIX timestamp for INTEGER column compatibility
		let now_timestamp = Utc::now().timestamp_millis();

		// Use sea-query for DELETE
		let stmt = Query::delete()
			.from_table(Alias::new("sessions"))
			.and_where(Expr::col(Alias::new("expire_date")).lt(now_timestamp))
			.to_owned();
		let sql = stmt.to_string(SqliteQueryBuilder);

		let result = sqlx::query(&sql)
			.execute(&*self.pool)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to cleanup sessions: {}", e)))?;

		Ok(result.rows_affected())
	}
}

#[async_trait]
impl SessionBackend for DatabaseSessionBackend {
	async fn load<T>(&self, session_key: &str) -> Result<Option<T>, SessionError>
	where
		T: for<'de> Deserialize<'de> + Send,
	{
		// Use sea-query for SELECT
		let stmt = Query::select()
			.columns([Alias::new("session_data"), Alias::new("expire_date")])
			.from(Alias::new("sessions"))
			.and_where(Expr::col(Alias::new("session_key")).eq(session_key))
			.to_owned();
		let sql = stmt.to_string(SqliteQueryBuilder);

		let row = sqlx::query(&sql)
			.fetch_optional(&*self.pool)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to load session: {}", e)))?;

		match row {
			Some(row) => {
				// Check if session has expired
				let expire_timestamp: i64 = row
					.try_get("expire_date")
					.map_err(|e| SessionError::CacheError(format!("Invalid expire_date: {}", e)))?;

				let expire_date =
					DateTime::from_timestamp_millis(expire_timestamp).unwrap_or_else(Utc::now);

				if expire_date < Utc::now() {
					// Session expired, delete it
					let _ = self.delete(session_key).await;
					return Ok(None);
				}

				let session_data: String = row.try_get("session_data").map_err(|e| {
					SessionError::CacheError(format!("Invalid session_data: {}", e))
				})?;

				let data: T = serde_json::from_str(&session_data).map_err(|e| {
					SessionError::SerializationError(format!("Deserialization error: {}", e))
				})?;

				Ok(Some(data))
			}
			None => Ok(None),
		}
	}

	async fn save<T>(
		&self,
		session_key: &str,
		data: &T,
		ttl: Option<u64>,
	) -> Result<(), SessionError>
	where
		T: Serialize + Send + Sync,
	{
		let session_data = serde_json::to_string(data)
			.map_err(|e| SessionError::SerializationError(format!("Serialization error: {}", e)))?;

		let now = Utc::now();
		let expire_date = match ttl {
			Some(seconds) => now + Duration::seconds(seconds as i64),
			None => now + Duration::days(14), // Default 14 days
		};

		// Convert to UNIX timestamp for INTEGER column compatibility
		let now_timestamp = now.timestamp_millis();
		let expire_timestamp = expire_date.timestamp_millis();

		// Use a transaction to ensure atomicity when handling created_at preservation
		let mut tx =
			self.pool.begin().await.map_err(|e| {
				SessionError::CacheError(format!("Failed to begin transaction: {}", e))
			})?;

		// Check if session exists to determine created_at value
		let select_stmt = Query::select()
			.column(Alias::new("created_at"))
			.from(Alias::new("sessions"))
			.and_where(Expr::col(Alias::new("session_key")).eq(session_key))
			.to_owned();
		let select_sql = select_stmt.to_string(SqliteQueryBuilder);

		let existing_created_at = sqlx::query(&select_sql)
			.fetch_optional(&mut *tx)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to check session: {}", e)))?
			.and_then(|row| row.try_get::<i64, _>("created_at").ok());

		let created_at_timestamp = existing_created_at.unwrap_or(now_timestamp);

		// Use sea-query for INSERT with ON CONFLICT (upsert, SQLite compatible)
		let stmt = Query::insert()
			.into_table(Alias::new("sessions"))
			.columns([
				Alias::new("session_key"),
				Alias::new("session_data"),
				Alias::new("expire_date"),
				Alias::new("created_at"),
				Alias::new("last_accessed"),
			])
			.values_panic(vec![
				session_key.into(),
				session_data.into(),
				expire_timestamp.into(),
				created_at_timestamp.into(),
				now_timestamp.into(),
			])
			.on_conflict(
				OnConflict::column(Alias::new("session_key"))
					.update_columns([
						Alias::new("session_data"),
						Alias::new("expire_date"),
						Alias::new("last_accessed"),
					])
					.to_owned(),
			)
			.to_owned();
		let sql = stmt.to_string(SqliteQueryBuilder);

		sqlx::query(&sql)
			.execute(&mut *tx)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to save session: {}", e)))?;

		tx.commit().await.map_err(|e| {
			SessionError::CacheError(format!("Failed to commit transaction: {}", e))
		})?;

		Ok(())
	}

	async fn delete(&self, session_key: &str) -> Result<(), SessionError> {
		let stmt = Query::delete()
			.from_table(Alias::new("sessions"))
			.and_where(Expr::col(Alias::new("session_key")).eq(session_key))
			.to_owned();
		let sql = stmt.to_string(SqliteQueryBuilder);

		sqlx::query(&sql)
			.execute(&*self.pool)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to delete session: {}", e)))?;

		Ok(())
	}

	async fn exists(&self, session_key: &str) -> Result<bool, SessionError> {
		// Use UNIX timestamp for INTEGER column compatibility
		let now_timestamp = Utc::now().timestamp_millis();

		// Use sea-query for SELECT
		let stmt = Query::select()
			.expr(Expr::value(1))
			.from(Alias::new("sessions"))
			.and_where(Expr::col(Alias::new("session_key")).eq(session_key))
			.and_where(Expr::col(Alias::new("expire_date")).gt(now_timestamp))
			.to_owned();
		let sql = stmt.to_string(SqliteQueryBuilder);

		let row = sqlx::query(&sql)
			.fetch_optional(&*self.pool)
			.await
			.map_err(|e| {
				SessionError::CacheError(format!("Failed to check session existence: {}", e))
			})?;

		Ok(row.is_some())
	}
}

#[async_trait]
impl CleanupableBackend for DatabaseSessionBackend {
	async fn get_all_keys(&self) -> Result<Vec<String>, SessionError> {
		let stmt = Query::select()
			.column(Alias::new("session_key"))
			.from(Alias::new("sessions"))
			.to_owned();
		let sql = stmt.to_string(SqliteQueryBuilder);

		let rows = sqlx::query(&sql)
			.fetch_all(&*self.pool)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to get all keys: {}", e)))?;

		let keys: Vec<String> = rows
			.into_iter()
			.filter_map(|row| row.try_get::<String, _>("session_key").ok())
			.collect();

		Ok(keys)
	}

	async fn get_metadata(
		&self,
		session_key: &str,
	) -> Result<Option<SessionMetadata>, SessionError> {
		let stmt = Query::select()
			.columns([Alias::new("created_at"), Alias::new("last_accessed")])
			.from(Alias::new("sessions"))
			.and_where(Expr::col(Alias::new("session_key")).eq(session_key))
			.to_owned();
		let sql = stmt.to_string(SqliteQueryBuilder);

		let row = sqlx::query(&sql)
			.fetch_optional(&*self.pool)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to get metadata: {}", e)))?;

		match row {
			Some(row) => {
				let created_at_timestamp: i64 = row
					.try_get("created_at")
					.map_err(|e| SessionError::CacheError(format!("Invalid created_at: {}", e)))?;

				let last_accessed_timestamp: Option<i64> = row.try_get("last_accessed").ok();

				let created_at =
					DateTime::from_timestamp_millis(created_at_timestamp).unwrap_or_else(Utc::now);

				let last_accessed =
					last_accessed_timestamp.and_then(DateTime::from_timestamp_millis);

				Ok(Some(SessionMetadata {
					created_at,
					last_accessed,
				}))
			}
			None => Ok(None),
		}
	}

	async fn list_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>, SessionError> {
		use sea_query::{Expr, Query, SqliteQueryBuilder};

		let pattern = format!("{}%", prefix);
		let stmt = Query::select()
			.column(Alias::new("session_key"))
			.from(Alias::new("sessions"))
			.and_where(Expr::col(Alias::new("session_key")).like(&pattern))
			.to_owned();

		let sql = stmt.to_string(SqliteQueryBuilder);

		let rows = sqlx::query(&sql)
			.fetch_all(&*self.pool)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to list session keys: {}", e)))?;

		let keys: Vec<String> = rows
			.into_iter()
			.filter_map(|row| row.try_get::<String, _>("session_key").ok())
			.collect();

		Ok(keys)
	}

	async fn count_keys_with_prefix(&self, prefix: &str) -> Result<usize, SessionError> {
		use sea_query::{Expr, Func, Query, SqliteQueryBuilder};

		let pattern = format!("{}%", prefix);
		let stmt = Query::select()
			.expr(Func::count(Expr::col(Alias::new("session_key"))))
			.from(Alias::new("sessions"))
			.and_where(Expr::col(Alias::new("session_key")).like(&pattern))
			.to_owned();

		let sql = stmt.to_string(SqliteQueryBuilder);

		let row = sqlx::query(&sql)
			.fetch_one(&*self.pool)
			.await
			.map_err(|e| {
				SessionError::CacheError(format!("Failed to count session keys: {}", e))
			})?;

		let count: i64 = row
			.try_get(0)
			.map_err(|e| SessionError::CacheError(format!("Failed to extract count: {}", e)))?;

		Ok(count as usize)
	}

	async fn delete_keys_with_prefix(&self, prefix: &str) -> Result<usize, SessionError> {
		use sea_query::{Expr, Query, SqliteQueryBuilder};

		let pattern = format!("{}%", prefix);
		let stmt = Query::delete()
			.from_table(Alias::new("sessions"))
			.and_where(Expr::col(Alias::new("session_key")).like(&pattern))
			.to_owned();

		let sql = stmt.to_string(SqliteQueryBuilder);

		let result = sqlx::query(&sql).execute(&*self.pool).await.map_err(|e| {
			SessionError::CacheError(format!("Failed to delete session keys: {}", e))
		})?;

		Ok(result.rows_affected() as usize)
	}
}
