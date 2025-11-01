//! Token storage backends for authentication
//!
//! Provides persistent storage for authentication tokens including
//! database and Redis backends.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Token storage error
#[derive(Debug, Clone)]
pub enum TokenStorageError {
	/// Token not found
	NotFound,
	/// Token expired
	Expired,
	/// Storage error
	StorageError(String),
	/// Invalid token format
	InvalidFormat,
}

impl std::fmt::Display for TokenStorageError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			TokenStorageError::NotFound => write!(f, "Token not found"),
			TokenStorageError::Expired => write!(f, "Token expired"),
			TokenStorageError::StorageError(msg) => write!(f, "Storage error: {}", msg),
			TokenStorageError::InvalidFormat => write!(f, "Invalid token format"),
		}
	}
}

impl std::error::Error for TokenStorageError {}

/// Result type for token storage operations
pub type TokenStorageResult<T> = Result<T, TokenStorageError>;

/// Stored token information
///
/// # Examples
///
/// ```
/// use reinhardt_auth::StoredToken;
///
/// let token = StoredToken {
///     token: "abc123".to_string(),
///     user_id: 42,
///     expires_at: None,
///     metadata: Default::default(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
	/// The token value
	pub token: String,
	/// Associated user ID
	pub user_id: i64,
	/// Expiration timestamp (Unix timestamp in seconds)
	pub expires_at: Option<i64>,
	/// Additional metadata
	pub metadata: HashMap<String, String>,
}

impl StoredToken {
	/// Create a new stored token
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::StoredToken;
	///
	/// let token = StoredToken::new("abc123", 42);
	/// assert_eq!(token.token(), "abc123");
	/// assert_eq!(token.user_id(), 42);
	/// ```
	pub fn new(token: impl Into<String>, user_id: i64) -> Self {
		Self {
			token: token.into(),
			user_id,
			expires_at: None,
			metadata: HashMap::new(),
		}
	}

	/// Set the expiration timestamp
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::StoredToken;
	///
	/// let token = StoredToken::new("abc123", 42)
	///     .with_expiration(1234567890);
	/// assert_eq!(token.expires_at(), Some(1234567890));
	/// ```
	pub fn with_expiration(mut self, expires_at: i64) -> Self {
		self.expires_at = Some(expires_at);
		self
	}

	/// Add metadata to the token
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::StoredToken;
	///
	/// let token = StoredToken::new("abc123", 42)
	///     .with_metadata("device", "mobile");
	/// ```
	pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
		self.metadata.insert(key.into(), value.into());
		self
	}

	/// Get the token value
	pub fn token(&self) -> &str {
		&self.token
	}

	/// Get the user ID
	pub fn user_id(&self) -> i64 {
		self.user_id
	}

	/// Get the expiration timestamp
	pub fn expires_at(&self) -> Option<i64> {
		self.expires_at
	}

	/// Check if the token is expired
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::StoredToken;
	///
	/// let token = StoredToken::new("abc123", 42)
	///     .with_expiration(1234567890);
	///
	/// // Check against a specific timestamp
	/// assert!(token.is_expired(2000000000));
	/// assert!(!token.is_expired(1000000000));
	/// ```
	pub fn is_expired(&self, current_time: i64) -> bool {
		if let Some(expires_at) = self.expires_at {
			current_time >= expires_at
		} else {
			false
		}
	}

	/// Get metadata value
	pub fn get_metadata(&self, key: &str) -> Option<&str> {
		self.metadata.get(key).map(|s| s.as_str())
	}
}

/// Token storage backend trait
///
/// Defines the interface for token storage implementations.
#[async_trait]
pub trait TokenStorage: Send + Sync {
	/// Store a token
	async fn store(&self, token: StoredToken) -> TokenStorageResult<()>;

	/// Retrieve a token by its value
	async fn get(&self, token: &str) -> TokenStorageResult<StoredToken>;

	/// Retrieve all tokens for a user
	async fn get_user_tokens(&self, user_id: i64) -> TokenStorageResult<Vec<StoredToken>>;

