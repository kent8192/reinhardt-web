//! In-memory backend implementation with automatic expiration
//!
//! This module provides a thread-safe, in-memory storage backend with TTL support.
//! Expired entries are automatically cleaned up during access.

use crate::{Backend, BackendError, BackendResult};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Entry stored in the memory backend
#[derive(Clone)]
struct Entry {
    /// Serialized value
    value: Vec<u8>,
    /// Expiration time (if any)
    expires_at: Option<Instant>,
}

impl Entry {
    /// Create a new entry with optional TTL
    fn new(value: Vec<u8>, ttl: Option<Duration>) -> Self {
        let expires_at = ttl.map(|duration| Instant::now() + duration);
        Self { value, expires_at }
    }

    /// Check if the entry has expired
    fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Instant::now() > expires_at
        } else {
            false
        }
    }
}

/// In-memory backend with automatic expiration
///
/// This backend stores data in memory using a concurrent hashmap (`DashMap`),
/// providing high-performance access with automatic cleanup of expired entries.
///
/// # Thread Safety
///
/// `MemoryBackend` is fully thread-safe and can be shared across multiple threads
/// using `Arc<MemoryBackend>` or cloned directly.
///
/// # Examples
///
/// ```
/// use reinhardt_backends::{Backend, MemoryBackend};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() {
///     let backend = MemoryBackend::new();
///
///     // Store with TTL
///     backend.set("session:abc", vec![1, 2, 3], Some(Duration::from_secs(3600))).await.unwrap();
///
///     // Retrieve
///     let data: Option<Vec<u8>> = backend.get("session:abc").await.unwrap();
///     assert_eq!(data, Some(vec![1, 2, 3]));
/// }
/// ```
#[derive(Clone)]
pub struct MemoryBackend {
    store: Arc<DashMap<String, Entry>>,
}

impl MemoryBackend {
    /// Create a new memory backend
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_backends::MemoryBackend;
    ///
    /// let backend = MemoryBackend::new();
    /// ```
    pub fn new() -> Self {
        Self {
            store: Arc::new(DashMap::new()),
        }
    }

