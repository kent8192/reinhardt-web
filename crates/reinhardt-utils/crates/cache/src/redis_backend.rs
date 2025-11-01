//! Redis cache backend
//!
//! Provides a Redis-backed cache implementation with connection pooling.

#![cfg(feature = "redis-backend")]

use crate::Cache;
use async_trait::async_trait;
use deadpool_redis::{Config as PoolConfig, Pool, Runtime};
use redis::AsyncCommands;
use reinhardt_exception::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Redis cache backend with connection pooling
///
/// Stores cached values in Redis for distributed caching.
/// Uses deadpool-redis for efficient connection management.
#[derive(Clone)]
pub struct RedisCache {
	pool: Pool,
	default_ttl: Option<Duration>,
	key_prefix: String,
}

impl RedisCache {
	/// Create a new Redis cache with the given connection URL
	///
	/// Uses default pool configuration (max_size: 16, timeouts: 5s).
	/// For custom pool configuration, use `with_pool_config()`.
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_cache::RedisCache;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let cache = RedisCache::new("redis://localhost:6379").await?;
	/// // Redis cache with connection pooling is now ready
	/// # Ok(())
	/// # }
	/// ```
	pub async fn new(connection_url: impl Into<String>) -> Result<Self> {
		let url = connection_url.into();
		let cfg = PoolConfig::from_url(url);
		let pool = cfg
			.create_pool(Some(Runtime::Tokio1))
			.map_err(|e| Error::Http(format!("Failed to create Redis pool: {}", e)))?;

		Ok(Self {
			pool,
			default_ttl: None,
			key_prefix: String::new(),
		})
	}

	/// Create a new Redis cache with custom pool configuration
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_cache::RedisCache;
	/// use deadpool_redis::{Config, Runtime};
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut config = Config::from_url("redis://localhost:6379");
	/// config.pool = Some(deadpool_redis::PoolConfig::new(32)); // 32 connections
	///
	/// let cache = RedisCache::with_pool_config(config)?;
	/// # Ok(())
	/// # }
	/// ```
	pub fn with_pool_config(config: PoolConfig) -> Result<Self> {
		let pool = config
			.create_pool(Some(Runtime::Tokio1))
			.map_err(|e| Error::Http(format!("Failed to create Redis pool: {}", e)))?;

		Ok(Self {
			pool,
			default_ttl: None,
			key_prefix: String::new(),
		})
	}

	/// Get the connection pool
	pub fn pool(&self) -> &Pool {
		&self.pool
	}
	/// Set default TTL for all cache entries
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_cache::RedisCache;
	/// use std::time::Duration;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let cache = RedisCache::new("redis://localhost:6379")
	///     .await?
	///     .with_default_ttl(Duration::from_secs(300));
	/// // All cache entries will expire after 300 seconds by default
	/// # Ok(())
	/// # }
	/// ```
	pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
		self.default_ttl = Some(ttl);
		self
	}

	/// Set key prefix for namespacing cache entries
	///
	/// # Examples
	///
	/// ```no_run
	/// use reinhardt_cache::RedisCache;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let cache = RedisCache::new("redis://localhost:6379")
	///     .await?
	///     .with_key_prefix("myapp");
	/// // All keys will be prefixed with "myapp:"
	/// # Ok(())
	/// # }
	/// ```
	pub fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self {
		self.key_prefix = prefix.into();
		self
	}

	/// Build the full key with prefix
	fn build_key(&self, key: &str) -> String {
		if self.key_prefix.is_empty() {
			key.to_string()
		} else {
			format!("{}:{}", self.key_prefix, key)
		}
	}
}

#[async_trait]
impl Cache for RedisCache {
	async fn get<T>(&self, key: &str) -> Result<Option<T>>
	where
		T: for<'de> Deserialize<'de> + Send,
	{
		let full_key = self.build_key(key);
		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::Http(format!("Failed to get connection from pool: {}", e)))?;

		let value: Option<Vec<u8>> = conn
			.get(&full_key)
			.await
			.map_err(|e| Error::Http(format!("Failed to get value from Redis: {}", e)))?;

