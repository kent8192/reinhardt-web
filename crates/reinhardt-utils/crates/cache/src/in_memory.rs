//! In-memory cache implementation

use crate::cache_trait::Cache;
use crate::entry::CacheEntry;
use crate::statistics::{CacheEntryInfo, CacheStatistics};
use async_trait::async_trait;
use reinhardt_exception::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

/// In-memory cache backend
#[derive(Clone)]
pub struct InMemoryCache {
    store: Arc<RwLock<HashMap<String, CacheEntry>>>,
    default_ttl: Option<Duration>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    cleanup_interval: Option<Duration>,
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
            cleanup_interval: None,
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

    /// Start automatic cleanup of expired entries
    ///
    /// Spawns a background task that periodically removes expired entries
    /// from the cache. The task runs at the specified interval.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_cache::InMemoryCache;
    /// use std::time::Duration;
    ///
    /// # async fn example() {
    /// let cache = InMemoryCache::new();
    ///
    /// // Start cleanup every 60 seconds
    /// cache.start_auto_cleanup(Duration::from_secs(60));
    ///
    /// // Cache will now automatically clean up expired entries
    /// # }
    /// ```
    pub fn start_auto_cleanup(&self, interval: Duration) {
        let cache = self.clone();
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                cache.cleanup_expired().await;
            }
        });
    }

    /// Set cleanup interval and start automatic cleanup
    ///
    /// This is a builder method that sets the cleanup interval
    /// and immediately starts the background cleanup task.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_cache::InMemoryCache;
    /// use std::time::Duration;
    ///
    /// # async fn example() {
    /// let cache = InMemoryCache::new()
    ///     .with_auto_cleanup(Duration::from_secs(60));
    ///
    /// // Cache will automatically clean up expired entries every 60 seconds
    /// # }
    /// ```
    pub fn with_auto_cleanup(mut self, interval: Duration) -> Self {
        self.cleanup_interval = Some(interval);
        self.start_auto_cleanup(interval);
        self
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
        cache.set("key2", &"a longer value", None).await.unwrap();

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
        cache.set("large", &"x".repeat(1000), None).await.unwrap();

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

    #[tokio::test]
    async fn test_start_auto_cleanup() {
        let cache = InMemoryCache::new();

        // Set some values with short TTL
        cache
            .set("key1", &"value1", Some(Duration::from_millis(50)))
            .await
            .unwrap();
        cache
            .set("key2", &"value2", Some(Duration::from_millis(50)))
            .await
            .unwrap();

        // Start auto cleanup with short interval
        cache.start_auto_cleanup(Duration::from_millis(30));

        // Keys should exist initially
        assert!(cache.has_key("key1").await.unwrap());
        assert!(cache.has_key("key2").await.unwrap());

        // Wait for keys to expire and cleanup to run
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Keys should be cleaned up automatically
        assert!(!cache.has_key("key1").await.unwrap());
        assert!(!cache.has_key("key2").await.unwrap());
    }

    #[tokio::test]
    async fn test_with_auto_cleanup() {
        let cache = InMemoryCache::new().with_auto_cleanup(Duration::from_millis(30));

        // Set values with short TTL
        cache
            .set("key1", &"value1", Some(Duration::from_millis(50)))
            .await
            .unwrap();
        cache
            .set("key2", &"value2", Some(Duration::from_millis(50)))
            .await
            .unwrap();

        // Keys should exist initially
        assert!(cache.has_key("key1").await.unwrap());
        assert!(cache.has_key("key2").await.unwrap());

        // Wait for expiration and cleanup
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Keys should be cleaned up automatically
        assert!(!cache.has_key("key1").await.unwrap());
        assert!(!cache.has_key("key2").await.unwrap());
    }

    #[tokio::test]
    async fn test_auto_cleanup_preserves_non_expired() {
        let cache = InMemoryCache::new();

        // Start auto cleanup
        cache.start_auto_cleanup(Duration::from_millis(30));

        // Set one key with short TTL and one without
        cache
            .set("short_lived", &"value1", Some(Duration::from_millis(50)))
            .await
            .unwrap();
        cache.set("long_lived", &"value2", None).await.unwrap();

        // Both should exist initially
        assert!(cache.has_key("short_lived").await.unwrap());
        assert!(cache.has_key("long_lived").await.unwrap());

        // Wait for first key to expire and cleanup to run
        tokio::time::sleep(Duration::from_millis(100)).await;

        // short_lived should be gone, long_lived should remain
        assert!(!cache.has_key("short_lived").await.unwrap());
        assert!(cache.has_key("long_lived").await.unwrap());
    }
}