    /// Remove expired entries
    ///
    /// This is called automatically during operations, but can also be
    /// called manually for cleanup.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_backends::MemoryBackend;
    ///
    /// let backend = MemoryBackend::new();
    /// backend.cleanup_expired();
    /// ```
    pub fn cleanup_expired(&self) {
        self.store.retain(|_, entry| !entry.is_expired());
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Backend for MemoryBackend {
    async fn set<V: Serialize + Send + Sync>(
        &self,
        key: &str,
        value: V,
        ttl: Option<Duration>,
    ) -> BackendResult<()> {
        let serialized =
            serde_json::to_vec(&value).map_err(|e| BackendError::Serialization(e.to_string()))?;

        let entry = Entry::new(serialized, ttl);
        self.store.insert(key.to_string(), entry);

        Ok(())
    }

    async fn get<V: DeserializeOwned>(&self, key: &str) -> BackendResult<Option<V>> {
        let entry = match self.store.get(key) {
            Some(entry) => entry,
            None => return Ok(None),
        };

        // Check expiration
        if entry.is_expired() {
            drop(entry); // Release read lock
            self.store.remove(key);
            return Ok(None);
        }

        // Deserialize
        let value = serde_json::from_slice(&entry.value)
            .map_err(|e| BackendError::Deserialization(e.to_string()))?;

        Ok(Some(value))
    }

    async fn delete(&self, key: &str) -> BackendResult<bool> {
        Ok(self.store.remove(key).is_some())
    }

    async fn exists(&self, key: &str) -> BackendResult<bool> {
        if let Some(entry) = self.store.get(key) {
            if entry.is_expired() {
                drop(entry);
                self.store.remove(key);
                Ok(false)
            } else {
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }

    async fn increment(&self, key: &str, ttl: Option<Duration>) -> BackendResult<i64> {
        // Try to get existing value
        let current: Option<i64> = self.get(key).await?;
        let new_value = current.unwrap_or(0) + 1;

        // Store new value
        self.set(key, new_value, ttl).await?;

        Ok(new_value)
    }

    async fn clear(&self) -> BackendResult<()> {
        self.store.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_set_and_get() {
        let backend = MemoryBackend::new();

        backend.set("key1", "value1", None).await.unwrap();
        backend.set("key2", vec![1, 2, 3], None).await.unwrap();

        let value1: Option<String> = backend.get("key1").await.unwrap();
        assert_eq!(value1, Some("value1".to_string()));

        let value2: Option<Vec<i32>> = backend.get("key2").await.unwrap();
        assert_eq!(value2, Some(vec![1, 2, 3]));
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let backend = MemoryBackend::new();

        backend
            .set("short_lived", "value", Some(Duration::from_millis(100)))
            .await
            .unwrap();

        // Should exist immediately
        let exists = backend.exists("short_lived").await.unwrap();
        assert!(exists);

        // Wait for expiration
        sleep(Duration::from_millis(150)).await;

        // Should be gone
        let exists = backend.exists("short_lived").await.unwrap();
        assert!(!exists);

        let value: Option<String> = backend.get("short_lived").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_delete() {
        let backend = MemoryBackend::new();

        backend.set("key", "value", None).await.unwrap();

        let deleted = backend.delete("key").await.unwrap();
        assert!(deleted);

        let exists = backend.exists("key").await.unwrap();
        assert!(!exists);

        // Deleting non-existent key
        let deleted = backend.delete("nonexistent").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_increment() {
        let backend = MemoryBackend::new();

        let count1 = backend.increment("counter", None).await.unwrap();
        assert_eq!(count1, 1);

        let count2 = backend.increment("counter", None).await.unwrap();
        assert_eq!(count2, 2);

        let count3 = backend.increment("counter", None).await.unwrap();
        assert_eq!(count3, 3);
    }

    #[tokio::test]
    async fn test_increment_with_ttl() {
        let backend = MemoryBackend::new();

        backend
            .increment("counter", Some(Duration::from_millis(100)))
            .await
            .unwrap();

        // Should exist immediately
        let exists = backend.exists("counter").await.unwrap();
        assert!(exists);

        // Wait for expiration
        sleep(Duration::from_millis(150)).await;

        // Should be gone
        let exists = backend.exists("counter").await.unwrap();
        assert!(!exists);

        // New increment should start from 1
        let count = backend
            .increment("counter", Some(Duration::from_millis(100)))
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_clear() {
        let backend = MemoryBackend::new();

        backend.set("key1", "value1", None).await.unwrap();
        backend.set("key2", "value2", None).await.unwrap();
        backend.set("key3", "value3", None).await.unwrap();

        backend.clear().await.unwrap();

        let exists1 = backend.exists("key1").await.unwrap();
        let exists2 = backend.exists("key2").await.unwrap();
        let exists3 = backend.exists("key3").await.unwrap();

        assert!(!exists1);
        assert!(!exists2);
        assert!(!exists3);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let backend = MemoryBackend::new();

        backend
            .set("expired1", "value1", Some(Duration::from_millis(10)))
            .await
            .unwrap();
        backend
            .set("expired2", "value2", Some(Duration::from_millis(10)))
            .await
            .unwrap();
        backend.set("permanent", "value3", None).await.unwrap();

        // Wait for expiration
        sleep(Duration::from_millis(50)).await;

        // Manual cleanup
        backend.cleanup_expired();

        // Permanent should still exist
        let exists = backend.exists("permanent").await.unwrap();
        assert!(exists);

        // Expired should be gone
        let exists1 = backend.exists("expired1").await.unwrap();
        let exists2 = backend.exists("expired2").await.unwrap();
        assert!(!exists1);
        assert!(!exists2);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let backend = Arc::new(MemoryBackend::new());

        let mut handles = vec![];

        // Spawn 10 tasks that increment the same counter
        for _ in 0..10 {
            let backend = backend.clone();
            let handle = tokio::spawn(async move {
                backend.increment("shared_counter", None).await.unwrap();
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // Final count should be 10
        let final_count: Option<i64> = backend.get("shared_counter").await.unwrap();
        assert_eq!(final_count, Some(10));
    }
}
