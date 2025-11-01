//! Shared Backend Infrastructure
//!
//! This crate provides a unified backend system for storing and retrieving data
//! across different components of the Reinhardt framework, including:
//! - Throttling/Rate limiting
//! - Caching
//! - Session storage
//! - Email sending
//!
//! # Architecture
//!
//! ## Storage Backends
//!
//! The backend system is built around the `Backend` trait, which provides
//! a simple key-value interface with TTL support. Multiple implementations
//! are available:
//!
//! - **MemoryBackend**: In-memory storage with automatic expiration
//! - **RedisBackend**: Distributed storage using Redis
//!
//! ## Email Backends
//!
//! The email system is built around the `EmailBackend` trait, which provides
//! a unified interface for sending emails. Multiple implementations are available:
//!
//! - **MemoryEmailBackend**: In-memory storage for testing
//! - **SmtpBackend**: Direct SMTP server connection
//! - **SendGridBackend**: SendGrid API integration
//! - **SesBackend**: AWS Simple Email Service
//! - **MailgunBackend**: Mailgun API integration
//!
//! # Examples
//!
//! ## Storage Backend
//!
//! ```
//! use reinhardt_backends::{Backend, MemoryBackend};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     let backend = MemoryBackend::new();
//!
//!     // Store a value with TTL
//!     backend.set("user:123", "active", Some(Duration::from_secs(3600))).await.unwrap();
//!
//!     // Retrieve the value
//!     let value: Option<String> = backend.get("user:123").await.unwrap();
//!     assert_eq!(value, Some("active".to_string()));
//! }
//! ```
//!
//! ## Email Backend
//!
//! ```
//! use reinhardt_backends::email::{Email, EmailBackend, MemoryEmailBackend};
//!
//! #[tokio::main]
//! async fn main() {
//!     let backend = MemoryEmailBackend::new();
//!
//!     let email = Email::builder()
//!         .from("sender@example.com")
//!         .to("recipient@example.com")
//!         .subject("Hello")
//!         .text_body("Hello, World!")
//!         .build();
//!
//!     backend.send_email(&email).await.unwrap();
//! }
//! ```

use async_trait::async_trait;
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;
use thiserror::Error;

pub mod adapters;
pub mod memory;

#[cfg(feature = "redis-backend")]
pub mod redis_backend;

// Cache backends module
pub mod cache;

// Email backends module
pub mod email;

// Re-exports
pub use adapters::{ThrottleBackend as ThrottleBackendTrait, ThrottleBackendAdapter};
pub use memory::MemoryBackend;

#[cfg(feature = "redis-backend")]
pub use redis_backend::RedisBackend;

/// Backend errors
#[derive(Debug, Error)]
pub enum BackendError {
	/// Key not found
	#[error("Key not found: {0}")]
	NotFound(String),

	/// Serialization error
	#[error("Serialization error: {0}")]
	Serialization(String),

	/// Deserialization error
	#[error("Deserialization error: {0}")]
	Deserialization(String),

	/// Connection error
	#[error("Connection error: {0}")]
	Connection(String),

	/// Internal error
	#[error("Internal error: {0}")]
	Internal(String),
}

/// Result type for backend operations
pub type BackendResult<T> = Result<T, BackendError>;

/// Backend trait for key-value storage with TTL support
///
/// This trait provides a unified interface for different storage backends.
/// All operations are asynchronous and support automatic expiration via TTL.
///
/// # Type Parameters
///
/// The trait is generic over the value type, allowing type-safe storage
/// and retrieval. Values must implement `Serialize` and `DeserializeOwned`.
#[async_trait]
pub trait Backend: Send + Sync {
	/// Store a value with an optional TTL
	///
	/// # Arguments
	///
	/// * `key` - The key to store the value under
	/// * `value` - The value to store (must be serializable)
	/// * `ttl` - Optional time-to-live duration
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_backends::{Backend, MemoryBackend};
	/// # use std::time::Duration;
	/// # #[tokio::main]
	/// # async fn main() {
	/// let backend = MemoryBackend::new();
	/// backend.set("key", "value", Some(Duration::from_secs(60))).await.unwrap();
	/// # }
	/// ```
	async fn set<V: Serialize + Send + Sync>(
		&self,
		key: &str,
		value: V,
		ttl: Option<Duration>,
	) -> BackendResult<()>;

