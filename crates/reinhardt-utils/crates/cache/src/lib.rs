//! # Reinhardt Cache
//!
//! Caching framework for Reinhardt.
//!
//! ## Features
//!
//! - **InMemoryCache**: Simple in-memory cache backend
//! - **RedisCache**: Redis-backed cache (requires redis feature)
//! - TTL support for automatic expiration
//! - Async-first API
//!
//! ## Example
//!
//! ```rust
//! use reinhardt_cache::{Cache, InMemoryCache};
//!
//! # async fn example() {
//! let cache = InMemoryCache::new();
//!
//! // Set a value
//! cache.set("key", &"value".to_string(), None).await.unwrap();
//!
//! // Get a value
//! let value: Option<String> = cache.get("key").await.unwrap();
//! assert_eq!(value, Some("value".to_string()));
//!
//! // Delete a value
//! cache.delete("key").await.unwrap();
//! # }
//! ```

pub mod di_support;
pub mod middleware;

#[cfg(feature = "redis-backend")]
pub mod redis_backend;

// Re-export middleware and Redis backend
pub use middleware::{CacheMiddleware, CacheMiddlewareConfig};

#[cfg(feature = "redis-backend")]
pub use redis_backend::RedisCache;

// Re-export DI support
pub use di_support::CacheService;
#[cfg(feature = "redis-backend")]
pub use di_support::RedisConfig;

use async_trait::async_trait;
use reinhardt_exception::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// Cache entry with expiration
#[derive(Debug, Clone)]
struct CacheEntry {
    value: Vec<u8>,
    expires_at: Option<SystemTime>,
}

impl CacheEntry {
    fn new(value: Vec<u8>, ttl: Option<Duration>) -> Self {
        let expires_at = ttl.map(|d| SystemTime::now() + d);
        Self { value, expires_at }
    }

    fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            SystemTime::now() > expires_at
        } else {
            false
        }
    }
}

/// Base cache trait
#[async_trait]
pub trait Cache: Send + Sync {
    /// Get a value from the cache
    async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Send;

    /// Set a value in the cache with optional TTL
    async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Send + Sync;

    /// Delete a value from the cache
    async fn delete(&self, key: &str) -> Result<()>;

    /// Check if a key exists in the cache
    async fn has_key(&self, key: &str) -> Result<bool>;

    /// Clear all values from the cache
    async fn clear(&self) -> Result<()>;

    /// Get multiple values at once
    async fn get_many<T>(&self, keys: &[&str]) -> Result<HashMap<String, T>>
    where
        T: for<'de> Deserialize<'de> + Send,
    {
        let mut results = HashMap::new();
        for key in keys {
            if let Some(value) = self.get::<T>(key).await? {
                results.insert(key.to_string(), value);
            }
        }
        Ok(results)
    }

    /// Set multiple values at once
    async fn set_many<T>(&self, values: HashMap<String, T>, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        for (key, value) in values.iter() {
            self.set(key, value, ttl).await?;
        }
        Ok(())
    }

    /// Delete multiple keys at once
    async fn delete_many(&self, keys: &[&str]) -> Result<()> {
        for key in keys {
            self.delete(key).await?;
        }
        Ok(())
    }

    /// Increment a numeric value
    async fn incr(&self, key: &str, delta: i64) -> Result<i64> {
        let current: i64 = self.get(key).await?.unwrap_or(0);
        let new_value = current + delta;
        self.set(key, &new_value, None).await?;
        Ok(new_value)
    }

    /// Decrement a numeric value
    async fn decr(&self, key: &str, delta: i64) -> Result<i64> {
        self.incr(key, -delta).await
    }
}

/// In-memory cache backend
#[derive(Clone)]
pub struct InMemoryCache {
    store: Arc<RwLock<HashMap<String, CacheEntry>>>,
    default_ttl: Option<Duration>,
}

