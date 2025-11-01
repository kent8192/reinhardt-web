//! Redis Cache Backend
//!
//! High-performance distributed cache using Redis with connection pooling.
//!
//! # Features
//!
//! - Connection pooling with automatic reconnection
//! - Pipeline support for batch operations
//! - Redis Cluster support
//! - Async/await based API
//!
//! # Examples
//!
//! ```no_run
//! use reinhardt_backends::cache::redis::RedisCache;
//! use reinhardt_backends::cache::CacheBackend;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create cache with default connection pool
//! let cache = RedisCache::new("redis://localhost:6379").await?;
//!
//! // Store and retrieve data
//! cache.set("key", b"value", Some(Duration::from_secs(60))).await?;
//! let value = cache.get("key").await?;
//! assert_eq!(value, Some(b"value".to_vec()));
//!
//! // Batch operations
//! let items = vec![
//!     ("key1".to_string(), b"value1".to_vec()),
//!     ("key2".to_string(), b"value2".to_vec()),
//! ];
//! cache.set_many(&items, Some(Duration::from_secs(60))).await?;
//! # Ok(())
//! # }
//! ```

use super::{CacheBackend, CacheError, CacheResult};
use async_trait::async_trait;
use deadpool_redis::{Config, Pool, Runtime};
use redis::{AsyncCommands, RedisError};
use std::time::Duration;

/// Redis cache backend with connection pooling
///
/// Provides high-performance caching using Redis as the backing store.
/// Uses connection pooling for efficient resource management.
pub struct RedisCache {
	pool: Pool,
}

impl RedisCache {
	/// Create a new Redis cache with default pool configuration
	///
	/// # Arguments
	///
	/// * `redis_url` - Redis connection URL (e.g., "redis://localhost:6379")
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_backends::cache::redis::RedisCache;
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let cache = RedisCache::new("redis://localhost:6379").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn new(redis_url: &str) -> CacheResult<Self> {
		let config = Config::from_url(redis_url);
		let pool = config
			.create_pool(Some(Runtime::Tokio1))
			.map_err(|e| CacheError::Configuration(format!("Failed to create pool: {}", e)))?;

		// Test connection
		let mut conn = pool
			.get()
			.await
			.map_err(|e| CacheError::Connection(format!("Failed to connect: {}", e)))?;

		redis::cmd("PING")
			.query_async::<String>(&mut conn)
			.await
			.map_err(|e| CacheError::Connection(format!("Connection test failed: {}", e)))?;

		Ok(Self { pool })
	}

	/// Create a new Redis cache with custom pool configuration
	///
	/// # Arguments
	///
	/// * `redis_url` - Redis connection URL
	/// * `max_size` - Maximum number of connections in the pool
	///
	/// # Examples
	///
	/// ```no_run
	/// # use reinhardt_backends::cache::redis::RedisCache;
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let cache = RedisCache::with_pool_size("redis://localhost:6379", 20).await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn with_pool_size(redis_url: &str, max_size: usize) -> CacheResult<Self> {
		let mut config = Config::from_url(redis_url);
		config.connection = None;
		let mut pool_config = deadpool_redis::PoolConfig::new(max_size);
		pool_config.timeouts = deadpool_redis::Timeouts::default();
		config.pool = Some(pool_config);

		let pool = config
			.create_pool(Some(Runtime::Tokio1))
			.map_err(|e| CacheError::Configuration(format!("Failed to create pool: {}", e)))?;

		// Test connection
		let mut conn = pool
			.get()
			.await
			.map_err(|e| CacheError::Connection(format!("Failed to connect: {}", e)))?;

		redis::cmd("PING")
			.query_async::<String>(&mut conn)
			.await
			.map_err(|e| CacheError::Connection(format!("Connection test failed: {}", e)))?;

		Ok(Self { pool })
	}

	/// Get a connection from the pool
	async fn get_connection(&self) -> CacheResult<deadpool_redis::Connection> {
		self.pool
			.get()
			.await
			.map_err(|e| CacheError::Connection(format!("Failed to get connection: {}", e)))
	}

	/// Convert Redis error to CacheError
	fn convert_error(e: RedisError) -> CacheError {
		match e.kind() {
			redis::ErrorKind::IoError => CacheError::Connection(format!("Connection error: {}", e)),
			redis::ErrorKind::TypeError => CacheError::Serialization(format!("Type error: {}", e)),
			_ => CacheError::Internal(format!("Redis error: {}", e)),
		}
	}
}

#[async_trait]
impl CacheBackend for RedisCache {
	async fn get(&self, key: &str) -> CacheResult<Option<Vec<u8>>> {
		let mut conn = self.get_connection().await?;

		let result: Option<Vec<u8>> = conn.get(key).await.map_err(Self::convert_error)?;

		Ok(result)
	}

	async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> CacheResult<()> {
		let mut conn = self.get_connection().await?;

		if let Some(ttl) = ttl {
			let seconds = ttl.as_secs();
			let _: () = conn
				.set_ex(key, value, seconds)
				.await
				.map_err(Self::convert_error)?;
		} else {
			let _: () = conn.set(key, value).await.map_err(Self::convert_error)?;
		}

		Ok(())
	}

	async fn delete(&self, key: &str) -> CacheResult<bool> {
		let mut conn = self.get_connection().await?;

		let deleted: i32 = conn.del(key).await.map_err(Self::convert_error)?;

		Ok(deleted > 0)
	}

