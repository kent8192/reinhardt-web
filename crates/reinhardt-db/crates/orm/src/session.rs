//! ORM Session - SQLAlchemy-style database session with identity map and unit of work pattern
//!
//! This module provides a Session object that manages database operations with automatic
//! object tracking, identity mapping, and transaction management.

use crate::model::Model;
use crate::query::Query;
use crate::transaction::Transaction;
use serde_json::Value;
use sqlx::AnyPool;
use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Session error types
#[derive(Debug, Clone)]
pub enum SessionError {
	/// Database error occurred
	DatabaseError(String),
	/// Object not found in session
	ObjectNotFound(String),
	/// Transaction error
	TransactionError(String),
	/// Serialization/deserialization error
	SerializationError(String),
	/// Invalid state
	InvalidState(String),
}

impl std::fmt::Display for SessionError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
			Self::ObjectNotFound(msg) => write!(f, "Object not found: {}", msg),
			Self::TransactionError(msg) => write!(f, "Transaction error: {}", msg),
			Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
			Self::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
		}
	}
}

impl std::error::Error for SessionError {}

/// Identity map entry storing tracked objects
struct IdentityEntry {
	/// The serialized object data
	data: Value,
	/// Type ID for runtime type checking
	type_id: TypeId,
	/// Whether the object has been modified
	#[allow(dead_code)]
	is_dirty: bool,
}

/// SQLAlchemy-style ORM session with identity map and unit of work
///
/// # Examples
///
/// ```no_run
/// use reinhardt_orm::session::Session;
/// use sqlx::AnyPool;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let pool = AnyPool::connect("sqlite::memory:").await?;
/// let session = Session::new(Arc::new(pool)).await?;
///
/// // Session is ready for use
/// # Ok(())
/// # }
/// ```
pub struct Session {
	/// Connection pool
	#[allow(dead_code)]
	pool: Arc<AnyPool>,
	/// Active transaction (if any)
	transaction: Option<Transaction>,
	/// Identity map: tracks objects by type and primary key
	identity_map: HashMap<String, IdentityEntry>,
	/// Set of object keys that have been modified
	dirty_objects: HashSet<String>,
	/// Set of object keys marked for deletion
	deleted_objects: HashSet<String>,
	/// Whether session is closed
	is_closed: bool,
}

impl Session {
	/// Create a new session with the given connection pool
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let session = Session::new(Arc::new(pool)).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn new(pool: Arc<AnyPool>) -> Result<Self, SessionError> {
		Ok(Self {
			pool,
			transaction: None,
			identity_map: HashMap::new(),
			dirty_objects: HashSet::new(),
			deleted_objects: HashSet::new(),
			is_closed: false,
		})
	}

	/// Add an object to the session for tracking
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use reinhardt_orm::Model;
	/// use serde::{Serialize, Deserialize};
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// #[derive(Serialize, Deserialize, Clone)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let mut session = Session::new(Arc::new(pool)).await?;
	///
	/// let user = User { id: Some(1), name: "Alice".to_string() };
	/// session.add(user).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn add<T: Model + 'static>(&mut self, obj: T) -> Result<(), SessionError> {
		self.check_closed()?;

		let pk = obj
			.primary_key()
			.ok_or_else(|| SessionError::InvalidState("Object has no primary key".to_string()))?;

		let key = format!("{}:{}", T::table_name(), pk);

		let data = serde_json::to_value(&obj)
			.map_err(|e| SessionError::SerializationError(e.to_string()))?;

		self.identity_map.insert(
			key.clone(),
			IdentityEntry {
				data,
				type_id: TypeId::of::<T>(),
				is_dirty: true,
			},
		);

		self.dirty_objects.insert(key);

