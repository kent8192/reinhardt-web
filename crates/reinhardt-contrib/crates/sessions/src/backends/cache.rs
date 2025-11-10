//! Cache-based session backend
//!
//! This module provides session storage using cache backends like Redis or in-memory cache.
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_sessions::backends::{InMemorySessionBackend, SessionBackend};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create an in-memory session backend
//! let backend = InMemorySessionBackend::new();
//!
//! // Store user session with login data
//! let session_data = json!({
//!     "user_id": 123,
//!     "username": "bob",
//!     "last_login": "2024-01-15T10:30:00Z",
//! });
//!
//! backend.save("session_xyz", &session_data, Some(3600)).await?;
//!
//! // Load session data
//! let loaded: Option<serde_json::Value> = backend.load("session_xyz").await?;
//! assert!(loaded.is_some());
//! assert_eq!(loaded.unwrap()["user_id"], 123);
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use reinhardt_utils::cache::{Cache, InMemoryCache};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Session backend errors
#[derive(Debug, Error)]
pub enum SessionError {
	#[error("Cache error: {0}")]
	CacheError(String),
	#[error("Serialization error: {0}")]
	SerializationError(String),
	#[error("Session has expired due to inactivity")]
	SessionExpired,
}

/// Session backend trait
#[async_trait]
pub trait SessionBackend: Send + Sync + Clone {
	/// Load session data by key
	async fn load<T>(&self, session_key: &str) -> Result<Option<T>, SessionError>
	where
		T: for<'de> Deserialize<'de> + Serialize + Send + Sync;

	/// Save session data with optional TTL (in seconds)
	async fn save<T>(
		&self,
		session_key: &str,
		data: &T,
		ttl: Option<u64>,
	) -> Result<(), SessionError>
	where
		T: Serialize + Send + Sync;

	/// Delete session by key
	async fn delete(&self, session_key: &str) -> Result<(), SessionError>;

	/// Check if session exists
	async fn exists(&self, session_key: &str) -> Result<bool, SessionError>;
}

/// In-memory session backend
///
/// Stores sessions in memory using the InMemoryCache backend.
/// Sessions are lost when the application restarts.
///
/// ## Example
///
/// ```rust
/// use reinhardt_sessions::backends::{InMemorySessionBackend, SessionBackend};
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let backend = InMemorySessionBackend::new();
///
// Store shopping cart in session
/// let cart_data = json!({
///     "items": ["item1", "item2"],
///     "total": 59.99,
/// });
///
/// backend.save("cart_session_456", &cart_data, Some(1800)).await?;
///
// Check if session exists
/// assert!(backend.exists("cart_session_456").await?);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct InMemorySessionBackend {
	cache: Arc<InMemoryCache>,
}

impl InMemorySessionBackend {
	/// Create a new in-memory session backend
	pub fn new() -> Self {
		Self {
			cache: Arc::new(InMemoryCache::new()),
		}
	}
}

impl Default for InMemorySessionBackend {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl SessionBackend for InMemorySessionBackend {
	async fn load<T>(&self, session_key: &str) -> Result<Option<T>, SessionError>
	where
		T: for<'de> Deserialize<'de> + Serialize + Send + Sync,
	{
		self.cache
			.get(session_key)
			.await
			.map_err(|e| SessionError::CacheError(e.to_string()))
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
		let duration = ttl.map(std::time::Duration::from_secs);
		self.cache
			.set(session_key, data, duration)
			.await
			.map_err(|e| SessionError::CacheError(e.to_string()))
	}

	async fn delete(&self, session_key: &str) -> Result<(), SessionError> {
		self.cache
			.delete(session_key)
			.await
			.map_err(|e| SessionError::CacheError(e.to_string()))
	}

	async fn exists(&self, session_key: &str) -> Result<bool, SessionError> {
		self.cache
			.has_key(session_key)
			.await
			.map_err(|e| SessionError::CacheError(e.to_string()))
	}
}

/// Cache-based session backend
///
/// Generic session backend that works with any cache implementation.
///
/// ## Example
///
/// ```rust
/// use reinhardt_sessions::backends::{CacheSessionBackend, SessionBackend};
/// use reinhardt_cache::InMemoryCache;
/// use serde_json::json;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let cache = Arc::new(InMemoryCache::new());
/// let backend = CacheSessionBackend::new(cache);
///
// Store user preferences in session
/// let preferences = json!({
///     "theme": "dark",
///     "language": "en",
///     "notifications": true,
/// });
///
/// backend.save("pref_session_789", &preferences, Some(7200)).await?;
///
// Load preferences
/// let loaded: Option<serde_json::Value> = backend.load("pref_session_789").await?;
/// assert_eq!(loaded.unwrap()["theme"], "dark");
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct CacheSessionBackend<C: Cache + Clone> {
	cache: Arc<C>,
}

impl<C: Cache + Clone> CacheSessionBackend<C> {
	/// Create a new cache-based session backend
	pub fn new(cache: Arc<C>) -> Self {
		Self { cache }
	}
}

#[async_trait]
impl<C: Cache + Clone + 'static> SessionBackend for CacheSessionBackend<C> {
	async fn load<T>(&self, session_key: &str) -> Result<Option<T>, SessionError>
	where
		T: for<'de> Deserialize<'de> + Serialize + Send + Sync,
	{
		self.cache
			.get(session_key)
			.await
			.map_err(|e| SessionError::CacheError(e.to_string()))
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
		let duration = ttl.map(std::time::Duration::from_secs);
		self.cache
			.set(session_key, data, duration)
			.await
			.map_err(|e| SessionError::CacheError(e.to_string()))
	}

	async fn delete(&self, session_key: &str) -> Result<(), SessionError> {
		self.cache
			.delete(session_key)
			.await
			.map_err(|e| SessionError::CacheError(e.to_string()))
	}

	async fn exists(&self, session_key: &str) -> Result<bool, SessionError> {
		self.cache
			.has_key(session_key)
			.await
			.map_err(|e| SessionError::CacheError(e.to_string()))
	}
}