impl InMemoryCache {
    /// Create a new in-memory cache
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_cache::InMemoryCache;
    ///
    /// let cache = InMemoryCache::new();
    // Cache is ready to use with no default TTL
    /// ```
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: None,
        }
    }
    /// Set a default TTL for all cache entries
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_cache::{InMemoryCache, Cache};
    /// use std::time::Duration;
    ///
    /// # async fn example() {
    /// let cache = InMemoryCache::new()
    ///     .with_default_ttl(Duration::from_secs(1));
    ///
    // Set a value without explicit TTL
    /// cache.set("key", &"value", None).await.unwrap();
    ///
    // Wait for default TTL to expire
    /// tokio::time::sleep(Duration::from_millis(1100)).await;
    ///
    // Value should be expired
    /// let value: Option<String> = cache.get("key").await.unwrap();
    /// assert_eq!(value, None);
    /// # }
    /// ```
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = Some(ttl);
        self
    }
    /// Clean up expired entries
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_cache::{InMemoryCache, Cache};
    /// use std::time::Duration;
    ///
    /// # async fn example() {
    /// let cache = InMemoryCache::new();
    ///
    // Set a value with short TTL
    /// cache.set("key1", &"value", Some(Duration::from_millis(10))).await.unwrap();
    ///
    // Wait for expiration
    /// tokio::time::sleep(Duration::from_millis(20)).await;
    ///
    // Clean up expired entries
    /// cache.cleanup_expired().await;
    ///
    // Verify the key is gone
    /// assert!(!cache.has_key("key1").await.unwrap());
    /// # }
    /// ```
    pub async fn cleanup_expired(&self) {
        let mut store = self.store.write().await;
        store.retain(|_, entry| !entry.is_expired());
    }
}

impl Default for InMemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cache for InMemoryCache {
    async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Send,
    {
        let store = self.store.read().await;

        if let Some(entry) = store.get(key) {
            if entry.is_expired() {
                // Entry expired, return None
                return Ok(None);
            }

            let value = serde_json::from_slice(&entry.value)
                .map_err(|e| Error::Serialization(e.to_string()))?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        let serialized =
            serde_json::to_vec(value).map_err(|e| Error::Serialization(e.to_string()))?;

        let ttl = ttl.or(self.default_ttl);
        let entry = CacheEntry::new(serialized, ttl);

        let mut store = self.store.write().await;
        store.insert(key.to_string(), entry);

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut store = self.store.write().await;
        store.remove(key);
        Ok(())
    }

    async fn has_key(&self, key: &str) -> Result<bool> {
        let store = self.store.read().await;

        if let Some(entry) = store.get(key) {
            Ok(!entry.is_expired())
        } else {
            Ok(false)
        }
    }

    async fn clear(&self) -> Result<()> {
        let mut store = self.store.write().await;
        store.clear();
        Ok(())
    }
}

/// Cache key builder for generating cache keys
#[derive(Clone)]
pub struct CacheKeyBuilder {
    prefix: String,
    version: u32,
}

impl CacheKeyBuilder {
    /// Create a new cache key builder with the given prefix
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_cache::CacheKeyBuilder;
    ///
    /// let builder = CacheKeyBuilder::new("myapp");
    /// assert_eq!(builder.build("user"), "myapp:1:user");
    /// ```
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
            version: 1,
        }
    }
    /// Set the version for cache key namespacing
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_cache::CacheKeyBuilder;
    ///
    /// let builder = CacheKeyBuilder::new("myapp").with_version(2);
    /// assert_eq!(builder.build("user"), "myapp:2:user");
    /// ```
    pub fn with_version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }
    /// Build a cache key with prefix and version
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_cache::CacheKeyBuilder;
    ///
    /// let builder = CacheKeyBuilder::new("app").with_version(3);
    /// let key = builder.build("user:123");
    /// assert_eq!(key, "app:3:user:123");
    /// ```
    pub fn build(&self, key: &str) -> String {
        format!("{}:{}:{}", self.prefix, self.version, key)
    }
    /// Build multiple cache keys at once
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_cache::CacheKeyBuilder;
    ///
    /// let builder = CacheKeyBuilder::new("app");
    /// let keys = builder.build_many(&["user", "session", "token"]);
    /// assert_eq!(keys, vec!["app:1:user", "app:1:session", "app:1:token"]);
    /// ```
    pub fn build_many(&self, keys: &[&str]) -> Vec<String> {
        keys.iter().map(|k| self.build(k)).collect()
    }
}