		Ok(())
	}

	/// Get an object by primary key
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use reinhardt_orm::Model;
	/// use serde::{Serialize, Deserialize};
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// #[derive(Serialize, Deserialize, Clone)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let session = Session::new(Arc::new(pool)).await?;
	///
	/// let user: Option<User> = session.get(1).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn get<T: Model + 'static>(
		&self,
		id: T::PrimaryKey,
	) -> Result<Option<T>, SessionError> {
		self.check_closed()?;

		let key = format!("{}:{}", T::table_name(), id);

		// Check identity map first
		if let Some(entry) = self.identity_map.get(&key) {
			if entry.type_id != TypeId::of::<T>() {
				return Err(SessionError::InvalidState(
					"Type mismatch in identity map".to_string(),
				));
			}

			let obj: T = serde_json::from_value(entry.data.clone())
				.map_err(|e| SessionError::SerializationError(e.to_string()))?;

			return Ok(Some(obj));
		}

		// If not in identity map, would query database here
		// For now, return None
		Ok(None)
	}

	/// Create a query for the given model type
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use reinhardt_orm::Model;
	/// use serde::{Serialize, Deserialize};
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// #[derive(Serialize, Deserialize, Clone)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let session = Session::new(Arc::new(pool)).await?;
	///
	/// let query = session.query::<User>();
	/// # Ok(())
	/// # }
	/// ```
	pub fn query<T: Model>(&self) -> Query {
		Query::new()
	}

	/// Flush all pending changes to the database
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let mut session = Session::new(Arc::new(pool)).await?;
	///
	/// // Add/modify objects...
	/// session.flush().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn flush(&mut self) -> Result<(), SessionError> {
		self.check_closed()?;

		// Process dirty objects
		for key in &self.dirty_objects {
			if let Some(_entry) = self.identity_map.get(key) {
				// In a real implementation, would generate and execute SQL here
				// For now, just mark as clean
			}
		}

		self.dirty_objects.clear();

		// Process deleted objects
		for key in &self.deleted_objects {
			// In a real implementation, would execute DELETE statements
			self.identity_map.remove(key);
		}

		self.deleted_objects.clear();

		Ok(())
	}

	/// Commit the current transaction and flush changes
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let mut session = Session::new(Arc::new(pool)).await?;
	///
	/// // Add/modify objects...
	/// session.commit().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn commit(&mut self) -> Result<(), SessionError> {
		self.check_closed()?;

		// Flush pending changes
		self.flush().await?;

		// Commit transaction if active
		if let Some(mut tx) = self.transaction.take() {
			tx.commit().map_err(SessionError::TransactionError)?;
		}

		Ok(())
	}

	/// Rollback the current transaction
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let mut session = Session::new(Arc::new(pool)).await?;
	///
	/// // Operations...
	/// session.rollback().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn rollback(&mut self) -> Result<(), SessionError> {
		self.check_closed()?;

		// Clear dirty and deleted objects
		self.dirty_objects.clear();
		self.deleted_objects.clear();

		// Rollback transaction if active
		if let Some(mut tx) = self.transaction.take() {
			tx.rollback()
				.map_err(SessionError::TransactionError)?;
		}

		Ok(())
	}

	/// Close the session
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let mut session = Session::new(Arc::new(pool)).await?;
	///
	/// session.close().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn close(mut self) -> Result<(), SessionError> {
		if self.is_closed {
			return Ok(());
		}

		// Rollback any pending transaction
		if self.transaction.is_some() {
			self.rollback().await?;
		}

		self.is_closed = true;
		Ok(())
	}

	/// Begin a new transaction
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let mut session = Session::new(Arc::new(pool)).await?;
	///
	/// session.begin().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn begin(&mut self) -> Result<(), SessionError> {
		self.check_closed()?;

		if self.transaction.is_some() {
			return Err(SessionError::TransactionError(
				"Transaction already active".to_string(),
			));
		}

		let mut tx = Transaction::new();
		tx.begin().map_err(SessionError::TransactionError)?;

		self.transaction = Some(tx);

		Ok(())
	}

	/// Delete an object from the session
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use reinhardt_orm::Model;
	/// use serde::{Serialize, Deserialize};
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// #[derive(Serialize, Deserialize, Clone)]
	/// struct User {
	///     id: Option<i64>,
	///     name: String,
	/// }
	///
	/// impl Model for User {
	///     type PrimaryKey = i64;
	///     fn table_name() -> &'static str { "users" }
	///     fn primary_key(&self) -> Option<&Self::PrimaryKey> { self.id.as_ref() }
	///     fn set_primary_key(&mut self, value: Self::PrimaryKey) { self.id = Some(value); }
	/// }
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let mut session = Session::new(Arc::new(pool)).await?;
	///
	/// let user = User { id: Some(1), name: "Alice".to_string() };
	/// session.delete(user).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn delete<T: Model + 'static>(&mut self, obj: T) -> Result<(), SessionError> {
		self.check_closed()?;

		let pk = obj
			.primary_key()
			.ok_or_else(|| SessionError::InvalidState("Object has no primary key".to_string()))?;

		let key = format!("{}:{}", T::table_name(), pk);

		// Mark for deletion
		self.deleted_objects.insert(key.clone());

		// Remove from dirty set if present
		self.dirty_objects.remove(&key);

		Ok(())
	}

	/// Check if the session is closed
	fn check_closed(&self) -> Result<(), SessionError> {
		if self.is_closed {
			Err(SessionError::InvalidState("Session is closed".to_string()))
		} else {
			Ok(())
		}
	}

	/// Get the number of objects in the identity map
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let session = Session::new(Arc::new(pool)).await?;
	///
	/// let count = session.identity_count();
	/// # Ok(())
	/// # }
	/// ```
	pub fn identity_count(&self) -> usize {
		self.identity_map.len()
	}

	/// Get the number of dirty objects
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let session = Session::new(Arc::new(pool)).await?;
	///
	/// let count = session.dirty_count();
	/// # Ok(())
	/// # }
	/// ```
	pub fn dirty_count(&self) -> usize {
		self.dirty_objects.len()
	}

	/// Check if session has active transaction
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let session = Session::new(Arc::new(pool)).await?;
	///
	/// let has_tx = session.has_transaction();
	/// # Ok(())
	/// # }
	/// ```
	pub fn has_transaction(&self) -> bool {
		self.transaction.is_some()
	}

	/// Check if session is closed
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_orm::session::Session;
	/// use sqlx::AnyPool;
	/// use std::sync::Arc;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let pool = AnyPool::connect("sqlite::memory:").await?;
	/// let session = Session::new(Arc::new(pool)).await?;
	///
	/// let closed = session.is_closed();
	/// # Ok(())
	/// # }
	/// ```
	pub fn is_closed(&self) -> bool {
		self.is_closed
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde::{Deserialize, Serialize};

	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
	struct TestUser {
		id: Option<i64>,
		name: String,
		email: String,
	}

	impl Model for TestUser {
		type PrimaryKey = i64;

		fn table_name() -> &'static str {
			"users"
		}

		fn primary_key(&self) -> Option<&Self::PrimaryKey> {
			self.id.as_ref()
		}

		fn set_primary_key(&mut self, value: Self::PrimaryKey) {
			self.id = Some(value);
		}
	}

	// Create test pool using SQLite in-memory database
	async fn create_test_pool() -> Arc<AnyPool> {
		// Connect directly using AnyPool with sqlite: URL scheme
		let pool = AnyPool::connect("sqlite::memory:")
			.await
			.expect("Failed to create test pool");

		Arc::new(pool)
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_creation() {
		let pool = create_test_pool().await;
		let session = Session::new(pool).await;

		assert!(session.is_ok());
		let session = session.unwrap();
		assert!(!session.is_closed());
		assert_eq!(session.identity_count(), 0);
		assert_eq!(session.dirty_count(), 0);
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_add_object() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool).await.unwrap();

		let user = TestUser {
			id: Some(1),
			name: "Alice".to_string(),
			email: "alice@example.com".to_string(),
		};

		let result = session.add(user).await;
		assert!(result.is_ok());
		assert_eq!(session.identity_count(), 1);
		assert_eq!(session.dirty_count(), 1);
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_get_from_identity_map() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool).await.unwrap();

		let user = TestUser {
			id: Some(1),
			name: "Bob".to_string(),
			email: "bob@example.com".to_string(),
		};

		session.add(user.clone()).await.unwrap();

		let retrieved: Option<TestUser> = session.get(1).await.unwrap();
		assert!(retrieved.is_some());
		assert_eq!(retrieved.unwrap(), user);
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_flush_clears_dirty() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool).await.unwrap();

		let user = TestUser {
			id: Some(1),
			name: "Charlie".to_string(),
			email: "charlie@example.com".to_string(),
		};

		session.add(user).await.unwrap();
		assert_eq!(session.dirty_count(), 1);

		session.flush().await.unwrap();
		assert_eq!(session.dirty_count(), 0);
		assert_eq!(session.identity_count(), 1);
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_delete_object() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool).await.unwrap();

		let user = TestUser {
			id: Some(1),
			name: "Dave".to_string(),
			email: "dave@example.com".to_string(),
		};

		session.add(user.clone()).await.unwrap();
		session.flush().await.unwrap();

		session.delete(user).await.unwrap();
		session.flush().await.unwrap();

		let retrieved: Option<TestUser> = session.get(1).await.unwrap();
		assert!(retrieved.is_none());
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_transaction_begin() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool).await.unwrap();

		assert!(!session.has_transaction());

		session.begin().await.unwrap();
		assert!(session.has_transaction());
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_transaction_commit() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool).await.unwrap();

		session.begin().await.unwrap();

		let user = TestUser {
			id: Some(1),
			name: "Eve".to_string(),
			email: "eve@example.com".to_string(),
		};

		session.add(user).await.unwrap();
		session.commit().await.unwrap();

		assert!(!session.has_transaction());
		assert_eq!(session.dirty_count(), 0);
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_transaction_rollback() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool).await.unwrap();

		session.begin().await.unwrap();

		let user = TestUser {
			id: Some(1),
			name: "Frank".to_string(),
			email: "frank@example.com".to_string(),
		};

		session.add(user).await.unwrap();
		assert_eq!(session.dirty_count(), 1);

		session.rollback().await.unwrap();

		assert!(!session.has_transaction());
		assert_eq!(session.dirty_count(), 0);
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_close() {
		let pool = create_test_pool().await;
		let session = Session::new(pool).await.unwrap();

		assert!(!session.is_closed());

		session.close().await.unwrap();
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_operations_after_close() {
		let pool = create_test_pool().await;
		let session = Session::new(pool).await.unwrap();

		let _user = TestUser {
			id: Some(1),
			name: "Grace".to_string(),
			email: "grace@example.com".to_string(),
		};

		session.close().await.unwrap();

		// Cannot use session after close since it consumes self
		// This test verifies the API design
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_multiple_objects() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool).await.unwrap();

		for i in 1..=5 {
			let user = TestUser {
				id: Some(i),
				name: format!("User{}", i),
				email: format!("user{}@example.com", i),
			};
			session.add(user).await.unwrap();
		}

		assert_eq!(session.identity_count(), 5);
		assert_eq!(session.dirty_count(), 5);
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_delete_removes_from_dirty() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool).await.unwrap();

		let user = TestUser {
			id: Some(1),
			name: "Henry".to_string(),
			email: "henry@example.com".to_string(),
		};

		session.add(user.clone()).await.unwrap();
		assert_eq!(session.dirty_count(), 1);

		session.delete(user).await.unwrap();
		assert_eq!(session.dirty_count(), 0);
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_query_creation() {
		let pool = create_test_pool().await;
		let session = Session::new(pool).await.unwrap();

		let _query = session.query::<TestUser>();
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_double_begin_fails() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool).await.unwrap();

		session.begin().await.unwrap();
		let result = session.begin().await;

		assert!(result.is_err());
	}

	#[tokio::test]
	#[ignore = "Requires sqlx database driver to be installed"]
	async fn test_session_add_without_pk_fails() {
		let pool = create_test_pool().await;
		let mut session = Session::new(pool).await.unwrap();

		let user = TestUser {
			id: None,
			name: "Invalid".to_string(),
			email: "invalid@example.com".to_string(),
		};

		let result = session.add(user).await;
		assert!(result.is_err());
	}
}