	/// Delete a token
	async fn delete(&self, token: &str) -> TokenStorageResult<()>;

	/// Delete all tokens for a user
	async fn delete_user_tokens(&self, user_id: i64) -> TokenStorageResult<()>;

	/// Delete expired tokens
	async fn cleanup_expired(&self, current_time: i64) -> TokenStorageResult<usize>;
}

/// In-memory token storage
///
/// Simple in-memory storage for development and testing.
/// Not recommended for production use.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::InMemoryTokenStorage;
///
/// let storage = InMemoryTokenStorage::new();
/// ```
#[derive(Debug, Clone)]
pub struct InMemoryTokenStorage {
	tokens: Arc<RwLock<HashMap<String, StoredToken>>>,
}

impl InMemoryTokenStorage {
	/// Create a new in-memory token storage
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::InMemoryTokenStorage;
	///
	/// let storage = InMemoryTokenStorage::new();
	/// ```
	pub fn new() -> Self {
		Self {
			tokens: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	/// Get the number of stored tokens
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::InMemoryTokenStorage;
	///
	/// let storage = InMemoryTokenStorage::new();
	/// assert_eq!(storage.len(), 0);
	/// ```
	pub fn len(&self) -> usize {
		self.tokens.read().unwrap().len()
	}

	/// Check if storage is empty
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::InMemoryTokenStorage;
	///
	/// let storage = InMemoryTokenStorage::new();
	/// assert!(storage.is_empty());
	/// ```
	pub fn is_empty(&self) -> bool {
		self.tokens.read().unwrap().is_empty()
	}

	/// Clear all tokens
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::InMemoryTokenStorage;
	///
	/// let storage = InMemoryTokenStorage::new();
	/// storage.clear();
	/// assert!(storage.is_empty());
	/// ```
	pub fn clear(&self) {
		self.tokens.write().unwrap().clear();
	}
}

impl Default for InMemoryTokenStorage {
	fn default() -> Self {
		Self::new()
	}
}

#[async_trait]
impl TokenStorage for InMemoryTokenStorage {
	async fn store(&self, token: StoredToken) -> TokenStorageResult<()> {
		let mut tokens = self.tokens.write().unwrap();
		tokens.insert(token.token.clone(), token);
		Ok(())
	}

	async fn get(&self, token: &str) -> TokenStorageResult<StoredToken> {
		let tokens = self.tokens.read().unwrap();
		tokens
			.get(token)
			.cloned()
			.ok_or(TokenStorageError::NotFound)
	}

	async fn get_user_tokens(&self, user_id: i64) -> TokenStorageResult<Vec<StoredToken>> {
		let tokens = self.tokens.read().unwrap();
		let user_tokens: Vec<StoredToken> = tokens
			.values()
			.filter(|t| t.user_id == user_id)
			.cloned()
			.collect();
		Ok(user_tokens)
	}

	async fn delete(&self, token: &str) -> TokenStorageResult<()> {
		let mut tokens = self.tokens.write().unwrap();
		tokens
			.remove(token)
			.ok_or(TokenStorageError::NotFound)
			.map(|_| ())
	}

	async fn delete_user_tokens(&self, user_id: i64) -> TokenStorageResult<()> {
		let mut tokens = self.tokens.write().unwrap();
		tokens.retain(|_, t| t.user_id != user_id);
		Ok(())
	}

	async fn cleanup_expired(&self, current_time: i64) -> TokenStorageResult<usize> {
		let mut tokens = self.tokens.write().unwrap();
		let before_count = tokens.len();
		tokens.retain(|_, t| !t.is_expired(current_time));
		let removed = before_count - tokens.len();
		Ok(removed)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_stored_token_creation() {
		let token = StoredToken::new("abc123", 42);
		assert_eq!(token.token(), "abc123");
		assert_eq!(token.user_id(), 42);
		assert_eq!(token.expires_at(), None);
	}

	#[test]
	fn test_stored_token_with_expiration() {
		let token = StoredToken::new("abc123", 42).with_expiration(1234567890);
		assert_eq!(token.expires_at(), Some(1234567890));
	}

	#[test]
	fn test_stored_token_is_expired() {
		let token = StoredToken::new("abc123", 42).with_expiration(1234567890);
		assert!(!token.is_expired(1000000000));
		assert!(token.is_expired(2000000000));
	}

	#[test]
	fn test_stored_token_metadata() {
		let token = StoredToken::new("abc123", 42)
			.with_metadata("device", "mobile")
			.with_metadata("ip", "192.168.1.1");

		assert_eq!(token.get_metadata("device"), Some("mobile"));
		assert_eq!(token.get_metadata("ip"), Some("192.168.1.1"));
		assert_eq!(token.get_metadata("nonexistent"), None);
	}

	#[tokio::test]
	async fn test_in_memory_storage_store_and_get() {
		let storage = InMemoryTokenStorage::new();
		let token = StoredToken::new("abc123", 42);

		storage.store(token.clone()).await.unwrap();
		let retrieved = storage.get("abc123").await.unwrap();

		assert_eq!(retrieved.token(), "abc123");
		assert_eq!(retrieved.user_id(), 42);
	}

	#[tokio::test]
	async fn test_in_memory_storage_get_nonexistent() {
		let storage = InMemoryTokenStorage::new();
		let result = storage.get("nonexistent").await;

		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), TokenStorageError::NotFound));
	}