impl Default for CacheKeyBuilder {
    fn default() -> Self {
        Self::new("app")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_cache_basic() {
        let cache = InMemoryCache::new();

        // Set and get
        cache.set("key1", &"value1", None).await.unwrap();
        let value: Option<String> = cache.get("key1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));

        // Has key
        assert!(cache.has_key("key1").await.unwrap());
        assert!(!cache.has_key("key2").await.unwrap());

        // Delete
        cache.delete("key1").await.unwrap();
        let value: Option<String> = cache.get("key1").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_in_memory_cache_ttl() {
        let cache = InMemoryCache::new();

        // Set with short TTL
        cache
            .set("key1", &"value1", Some(Duration::from_millis(100)))
            .await
            .unwrap();

        // Should exist immediately
        let value: Option<String> = cache.get("key1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be expired
        let value: Option<String> = cache.get("key1").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_in_memory_cache_many() {
        let cache = InMemoryCache::new();

        // Set many
        let mut values = HashMap::new();
        values.insert("key1".to_string(), "value1".to_string());
        values.insert("key2".to_string(), "value2".to_string());
        cache.set_many(values, None).await.unwrap();

        // Get many
        let results: HashMap<String, String> =
            cache.get_many(&["key1", "key2", "key3"]).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results.get("key1"), Some(&"value1".to_string()));
        assert_eq!(results.get("key2"), Some(&"value2".to_string()));

        // Delete many
        cache.delete_many(&["key1", "key2"]).await.unwrap();
        assert!(!cache.has_key("key1").await.unwrap());
        assert!(!cache.has_key("key2").await.unwrap());
    }

    #[tokio::test]
    async fn test_in_memory_cache_incr_decr() {
        let cache = InMemoryCache::new();

        // Increment from zero
        let value = cache.incr("counter", 5).await.unwrap();
        assert_eq!(value, 5);

        // Increment again
        let value = cache.incr("counter", 3).await.unwrap();
        assert_eq!(value, 8);

        // Decrement
        let value = cache.decr("counter", 2).await.unwrap();
        assert_eq!(value, 6);
    }

    #[tokio::test]
    async fn test_cache_key_builder() {
        let builder = CacheKeyBuilder::new("myapp").with_version(2);

        assert_eq!(builder.build("user:123"), "myapp:2:user:123");

        let keys = builder.build_many(&["key1", "key2"]);
        assert_eq!(keys, vec!["myapp:2:key1", "myapp:2:key2"]);
    }

    #[tokio::test]
    async fn test_in_memory_cache_clear() {
        let cache = InMemoryCache::new();

        cache.set("key1", &"value1", None).await.unwrap();
        cache.set("key2", &"value2", None).await.unwrap();

        cache.clear().await.unwrap();

        assert!(!cache.has_key("key1").await.unwrap());
        assert!(!cache.has_key("key2").await.unwrap());
    }

    #[tokio::test]
    async fn test_cache_cleanup_expired() {
        let cache = InMemoryCache::new();

        // Set some values with different TTLs
        cache
            .set("key1", &"value1", Some(Duration::from_millis(100)))
            .await
            .unwrap();
        cache.set("key2", &"value2", None).await.unwrap();

        // Wait for first key to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Cleanup expired entries
        cache.cleanup_expired().await;

        // key1 should be gone, key2 should remain
        assert!(!cache.has_key("key1").await.unwrap());
        assert!(cache.has_key("key2").await.unwrap());
    }
}
