//! Redis Cluster cache backend
//!
//! Provides a Redis Cluster-backed cache implementation for distributed caching
//! with automatic sharding and failover support.
//!
//! # Features
//!
//! - **Automatic sharding**: Data is automatically distributed across cluster nodes
//! - **Failover support**: Automatically handles node failures
//! - **Read replicas**: Supports reading from replica nodes for better performance
//! - **Connection pooling**: Efficient connection management
//!
//! # Examples
//!
//! ```no_run
//! use reinhardt_cache::{Cache, RedisClusterCache};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to Redis Cluster
//! let cache = RedisClusterCache::new(vec![
//!     "redis://node1:6379",
//!     "redis://node2:6379",
//!     "redis://node3:6379",
//! ]).await?;
//!
//! // Set a value with TTL
//! cache.set("user:123", &"John Doe", Some(Duration::from_secs(300))).await?;
//!
//! // Get a value
//! let name: Option<String> = cache.get("user:123").await?;
//! assert_eq!(name, Some("John Doe".to_string()));
//!
//! // Delete a value
//! cache.delete("user:123").await?;
//! # Ok(())
//! # }
//! ```

#![cfg(feature = "redis-cluster")]

use crate::Cache;
use async_trait::async_trait;
use redis::cluster::{ClusterClient, ClusterClientBuilder};
use redis::AsyncCommands;
use reinhardt_exception::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Redis Cluster cache backend with automatic sharding
///
/// Stores cached values in Redis Cluster for distributed caching
/// with automatic data sharding and failover support.
#[derive(Clone)]
pub struct RedisClusterCache {
    client: ClusterClient,
    default_ttl: Option<Duration>,
    key_prefix: String,
}

impl RedisClusterCache {
    /// Create a new Redis Cluster cache with the given node URLs
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_cache::RedisClusterCache;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let cache = RedisClusterCache::new(vec![
    ///     "redis://node1:6379",
    ///     "redis://node2:6379",
    ///     "redis://node3:6379",
    /// ]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(urls: Vec<impl Into<String>>) -> Result<Self> {
        let nodes: Vec<String> = urls.into_iter().map(|u| u.into()).collect();

        let client = ClusterClient::new(nodes)
            .map_err(|e| Error::Http(format!("Failed to create Redis Cluster client: {}", e)))?;

        Ok(Self {
            client,
            default_ttl: None,
            key_prefix: String::new(),
        })
    }

    /// Create a new Redis Cluster cache with custom configuration
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_cache::RedisClusterCache;
    /// use redis::cluster::ClusterClientBuilder;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let builder = ClusterClientBuilder::new(vec![
    ///     "redis://node1:6379",
    ///     "redis://node2:6379",
    /// ])
    /// .read_from_replicas()
    /// .retries(3);
    ///
    /// let cache = RedisClusterCache::with_builder(builder).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn with_builder(builder: ClusterClientBuilder) -> Result<Self> {
        let client = builder
            .build()
            .map_err(|e| Error::Http(format!("Failed to build Redis Cluster client: {}", e)))?;

        Ok(Self {
            client,
            default_ttl: None,
            key_prefix: String::new(),
        })
    }

    /// Set default TTL for all cache entries
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_cache::RedisClusterCache;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let cache = RedisClusterCache::new(vec!["redis://node1:6379"])
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
    /// use reinhardt_cache::RedisClusterCache;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let cache = RedisClusterCache::new(vec!["redis://node1:6379"])
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

    /// Get a connection to the cluster
    async fn get_connection(&self) -> Result<redis::cluster::ClusterConnection> {
        self.client
            .get_connection()
            .map_err(|e| Error::Http(format!("Failed to get connection to Redis Cluster: {}", e)))
    }

    /// Get an async connection to the cluster
    async fn get_async_connection(&self) -> Result<redis::cluster_async::ClusterConnection> {
        self.client.get_async_connection().await.map_err(|e| {
            Error::Http(format!(
                "Failed to get async connection to Redis Cluster: {}",
                e
            ))
        })
    }
}

