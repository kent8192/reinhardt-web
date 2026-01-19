//! # Reinhardt Cache
//!
//! Caching framework for Reinhardt.
//!
//! ## Features
//!
//! - **InMemoryCache**: Simple in-memory cache backend with optional layered cleanup
//!   - Naive cleanup: Traditional O(n) full scan (simple, suitable for small caches)
//!   - Layered cleanup: Redis 6.0-inspired O(1) amortized strategy (100-1000x faster for large caches)
//! - **LayeredCacheStore**: Standalone layered cache storage with optimized TTL cleanup
//! - **FileCache**: File-based persistent cache backend
//! - **RedisCache**: Redis-backed cache (requires redis-backend feature)
//! - **MemcachedCache**: Memcached-backed cache (requires memcached-backend feature)
//! - **HybridCache**: Multi-tier caching (memory + distributed)
//! - **RedisSentinelCache**: Redis Sentinel support (requires redis-sentinel feature)
//! - **Pub/Sub**: Cache invalidation via Redis channels (requires redis-backend feature)
//! - **Cache Warming**: Pre-populate cache on startup
//! - **Cache Tags**: Tag-based invalidation for related entries
//! - TTL support for automatic expiration
//! - Async-first API
//!
//! ## Examples
//!
//! ### Basic Usage (Naive Cleanup)
//!
//! ```rust
//! use reinhardt_utils::cache::{Cache, InMemoryCache};
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
//! ### Optimized Usage (Layered Cleanup for Large Caches)
//!
//! ```rust
//! use reinhardt_utils::cache::{Cache, InMemoryCache};
//! use std::time::Duration;
//!
//! # async fn example() {
//! // Use layered cleanup for better performance (100-1000x faster for large caches)
//! let cache = InMemoryCache::with_layered_cleanup();
//!
//! // Or customize sampling parameters
//! let cache = InMemoryCache::with_custom_layered_cleanup(50, 0.30);
//!
//! // Same API as naive strategy
//! cache.set("key", &"value", Some(Duration::from_secs(60))).await.unwrap();
//! let value: Option<String> = cache.get("key").await.unwrap();
//! # }
//! ```
//!
//! ### Memcached Backend
//!
//! Memcached support is available with the `memcached-backend` feature:
//!
//! ```toml
//! [dependencies]
//! reinhardt-cache = { version = "0.1", features = ["memcached-backend"] }
//! ```
//!
//! ```rust,ignore
//! use reinhardt_utils::cache::{Cache, MemcachedCache, MemcachedConfig};
//! use std::time::Duration;
//!
//! # async fn example() {
//! let config = MemcachedConfig {
//!     servers: vec!["127.0.0.1:11211".to_string()],
//!     pool_size: 10,
//!     timeout_ms: 1000,
//! };
//!
//! let cache = MemcachedCache::new(config).await.unwrap();
//! cache.set("key", b"value", Some(Duration::from_secs(3600))).await.unwrap();
//! # }
//! ```
//!

mod cache_trait;
mod entry;
mod in_memory;
mod key_builder;
mod layered;
mod statistics;

pub mod file_backend;
pub mod tags;
pub mod warming;

#[cfg(feature = "redis-backend")]
pub mod redis_backend;

#[cfg(feature = "memcached-backend")]
pub mod memcached;

pub mod hybrid;

#[cfg(feature = "redis-sentinel")]
pub mod redis_sentinel;

#[cfg(feature = "redis-backend")]
pub mod pubsub;

// Re-export exception types
pub use reinhardt_core::exception::Result;

// Re-export core items
pub use cache_trait::Cache;
pub use in_memory::{CleanupStrategy, InMemoryCache};
pub use key_builder::CacheKeyBuilder;
pub use layered::LayeredCacheStore;
pub use statistics::{CacheEntryInfo, CacheStatistics};

#[cfg(feature = "redis-backend")]
pub use redis_backend::RedisCache;

#[cfg(feature = "memcached-backend")]
pub use memcached::{MemcachedCache, MemcachedConfig};

pub use hybrid::HybridCache;

#[cfg(feature = "redis-sentinel")]
pub use redis_sentinel::{RedisSentinelCache, RedisSentinelConfig};

#[cfg(feature = "redis-backend")]
pub use pubsub::{CacheInvalidationChannel, CacheInvalidationMessage, CacheInvalidationSubscriber};

// Re-export file backend
pub use file_backend::FileCache;

// Re-export cache warming
pub use warming::{BatchWarmer, CacheWarmer, FunctionWarmer, ParallelWarmer};

// Re-export cache tags
pub use tags::{TaggedCache, TaggedCacheWrapper};