	/// Retrieve a value by key
	///
	/// Returns `None` if the key doesn't exist or has expired.
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_backends::{Backend, MemoryBackend};
	/// # #[tokio::main]
	/// # async fn main() {
	/// let backend = MemoryBackend::new();
	/// backend.set("key", "value", None).await.unwrap();
	///
	/// let value: Option<String> = backend.get("key").await.unwrap();
	/// assert_eq!(value, Some("value".to_string()));
	/// # }
	/// ```
	async fn get<V: DeserializeOwned>(&self, key: &str) -> BackendResult<Option<V>>;

	/// Delete a key
	///
	/// Returns `true` if the key existed, `false` otherwise.
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_backends::{Backend, MemoryBackend};
	/// # #[tokio::main]
	/// # async fn main() {
	/// let backend = MemoryBackend::new();
	/// backend.set("key", "value", None).await.unwrap();
	///
	/// let deleted = backend.delete("key").await.unwrap();
	/// assert!(deleted);
	/// # }
	/// ```
	async fn delete(&self, key: &str) -> BackendResult<bool>;

	/// Check if a key exists
	///
	/// Returns `true` if the key exists and hasn't expired.
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_backends::{Backend, MemoryBackend};
	/// # #[tokio::main]
	/// # async fn main() {
	/// let backend = MemoryBackend::new();
	/// backend.set("key", "value", None).await.unwrap();
	///
	/// let exists = backend.exists("key").await.unwrap();
	/// assert!(exists);
	/// # }
	/// ```
	async fn exists(&self, key: &str) -> BackendResult<bool>;

	/// Increment a counter
	///
	/// If the key doesn't exist, it will be created with the initial value of 1.
	/// Returns the new value after incrementing.
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_backends::{Backend, MemoryBackend};
	/// # #[tokio::main]
	/// # async fn main() {
	/// let backend = MemoryBackend::new();
	///
	/// let count1 = backend.increment("counter", Some(std::time::Duration::from_secs(60))).await.unwrap();
	/// assert_eq!(count1, 1);
	///
	/// let count2 = backend.increment("counter", Some(std::time::Duration::from_secs(60))).await.unwrap();
	/// assert_eq!(count2, 2);
	/// # }
	/// ```
	async fn increment(&self, key: &str, ttl: Option<Duration>) -> BackendResult<i64>;

	/// Clear all keys
	///
	/// **Warning**: This operation removes all data from the backend.
	///
	/// # Examples
	///
	/// ```
	/// # use reinhardt_backends::{Backend, MemoryBackend};
	/// # #[tokio::main]
	/// # async fn main() {
	/// let backend = MemoryBackend::new();
	/// backend.set("key1", "value1", None).await.unwrap();
	/// backend.set("key2", "value2", None).await.unwrap();
	///
	/// backend.clear().await.unwrap();
	///
	/// let exists = backend.exists("key1").await.unwrap();
	/// assert!(!exists);
	/// # }
	/// ```
	async fn clear(&self) -> BackendResult<()>;
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_backend_generic_usage() {
		// Test Backend usage with generic types
		async fn use_backend<B: Backend>(backend: &B) {
			backend.set("test", "value", None).await.unwrap();
			let value: Option<String> = backend.get("test").await.unwrap();
			assert_eq!(value, Some("value".to_string()));
		}

		let backend = MemoryBackend::new();
		use_backend(&backend).await;
	}

	#[tokio::test]
	async fn test_backend_arc_sharing() {
		// Test Backend sharing with Arc
		use std::sync::Arc;

		let backend = Arc::new(MemoryBackend::new());

		let backend1 = backend.clone();
		let backend2 = backend.clone();

		backend1.set("shared_key", "value", None).await.unwrap();

		let value: Option<String> = backend2.get("shared_key").await.unwrap();
		assert_eq!(value, Some("value".to_string()));
	}
}
