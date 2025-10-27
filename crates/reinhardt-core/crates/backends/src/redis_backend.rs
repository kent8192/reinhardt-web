//! Redis backend implementation
//!
//! This module provides a distributed storage backend using Redis.
//! It supports all Redis data types and operations with automatic serialization.

use crate::{Backend, BackendError, BackendResult};
use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Redis backend for distributed storage
///
/// This backend uses Redis for storage, providing:
/// - Distributed state across multiple servers
/// - Automatic TTL management
/// - High performance with connection pooling
///
/// # Examples
///
/// ```no_run
/// use reinhardt_backends::{Backend, RedisBackend};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     let backend = RedisBackend::new("redis://localhost:6379").await.unwrap();
///
///     backend.set("user:123", "active", Some(Duration::from_secs(3600))).await.unwrap();
///
///     let value: Option<String> = backend.get("user:123").await.unwrap();
///     assert_eq!(value, Some("active".to_string()));
/// }
/// ```
#[derive(Clone)]
pub struct RedisBackend {
    connection: Arc<ConnectionManager>,
}

impl RedisBackend {
    /// Create a new Redis backend from a connection string
    ///
    /// # Arguments
    ///
    /// * `redis_url` - Redis connection URL (e.g., "redis://localhost:6379")
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_backends::RedisBackend;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let backend = RedisBackend::new("redis://localhost:6379").await.unwrap();
    /// }
    /// ```
    pub async fn new(redis_url: &str) -> BackendResult<Self> {
        let client = Client::open(redis_url).map_err(|e| {
            BackendError::Connection(format!("Failed to create Redis client: {}", e))
        })?;

        let connection = ConnectionManager::new(client)
            .await
            .map_err(|e| BackendError::Connection(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self {
            connection: Arc::new(connection),
        })
    }

    /// Create from an existing connection manager
    ///
    /// This is useful when you want to share a connection pool across multiple backends.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_backends::RedisBackend;
    /// use redis::{Client, aio::ConnectionManager};
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let client = Client::open("redis://localhost:6379").unwrap();
    ///     let manager = ConnectionManager::new(client).await.unwrap();
    ///
    ///     let backend = RedisBackend::from_connection_manager(manager);
    /// }
    /// ```
    pub fn from_connection_manager(connection: ConnectionManager) -> Self {
        Self {
            connection: Arc::new(connection),
        }
    }

    /// Get a mutable connection for executing Redis commands
    async fn get_connection(&self) -> BackendResult<ConnectionManager> {
        Ok(self.connection.as_ref().clone())
    }
}

#[async_trait]
impl Backend for RedisBackend {
    async fn set<V: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: V,
        ttl: Option<Duration>,
    ) -> BackendResult<()> {
        let serialized = serde_json::to_string(&value)
            .map_err(|e| BackendError::Serialization(e.to_string()))?;

        let mut conn = self.get_connection().await?;

        if let Some(duration) = ttl {
            // SET with EX (seconds) option
            conn.set_ex::<_, _, ()>(key, serialized, duration.as_secs() as u64)
                .await
                .map_err(|e| BackendError::Internal(format!("Redis SET error: {}", e)))?;
        } else {
            // SET without TTL
            conn.set::<_, _, ()>(key, serialized)
                .await
                .map_err(|e| BackendError::Internal(format!("Redis SET error: {}", e)))?;
        }

        Ok(())
    }

