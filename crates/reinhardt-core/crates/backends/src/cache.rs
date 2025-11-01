//! Cache Backend Infrastructure
//!
//! This module provides specialized cache backends for high-performance caching.
//! Unlike the generic `Backend` trait, cache backends are optimized for
//! typical caching patterns including batch operations and binary data storage.
//!
//! # Available Backends
//!
//! - **Redis**: Distributed cache with connection pooling
//! - **Memcached**: High-performance memory cache with consistent hashing
//! - **DynamoDB**: Persistent cache with automatic TTL management
//!
//! # Examples
//!
//! ```
//! use reinhardt_backends::cache::{CacheBackend, redis::RedisCache};
//! use std::time::Duration;
//!
//! # #[cfg(feature = "redis-cache")]
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create Redis cache with connection pool
//! let cache = RedisCache::new("redis://localhost:6379").await?;
//!
//! // Store a value with TTL
//! cache.set("user:123", b"John Doe", Some(Duration::from_secs(3600))).await?;
//!
//! // Retrieve the value
//! let value = cache.get("user:123").await?;
//! assert_eq!(value, Some(b"John Doe".to_vec()));
//!
//! // Batch operations
//! let keys = vec!["key1".to_string(), "key2".to_string()];
//! let values = cache.get_many(&keys).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use std::time::Duration;
use thiserror::Error;

#[cfg(feature = "redis-cache")]
pub mod redis;

#[cfg(feature = "memcached-cache")]
pub mod memcached;

#[cfg(feature = "dynamodb-cache")]
pub mod dynamodb;

/// Cache-specific errors
#[derive(Debug, Error)]
pub enum CacheError {
	/// Key not found in cache
	#[error("Key not found: {0}")]
	NotFound(String),

	/// Connection error
	#[error("Connection error: {0}")]
	Connection(String),

	/// Serialization error
	#[error("Serialization error: {0}")]
	Serialization(String),

	/// Operation timeout
	#[error("Operation timeout: {0}")]
	Timeout(String),

	/// Internal cache error
	#[error("Internal error: {0}")]
	Internal(String),

	/// Configuration error
	#[error("Configuration error: {0}")]
	Configuration(String),
}

/// Result type for cache operations
pub type CacheResult<T> = Result<T, CacheError>;

/// Cache backend trait for high-performance caching
///
/// This trait provides a specialized interface for cache backends,
/// optimized for common caching patterns including batch operations
/// and binary data storage.
///
/// # Design Principles
///
/// - All values are stored as binary data (`Vec<u8>`) for flexibility
/// - Batch operations are first-class citizens for performance
/// - TTL support is mandatory for cache expiration
/// - All operations are async for non-blocking I/O
///
/// # Examples
///
/// ```rust
/// use reinhardt_backends::cache::CacheBackend;
/// use reinhardt_test::mock::DummyCache;
/// use std::time::Duration;
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let cache = DummyCache::new();
///
/// // Basic operations
/// cache.set("key", b"value", Some(Duration::from_secs(60))).await.unwrap();
/// let value = cache.get("key").await.unwrap();
/// assert_eq!(value, Some(b"value".to_vec()));
///
/// // Batch operations
/// let items = vec![
///     ("key1".to_string(), b"value1".to_vec()),
///     ("key2".to_string(), b"value2".to_vec()),
/// ];
/// cache.set_many(&items, Some(Duration::from_secs(60))).await.unwrap();
/// # });
/// ```
#[async_trait]
pub trait CacheBackend: Send + Sync {
	/// Retrieve a value from the cache
	///
	/// Returns `None` if the key doesn't exist or has expired.
	///
	/// # Arguments
	///
	/// * `key` - The key to retrieve
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_backends::cache::CacheBackend;
	/// use reinhardt_test::mock::DummyCache;
	///
	/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
	/// # let cache = DummyCache::new();
	/// # cache.set("user:123", b"data", None).await.unwrap();
	/// let value = cache.get("user:123").await.unwrap();
	/// if let Some(data) = value {
	///     assert_eq!(data, b"data");
	/// }
	/// # });
	/// ```
	async fn get(&self, key: &str) -> CacheResult<Option<Vec<u8>>>;