	#[tokio::test]
	async fn test_in_memory_storage_get_user_tokens() {
		let storage = InMemoryTokenStorage::new();

		storage.store(StoredToken::new("token1", 1)).await.unwrap();
		storage.store(StoredToken::new("token2", 1)).await.unwrap();
		storage.store(StoredToken::new("token3", 2)).await.unwrap();

		let user1_tokens = storage.get_user_tokens(1).await.unwrap();
		assert_eq!(user1_tokens.len(), 2);

		let user2_tokens = storage.get_user_tokens(2).await.unwrap();
		assert_eq!(user2_tokens.len(), 1);
	}

	#[tokio::test]
	async fn test_in_memory_storage_delete() {
		let storage = InMemoryTokenStorage::new();
		storage.store(StoredToken::new("abc123", 42)).await.unwrap();

		assert!(storage.get("abc123").await.is_ok());

		storage.delete("abc123").await.unwrap();

		assert!(storage.get("abc123").await.is_err());
	}

	#[tokio::test]
	async fn test_in_memory_storage_delete_user_tokens() {
		let storage = InMemoryTokenStorage::new();

		storage.store(StoredToken::new("token1", 1)).await.unwrap();
		storage.store(StoredToken::new("token2", 1)).await.unwrap();
		storage.store(StoredToken::new("token3", 2)).await.unwrap();

		storage.delete_user_tokens(1).await.unwrap();

		let user1_tokens = storage.get_user_tokens(1).await.unwrap();
		assert_eq!(user1_tokens.len(), 0);

		let user2_tokens = storage.get_user_tokens(2).await.unwrap();
		assert_eq!(user2_tokens.len(), 1);
	}

	#[tokio::test]
	async fn test_in_memory_storage_cleanup_expired() {
		let storage = InMemoryTokenStorage::new();

		storage
			.store(StoredToken::new("token1", 1).with_expiration(1000))
			.await
			.unwrap();
		storage
			.store(StoredToken::new("token2", 2).with_expiration(2000))
			.await
			.unwrap();
		storage
			.store(StoredToken::new("token3", 3).with_expiration(3000))
			.await
			.unwrap();

		let removed = storage.cleanup_expired(1500).await.unwrap();
		assert_eq!(removed, 1);

		assert!(storage.get("token1").await.is_err());
		assert!(storage.get("token2").await.is_ok());
		assert!(storage.get("token3").await.is_ok());
	}

	#[test]
	fn test_in_memory_storage_len_and_is_empty() {
		let storage = InMemoryTokenStorage::new();
		assert_eq!(storage.len(), 0);
		assert!(storage.is_empty());
	}

	#[test]
	fn test_in_memory_storage_clear() {
		let storage = InMemoryTokenStorage::new();
		storage.clear();
		assert!(storage.is_empty());
	}
}
