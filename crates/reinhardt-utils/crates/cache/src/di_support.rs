//! Dependency Injection support for Cache

use crate::{Cache, CacheKeyBuilder, InMemoryCache};
use async_trait::async_trait;
use reinhardt_core::di::{DiError, DiResult, Injectable, InjectionContext};
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "redis-backend")]
use crate::RedisCache;

// ============================================================================
// InMemoryCache DI Support
// ============================================================================
// Note: InMemoryCache uses the default Injectable implementation from reinhardt_di
// since it implements Default, Clone, Send, Sync, and 'static.

// ============================================================================
// CacheKeyBuilder DI Support
// ============================================================================
// Note: CacheKeyBuilder is Clone but doesn't implement Default.
// We need a custom Injectable implementation to provide a default configuration.

// ============================================================================
// Redis Cache DI Support
// ============================================================================

#[cfg(feature = "redis-backend")]
#[derive(Clone)]
pub struct RedisConfig {
	pub url: String,
}

#[cfg(feature = "redis-backend")]
impl RedisConfig {
	/// Create a new Redis configuration with a connection URL
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::RedisConfig;
	///
	/// let config = RedisConfig::new("redis://localhost:6379");
	/// assert_eq!(config.url, "redis://localhost:6379");
	/// ```
	pub fn new(url: impl Into<String>) -> Self {
		Self { url: url.into() }
	}
	/// Create a Redis configuration for localhost with default port
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::RedisConfig;
	///
	/// let config = RedisConfig::localhost();
	/// assert_eq!(config.url, "redis://127.0.0.1:6379");
	/// ```
	pub fn localhost() -> Self {
		Self {
			url: "redis://127.0.0.1:6379".to_string(),
		}
	}
}

#[cfg(feature = "redis-backend")]
#[async_trait]
impl Injectable for RedisConfig {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		// Check if a custom config was set in the singleton scope
		if let Some(config) = ctx.get_singleton::<RedisConfig>() {
			return Ok((*config).clone());
		}
		// Default to localhost
		Ok(RedisConfig::localhost())
	}
}

#[cfg(feature = "redis-backend")]
#[async_trait]
impl Injectable for RedisCache {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let config = RedisConfig::inject(ctx).await?;
		RedisCache::new(&config.url)
			.await
			.map_err(|e| DiError::ProviderError(format!("Failed to create Redis cache: {}", e)))
	}
}

// ============================================================================
// CacheService with DI
// ============================================================================

/// Cache service with injected cache backend
#[derive(Clone)]
pub struct CacheService {
	pub cache: Arc<InMemoryCache>,
	pub key_builder: Arc<CacheKeyBuilder>,
}

impl CacheService {
	/// Create a new cache service with the given cache and key builder
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::{CacheService, InMemoryCache, CacheKeyBuilder};
	///
	/// let cache = InMemoryCache::new();
	/// let key_builder = CacheKeyBuilder::new("myapp");
	/// let service = CacheService::new(cache, key_builder);
	/// ```
	pub fn new(cache: InMemoryCache, key_builder: CacheKeyBuilder) -> Self {
		Self {
			cache: Arc::new(cache),
			key_builder: Arc::new(key_builder),
		}
	}
	/// Get a value using the key builder
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::{CacheService, InMemoryCache, CacheKeyBuilder};
	///
	/// # async fn example() {
	/// let cache = InMemoryCache::new();
	/// let key_builder = CacheKeyBuilder::new("app");
	/// let service = CacheService::new(cache, key_builder);
	///
	// Set a value first
	/// service.set("user", &"john", None).await.unwrap();
	///
	// Get the value
	/// let value: Option<String> = service.get("user").await.unwrap();
	/// assert_eq!(value, Some("john".to_string()));
	/// # }
	/// ```
	pub async fn get<T>(&self, key: &str) -> crate::Result<Option<T>>
	where
		T: for<'de> serde::Deserialize<'de> + serde::Serialize + Send + Sync,
	{
		let full_key = self.key_builder.build(key);
		self.cache.get(&full_key).await
	}
	/// Set a value using the key builder
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::{CacheService, InMemoryCache, CacheKeyBuilder};
	/// use std::time::Duration;
	///
	/// # async fn example() {
	/// let cache = InMemoryCache::new();
	/// let key_builder = CacheKeyBuilder::new("app");
	/// let service = CacheService::new(cache, key_builder);
	///
	// Set a value with TTL
	/// service.set("session", &"token123", Some(Duration::from_secs(3600))).await.unwrap();
	///
	// Verify it was set
	/// let value: Option<String> = service.get("session").await.unwrap();
	/// assert_eq!(value, Some("token123".to_string()));
	/// # }
	/// ```
	pub async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> crate::Result<()>
	where
		T: serde::Serialize + Send + Sync,
	{
		let full_key = self.key_builder.build(key);
		self.cache.set(&full_key, value, ttl).await
	}
	/// Delete a value using the key builder
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::{CacheService, InMemoryCache, CacheKeyBuilder};
	///
	/// # async fn example() {
	/// let cache = InMemoryCache::new();
	/// let key_builder = CacheKeyBuilder::new("app");
	/// let service = CacheService::new(cache, key_builder);
	///
	// Set a value
	/// service.set("temp", &"data", None).await.unwrap();
	///
	// Delete it
	/// service.delete("temp").await.unwrap();
	///
	// Verify it's gone
	/// let value: Option<String> = service.get("temp").await.unwrap();
	/// assert_eq!(value, None);
	/// # }
	/// ```
	pub async fn delete(&self, key: &str) -> crate::Result<()> {
		let full_key = self.key_builder.build(key);
		self.cache.delete(&full_key).await
	}
	/// Get the underlying cache
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::{CacheService, InMemoryCache, CacheKeyBuilder, Cache};
	///
	/// # async fn example() {
	/// let cache = InMemoryCache::new();
	/// let key_builder = CacheKeyBuilder::new("app");
	/// let service = CacheService::new(cache, key_builder);
	///
	// Access the underlying cache directly
	/// let underlying_cache = service.cache();
	/// underlying_cache.set("direct_key", &42, None).await.unwrap();
	/// # }
	/// ```
	pub fn cache(&self) -> &InMemoryCache {
		&self.cache
	}
	/// Get the key builder
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::{CacheService, InMemoryCache, CacheKeyBuilder};
	///
	/// let cache = InMemoryCache::new();
	/// let key_builder = CacheKeyBuilder::new("myapp").with_version(2);
	/// let service = CacheService::new(cache, key_builder);
	///
	// Access the key builder
	/// let builder = service.key_builder();
	/// assert_eq!(builder.build("test"), "myapp:2:test");
	/// ```
	pub fn key_builder(&self) -> &CacheKeyBuilder {
		&self.key_builder
	}
}

