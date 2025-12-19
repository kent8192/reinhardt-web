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
use reinhardt_db::DatabaseConnection;
use reinhardt_db::orm::{Filter, FilterOperator, FilterValue, Manager, Model};
use reinhardt_macros::model;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{CleanupableBackend, SessionMetadata};

use super::cache::{SessionBackend, SessionError};

/// Database session model
///
/// Represents a session stored in the database.
/// Uses Unix timestamps (milliseconds) for date fields for database compatibility.
///
/// ## Example
///
/// ```rust
/// use reinhardt_sessions::backends::database::Session;
/// use chrono::Utc;
///
/// let now_ms = Utc::now().timestamp_millis();
/// let session = Session {
///     session_key: "abc123".to_string(),
///     session_data: "{\"user_id\": 42}".to_string(),
///     expire_date: now_ms + 3600000, // 1 hour
///     created_at: now_ms,
///     last_accessed: Some(now_ms),
/// };
/// ```
#[model(table_name = "sessions", primary_key = "session_key")]
#[derive(Debug, Clone)]
pub struct Session {
	/// Unique session key (primary key)
	pub session_key: String,
	/// Session data stored as JSON string
	pub session_data: String,
	/// Session expiration timestamp (Unix timestamp in milliseconds)
	pub expire_date: i64,
	/// Session creation timestamp (Unix timestamp in milliseconds)
	pub created_at: i64,
	/// Last accessed timestamp (Unix timestamp in milliseconds)
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
/// use reinhardt_sessions::backends::{DatabaseSessionBackend, SessionBackend};
/// use serde_json::json;
/// use reinhardt_db::DatabaseConnection;
///
/// # async fn example() {
/// // Initialize backend with database connection
/// let connection = DatabaseConnection::new("sqlite::memory:").await.unwrap();
/// let backend = DatabaseSessionBackend::from_connection(connection);
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
	connection: Arc<DatabaseConnection>,
}

impl DatabaseSessionBackend {
	/// Create a new database session backend
	///
	/// Initializes a connection to the specified database URL.
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
		let connection = DatabaseConnection::connect(database_url)
			.await
			.map_err(|e| SessionError::CacheError(format!("Database connection error: {}", e)))?;

		Ok(Self {
			connection: Arc::new(connection),
		})
	}

	/// Create a new backend from an existing database connection
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_sessions::backends::DatabaseSessionBackend;
	/// use reinhardt_db::DatabaseConnection;
	/// use std::sync::Arc;
	///
	/// # async fn example() {
	/// let connection = DatabaseConnection::new("sqlite::memory:").await.unwrap();
	/// let backend = DatabaseSessionBackend::from_connection(Arc::new(connection));
	/// // Backend created from existing connection
	/// # }
	/// # tokio::runtime::Runtime::new().unwrap().block_on(example());
	/// ```
	pub fn from_connection(connection: Arc<DatabaseConnection>) -> Self {
		Self { connection }
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

		// Use ORM to delete expired sessions
		let manager = Session::objects(&self.connection);
		let filter = Filter::new("expire_date", now_timestamp).lt();
		let result =
			manager.filter(filter).delete().await.map_err(|e| {
				SessionError::CacheError(format!("Failed to cleanup sessions: {}", e))
			})?;

		Ok(result as u64)
	}
}

#[async_trait]
impl SessionBackend for DatabaseSessionBackend {
	async fn load<T>(&self, session_key: &str) -> Result<Option<T>, SessionError>
	where
		T: for<'de> Deserialize<'de> + Send,
	{
		// Use ORM to load session
		let manager = Session::objects(&self.connection);
		let session = manager
			.filter(Filter::new("session_key", session_key))
			.first()
			.await
			.ok();

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

		// Use ORM to save session
		let manager = Session::objects(&self.connection);

		// Try to get existing session to preserve created_at
		let existing = manager
			.filter(Filter::new("session_key", session_key))
			.first()
			.await
			.ok();

		let created_at_timestamp = existing
			.as_ref()
			.map(|s| s.created_at)
			.unwrap_or(now_timestamp);

		let session = Session {
			session_key: session_key.to_string(),
			session_data,
			expire_date: expire_timestamp,
			created_at: created_at_timestamp,
			last_accessed: Some(now_timestamp),
		};

		if existing.is_some() {
			// Update existing session
			manager
				.update(session_key, &session)
				.await
				.map_err(|e| SessionError::CacheError(format!("Failed to save session: {}", e)))?;
		} else {
			// Create new session
			manager
				.create(&session)
				.await
				.map_err(|e| SessionError::CacheError(format!("Failed to save session: {}", e)))?;
		}

		Ok(())
	}

	async fn delete(&self, session_key: &str) -> Result<(), SessionError> {
		// Use ORM to delete session
		let manager = Session::objects(&self.connection);
		manager
			.filter(Filter::new("session_key", session_key))
			.delete()
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to delete session: {}", e)))?;

		Ok(())
	}

	async fn exists(&self, session_key: &str) -> Result<bool, SessionError> {
		let now_timestamp = Utc::now().timestamp_millis();

		// Use ORM to check if session exists and is not expired
		let manager = Session::objects(&self.connection);
		let session = manager
			.filter(Filter::new("session_key", session_key))
			.filter(Filter::new("expire_date", now_timestamp).gt())
			.first()
			.await
			.ok();

		Ok(session.is_some())
	}
}

#[async_trait]
impl CleanupableBackend for DatabaseSessionBackend {
	async fn get_all_keys(&self) -> Result<Vec<String>, SessionError> {
		// Use ORM to get all session keys
		let manager = Session::objects(&self.connection);
		let sessions = manager
			.all()
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to get all keys: {}", e)))?;

		let keys: Vec<String> = sessions.into_iter().map(|s| s.session_key).collect();

		Ok(keys)
	}

	async fn get_metadata(
		&self,
		session_key: &str,
	) -> Result<Option<SessionMetadata>, SessionError> {
		// Use ORM to get session metadata
		let manager = Session::objects(&self.connection);
		let session = manager
			.filter(Filter::new("session_key", session_key))
			.first()
			.await
			.ok();

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
		// Use ORM to list session keys with prefix
		let manager = Session::objects(&self.connection);
		let pattern = format!("{}%", prefix);
		let sessions = manager
			.filter(Filter::new("session_key", &pattern).like())
			.all()
			.await
			.map_err(|e| SessionError::CacheError(format!("Failed to list session keys: {}", e)))?;

		let keys: Vec<String> = sessions.into_iter().map(|s| s.session_key).collect();

		Ok(keys)
	}

	async fn count_keys_with_prefix(&self, prefix: &str) -> Result<usize, SessionError> {
		// Use ORM to count session keys with prefix
		let manager = Session::objects(&self.connection);
		let pattern = format!("{}%", prefix);
		let count = manager
			.filter(Filter::new("session_key", &pattern).like())
			.count()
			.await
			.map_err(|e| {
				SessionError::CacheError(format!("Failed to count session keys: {}", e))
			})?;

		Ok(count)
	}

	async fn delete_keys_with_prefix(&self, prefix: &str) -> Result<usize, SessionError> {
		// Use ORM to delete session keys with prefix
		let manager = Session::objects(&self.connection);
		let pattern = format!("{}%", prefix);
		let deleted = manager
			.filter(Filter::new("session_key", &pattern).like())
			.delete()
			.await
			.map_err(|e| {
				SessionError::CacheError(format!("Failed to delete session keys: {}", e))
			})?;

		Ok(deleted as usize)
	}
}