	/// Store a value in the cache with optional TTL
	///
	/// # Arguments
	///
	/// * `key` - The key to store the value under
	/// * `value` - The binary data to store
	/// * `ttl` - Optional time-to-live duration
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_backends::cache::CacheBackend;
	/// use reinhardt_test::mock::DummyCache;
	/// use std::time::Duration;
	///
	/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
	/// # let cache = DummyCache::new();
	/// // Store with TTL
	/// cache.set("session:abc", b"user_data", Some(Duration::from_secs(3600))).await.unwrap();
	///
	/// // Store without TTL (cache-dependent behavior)
	/// cache.set("permanent", b"data", None).await.unwrap();
	/// # });
	/// ```
	async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> CacheResult<()>;

	/// Delete a key from the cache
	///
	/// Returns `true` if the key existed, `false` otherwise.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_backends::cache::CacheBackend;
	/// use reinhardt_test::mock::DummyCache;
	///
	/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
	/// # let cache = DummyCache::new();
	/// # cache.set("old_key", b"data", None).await.unwrap();
	/// let deleted = cache.delete("old_key").await.unwrap();
	/// assert!(deleted);
	/// # });
	/// ```
	async fn delete(&self, key: &str) -> CacheResult<bool>;

	/// Check if a key exists in the cache
	///
	/// Returns `true` if the key exists and hasn't expired.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_backends::cache::CacheBackend;
	/// use reinhardt_test::mock::DummyCache;
	///
	/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
	/// # let cache = DummyCache::new();
	/// # cache.set("user:123", b"data", None).await.unwrap();
	/// assert!(cache.exists("user:123").await.unwrap());
	/// # });
	/// ```
	async fn exists(&self, key: &str) -> CacheResult<bool>;

	/// Clear all keys from the cache
	///
	/// **Warning**: This operation removes all data from the cache.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_backends::cache::CacheBackend;
	/// use reinhardt_test::mock::DummyCache;
	///
	/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
	/// # let cache = DummyCache::new();
	/// # cache.set("key1", b"val1", None).await.unwrap();
	/// cache.clear().await.unwrap();
	/// assert!(!cache.exists("key1").await.unwrap());
	/// # });
	/// ```
	async fn clear(&self) -> CacheResult<()>;

	/// Retrieve multiple values in a single operation
	///
	/// Returns a vector of `Option<Vec<u8>>` corresponding to each key.
	/// Missing or expired keys will be `None`.
	///
	/// # Arguments
	///
	/// * `keys` - Slice of keys to retrieve
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_backends::cache::CacheBackend;
	/// use reinhardt_test::mock::DummyCache;
	///
	/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
	/// # let cache = DummyCache::new();
	/// # cache.set("key1", b"val1", None).await.unwrap();
	/// # cache.set("key2", b"val2", None).await.unwrap();
	/// let keys = vec!["key1".to_string(), "key2".to_string(), "key3".to_string()];
	/// let values = cache.get_many(&keys).await.unwrap();
	/// assert_eq!(values[0], Some(b"val1".to_vec()));
	/// assert_eq!(values[1], Some(b"val2".to_vec()));
	/// assert_eq!(values[2], None);
	/// # });
	/// ```
	async fn get_many(&self, keys: &[String]) -> CacheResult<Vec<Option<Vec<u8>>>>;

