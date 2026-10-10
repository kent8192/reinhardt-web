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
//! use reinhardt_auth::sessions::backends::{DatabaseSessionBackend, SessionBackend};
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
use reinhardt_core::macros::model;
use reinhardt_db::backends::DatabaseConnection as BackendsConnection;
use reinhardt_db::orm::{
	DatabaseBackend, DatabaseConnection, DatabaseConnectionLease, Filter, FilterOperator,
	FilterValue, Model, OrmExecutor,
};
use reinhardt_query::prelude::{
	Alias, CockroachDBQueryBuilder, ColumnDef, CreateIndexStatement, CreateTableStatement,
	DeleteStatement, Expr, ExprTrait, Func, InsertStatement, IntoValue, MySqlQueryBuilder,
	OnConflict, PostgresQueryBuilder, Query, SelectStatement, SqliteQueryBuilder, Values,
};
use serde::{Deserialize, Serialize};

use crate::sessions::cleanup::{CleanupableBackend, SessionMetadata};

use super::cache::{AtomicSessionBackend, SessionBackend, SessionError};

enum SessionStatement {
	Select(SelectStatement),
	Insert(InsertStatement),
	Delete(DeleteStatement),
	CreateTable(CreateTableStatement),
	CreateIndex(CreateIndexStatement),
}

fn build_session_statement(
	backend: DatabaseBackend,
	is_cockroachdb: bool,
	statement: SessionStatement,
) -> Result<(String, Values), SessionError> {
	macro_rules! build {
		($statement:expr, $method:ident) => {
			if is_cockroachdb {
				CockroachDBQueryBuilder::new().$method($statement)
			} else {
				match backend {
					DatabaseBackend::Postgres => PostgresQueryBuilder.$method($statement),
					DatabaseBackend::MySql => MySqlQueryBuilder.$method($statement),
					DatabaseBackend::Sqlite => SqliteQueryBuilder.$method($statement),
				}
			}
		};
	}
	match &statement {
		SessionStatement::Select(stmt) => build!(stmt, build_select_checked),
		SessionStatement::Insert(stmt) => build!(stmt, build_insert_checked),
		SessionStatement::Delete(stmt) => build!(stmt, build_delete_checked),
		SessionStatement::CreateTable(stmt) => build!(stmt, build_create_table_checked),
		SessionStatement::CreateIndex(stmt) => build!(stmt, build_create_index_checked),
	}
	.map_err(|error| SessionError::CacheError(format!("Failed to build session query: {error}")))
}

async fn connect_backend(database_url: &str) -> Result<BackendsConnection, SessionError> {
	BackendsConnection::connect(database_url)
		.await
		.map_err(|error| SessionError::CacheError(format!("Database connection error: {error}")))
}

/// Database session model
///
/// Represents a session stored in the database.
/// Uses Unix timestamps (milliseconds) for date fields for database compatibility.
///
/// ## Example
///
/// ```rust
/// use reinhardt_auth::sessions::backends::database::Session;
/// use chrono::Utc;
///
/// let now_ms = Utc::now().timestamp_millis();
/// let session = Session::build()
///     .session_key("abc123")
///     .session_data("{\"user_id\": 42}")
///     .expire_date(now_ms + 3600000) // 1 hour
///     .created_at(now_ms)
///     .last_accessed(Some(now_ms))
///     .finish();
/// ```
#[model(app_label = "default", table_name = "sessions")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
	/// Unique session key (primary key)
	#[field(primary_key = true, max_length = 255)]
	pub session_key: String,
	/// Session data stored as JSON string
	#[field(max_length = 65535)]
	pub session_data: String,
	/// Session expiration timestamp (Unix timestamp in milliseconds)
	#[field]
	pub expire_date: i64,
	/// Session creation timestamp (Unix timestamp in milliseconds)
	#[field]
	pub created_at: i64,
	/// Last accessed timestamp (Unix timestamp in milliseconds)
	#[field]
	pub last_accessed: Option<i64>,
}