	async fn exists(&self, key: &str) -> CacheResult<bool> {
		let mut conn = self.get_connection().await?;

		let exists: bool = conn.exists(key).await.map_err(Self::convert_error)?;

		Ok(exists)
	}

	async fn clear(&self) -> CacheResult<()> {
		let mut conn = self.get_connection().await?;

		let _: () = redis::cmd("FLUSHDB")
			.query_async(&mut conn)
			.await
			.map_err(Self::convert_error)?;

		Ok(())
	}

	async fn get_many(&self, keys: &[String]) -> CacheResult<Vec<Option<Vec<u8>>>> {
		if keys.is_empty() {
			return Ok(Vec::new());
		}

		let mut conn = self.get_connection().await?;

		// Use pipeline for batch get
		let mut pipe = redis::pipe();
		for key in keys {
			pipe.get(key);
		}

		let results: Vec<Option<Vec<u8>>> = pipe
			.query_async(&mut conn)
			.await
			.map_err(Self::convert_error)?;

		Ok(results)
	}

	async fn set_many(
		&self,
		items: &[(String, Vec<u8>)],
		ttl: Option<Duration>,
	) -> CacheResult<()> {
		if items.is_empty() {
			return Ok(());
		}

		let mut conn = self.get_connection().await?;

		// Use pipeline for batch set
		let mut pipe = redis::pipe();

		if let Some(ttl) = ttl {
			let seconds = ttl.as_secs();
			for (key, value) in items {
				pipe.set_ex(key, value, seconds);
			}
		} else {
			for (key, value) in items {
				pipe.set(key, value);
			}
		}

		let _: () = pipe
			.query_async(&mut conn)
			.await
			.map_err(Self::convert_error)?;

		Ok(())
	}

	async fn delete_many(&self, keys: &[String]) -> CacheResult<usize> {
		if keys.is_empty() {
			return Ok(0);
		}

		let mut conn = self.get_connection().await?;

		let deleted: usize = conn.del(keys).await.map_err(Self::convert_error)?;

		Ok(deleted)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn get_redis_url() -> String {
		std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
	}

	async fn create_test_cache() -> CacheResult<RedisCache> {
		RedisCache::new(&get_redis_url()).await
	}

	#[tokio::test]
	#[ignore = "Requires Redis server"]
	async fn test_redis_cache_set_get() {
		let cache = create_test_cache().await.unwrap();

		cache
			.set("test_key", b"test_value", Some(Duration::from_secs(60)))
			.await
			.unwrap();

		let value = cache.get("test_key").await.unwrap();
		assert_eq!(value, Some(b"test_value".to_vec()));

		cache.delete("test_key").await.unwrap();
	}

	#[tokio::test]
	#[ignore = "Requires Redis server"]
	async fn test_redis_cache_delete() {
		let cache = create_test_cache().await.unwrap();

		cache.set("delete_key", b"value", None).await.unwrap();

		let deleted = cache.delete("delete_key").await.unwrap();
		assert!(deleted);

		let value = cache.get("delete_key").await.unwrap();
		assert_eq!(value, None);
	}

	#[tokio::test]
	#[ignore = "Requires Redis server"]
	async fn test_redis_cache_exists() {
		let cache = create_test_cache().await.unwrap();

		cache.set("exists_key", b"value", None).await.unwrap();

		let exists = cache.exists("exists_key").await.unwrap();
		assert!(exists);

		cache.delete("exists_key").await.unwrap();

		let exists = cache.exists("exists_key").await.unwrap();
		assert!(!exists);
	}

	#[tokio::test]
	#[ignore = "Requires Redis server"]
	async fn test_redis_cache_ttl() {
		let cache = create_test_cache().await.unwrap();

		cache
			.set("ttl_key", b"value", Some(Duration::from_secs(1)))
			.await
			.unwrap();

		let exists = cache.exists("ttl_key").await.unwrap();
		assert!(exists);

		tokio::time::sleep(Duration::from_secs(2)).await;

		let exists = cache.exists("ttl_key").await.unwrap();
		assert!(!exists);
	}

	#[tokio::test]
	#[ignore = "Requires Redis server"]
	async fn test_redis_cache_batch_operations() {
		let cache = create_test_cache().await.unwrap();

		let items = vec![
			("batch_key1".to_string(), b"value1".to_vec()),
			("batch_key2".to_string(), b"value2".to_vec()),
			("batch_key3".to_string(), b"value3".to_vec()),
		];

		cache
			.set_many(&items, Some(Duration::from_secs(60)))
			.await
			.unwrap();

		let keys = vec![
			"batch_key1".to_string(),
			"batch_key2".to_string(),
			"batch_key3".to_string(),
		];

		let values = cache.get_many(&keys).await.unwrap();
		assert_eq!(values.len(), 3);
		assert_eq!(values[0], Some(b"value1".to_vec()));
		assert_eq!(values[1], Some(b"value2".to_vec()));
		assert_eq!(values[2], Some(b"value3".to_vec()));

		let deleted = cache.delete_many(&keys).await.unwrap();
		assert_eq!(deleted, 3);
	}

	#[tokio::test]
	#[ignore = "Requires Redis server"]
	async fn test_redis_cache_custom_pool_size() {
		let cache = RedisCache::with_pool_size(&get_redis_url(), 5)
			.await
			.unwrap();

		cache.set("pool_test", b"value", None).await.unwrap();

		let value = cache.get("pool_test").await.unwrap();
		assert_eq!(value, Some(b"value".to_vec()));

		cache.delete("pool_test").await.unwrap();
	}
}