	/// Store multiple key-value pairs in a single operation
	///
	/// All items will have the same TTL applied.
	///
	/// # Arguments
	///
	/// * `items` - Slice of (key, value) tuples
	/// * `ttl` - Optional time-to-live duration for all items
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_backends::cache::CacheBackend;
	/// use reinhardt_test::mock::DummyCache;
	/// use std::time::Duration;
	///
	/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
	/// # let cache = DummyCache::new();
	/// let items = vec![
	///     ("user:1".to_string(), b"Alice".to_vec()),
	///     ("user:2".to_string(), b"Bob".to_vec()),
	///     ("user:3".to_string(), b"Charlie".to_vec()),
	/// ];
	///
	/// cache.set_many(&items, Some(Duration::from_secs(3600))).await.unwrap();
	/// assert!(cache.exists("user:1").await.unwrap());
	/// assert!(cache.exists("user:2").await.unwrap());
	/// # });
	/// ```
	async fn set_many(&self, items: &[(String, Vec<u8>)], ttl: Option<Duration>)
	-> CacheResult<()>;

	/// Delete multiple keys in a single operation
	///
	/// Returns the number of keys that were actually deleted.
	///
	/// # Arguments
	///
	/// * `keys` - Slice of keys to delete
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_backends::cache::CacheBackend;
	/// use reinhardt_test::mock::DummyCache;
	///
	/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
	/// # let cache = DummyCache::new();
	/// # cache.set("old:1", b"val1", None).await.unwrap();
	/// # cache.set("old:2", b"val2", None).await.unwrap();
	/// let keys = vec!["old:1".to_string(), "old:2".to_string()];
	/// let deleted = cache.delete_many(&keys).await.unwrap();
	/// assert_eq!(deleted, 2);
	/// # });
	/// ```
	async fn delete_many(&self, keys: &[String]) -> CacheResult<usize>;
}

#[cfg(test)]
mod tests {
	use super::*;

	// Test that CacheBackend trait can be used with generic types
	async fn test_cache_generic<C: CacheBackend>(cache: &C) -> CacheResult<()> {
		cache
			.set("test", b"value", Some(Duration::from_secs(60)))
			.await?;
		let value = cache.get("test").await?;
		assert_eq!(value, Some(b"value".to_vec()));
		Ok(())
	}

	struct DummyCache {
		storage: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>>,
	}

	impl DummyCache {
		fn new() -> Self {
			Self {
				storage: std::sync::Arc::new(std::sync::Mutex::new(
					std::collections::HashMap::new(),
				)),
			}
		}
	}

	#[async_trait]
	impl CacheBackend for DummyCache {
		async fn get(&self, key: &str) -> CacheResult<Option<Vec<u8>>> {
			Ok(self.storage.lock().unwrap().get(key).cloned())
		}

		async fn set(&self, key: &str, value: &[u8], _ttl: Option<Duration>) -> CacheResult<()> {
			self.storage
				.lock()
				.unwrap()
				.insert(key.to_string(), value.to_vec());
			Ok(())
		}

		async fn delete(&self, key: &str) -> CacheResult<bool> {
			Ok(self.storage.lock().unwrap().remove(key).is_some())
		}

		async fn exists(&self, key: &str) -> CacheResult<bool> {
			Ok(self.storage.lock().unwrap().contains_key(key))
		}

		async fn clear(&self) -> CacheResult<()> {
			self.storage.lock().unwrap().clear();
			Ok(())
		}

		async fn get_many(&self, keys: &[String]) -> CacheResult<Vec<Option<Vec<u8>>>> {
			let storage = self.storage.lock().unwrap();
			Ok(keys.iter().map(|k| storage.get(k).cloned()).collect())
		}

		async fn set_many(
			&self,
			items: &[(String, Vec<u8>)],
			_ttl: Option<Duration>,
		) -> CacheResult<()> {
			let mut storage = self.storage.lock().unwrap();
			for (key, value) in items {
				storage.insert(key.clone(), value.clone());
			}
			Ok(())
		}

		async fn delete_many(&self, keys: &[String]) -> CacheResult<usize> {
			let mut storage = self.storage.lock().unwrap();
			let mut count = 0;
			for key in keys {
				if storage.remove(key).is_some() {
					count += 1;
				}
			}
			Ok(count)
		}
	}

	#[tokio::test]
	async fn test_cache_trait_usage() {
		let cache = DummyCache::new();
		test_cache_generic(&cache).await.unwrap();
	}
}