    async fn get<V: DeserializeOwned>(&self, key: &str) -> BackendResult<Option<V>> {
        let mut conn = self.get_connection().await?;

        let value: Option<String> = conn
            .get(key)
            .await
            .map_err(|e| BackendError::Internal(format!("Redis GET error: {}", e)))?;

        match value {
            Some(serialized) => {
                let deserialized = serde_json::from_str(&serialized)
                    .map_err(|e| BackendError::Deserialization(e.to_string()))?;
                Ok(Some(deserialized))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, key: &str) -> BackendResult<bool> {
        let mut conn = self.get_connection().await?;

        let deleted: i32 = conn
            .del(key)
            .await
            .map_err(|e| BackendError::Internal(format!("Redis DEL error: {}", e)))?;

        Ok(deleted > 0)
    }

    async fn exists(&self, key: &str) -> BackendResult<bool> {
        let mut conn = self.get_connection().await?;

        let exists: bool = conn
            .exists(key)
            .await
            .map_err(|e| BackendError::Internal(format!("Redis EXISTS error: {}", e)))?;

        Ok(exists)
    }

    async fn increment(&self, key: &str, ttl: Option<Duration>) -> BackendResult<i64> {
        let mut conn = self.get_connection().await?;

        // Increment the key
        let new_value: i64 = conn
            .incr(key, 1)
            .await
            .map_err(|e| BackendError::Internal(format!("Redis INCR error: {}", e)))?;

        // Set TTL if provided and this is the first increment
        if let Some(duration) = ttl {
            if new_value == 1 {
                conn.expire::<_, ()>(key, duration.as_secs() as i64)
                    .await
                    .map_err(|e| BackendError::Internal(format!("Redis EXPIRE error: {}", e)))?;
            }
        }

        Ok(new_value)
    }

    async fn clear(&self) -> BackendResult<()> {
        let mut conn = self.get_connection().await?;

        let _: () = redis::cmd("FLUSHDB")
            .query_async(&mut conn)
            .await
            .map_err(|e| BackendError::Internal(format!("Redis FLUSHDB error: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a test backend
    async fn create_test_backend() -> RedisBackend {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

        match RedisBackend::new(&redis_url).await {
            Ok(backend) => backend,
            Err(e) => {
                eprintln!("Skipping Redis tests: {}", e);
                panic!("Redis not available");
            }
        }
    }

    #[tokio::test]
    #[ignore] // Run only when Redis is available
    async fn test_set_and_get() {
        let backend = create_test_backend().await;
        backend.clear().await.unwrap();

        backend.set("test:key1", "value1", None).await.unwrap();
        backend.set("test:key2", vec![1, 2, 3], None).await.unwrap();

        let value1: Option<String> = backend.get("test:key1").await.unwrap();
        assert_eq!(value1, Some("value1".to_string()));

        let value2: Option<Vec<i32>> = backend.get("test:key2").await.unwrap();
        assert_eq!(value2, Some(vec![1, 2, 3]));
    }

    #[tokio::test]
    #[ignore] // Run only when Redis is available
    async fn test_ttl_expiration() {
        let backend = create_test_backend().await;
        backend.clear().await.unwrap();

        backend
            .set("test:short_lived", "value", Some(Duration::from_secs(1)))
            .await
            .unwrap();

        // Should exist immediately
        let exists = backend.exists("test:short_lived").await.unwrap();
        assert!(exists);

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should be gone
        let exists = backend.exists("test:short_lived").await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    #[ignore] // Run only when Redis is available
    async fn test_delete() {
        let backend = create_test_backend().await;
        backend.clear().await.unwrap();

        backend.set("test:key", "value", None).await.unwrap();

        let deleted = backend.delete("test:key").await.unwrap();
        assert!(deleted);

        let exists = backend.exists("test:key").await.unwrap();
        assert!(!exists);

        // Deleting non-existent key
        let deleted = backend.delete("test:nonexistent").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    #[ignore] // Run only when Redis is available
    async fn test_increment() {
        let backend = create_test_backend().await;
        backend.clear().await.unwrap();

        let count1 = backend.increment("test:counter", None).await.unwrap();
        assert_eq!(count1, 1);

        let count2 = backend.increment("test:counter", None).await.unwrap();
        assert_eq!(count2, 2);

        let count3 = backend.increment("test:counter", None).await.unwrap();
        assert_eq!(count3, 3);
    }

    #[tokio::test]
    #[ignore] // Run only when Redis is available
    async fn test_increment_with_ttl() {
        let backend = create_test_backend().await;
        backend.clear().await.unwrap();

        backend
            .increment("test:counter_ttl", Some(Duration::from_secs(1)))
            .await
            .unwrap();

        // Should exist immediately
        let exists = backend.exists("test:counter_ttl").await.unwrap();
        assert!(exists);

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should be gone
        let exists = backend.exists("test:counter_ttl").await.unwrap();
        assert!(!exists);

        // New increment should start from 1
        let count = backend
            .increment("test:counter_ttl", Some(Duration::from_secs(1)))
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    #[ignore] // Run only when Redis is available
    async fn test_clear() {
        let backend = create_test_backend().await;

        backend.set("test:key1", "value1", None).await.unwrap();
        backend.set("test:key2", "value2", None).await.unwrap();

        backend.clear().await.unwrap();

        let exists1 = backend.exists("test:key1").await.unwrap();
        let exists2 = backend.exists("test:key2").await.unwrap();

        assert!(!exists1);
        assert!(!exists2);
    }
}