		match value {
			Some(bytes) => {
				let deserialized = serde_json::from_slice(&bytes)
					.map_err(|e| Error::Serialization(e.to_string()))?;
				Ok(Some(deserialized))
			}
			None => Ok(None),
		}
	}

	async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<()>
	where
		T: Serialize + Send + Sync,
	{
		let full_key = self.build_key(key);
		let serialized =
			serde_json::to_vec(value).map_err(|e| Error::Serialization(e.to_string()))?;
		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::Http(format!("Failed to get connection from pool: {}", e)))?;

		let effective_ttl = ttl.or(self.default_ttl);

		if let Some(ttl_duration) = effective_ttl {
			let seconds = ttl_duration.as_secs();
			let _: () = conn
				.set_ex(&full_key, serialized, seconds)
				.await
				.map_err(|e| Error::Http(format!("Failed to set value in Redis: {}", e)))?;
		} else {
			let _: () = conn
				.set(&full_key, serialized)
				.await
				.map_err(|e| Error::Http(format!("Failed to set value in Redis: {}", e)))?;
		}

		Ok(())
	}

	async fn delete(&self, key: &str) -> Result<()> {
		let full_key = self.build_key(key);
		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::Http(format!("Failed to get connection from pool: {}", e)))?;

		let _: () = conn
			.del(&full_key)
			.await
			.map_err(|e| Error::Http(format!("Failed to delete value from Redis: {}", e)))?;

		Ok(())
	}

	async fn has_key(&self, key: &str) -> Result<bool> {
		let full_key = self.build_key(key);
		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::Http(format!("Failed to get connection from pool: {}", e)))?;

		let exists: bool = conn
			.exists(&full_key)
			.await
			.map_err(|e| Error::Http(format!("Failed to check key existence in Redis: {}", e)))?;

		Ok(exists)
	}

	async fn clear(&self) -> Result<()> {
		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::Http(format!("Failed to get connection from pool: {}", e)))?;

		if self.key_prefix.is_empty() {
			// Clear all keys if no prefix
			let _: () = redis::cmd("FLUSHDB")
				.query_async(&mut *conn)
				.await
				.map_err(|e| Error::Http(format!("Failed to clear Redis cache: {}", e)))?;
		} else {
			// Delete all keys with the prefix
			let pattern = format!("{}:*", self.key_prefix);
			let keys: Vec<String> = redis::cmd("KEYS")
				.arg(&pattern)
				.query_async(&mut *conn)
				.await
				.map_err(|e| Error::Http(format!("Failed to get keys matching pattern: {}", e)))?;

			if !keys.is_empty() {
				let _: () = conn
					.del(keys)
					.await
					.map_err(|e| Error::Http(format!("Failed to delete keys: {}", e)))?;
			}
		}

		Ok(())
	}

	async fn get_many<T>(&self, keys: &[&str]) -> Result<std::collections::HashMap<String, T>>
	where
		T: for<'de> Deserialize<'de> + Send,
	{
		let full_keys: Vec<String> = keys.iter().map(|k| self.build_key(k)).collect();
		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::Http(format!("Failed to get connection from pool: {}", e)))?;

		let values: Vec<Option<Vec<u8>>> = conn
			.get(&full_keys)
			.await
			.map_err(|e| Error::Http(format!("Failed to get multiple values from Redis: {}", e)))?;

		let mut results = std::collections::HashMap::new();
		for (i, value_opt) in values.into_iter().enumerate() {
			if let Some(bytes) = value_opt {
				let deserialized: T = serde_json::from_slice(&bytes)
					.map_err(|e| Error::Serialization(e.to_string()))?;
				results.insert(keys[i].to_string(), deserialized);
			}
		}

		Ok(results)
	}

	async fn set_many<T>(
		&self,
		values: std::collections::HashMap<String, T>,
		ttl: Option<Duration>,
	) -> Result<()>
	where
		T: Serialize + Send + Sync,
	{
		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::Http(format!("Failed to get connection from pool: {}", e)))?;
		let effective_ttl = ttl.or(self.default_ttl);

		for (key, value) in values.iter() {
			let full_key = self.build_key(key);
			let serialized =
				serde_json::to_vec(value).map_err(|e| Error::Serialization(e.to_string()))?;

			if let Some(ttl_duration) = effective_ttl {
				let seconds = ttl_duration.as_secs();
				let _: () = conn
					.set_ex(&full_key, serialized, seconds)
					.await
					.map_err(|e| Error::Http(format!("Failed to set value in Redis: {}", e)))?;
			} else {
				let _: () = conn
					.set(&full_key, serialized)
					.await
					.map_err(|e| Error::Http(format!("Failed to set value in Redis: {}", e)))?;
			}
		}

		Ok(())
	}

	async fn delete_many(&self, keys: &[&str]) -> Result<()> {
		let full_keys: Vec<String> = keys.iter().map(|k| self.build_key(k)).collect();
		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::Http(format!("Failed to get connection from pool: {}", e)))?;

		let _: () = conn.del(full_keys).await.map_err(|e| {
			Error::Http(format!(
				"Failed to delete multiple values from Redis: {}",
				e
			))
		})?;

		Ok(())
	}

	async fn incr(&self, key: &str, delta: i64) -> Result<i64> {
		let full_key = self.build_key(key);
		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::Http(format!("Failed to get connection from pool: {}", e)))?;

		let result: i64 = conn
			.incr(&full_key, delta)
			.await
			.map_err(|e| Error::Http(format!("Failed to increment value in Redis: {}", e)))?;

		Ok(result)
	}

	async fn decr(&self, key: &str, delta: i64) -> Result<i64> {
		let full_key = self.build_key(key);
		let mut conn = self
			.pool
			.get()
			.await
			.map_err(|e| Error::Http(format!("Failed to get connection from pool: {}", e)))?;

		let result: i64 = conn
			.decr(&full_key, delta)
			.await
			.map_err(|e| Error::Http(format!("Failed to decrement value in Redis: {}", e)))?;

		Ok(result)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// Note: These tests require a running Redis instance
	// They will be skipped in CI unless REDIS_URL is set

	fn get_redis_url() -> String {
		std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
	}

	#[tokio::test]
	#[ignore] // Requires Redis to be running
	async fn test_redis_cache_creation() {
		let cache = RedisCache::new(get_redis_url())
			.await
			.unwrap()
			.with_default_ttl(Duration::from_secs(300))
			.with_key_prefix("myapp");

		assert_eq!(cache.key_prefix, "myapp");
		assert!(cache.default_ttl.is_some());
	}

	#[tokio::test]
	#[ignore] // Requires Redis to be running
	async fn test_build_key_with_prefix() {
		let cache = RedisCache::new(get_redis_url())
			.await
			.unwrap()
			.with_key_prefix("app");

		assert_eq!(cache.build_key("user:123"), "app:user:123");
	}

	#[tokio::test]
	#[ignore] // Requires Redis to be running
	async fn test_build_key_without_prefix() {
		let cache = RedisCache::new(get_redis_url()).await.unwrap();
		assert_eq!(cache.build_key("user:123"), "user:123");
	}

	#[tokio::test]
	#[ignore] // Requires Redis to be running
	async fn test_redis_cache_basic_operations() {
		let cache = RedisCache::new(get_redis_url()).await.unwrap();

		// Clear any existing data
		cache.clear().await.unwrap();

		// Set and get
		cache.set("test:key1", &"value1", None).await.unwrap();
		let value: Option<String> = cache.get("test:key1").await.unwrap();
		assert_eq!(value, Some("value1".to_string()));

		// Has key
		assert!(cache.has_key("test:key1").await.unwrap());
		assert!(!cache.has_key("test:nonexistent").await.unwrap());

		// Delete
		cache.delete("test:key1").await.unwrap();
		let value: Option<String> = cache.get("test:key1").await.unwrap();
		assert_eq!(value, None);

		// Cleanup
		cache.clear().await.unwrap();
	}

	#[tokio::test]
	#[ignore] // Requires Redis to be running
	async fn test_redis_cache_ttl() {
		let cache = RedisCache::new(get_redis_url()).await.unwrap();
		cache.clear().await.unwrap();

		// Set with short TTL
		cache
			.set("test:ttl", &"value", Some(Duration::from_secs(1)))
			.await
			.unwrap();

		// Should exist immediately
		let value: Option<String> = cache.get("test:ttl").await.unwrap();
		assert_eq!(value, Some("value".to_string()));

		// Wait for expiration
		tokio::time::sleep(Duration::from_secs(2)).await;

		// Should be expired
		let value: Option<String> = cache.get("test:ttl").await.unwrap();
		assert_eq!(value, None);

		cache.clear().await.unwrap();
	}

	#[tokio::test]
	#[ignore] // Requires Redis to be running
	async fn test_redis_cache_batch_operations() {
		let cache = RedisCache::new(get_redis_url()).await.unwrap();
		cache.clear().await.unwrap();

		// Set many
		let mut values = std::collections::HashMap::new();
		values.insert("test:key1".to_string(), "value1".to_string());
		values.insert("test:key2".to_string(), "value2".to_string());
		cache.set_many(values, None).await.unwrap();

		// Get many
		let results: std::collections::HashMap<String, String> = cache
			.get_many(&["test:key1", "test:key2", "test:key3"])
			.await
			.unwrap();
		assert_eq!(results.len(), 2);
		assert_eq!(results.get("test:key1"), Some(&"value1".to_string()));
		assert_eq!(results.get("test:key2"), Some(&"value2".to_string()));

		// Delete many
		cache
			.delete_many(&["test:key1", "test:key2"])
			.await
			.unwrap();
		assert!(!cache.has_key("test:key1").await.unwrap());
		assert!(!cache.has_key("test:key2").await.unwrap());

		cache.clear().await.unwrap();
	}

	#[tokio::test]
	#[ignore] // Requires Redis to be running
	async fn test_redis_cache_atomic_operations() {
		let cache = RedisCache::new(get_redis_url()).await.unwrap();
		cache.clear().await.unwrap();

		// Increment from zero
		let value = cache.incr("test:counter", 5).await.unwrap();
		assert_eq!(value, 5);

		// Increment again
		let value = cache.incr("test:counter", 3).await.unwrap();
		assert_eq!(value, 8);

		// Decrement
		let value = cache.decr("test:counter", 2).await.unwrap();
		assert_eq!(value, 6);

		cache.clear().await.unwrap();
	}

	#[tokio::test]
	#[ignore] // Requires Redis to be running
	async fn test_redis_cache_prefix() {
		let cache = RedisCache::new(get_redis_url())
			.await
			.unwrap()
			.with_key_prefix("myapp");

		cache.clear().await.unwrap();

		cache.set("user:123", &"data", None).await.unwrap();

		// Verify the key is stored with prefix
		let value: Option<String> = cache.get("user:123").await.unwrap();
		assert_eq!(value, Some("data".to_string()));

		// Clear should only clear prefixed keys
		cache.clear().await.unwrap();
		let value: Option<String> = cache.get("user:123").await.unwrap();
		assert_eq!(value, None);
	}
}