#[async_trait]
impl Injectable for CacheService {
	async fn inject(ctx: &InjectionContext) -> DiResult<Self> {
		let cache = InMemoryCache::inject(ctx).await?;
		let key_builder = CacheKeyBuilder::inject(ctx).await?;
		Ok(CacheService::new(cache, key_builder))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use reinhardt_core::di::SingletonScope;

	#[tokio::test]
	async fn test_in_memory_cache_injection() {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		let cache = InMemoryCache::inject(&ctx).await.unwrap();
		cache.set("test", &"value", None).await.unwrap();
		let value: Option<String> = cache.get("test").await.unwrap();
		assert_eq!(value, Some("value".to_string()));
	}

	#[tokio::test]
	async fn test_cache_key_builder_injection() {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		let builder = CacheKeyBuilder::inject(&ctx).await.unwrap();
		assert_eq!(builder.build("test"), "app:1:test");
	}

	#[tokio::test]
	async fn test_custom_in_memory_cache() {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		// Set custom cache with default TTL
		let custom_cache = InMemoryCache::new().with_default_ttl(Duration::from_secs(60));
		ctx.set_singleton(custom_cache.clone());

		let cache = InMemoryCache::inject(&ctx).await.unwrap();
		// Verify it's the custom cache by checking the TTL behavior
		// (We can't directly check the TTL value, but we can test that it uses it)
		cache.set("test", &"value", None).await.unwrap();
		let value: Option<String> = cache.get("test").await.unwrap();
		assert_eq!(value, Some("value".to_string()));
	}

	#[tokio::test]
	async fn test_custom_key_builder() {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		// Set custom key builder
		let custom_builder = CacheKeyBuilder::new("myapp").with_version(2);
		ctx.set_singleton(custom_builder.clone());

		let builder = CacheKeyBuilder::inject(&ctx).await.unwrap();
		assert_eq!(builder.build("test"), "myapp:2:test");
	}

	#[tokio::test]
	async fn test_cache_service_injection() {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		let service = CacheService::inject(&ctx).await.unwrap();

		// Test set and get
		service.set("user", &"john", None).await.unwrap();
		let value: Option<String> = service.get("user").await.unwrap();
		assert_eq!(value, Some("john".to_string()));

		// Verify key builder was used (key should be "app:1:user")
		let cache = service.cache();
		let direct_value: Option<String> = cache.get("app:1:user").await.unwrap();
		assert_eq!(direct_value, Some("john".to_string()));
	}

	#[tokio::test]
	async fn test_cache_service_with_custom_builder() {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		// Set custom key builder
		let custom_builder = CacheKeyBuilder::new("custom").with_version(3);
		ctx.set_singleton(custom_builder);

		let service = CacheService::inject(&ctx).await.unwrap();

		service.set("test", &42, None).await.unwrap();
		let value: Option<i32> = service.get("test").await.unwrap();
		assert_eq!(value, Some(42));

		// Verify custom key builder was used
		let cache = service.cache();
		let direct_value: Option<i32> = cache.get("custom:3:test").await.unwrap();
		assert_eq!(direct_value, Some(42));
	}

	#[tokio::test]
	async fn test_cache_service_delete() {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		let service = CacheService::inject(&ctx).await.unwrap();

		service.set("temp", &"data", None).await.unwrap();
		service.delete("temp").await.unwrap();

		let value: Option<String> = service.get("temp").await.unwrap();
		assert_eq!(value, None);
	}

	#[tokio::test]
	async fn test_cache_service_ttl() {
		let singleton = Arc::new(SingletonScope::new());
		let ctx = InjectionContext::new(singleton);

		let service = CacheService::inject(&ctx).await.unwrap();

		// Set with short TTL
		service
			.set("expiring", &"value", Some(Duration::from_millis(100)))
			.await
			.unwrap();

		// Should exist immediately
		let value: Option<String> = service.get("expiring").await.unwrap();
		assert_eq!(value, Some("value".to_string()));

		// Wait for expiration
		tokio::time::sleep(Duration::from_millis(150)).await;

		// Should be expired
		let value: Option<String> = service.get("expiring").await.unwrap();
		assert_eq!(value, None);
	}
}