/// Database-backed session storage
///
/// Stores sessions in a database table with automatic expiration handling.
/// Supports PostgreSQL, MySQL, and SQLite.
///
/// ## Database Schema
///
/// The backend expects a table with the following structure (created via migrations):
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
/// Note: Timestamps are stored as Unix timestamps (milliseconds since epoch) in BIGINT columns.
///
/// ## Example
///
/// ```rust,no_run
/// use reinhardt_auth::sessions::backends::{DatabaseSessionBackend, SessionBackend};
/// use serde_json::json;
/// use reinhardt_db::backends::DatabaseConnection as BackendsConnection;
///
/// # async fn example() {
/// // Initialize backend with database connection
/// let owner = BackendsConnection::connect_sqlite("sqlite::memory:").await.unwrap();
/// let backend = DatabaseSessionBackend::from_connection(owner).unwrap();
///
/// // Note: Table should be created via migrations
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
	_lease: DatabaseConnectionLease,
	connection: DatabaseConnection,
}

impl DatabaseSessionBackend {
	/// Create a new database session backend
	///
	/// Initializes a connection to the specified database URL.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_auth::sessions::backends::DatabaseSessionBackend;
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
		let owner = connect_backend(database_url).await?;

		Self::from_connection(owner)
	}

	/// Create a new backend from an existing database connection
	///
	/// Session loads, saves, deletions, and existence checks all use the
	/// injected connection. The global ORM connection does not need to be
	/// initialized.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_auth::sessions::backends::DatabaseSessionBackend;
	/// use reinhardt_db::backends::DatabaseConnection as BackendsConnection;
	///
	/// # async fn example() {
	/// let owner = BackendsConnection::connect_sqlite("sqlite::memory:").await.unwrap();
	/// let backend = DatabaseSessionBackend::from_connection(owner).unwrap();
	/// // Backend created from existing connection
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub fn from_connection(owner: BackendsConnection) -> Result<Self, SessionError> {
		let lease = DatabaseConnectionLease::register(owner)
			.map_err(|e| SessionError::CacheError(format!("Database connection error: {}", e)))?;
		Ok(Self {
			connection: lease.handle(),
			_lease: lease,
		})
	}

	/// Render a checked statement while keeping SQL and its native arguments paired.
	fn build_statement(
		&self,
		statement: SessionStatement,
	) -> Result<(String, Values), SessionError> {
		build_session_statement(
			self.connection.backend(),
			self.connection.is_cockroachdb(),
			statement,
		)
	}

	/// Clean up expired sessions
	///
	/// Deletes all sessions that have passed their expiration time.
	/// This should be called periodically to prevent database bloat.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_auth::sessions::backends::DatabaseSessionBackend;
	///
	/// # async fn example() {
	/// let backend = DatabaseSessionBackend::new("sqlite::memory:").await.unwrap();
	///
	/// // Note: Table should be created via migrations
	///
	/// // Clean up expired sessions
	/// let deleted_count = backend.cleanup_expired().await.unwrap();
	/// assert!(deleted_count >= 0); // Returns number of deleted sessions
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn cleanup_expired(&self) -> Result<u64, SessionError> {
		let now_timestamp = Utc::now().timestamp_millis();

		// Build DELETE query using reinhardt-query
		let stmt = Query::delete()
			.from_table(Alias::new("sessions"))
			.and_where(Expr::col(Alias::new("expire_date")).lt(now_timestamp))
			.to_owned();

		let built = self.build_statement(SessionStatement::Delete(stmt))?;
		let mut connection = self.connection;
		let rows_affected = connection
			.execute_generated(built, None)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to cleanup sessions: {}", e)))?
			.rows_affected;

		Ok(rows_affected)
	}

	/// Create the sessions table
	///
	/// Creates the sessions table in the database. This is primarily intended for testing.
	/// In production, migrations should be used to create the table.
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_auth::sessions::backends::DatabaseSessionBackend;
	///
	/// # async fn example() {
	/// let backend = DatabaseSessionBackend::new("sqlite::memory:").await.unwrap();
	/// backend.create_table().await.unwrap();
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub async fn create_table(&self) -> Result<(), SessionError> {
		// Build CREATE TABLE statement using reinhardt-query
		let stmt = Query::create_table()
			.table(Alias::new("sessions"))
			.if_not_exists()
			.col(
				ColumnDef::new(Alias::new("session_key"))
					.string_len(255)
					.not_null(true)
					.primary_key(true),
			)
			.col(
				ColumnDef::new(Alias::new("session_data"))
					.text()
					.not_null(true),
			)
			.col(
				ColumnDef::new(Alias::new("expire_date"))
					.big_integer()
					.not_null(true),
			)
			.col(
				ColumnDef::new(Alias::new("created_at"))
					.big_integer()
					.not_null(true),
			)
			.col(ColumnDef::new(Alias::new("last_accessed")).big_integer())
			.to_owned();

		let built = self.build_statement(SessionStatement::CreateTable(stmt))?;
		let mut connection = self.connection;
		connection
			.execute_generated(built, None)
			.await
			.map_err(|e| {
				SessionError::CacheError(format!("Failed to create sessions table: {}", e))
			})?;

		// Create index for expire_date using reinhardt-query
		let index_stmt = Query::create_index()
			.if_not_exists()
			.name("idx_sessions_expire_date")
			.table(Alias::new("sessions"))
			.col(Alias::new("expire_date"))
			.to_owned();

		if let Ok(built) = self.build_statement(SessionStatement::CreateIndex(index_stmt)) {
			let _ = connection.execute_generated(built, None).await;
		}

		Ok(())
	}
}