#[async_trait]
impl Cache for RedisClusterCache {
    async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de> + Send,
    {
        let full_key = self.build_key(key);
        let mut conn = self.get_async_connection().await?;

        let value: Option<Vec<u8>> = conn
            .get(&full_key)
            .await
            .map_err(|e| Error::Http(format!("Failed to get value from Redis Cluster: {}", e)))?;

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
        let mut conn = self.get_async_connection().await?;

        let effective_ttl = ttl.or(self.default_ttl);

        if let Some(ttl_duration) = effective_ttl {
            let seconds = ttl_duration.as_secs();
            let _: () = conn
                .set_ex(&full_key, serialized, seconds)
                .await
                .map_err(|e| Error::Http(format!("Failed to set value in Redis Cluster: {}", e)))?;
        } else {
            let _: () = conn
                .set(&full_key, serialized)
                .await
                .map_err(|e| Error::Http(format!("Failed to set value in Redis Cluster: {}", e)))?;
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let full_key = self.build_key(key);
        let mut conn = self.get_async_connection().await?;

        let _: () = conn.del(&full_key).await.map_err(|e| {
            Error::Http(format!("Failed to delete value from Redis Cluster: {}", e))
        })?;

        Ok(())
    }

    async fn has_key(&self, key: &str) -> Result<bool> {
        let full_key = self.build_key(key);
        let mut conn = self.get_async_connection().await?;

        let exists: bool = conn.exists(&full_key).await.map_err(|e| {
            Error::Http(format!(
                "Failed to check key existence in Redis Cluster: {}",
                e
            ))
        })?;

        Ok(exists)
    }

    async fn clear(&self) -> Result<()> {
        let mut conn = self.get_async_connection().await?;

        if self.key_prefix.is_empty() {
            // Clear all keys if no prefix
            // Note: FLUSHALL in cluster mode affects all nodes
            let _: () = redis::cmd("FLUSHALL")
                .query_async(&mut conn)
                .await
                .map_err(|e| Error::Http(format!("Failed to clear Redis Cluster cache: {}", e)))?;
        } else {
            // Delete all keys with the prefix
            let pattern = format!("{}:*", self.key_prefix);
            let keys: Vec<String> = redis::cmd("KEYS")
                .arg(&pattern)
                .query_async(&mut conn)
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
        let mut conn = self.get_async_connection().await?;

        let values: Vec<Option<Vec<u8>>> = conn.get(&full_keys).await.map_err(|e| {
            Error::Http(format!(
                "Failed to get multiple values from Redis Cluster: {}",
                e
            ))
        })?;

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
        let mut conn = self.get_async_connection().await?;
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
                    .map_err(|e| {
                        Error::Http(format!("Failed to set value in Redis Cluster: {}", e))
                    })?;
            } else {
                let _: () = conn.set(&full_key, serialized).await.map_err(|e| {
                    Error::Http(format!("Failed to set value in Redis Cluster: {}", e))
                })?;
            }
        }

        Ok(())
    }

    async fn delete_many(&self, keys: &[&str]) -> Result<()> {
        let full_keys: Vec<String> = keys.iter().map(|k| self.build_key(k)).collect();
        let mut conn = self.get_async_connection().await?;

        let _: () = conn.del(full_keys).await.map_err(|e| {
            Error::Http(format!(
                "Failed to delete multiple values from Redis Cluster: {}",
                e
            ))
        })?;

        Ok(())
    }

    async fn incr(&self, key: &str, delta: i64) -> Result<i64> {
        let full_key = self.build_key(key);
        let mut conn = self.get_async_connection().await?;

        let result: i64 = conn.incr(&full_key, delta).await.map_err(|e| {
            Error::Http(format!("Failed to increment value in Redis Cluster: {}", e))
        })?;

        Ok(result)
    }

    async fn decr(&self, key: &str, delta: i64) -> Result<i64> {
        let full_key = self.build_key(key);
        let mut conn = self.get_async_connection().await?;

        let result: i64 = conn.decr(&full_key, delta).await.map_err(|e| {
            Error::Http(format!("Failed to decrement value in Redis Cluster: {}", e))
        })?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running Redis Cluster
    // They will be skipped in CI unless REDIS_CLUSTER_URLS is set

    fn get_redis_cluster_urls() -> Vec<String> {
        std::env::var("REDIS_CLUSTER_URLS")
            .unwrap_or_else(|_| {
                "redis://localhost:7000,redis://localhost:7001,redis://localhost:7002".to_string()
            })
            .split(',')
            .map(|s| s.to_string())
            .collect()
    }

    #[tokio::test]
    #[ignore] // Requires Redis Cluster to be running
    async fn test_redis_cluster_cache_creation() {
        let cache = RedisClusterCache::new(get_redis_cluster_urls())
            .await
            .unwrap()
            .with_default_ttl(Duration::from_secs(300))
            .with_key_prefix("myapp");

        assert_eq!(cache.key_prefix, "myapp");
        assert!(cache.default_ttl.is_some());
    }

    #[tokio::test]
    #[ignore] // Requires Redis Cluster to be running
    async fn test_build_key_with_prefix() {
        let cache = RedisClusterCache::new(get_redis_cluster_urls())
            .await
            .unwrap()
            .with_key_prefix("app");

        assert_eq!(cache.build_key("user:123"), "app:user:123");
    }

    #[tokio::test]
    #[ignore] // Requires Redis Cluster to be running
    async fn test_build_key_without_prefix() {
        let cache = RedisClusterCache::new(get_redis_cluster_urls())
            .await
            .unwrap();
        assert_eq!(cache.build_key("user:123"), "user:123");
    }

    #[tokio::test]
    #[ignore] // Requires Redis Cluster to be running
    async fn test_redis_cluster_cache_basic_operations() {
        let cache = RedisClusterCache::new(get_redis_cluster_urls())
            .await
            .unwrap();

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
    #[ignore] // Requires Redis Cluster to be running
    async fn test_redis_cluster_cache_ttl() {
        let cache = RedisClusterCache::new(get_redis_cluster_urls())
            .await
            .unwrap();
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
    #[ignore] // Requires Redis Cluster to be running
    async fn test_redis_cluster_cache_batch_operations() {
        let cache = RedisClusterCache::new(get_redis_cluster_urls())
            .await
            .unwrap();
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
    #[ignore] // Requires Redis Cluster to be running
    async fn test_redis_cluster_cache_atomic_operations() {
        let cache = RedisClusterCache::new(get_redis_cluster_urls())
            .await
            .unwrap();
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
}
