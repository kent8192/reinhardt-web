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
//!
//! ## Planned Features
//! TODO: File-based cache - Persistent file system cache
//! TODO: Memcached backend - Memcached integration (dependency declared but not implemented)
//! TODO: Hybrid cache - Multi-tier caching (memory + distributed)
//! TODO: Per-view caching - View-level cache decorators
//! TODO: Template fragment caching - Selective template output caching
//! TODO: QuerySet caching - Automatic ORM query result caching
//! TODO: Cache warming - Pre-populate cache on startup
//! TODO: Cache tags - Tag-based invalidation for related entries
//! TODO: Write-through - Synchronous cache updates
//! TODO: Write-behind - Asynchronous cache updates
//! TODO: Cache-aside - Application-managed caching
//! TODO: Read-through - Automatic cache population on miss
//! TODO: Automatic cleanup - Background task for expired entry removal
//! TODO: Event hooks - Pre/post cache operations callbacks
//! TODO: Full Redis integration - Complete implementation of Redis operations
//! TODO: Connection pooling - Efficient connection management
//! TODO: Redis Cluster support - Distributed Redis deployments
//! TODO: Redis Sentinel support - High availability configurations
//! TODO: Pub/Sub support - Cache invalidation via Redis channels

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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// Cache entry information for inspection
#[derive(Debug, Clone)]
pub struct CacheEntryInfo {
    /// The key of the entry
    pub key: String,
    /// Size of the value in bytes
    pub size: usize,
    /// Whether the entry has an expiration time
    pub has_expiry: bool,
    /// Seconds until expiration (if applicable)
    pub ttl_seconds: Option<u64>,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStatistics {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Total number of requests
    pub total_requests: u64,
    /// Current number of entries in cache
    pub entry_count: u64,
    /// Approximate memory usage in bytes
    pub memory_usage: u64,
}

impl CacheStatistics {
    /// Calculate hit rate (0.0 to 1.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_cache::CacheStatistics;
    ///
    /// let mut stats = CacheStatistics::default();
    /// stats.hits = 75;
    /// stats.misses = 25;
    /// stats.total_requests = 100;
    ///
    /// assert_eq!(stats.hit_rate(), 0.75);
    /// ```
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.hits as f64 / self.total_requests as f64
        }
    }

    /// Calculate miss rate (0.0 to 1.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_cache::CacheStatistics;
    ///
    /// let mut stats = CacheStatistics::default();
    /// stats.hits = 75;
    /// stats.misses = 25;
    /// stats.total_requests = 100;
    ///
    /// assert_eq!(stats.miss_rate(), 0.25);
    /// ```
    pub fn miss_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.misses as f64 / self.total_requests as f64
        }
    }
}

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
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
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
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
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

    /// Get cache statistics
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_cache::{InMemoryCache, Cache};
    ///
    /// # async fn example() {
    /// let cache = InMemoryCache::new();
    ///
    /// // Set and get some values
    /// cache.set("key1", &"value1", None).await.unwrap();
    /// cache.set("key2", &"value2", None).await.unwrap();
    ///
    /// let _: Option<String> = cache.get("key1").await.unwrap(); // Hit
    /// let _: Option<String> = cache.get("key2").await.unwrap(); // Hit
    /// let _: Option<String> = cache.get("key3").await.unwrap(); // Miss
    ///
    /// let stats = cache.get_statistics().await;
    /// assert_eq!(stats.hits, 2);
    /// assert_eq!(stats.misses, 1);
    /// assert_eq!(stats.total_requests, 3);
    /// assert_eq!(stats.entry_count, 2);
    /// assert_eq!(stats.hit_rate(), 2.0 / 3.0);
    /// # }
    /// ```
    pub async fn get_statistics(&self) -> CacheStatistics {
        let store = self.store.read().await;
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let entry_count = store.len() as u64;
        let memory_usage = store
            .values()
            .map(|entry| entry.value.len() as u64)
            .sum::<u64>();

        CacheStatistics {
            hits,
            misses,
            total_requests: hits + misses,
            entry_count,
            memory_usage,
        }
    }

    /// List all keys in the cache
    ///
    /// Returns a vector of all keys currently stored in the cache,
    /// including expired entries that haven't been cleaned up yet.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_cache::{InMemoryCache, Cache};
    ///
    /// # async fn example() {
    /// let cache = InMemoryCache::new();
    ///
    /// cache.set("key1", &"value1", None).await.unwrap();
    /// cache.set("key2", &"value2", None).await.unwrap();
    /// cache.set("key3", &"value3", None).await.unwrap();
    ///
    /// let keys = cache.list_keys().await;
    /// assert_eq!(keys.len(), 3);
    /// assert!(keys.contains(&"key1".to_string()));
    /// assert!(keys.contains(&"key2".to_string()));
    /// assert!(keys.contains(&"key3".to_string()));
    /// # }
    /// ```
    pub async fn list_keys(&self) -> Vec<String> {
        let store = self.store.read().await;
        store.keys().cloned().collect()
    }

    /// Inspect a cache entry
    ///
    /// Returns detailed information about a specific cache entry,
    /// or None if the entry doesn't exist.
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
    /// // Set a value with TTL
    /// cache.set("key1", &"value1", Some(Duration::from_secs(300))).await.unwrap();
    ///
    /// // Inspect the entry
    /// let info = cache.inspect_entry("key1").await;
    /// assert!(info.is_some());
    ///
    /// let info = info.unwrap();
    /// assert_eq!(info.key, "key1");
    /// assert!(info.has_expiry);
    /// assert!(info.ttl_seconds.is_some());
    /// assert!(info.ttl_seconds.unwrap() <= 300);
    ///
    /// // Non-existent key
    /// let info = cache.inspect_entry("nonexistent").await;
    /// assert!(info.is_none());
    /// # }
    /// ```
    pub async fn inspect_entry(&self, key: &str) -> Option<CacheEntryInfo> {
        let store = self.store.read().await;
        store.get(key).map(|entry| {
            let ttl_seconds = entry.expires_at.and_then(|expires_at| {
                expires_at
                    .duration_since(SystemTime::now())
                    .ok()
                    .map(|d| d.as_secs())
            });

            CacheEntryInfo {
                key: key.to_string(),
                size: entry.value.len(),
                has_expiry: entry.expires_at.is_some(),
                ttl_seconds,
            }
        })
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
                // Entry expired, count as miss
                self.misses.fetch_add(1, Ordering::Relaxed);
                return Ok(None);
            }

            // Cache hit
            self.hits.fetch_add(1, Ordering::Relaxed);

            let value = serde_json::from_slice(&entry.value)
                .map_err(|e| Error::Serialization(e.to_string()))?;
            Ok(Some(value))
        } else {
            // Cache miss
            self.misses.fetch_add(1, Ordering::Relaxed);
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

    #[tokio::test]
    async fn test_cache_statistics_basic() {
        let cache = InMemoryCache::new();

        // Initially, stats should be zero
        let stats = cache.get_statistics().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.entry_count, 0);
        assert_eq!(stats.memory_usage, 0);

        // Set some values
        cache.set("key1", &"value1", None).await.unwrap();
        cache.set("key2", &"value2", None).await.unwrap();

        // Check entry count
        let stats = cache.get_statistics().await;
        assert_eq!(stats.entry_count, 2);
        assert!(stats.memory_usage > 0);

        // Get existing keys (hits)
        let _: Option<String> = cache.get("key1").await.unwrap();
        let _: Option<String> = cache.get("key2").await.unwrap();

        let stats = cache.get_statistics().await;
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.total_requests, 2);

        // Get non-existing key (miss)
        let _: Option<String> = cache.get("key3").await.unwrap();

        let stats = cache.get_statistics().await;
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.total_requests, 3);
    }

    #[tokio::test]
    async fn test_cache_statistics_hit_miss_rate() {
        let cache = InMemoryCache::new();

        cache.set("key1", &"value1", None).await.unwrap();
        cache.set("key2", &"value2", None).await.unwrap();

        // 2 hits
        let _: Option<String> = cache.get("key1").await.unwrap();
        let _: Option<String> = cache.get("key2").await.unwrap();

        // 1 miss
        let _: Option<String> = cache.get("key3").await.unwrap();

        let stats = cache.get_statistics().await;
        assert_eq!(stats.hit_rate(), 2.0 / 3.0);
        assert_eq!(stats.miss_rate(), 1.0 / 3.0);
    }

    #[tokio::test]
    async fn test_cache_statistics_expired_counts_as_miss() {
        let cache = InMemoryCache::new();

        // Set with short TTL
        cache
            .set("key1", &"value1", Some(Duration::from_millis(10)))
            .await
            .unwrap();

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Try to get expired key
        let _: Option<String> = cache.get("key1").await.unwrap();

        let stats = cache.get_statistics().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_statistics_memory_usage() {
        let cache = InMemoryCache::new();

        // Set some values
        cache.set("key1", &"short", None).await.unwrap();
        cache
            .set("key2", &"a longer value", None)
            .await
            .unwrap();

        let stats = cache.get_statistics().await;
        assert!(stats.memory_usage > 0);

        // Memory usage should increase with more data
        let initial_usage = stats.memory_usage;

        cache
            .set("key3", &"even longer value here", None)
            .await
            .unwrap();

        let stats = cache.get_statistics().await;
        assert!(stats.memory_usage > initial_usage);
    }

    #[tokio::test]
    async fn test_statistics_hit_miss_rate_zero_requests() {
        let stats = CacheStatistics::default();
        assert_eq!(stats.hit_rate(), 0.0);
        assert_eq!(stats.miss_rate(), 0.0);
    }

    #[tokio::test]
    async fn test_list_keys() {
        let cache = InMemoryCache::new();

        // Initially empty
        let keys = cache.list_keys().await;
        assert_eq!(keys.len(), 0);

        // Add some keys
        cache.set("key1", &"value1", None).await.unwrap();
        cache.set("key2", &"value2", None).await.unwrap();
        cache.set("key3", &"value3", None).await.unwrap();

        let keys = cache.list_keys().await;
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
        assert!(keys.contains(&"key3".to_string()));

        // Delete a key
        cache.delete("key2").await.unwrap();

        let keys = cache.list_keys().await;
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(!keys.contains(&"key2".to_string()));
        assert!(keys.contains(&"key3".to_string()));
    }

    #[tokio::test]
    async fn test_list_keys_includes_expired() {
        let cache = InMemoryCache::new();

        // Set a key with short TTL
        cache
            .set("expired_key", &"value", Some(Duration::from_millis(10)))
            .await
            .unwrap();

        // Set a key without TTL
        cache.set("valid_key", &"value", None).await.unwrap();

        // Wait for first key to expire
        tokio::time::sleep(Duration::from_millis(20)).await;

        // list_keys should include expired keys (until cleanup)
        let keys = cache.list_keys().await;
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"expired_key".to_string()));
        assert!(keys.contains(&"valid_key".to_string()));

        // After cleanup, expired key should be gone
        cache.cleanup_expired().await;
        let keys = cache.list_keys().await;
        assert_eq!(keys.len(), 1);
        assert!(!keys.contains(&"expired_key".to_string()));
        assert!(keys.contains(&"valid_key".to_string()));
    }

    #[tokio::test]
    async fn test_inspect_entry_basic() {
        let cache = InMemoryCache::new();

        // Non-existent key
        let info = cache.inspect_entry("nonexistent").await;
        assert!(info.is_none());

        // Add a key without expiry
        cache.set("key1", &"value1", None).await.unwrap();

        let info = cache.inspect_entry("key1").await;
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.key, "key1");
        assert!(!info.has_expiry);
        assert!(info.ttl_seconds.is_none());
        assert!(info.size > 0);
    }

    #[tokio::test]
    async fn test_inspect_entry_with_ttl() {
        let cache = InMemoryCache::new();

        // Set a value with TTL
        cache
            .set("key1", &"value1", Some(Duration::from_secs(300)))
            .await
            .unwrap();

        let info = cache.inspect_entry("key1").await;
        assert!(info.is_some());

        let info = info.unwrap();
        assert_eq!(info.key, "key1");
        assert!(info.has_expiry);
        assert!(info.ttl_seconds.is_some());

        // TTL should be <= 300 seconds
        let ttl = info.ttl_seconds.unwrap();
        assert!(ttl <= 300);
        assert!(ttl > 0);
    }

    #[tokio::test]
    async fn test_inspect_entry_size() {
        let cache = InMemoryCache::new();

        // Set values of different sizes
        cache.set("small", &"x", None).await.unwrap();
        cache
            .set("large", &"x".repeat(1000), None)
            .await
            .unwrap();

        let small_info = cache.inspect_entry("small").await.unwrap();
        let large_info = cache.inspect_entry("large").await.unwrap();

        assert!(large_info.size > small_info.size);
    }

    #[tokio::test]
    async fn test_inspect_entry_expired() {
        let cache = InMemoryCache::new();

        // Set with short TTL
        cache
            .set("key1", &"value1", Some(Duration::from_millis(10)))
            .await
            .unwrap();

        // Inspect before expiration
        let info = cache.inspect_entry("key1").await;
        assert!(info.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(20)).await;

        // Entry still exists (not cleaned up)
        let info = cache.inspect_entry("key1").await;
        assert!(info.is_some());

        let info = info.unwrap();
        assert!(info.has_expiry);
        // TTL should be None because it's expired
        assert!(info.ttl_seconds.is_none());

        // After cleanup, entry should be gone
        cache.cleanup_expired().await;
        let info = cache.inspect_entry("key1").await;
        assert!(info.is_none());
    }
}