#[async_trait]
impl SessionBackend for DatabaseSessionBackend {
	async fn load<T>(&self, session_key: &str) -> Result<Option<T>, SessionError>
	where
		T: for<'de> Deserialize<'de> + Send,
	{
		let mut connection = self.connection;

		// Use ORM to load the session through the backend's injected connection.
		let session = Session::objects()
			.filter(Filter::new(
				"session_key".to_string(),
				FilterOperator::Eq,
				FilterValue::String(session_key.to_string()),
			))
			.first_with_db(&mut connection)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to load session: {}", e)))?;

		match session {
			Some(session) => {
				// Check if session has expired
				let expire_date =
					DateTime::from_timestamp_millis(session.expire_date).unwrap_or_else(Utc::now);

				if expire_date < Utc::now() {
					// Session expired, delete it
					let _ = self.delete(session_key).await;
					return Ok(None);
				}

				let data: T = serde_json::from_str(&session.session_data).map_err(|e| {
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

		let now_timestamp = now.timestamp_millis();
		let expire_timestamp = expire_date.timestamp_millis();

		// Build UPSERT statement using reinhardt-query
		// reinhardt-query handles database-specific UPSERT syntax (ON CONFLICT vs ON DUPLICATE KEY)
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
				session_key.into_value(),
				session_data.into_value(),
				expire_timestamp.into_value(),
				now_timestamp.into_value(),
				now_timestamp.into_value(),
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

		let built = self.build_statement(SessionStatement::Insert(stmt))?;
		let mut connection = self.connection;
		connection
			.execute_generated(built, None)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to save session: {}", e)))?;

		Ok(())
	}

	async fn delete(&self, session_key: &str) -> Result<(), SessionError> {
		// Build DELETE query using reinhardt-query
		let stmt = Query::delete()
			.from_table(Alias::new("sessions"))
			.and_where(Expr::col(Alias::new("session_key")).eq(session_key))
			.to_owned();

		let built = self.build_statement(SessionStatement::Delete(stmt))?;
		let mut connection = self.connection;
		connection
			.execute_generated(built, None)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to delete session: {}", e)))?;

		Ok(())
	}

	async fn exists(&self, session_key: &str) -> Result<bool, SessionError> {
		let now_timestamp = Utc::now().timestamp_millis();
		let mut connection = self.connection;

		// Use ORM to check the backend's injected connection.
		let session = Session::objects()
			.filter(Filter::new(
				"session_key".to_string(),
				FilterOperator::Eq,
				FilterValue::String(session_key.to_string()),
			))
			.filter(Filter::new(
				"expire_date".to_string(),
				FilterOperator::Gt,
				FilterValue::Integer(now_timestamp),
			))
			.first_with_db(&mut connection)
			.await
			.map_err(|e| {
				SessionError::CacheError(format!("Failed to check session existence: {}", e))
			})?;

		Ok(session.is_some())
	}
}

#[async_trait]
impl AtomicSessionBackend for DatabaseSessionBackend {
	/// Atomically load and delete one unexpired session.
	///
	/// The row is read, then deleted with a compare-and-delete predicate on
	/// the values that were read. Row-level locking lets exactly one
	/// concurrent `DELETE` affect the row, so only the caller whose delete
	/// reports an affected row receives the data. The predicate also keeps a
	/// concurrent overwrite of the same key from being deleted on behalf of
	/// the stale read. This works on every supported database, including
	/// MySQL, which lacks `DELETE ... RETURNING`.
	async fn take<T>(&self, session_key: &str) -> Result<Option<T>, SessionError>
	where
		T: for<'de> Deserialize<'de> + Send,
	{
		let mut connection = self.connection;

		let session = Session::objects()
			.filter(Filter::new(
				"session_key".to_string(),
				FilterOperator::Eq,
				FilterValue::String(session_key.to_string()),
			))
			.first_with_db(&mut connection)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to load session: {}", e)))?;
		let Some(session) = session else {
			return Ok(None);
		};

		let stmt = Query::delete()
			.from_table(Alias::new("sessions"))
			.and_where(Expr::col(Alias::new("session_key")).eq(session_key))
			.and_where(Expr::col(Alias::new("session_data")).eq(session.session_data.as_str()))
			.and_where(Expr::col(Alias::new("expire_date")).eq(session.expire_date))
			.to_owned();
		let built = self.build_statement(SessionStatement::Delete(stmt))?;
		let rows_affected = connection
			.execute_generated(built, None)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to take session: {}", e)))?
			.rows_affected;
		if rows_affected == 0 {
			// Another caller consumed or replaced the row after this read.
			return Ok(None);
		}

		if session.expire_date < Utc::now().timestamp_millis() {
			return Ok(None);
		}

		let data: T = serde_json::from_str(&session.session_data).map_err(|e| {
			SessionError::SerializationError(format!("Deserialization error: {}", e))
		})?;
		Ok(Some(data))
	}
}

#[async_trait]
impl CleanupableBackend for DatabaseSessionBackend {
	async fn get_all_keys(&self) -> Result<Vec<String>, SessionError> {
		let mut connection = self.connection;

		// Use ORM to get all session keys
		// Manager::all() returns QuerySet, QuerySet::all() executes and returns Vec<T>
		let sessions = Session::objects()
			.all()
			.all_with_db(&mut connection)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to get all keys: {}", e)))?;

		let keys: Vec<String> = sessions.into_iter().map(|s| s.session_key).collect();

		Ok(keys)
	}

	async fn get_metadata(
		&self,
		session_key: &str,
	) -> Result<Option<SessionMetadata>, SessionError> {
		let mut connection = self.connection;

		// Use ORM to get session metadata
		let session = Session::objects()
			.filter(Filter::new(
				"session_key".to_string(),
				FilterOperator::Eq,
				FilterValue::String(session_key.to_string()),
			))
			.first_with_db(&mut connection)
			.await
			.ok()
			.flatten();

		match session {
			Some(session) => {
				let created_at =
					DateTime::from_timestamp_millis(session.created_at).unwrap_or_else(Utc::now);

				let last_accessed = session
					.last_accessed
					.and_then(DateTime::from_timestamp_millis);

				Ok(Some(SessionMetadata {
					created_at,
					last_accessed,
				}))
			}
			None => Ok(None),
		}
	}

	async fn list_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>, SessionError> {
		let mut connection = self.connection;

		// Use ORM to list session keys with prefix
		let sessions = Session::objects()
			.filter(Filter::new(
				"session_key".to_string(),
				FilterOperator::StartsWith,
				FilterValue::String(prefix.to_string()),
			))
			.all_with_db(&mut connection)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to list session keys: {}", e)))?;

		let keys: Vec<String> = sessions.into_iter().map(|s| s.session_key).collect();

		Ok(keys)
	}

	async fn count_keys_with_prefix(&self, prefix: &str) -> Result<usize, SessionError> {
		// Count matching keys in the database without loading session payloads.
		let stmt = Query::select()
			.from(Alias::new("sessions"))
			.expr_as(Func::count(Expr::asterisk().into()), Alias::new("count"))
			.and_where(Expr::col(Alias::new("session_key")).starts_with(prefix))
			.to_owned();
		let built = self.build_statement(SessionStatement::Select(stmt))?;
		let mut connection = self.connection;
		let count: i64 = connection
			.fetch_one_generated(built, None)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to count session keys: {}", e)))?
			.get("count")
			.map_err(|_| {
				SessionError::CacheError("Failed to read session key count".to_string())
			})?;

		usize::try_from(count).map_err(|e| {
			SessionError::CacheError(format!("Failed to convert session key count: {}", e))
		})
	}

	async fn delete_keys_with_prefix(&self, prefix: &str) -> Result<usize, SessionError> {
		// Build DELETE query with LIKE condition using reinhardt-query
		let pattern = format!("{}%", prefix);
		let stmt = Query::delete()
			.from_table(Alias::new("sessions"))
			.and_where(Expr::col(Alias::new("session_key")).like(pattern.as_str()))
			.to_owned();

		let built = self.build_statement(SessionStatement::Delete(stmt))?;
		let mut connection = self.connection;
		let rows_affected = connection
			.execute_generated(built, None)
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to delete session keys: {}", e)))?
			.rows_affected;

		Ok(rows_affected as usize)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[rstest::rstest]
	#[case(
		DatabaseBackend::Postgres,
		"INSERT INTO \"sessions\" (\"session_key\", \"session_data\", \"expire_date\", \"last_accessed\") VALUES ($1, $2, $3, NULL)"
	)]
	#[case(
		DatabaseBackend::MySql,
		"INSERT INTO `sessions` (`session_key`, `session_data`, `expire_date`, `last_accessed`) VALUES (?, ?, ?, NULL)"
	)]
	#[case(
		DatabaseBackend::Sqlite,
		"INSERT INTO \"sessions\" (\"session_key\", \"session_data\", \"expire_date\", \"last_accessed\") VALUES (?, ?, ?, NULL)"
	)]
	fn session_insert_keeps_exact_arguments(
		#[case] backend: DatabaseBackend,
		#[case] expected_sql: &str,
	) {
		// Arrange: quote-bearing keys and JSON remain data, and NULL consumes no slot.
		let key = "tenant' ? $42";
		let data = r#"{"payload":"quoted' ? $43"}"#;
		let statement = Query::insert()
			.into_table(Alias::new("sessions"))
			.columns([
				Alias::new("session_key"),
				Alias::new("session_data"),
				Alias::new("expire_date"),
				Alias::new("last_accessed"),
			])
			.values_panic([
				key.into(),
				data.into(),
				123_i64.into(),
				reinhardt_query::Value::Int(None),
			])
			.take();
		// Act
		let (sql, values) =
			build_session_statement(backend, false, SessionStatement::Insert(statement)).unwrap();
		// Assert
		assert_eq!(sql, expected_sql);
		assert_eq!(
			values,
			Values(vec![key.into(), data.into(), 123_i64.into()])
		);
	}

	fn session(
		session_key: impl Into<String>,
		session_data: impl Into<String>,
		expire_date: i64,
		created_at: i64,
		last_accessed: Option<i64>,
	) -> Session {
		Session::build()
			.session_key(session_key)
			.session_data(session_data)
			.expire_date(expire_date)
			.created_at(created_at)
			.last_accessed(last_accessed)
			.finish()
	}

	#[test]
	fn test_session_struct_fields() {
		let now_ms = Utc::now().timestamp_millis();
		let session = session(
			"test_key",
			r#"{"user_id": 42}"#,
			now_ms + 3600000, // 1 hour from now
			now_ms,
			Some(now_ms),
		);

		assert_eq!(session.session_key, "test_key");
		assert_eq!(session.session_data, r#"{"user_id": 42}"#);
		assert_eq!(session.expire_date, now_ms + 3600000);
		assert_eq!(session.created_at, now_ms);
		assert_eq!(session.last_accessed, Some(now_ms));
	}

	#[test]
	fn test_session_struct_without_last_accessed() {
		let now_ms = Utc::now().timestamp_millis();
		let session = session("key", "{}", now_ms + 1000, now_ms, None);

		assert!(session.last_accessed.is_none());
	}

	#[test]
	fn test_session_clone() {
		let now_ms = Utc::now().timestamp_millis();
		let session = session(
			"clone_test",
			r#"{"data": "value"}"#,
			now_ms + 3600000,
			now_ms,
			Some(now_ms),
		);

		let cloned = session.clone();

		assert_eq!(cloned.session_key, session.session_key);
		assert_eq!(cloned.session_data, session.session_data);
		assert_eq!(cloned.expire_date, session.expire_date);
		assert_eq!(cloned.created_at, session.created_at);
		assert_eq!(cloned.last_accessed, session.last_accessed);
	}

	#[test]
	fn test_session_debug() {
		let now_ms = Utc::now().timestamp_millis();
		let session = session("debug_key", "{}", now_ms, now_ms, None);

		let debug_str = format!("{:?}", session);

		assert!(debug_str.contains("Session"));
		assert!(debug_str.contains("debug_key"));
	}

	#[test]
	fn test_session_serialize() {
		let now_ms = Utc::now().timestamp_millis();
		let session = session(
			"serialize_key",
			r#"{"count": 10}"#,
			now_ms + 3600000,
			now_ms,
			Some(now_ms),
		);

		let json = serde_json::to_string(&session).unwrap();

		assert!(json.contains("serialize_key"));
		// session_data is serialized as a JSON string, so internal quotes are escaped
		assert!(json.contains(r#"{\"count\": 10}"#));
	}

	#[test]
	fn test_session_deserialize() {
		let now_ms = 1700000000000_i64; // Fixed timestamp for test
		let json = format!(
			r#"{{
				"session_key": "deserialize_key",
				"session_data": "{{\"user\": \"test\"}}",
				"expire_date": {},
				"created_at": {},
				"last_accessed": {}
			}}"#,
			now_ms + 3600000,
			now_ms,
			now_ms
		);

		let session: Session = serde_json::from_str(&json).unwrap();

		assert_eq!(session.session_key, "deserialize_key");
		assert_eq!(session.session_data, r#"{"user": "test"}"#);
		assert_eq!(session.expire_date, now_ms + 3600000);
		assert_eq!(session.created_at, now_ms);
		assert_eq!(session.last_accessed, Some(now_ms));
	}

	#[test]
	fn test_session_deserialize_without_last_accessed() {
		let now_ms = 1700000000000_i64;
		let json = format!(
			r#"{{
				"session_key": "no_access",
				"session_data": "{{}}",
				"expire_date": {},
				"created_at": {},
				"last_accessed": null
			}}"#,
			now_ms + 3600000,
			now_ms
		);

		let session: Session = serde_json::from_str(&json).unwrap();

		assert_eq!(session.session_key, "no_access");
		assert!(session.last_accessed.is_none());
	}

	#[test]
	fn test_database_session_backend_clone() {
		// DatabaseSessionBackend implements Clone via Arc
		// We can't test this without a real connection, but we can verify the trait is implemented
		fn assert_clone<T: Clone>() {}
		assert_clone::<DatabaseSessionBackend>();
	}

	#[tokio::test]
	async fn injected_connection_handles_session_lifecycle_without_global_orm_connection() {
		let owner = connect_backend("sqlite::memory:").await.unwrap();
		let backend = DatabaseSessionBackend::from_connection(owner).unwrap();
		let session_key = "injected-connection";
		let session_data = serde_json::json!({"user_id": 42});

		backend.create_table().await.unwrap();
		backend
			.save(session_key, &session_data, Some(60))
			.await
			.unwrap();

		let loaded: Option<serde_json::Value> = backend.load(session_key).await.unwrap();
		assert_eq!(loaded, Some(session_data));
		assert!(backend.exists(session_key).await.unwrap());

		backend.delete(session_key).await.unwrap();

		assert!(!backend.exists(session_key).await.unwrap());
	}

	async fn sqlite_backend() -> DatabaseSessionBackend {
		let owner = connect_backend("sqlite::memory:").await.unwrap();
		let backend = DatabaseSessionBackend::from_connection(owner).unwrap();
		backend.create_table().await.unwrap();
		backend
	}

	#[tokio::test]
	async fn take_returns_session_once_and_deletes_row() {
		// Arrange
		let backend = sqlite_backend().await;
		let session_data = serde_json::json!({"state": "one-time"});
		backend
			.save("take-key", &session_data, Some(60))
			.await
			.unwrap();

		// Act
		let first: Option<serde_json::Value> = backend.take("take-key").await.unwrap();
		let second: Option<serde_json::Value> = backend.take("take-key").await.unwrap();

		// Assert
		assert_eq!(first, Some(session_data));
		assert_eq!(second, None);
		assert!(backend.get_all_keys().await.unwrap().is_empty());
	}

	#[tokio::test]
	async fn take_omits_and_deletes_expired_row() {
		// Arrange
		let backend = sqlite_backend().await;
		let expired = Utc::now().timestamp_millis() - 1_000;
		let stmt = Query::insert()
			.into_table(Alias::new("sessions"))
			.columns([
				Alias::new("session_key"),
				Alias::new("session_data"),
				Alias::new("expire_date"),
				Alias::new("created_at"),
			])
			.values_panic([
				"expired-key".into_value(),
				r#"{"state":"stale"}"#.into_value(),
				expired.into_value(),
				expired.into_value(),
			])
			.to_owned();
		let built = backend
			.build_statement(SessionStatement::Insert(stmt))
			.unwrap();
		let mut connection = backend.connection;
		connection.execute_generated(built, None).await.unwrap();

		// Act
		let taken: Option<serde_json::Value> = backend.take("expired-key").await.unwrap();

		// Assert
		assert_eq!(taken, None);
		assert!(backend.get_all_keys().await.unwrap().is_empty());
	}
}
