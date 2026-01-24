//! State storage for OAuth2/OIDC CSRF protection
//!
//! Manages state, nonce, and code_verifier with TTL expiration.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::social::core::SocialAuthError;

/// Data stored for each OAuth2 state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateData {
	/// OAuth2 state parameter
	pub state: String,
	/// OIDC nonce parameter (optional)
	pub nonce: Option<String>,
	/// PKCE code verifier (optional)
	pub code_verifier: Option<String>,
	/// Expiration timestamp
	pub expires_at: DateTime<Utc>,
}

impl StateData {
	/// Creates new state data with default TTL (10 minutes)
	pub fn new(state: String, nonce: Option<String>, code_verifier: Option<String>) -> Self {
		Self {
			state,
			nonce,
			code_verifier,
			expires_at: Utc::now() + Duration::minutes(10),
		}
	}

	/// Creates new state data with custom TTL
	pub fn with_ttl(
		state: String,
		nonce: Option<String>,
		code_verifier: Option<String>,
		ttl: Duration,
	) -> Self {
		Self {
			state,
			nonce,
			code_verifier,
			expires_at: Utc::now() + ttl,
		}
	}

	/// Checks if the state has expired
	pub fn is_expired(&self) -> bool {
		Utc::now() > self.expires_at
	}
}

/// Trait for state storage implementations
#[async_trait]
pub trait StateStore: Send + Sync {
	/// Stores state data
	async fn store(&self, data: StateData) -> Result<(), SocialAuthError>;

	/// Retrieves state data by state string
	async fn retrieve(&self, state: &str) -> Result<StateData, SocialAuthError>;

	/// Removes state data by state string
	async fn remove(&self, state: &str) -> Result<(), SocialAuthError>;
}

/// In-memory state store for development and testing
///
/// This implementation is NOT suitable for production use in multi-instance deployments.
/// For production, use a distributed store like Redis or database-backed storage.
#[derive(Debug, Default)]
pub struct InMemoryStateStore {
	store: RwLock<HashMap<String, StateData>>,
}

impl InMemoryStateStore {
	/// Creates a new in-memory state store
	pub fn new() -> Self {
		Self {
			store: RwLock::new(HashMap::new()),
		}
	}

	/// Removes expired entries from the store
	async fn cleanup_expired(&self) {
		let mut store = self.store.write().await;
		store.retain(|_, data| !data.is_expired());
	}
}

#[async_trait]
impl StateStore for InMemoryStateStore {
	async fn store(&self, data: StateData) -> Result<(), SocialAuthError> {
		// Cleanup expired entries before storing
		self.cleanup_expired().await;

		let mut store = self.store.write().await;
		store.insert(data.state.clone(), data);
		Ok(())
	}

	async fn retrieve(&self, state: &str) -> Result<StateData, SocialAuthError> {
		let store = self.store.read().await;
		let data = store
			.get(state)
			.ok_or(SocialAuthError::InvalidState)?
			.clone();

		if data.is_expired() {
			return Err(SocialAuthError::InvalidState);
		}

		Ok(data)
	}

	async fn remove(&self, state: &str) -> Result<(), SocialAuthError> {
		let mut store = self.store.write().await;
		store.remove(state).ok_or(SocialAuthError::InvalidState)?;
		Ok(())
	}
}

/// Session-based state store for production use
///
/// This will integrate with Reinhardt's session management system.
pub struct SessionStateStore {
	// TODO: Integrate with session backend
}

impl Default for SessionStateStore {
	fn default() -> Self {
		Self::new()
	}
}

impl SessionStateStore {
	/// Creates a new session-based state store
	pub fn new() -> Self {
		todo!("Implement session-based state store")
	}
}

#[async_trait]
impl StateStore for SessionStateStore {
	async fn store(&self, _data: StateData) -> Result<(), SocialAuthError> {
		todo!("Implement session-based store")
	}

	async fn retrieve(&self, _state: &str) -> Result<StateData, SocialAuthError> {
		todo!("Implement session-based retrieve")
	}

	async fn remove(&self, _state: &str) -> Result<(), SocialAuthError> {
		todo!("Implement session-based remove")
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_state_data_expiration() {
		let data = StateData::new("test_state".to_string(), None, None);
		assert!(!data.is_expired());

		let expired_data = StateData::with_ttl(
			"expired_state".to_string(),
			None,
			None,
			Duration::seconds(-1),
		);
		assert!(expired_data.is_expired());
	}

	#[tokio::test]
	async fn test_in_memory_store_retrieve() {
		let store = InMemoryStateStore::new();
		let data = StateData::new(
			"test_state".to_string(),
			Some("test_nonce".to_string()),
			Some("test_verifier".to_string()),
		);

		store.store(data.clone()).await.unwrap();
		let retrieved = store.retrieve("test_state").await.unwrap();

		assert_eq!(retrieved.state, "test_state");
		assert_eq!(retrieved.nonce, Some("test_nonce".to_string()));
		assert_eq!(retrieved.code_verifier, Some("test_verifier".to_string()));
	}

	#[tokio::test]
	async fn test_in_memory_store_remove() {
		let store = InMemoryStateStore::new();
		let data = StateData::new("test_state".to_string(), None, None);

		store.store(data).await.unwrap();
		store.remove("test_state").await.unwrap();

		let result = store.retrieve("test_state").await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_in_memory_store_nonexistent() {
		let store = InMemoryStateStore::new();
		let result = store.retrieve("nonexistent").await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_in_memory_store_expired() {
		let store = InMemoryStateStore::new();
		let expired_data = StateData::with_ttl(
			"expired_state".to_string(),
			None,
			None,
			Duration::seconds(-1),
		);

		store.store(expired_data).await.unwrap();
		let result = store.retrieve("expired_state").await;
		assert!(result.is_err());
	}

	#[tokio::test]
	async fn test_cleanup_expired() {
		let store = InMemoryStateStore::new();

		// Store valid and expired data
		let valid_data = StateData::new("valid".to_string(), None, None);
		let expired_data =
			StateData::with_ttl("expired".to_string(), None, None, Duration::seconds(-1));

		store.store(valid_data).await.unwrap();
		store.store(expired_data).await.unwrap();

		// Cleanup should happen on next store operation
		let new_data = StateData::new("new".to_string(), None, None);
		store.store(new_data).await.unwrap();

		// Valid should still exist
		assert!(store.retrieve("valid").await.is_ok());
		// New should exist
		assert!(store.retrieve("new").await.is_ok());
		// Expired should be removed
		assert!(store.retrieve("expired").await.is_err());
	}
}
