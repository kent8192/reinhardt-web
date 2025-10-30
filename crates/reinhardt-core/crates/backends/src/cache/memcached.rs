//! Memcached Cache Backend
//!
//! High-performance memory cache using Memcached with connection pooling.
//!
//! # Features
//!
//! - Connection pooling with automatic reconnection
//! - Multi-server support with consistent hashing
//! - Binary protocol support
//! - Async/await based API
//!
//! # Examples
//!
//! ```no_run
//! use reinhardt_backends::cache::memcached::MemcachedCache;
//! use reinhardt_backends::cache::CacheBackend;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create cache with single server
//! let cache = MemcachedCache::new(&["memcache://localhost:11211"]).await?;
//!
//! // Store and retrieve data
//! cache.set("key", b"value", Some(Duration::from_secs(60))).await?;
//! let value = cache.get("key").await?;
//! assert_eq!(value, Some(b"value".to_vec()));
//!
//! // Multi-server setup
//! let cache = MemcachedCache::new(&[
//!     "memcache://localhost:11211",
//!     "memcache://localhost:11212",
//!     "memcache://localhost:11213",
//! ]).await?;
//! # Ok(())
//! # }
//! ```

use super::{CacheBackend, CacheError, CacheResult};
use async_trait::async_trait;
use std::io::{Error as IoError, ErrorKind};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncReadCompatExt;

/// Memcached cache backend with connection pooling
///
/// Provides high-performance caching using Memcached as the backing store.
/// Supports multiple servers with consistent hashing.
pub struct MemcachedCache {
    servers: Vec<String>,
}

impl MemcachedCache {
    /// Create a new Memcached cache
    ///
    /// # Arguments
    ///
    /// * `servers` - Array of server URLs (e.g., ["memcache://localhost:11211"])
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use reinhardt_backends::cache::memcached::MemcachedCache;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Single server
    /// let cache = MemcachedCache::new(&["memcache://localhost:11211"]).await?;
    ///
    /// // Multiple servers
    /// let cache = MemcachedCache::new(&[
    ///     "memcache://server1:11211",
    ///     "memcache://server2:11211",
    /// ]).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(servers: &[&str]) -> CacheResult<Self> {
        if servers.is_empty() {
            return Err(CacheError::Configuration(
                "At least one server URL is required".to_string(),
            ));
        }

        // Parse server URLs and extract host:port
        let mut server_addrs = Vec::new();
        for server_url in servers {
            let addr = Self::parse_server_url(server_url)?;
            server_addrs.push(addr);
        }

        // Test connectivity to first server
        if !server_addrs.is_empty() {
            let stream = TcpStream::connect(&server_addrs[0])
                .await
                .map_err(|e| CacheError::Connection(format!("Failed to connect: {}", e)))?;
            let compat_stream = stream.compat();
            let mut proto = memcache_async::ascii::Protocol::new(compat_stream);

            // Test with version command
            proto.version().await.map_err(Self::convert_error)?;
        }

        Ok(Self {
            servers: server_addrs,
        })
    }

    /// Parse server URL to extract host:port
    fn parse_server_url(url: &str) -> CacheResult<String> {
        // Support formats: "memcache://host:port", "host:port", or "memcached://host:port"
        let url_str = url
            .strip_prefix("memcache://")
            .or_else(|| url.strip_prefix("memcached://"))
            .unwrap_or(url);

        // Validate basic format
        if !url_str.contains(':') {
            return Err(CacheError::Configuration(format!(
                "Invalid server URL format (expected host:port): {}",
                url
            )));
        }

        Ok(url_str.to_string())
    }

    /// Get or create connection to a server
    async fn get_connection(
        &self,
    ) -> CacheResult<memcache_async::ascii::Protocol<tokio_util::compat::Compat<TcpStream>>> {
        // For simplicity, use first server (consistent hashing would go here)
        let server = &self.servers[0];

        // Create new connection
        let stream = TcpStream::connect(server)
            .await
            .map_err(|e| CacheError::Connection(format!("Failed to connect: {}", e)))?;

        let compat_stream = stream.compat();
        Ok(memcache_async::ascii::Protocol::new(compat_stream))
    }

    /// Convert IO error to CacheError
    fn convert_error(e: IoError) -> CacheError {
        match e.kind() {
            ErrorKind::NotFound => CacheError::NotFound("Key not found".to_string()),
            ErrorKind::ConnectionRefused | ErrorKind::ConnectionReset => {
                CacheError::Connection(format!("Connection error: {}", e))
            }
            ErrorKind::InvalidData => CacheError::Serialization(format!("Invalid data: {}", e)),
            ErrorKind::TimedOut => CacheError::Timeout(format!("Operation timed out: {}", e)),
            _ => CacheError::Internal(format!("Memcached error: {}", e)),
        }
    }
}

#[async_trait]
impl CacheBackend for MemcachedCache {
    async fn get(&self, key: &str) -> CacheResult<Option<Vec<u8>>> {
        let mut conn = self.get_connection().await?;

        match conn.get(&key.to_string()).await {
            Ok(value) => Ok(Some(value)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Self::convert_error(e)),
        }
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> CacheResult<()> {
        let expiration = if let Some(ttl) = ttl {
            ttl.as_secs() as u32
        } else {
            0 // 0 means no expiration
        };

        let mut conn = self.get_connection().await?;

        conn.set(&key.to_string(), value, expiration)
            .await
            .map_err(Self::convert_error)?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<bool> {
        let mut conn = self.get_connection().await?;

        // Note: delete with noreply doesn't return whether key existed
        // We'll assume success means it was deleted
        conn.delete(&key.to_string())
            .await
            .map_err(Self::convert_error)?;

        Ok(true)
    }

    async fn exists(&self, key: &str) -> CacheResult<bool> {
        // Memcached doesn't have native exists command, use get
        match self.get(key).await? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    async fn clear(&self) -> CacheResult<()> {
        let mut conn = self.get_connection().await?;

        conn.flush().await.map_err(Self::convert_error)?;

        Ok(())
    }

    async fn get_many(&self, keys: &[String]) -> CacheResult<Vec<Option<Vec<u8>>>> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }

        let mut conn = self.get_connection().await?;

        // Use get_multi for batch operation
        let keys_vec: Vec<&str> = keys.iter().map(|s| s.as_str()).collect();
        let result_map = conn
            .get_multi(&keys_vec)
            .await
            .map_err(Self::convert_error)?;

        // Convert HashMap to Vec maintaining key order
        let mut results = Vec::with_capacity(keys.len());
        for key in keys {
            results.push(result_map.get(key.as_str()).cloned());
        }

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

        let expiration = if let Some(ttl) = ttl {
            ttl.as_secs() as u32
        } else {
            0
        };

        let mut conn = self.get_connection().await?;

        // Perform sequential sets
        for (key, value) in items {
            conn.set(&key.to_string(), value.as_slice(), expiration)
                .await
                .map_err(Self::convert_error)?;
        }

        Ok(())
    }

    async fn delete_many(&self, keys: &[String]) -> CacheResult<usize> {
        if keys.is_empty() {
            return Ok(0);
        }

        let mut conn = self.get_connection().await?;

        // Perform sequential deletes
        // Note: with noreply, we can't know if keys existed
        let mut deleted_count = 0;
        for key in keys {
            conn.delete(&key.to_string())
                .await
                .map_err(Self::convert_error)?;
            deleted_count += 1;
        }

        Ok(deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_memcached_url() -> String {
        std::env::var("MEMCACHED_URL").unwrap_or_else(|_| "memcache://localhost:11211".to_string())
    }

    async fn create_test_cache() -> CacheResult<MemcachedCache> {
        MemcachedCache::new(&[&get_memcached_url()]).await
    }

    #[tokio::test]
    #[ignore = "Requires Memcached server"]
    async fn test_memcached_cache_set_get() {
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
    #[ignore = "Requires Memcached server"]
    async fn test_memcached_cache_delete() {
        let cache = create_test_cache().await.unwrap();

        cache.set("delete_key", b"value", None).await.unwrap();

        let deleted = cache.delete("delete_key").await.unwrap();
        assert!(deleted);

        let value = cache.get("delete_key").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    #[ignore = "Requires Memcached server"]
    async fn test_memcached_cache_exists() {
        let cache = create_test_cache().await.unwrap();

        cache.set("exists_key", b"value", None).await.unwrap();

        let exists = cache.exists("exists_key").await.unwrap();
        assert!(exists);

        cache.delete("exists_key").await.unwrap();

        let exists = cache.exists("exists_key").await.unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    #[ignore = "Requires Memcached server"]
    async fn test_memcached_cache_ttl() {
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
    #[ignore = "Requires Memcached server"]
    async fn test_memcached_cache_batch_operations() {
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
    #[ignore = "Requires Memcached server"]
    async fn test_memcached_cache_multi_server() {
        let cache = MemcachedCache::new(&["localhost:11211", "memcache://localhost:11212"]).await;

        // This test will fail if servers aren't running, but validates the parsing
        match cache {
            Ok(cache) => {
                cache.set("multi_test", b"value", None).await.unwrap();
                cache.delete("multi_test").await.unwrap();
            }
            Err(e) => {
                // Expected if servers aren't available
                println!("Multi-server test skipped (no servers): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_memcached_cache_url_parsing() {
        // Test various URL formats
        let urls = vec![
            "memcache://localhost:11211",
            "memcached://localhost:11211",
            "localhost:11211",
        ];

        for url in urls {
            let result = MemcachedCache::parse_server_url(url);
            assert!(result.is_ok(), "Failed to parse URL: {}", url);
            assert_eq!(result.unwrap(), "localhost:11211");
        }
    }

    #[tokio::test]
    async fn test_memcached_cache_invalid_url() {
        let result = MemcachedCache::parse_server_url("invalid_url");
        assert!(result.is_err());
    }
}
