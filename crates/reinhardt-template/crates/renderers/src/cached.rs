//! Cached renderer implementation
//!
//! Caches rendering results to avoid redundant rendering of the same data.

use async_trait::async_trait;
use bytes::Bytes;
use moka::future::Cache;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;

use crate::renderer::{RenderResult, Renderer, RendererContext};

/// Cache configuration
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::CacheConfig;
/// use std::time::Duration;
///
/// let config = CacheConfig::default();
/// assert_eq!(config.ttl, Duration::from_secs(300));
/// assert_eq!(config.max_capacity, 1000);
/// ```
#[derive(Debug, Clone)]
pub struct CacheConfig {
	/// Time-To-Live (TTL) for cache entries
	pub ttl: Duration,
	/// Maximum cache size (number of entries)
	pub max_capacity: u64,
}

impl Default for CacheConfig {
	fn default() -> Self {
		Self {
			ttl: Duration::from_secs(300), // 5 minutes
			max_capacity: 1000,
		}
	}
}

impl CacheConfig {
	/// Create a new cache configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::CacheConfig;
	/// use std::time::Duration;
	///
	/// let config = CacheConfig::new()
	///     .with_ttl(Duration::from_secs(600))
	///     .with_max_capacity(5000);
	///
	/// assert_eq!(config.ttl, Duration::from_secs(600));
	/// assert_eq!(config.max_capacity, 5000);
	/// ```
	pub fn new() -> Self {
		Self::default()
	}

	/// Set TTL
	pub fn with_ttl(mut self, ttl: Duration) -> Self {
		self.ttl = ttl;
		self
	}

	/// Set maximum capacity
	pub fn with_max_capacity(mut self, capacity: u64) -> Self {
		self.max_capacity = capacity;
		self
	}
}

/// Renderer with caching functionality
///
/// Wraps an internal renderer and caches rendering results.
///
/// # Examples
///
/// ```
/// use reinhardt_renderers::{CachedRenderer, JSONRenderer, CacheConfig, Renderer};
/// use std::time::Duration;
/// use serde_json::json;
///
/// # #[tokio::main]
/// # async fn main() {
/// let config = CacheConfig::new()
///     .with_ttl(Duration::from_secs(300));
///
/// let cached_renderer = CachedRenderer::new(
///     JSONRenderer::new(),
///     config,
/// );
///
/// let data = json!({"message": "hello"});
///
/// // First time: execute rendering
/// let result1 = cached_renderer.render(&data, None).await.unwrap();
///
/// // Second time: retrieve from cache
/// let result2 = cached_renderer.render(&data, None).await.unwrap();
///
/// assert_eq!(result1, result2);
/// # }
/// ```
pub struct CachedRenderer<R: Renderer> {
	/// Internal renderer
	inner: Arc<R>,
	/// Cache for rendering results
	cache: Cache<String, Bytes>,
	/// Cache configuration
	config: CacheConfig,
}

impl<R: Renderer> CachedRenderer<R> {
	/// Create a new cached renderer
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{CachedRenderer, JSONRenderer, CacheConfig};
	///
	/// let renderer = CachedRenderer::new(
	///     JSONRenderer::new(),
	///     CacheConfig::default(),
	/// );
	/// ```
	pub fn new(renderer: R, config: CacheConfig) -> Self {
		let cache = Cache::builder()
			.max_capacity(config.max_capacity)
			.time_to_live(config.ttl)
			.build();

		Self {
			inner: Arc::new(renderer),
			cache,
			config,
		}
	}

	/// Generate cache key
	///
	/// Generates SHA256 hash from data and context.
	fn generate_cache_key(data: &Value, context: Option<&RendererContext>) -> String {
		let mut hasher = Sha256::new();

		// Add data to hash
		let data_str = serde_json::to_string(data).unwrap_or_default();
		hasher.update(data_str.as_bytes());

		// Add context information to hash
		if let Some(ctx) = context {
			if let Some((method, path)) = &ctx.request {
				hasher.update(method.as_bytes());
				hasher.update(path.as_bytes());
			}
			if let Some((name, desc)) = &ctx.view {
				hasher.update(name.as_bytes());
				hasher.update(desc.as_bytes());
			}
			if let Some(accept) = &ctx.accept_header {
				hasher.update(accept.as_bytes());
			}
			if let Some(format) = &ctx.format_param {
				hasher.update(format.as_bytes());
			}
			for (key, value) in &ctx.extra {
				hasher.update(key.as_bytes());
				hasher.update(value.as_bytes());
			}
		}

		let result = hasher.finalize();
		hex::encode(result)
	}

	/// Invalidate cache
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{CachedRenderer, JSONRenderer, CacheConfig};
	///
	/// # #[tokio::main]
	/// # async fn main() {
	/// let renderer = CachedRenderer::new(
	///     JSONRenderer::new(),
	///     CacheConfig::default(),
	/// );
	///
	/// renderer.invalidate().await;
	/// # }
	/// ```
	pub async fn invalidate(&self) {
		self.cache.invalidate_all();
		self.cache.run_pending_tasks().await;
	}

	/// Invalidate specific cache entry
	pub async fn invalidate_key(&self, data: &Value, context: Option<&RendererContext>) {
		let key = Self::generate_cache_key(data, context);
		self.cache.invalidate(&key).await;
	}

	/// Get cache statistics
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_renderers::{CachedRenderer, JSONRenderer, CacheConfig};
	///
	/// let renderer = CachedRenderer::new(
	///     JSONRenderer::new(),
	///     CacheConfig::default(),
	/// );
	///
	/// let entry_count = renderer.entry_count();
	/// assert_eq!(entry_count, 0);
	/// ```
	pub fn entry_count(&self) -> u64 {
		self.cache.entry_count()
	}

	/// Get reference to internal renderer
	pub fn inner(&self) -> &R {
		&self.inner
	}

	/// Get cache configuration
	pub fn config(&self) -> &CacheConfig {
		&self.config
	}
}

#[async_trait]
impl<R: Renderer + 'static> Renderer for CachedRenderer<R> {
	fn media_type(&self) -> String {
		self.inner.media_type()
	}

	fn media_types(&self) -> Vec<String> {
		self.inner.media_types()
	}

	fn format(&self) -> Option<&str> {
		self.inner.format()
	}

	async fn render(&self, data: &Value, context: Option<&RendererContext>) -> RenderResult<Bytes> {
		let cache_key = Self::generate_cache_key(data, context);

		// Check cache
		if let Some(cached_result) = self.cache.get(&cache_key).await {
			return Ok(cached_result);
		}

		// Cache miss - perform actual rendering
		let result = self.inner.render(data, context).await?;

		// Save result to cache
		self.cache.insert(cache_key, result.clone()).await;

		Ok(result)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::JSONRenderer;
	use serde_json::json;
	use std::sync::atomic::{AtomicUsize, Ordering};

	/// Test counting renderer
	/// Counts the number of rendering executions
	struct CountingRenderer {
		counter: Arc<AtomicUsize>,
		inner: JSONRenderer,
	}

	impl CountingRenderer {
		fn new(counter: Arc<AtomicUsize>) -> Self {
			Self {
				counter,
				inner: JSONRenderer::new(),
			}
		}
	}

	#[async_trait]
	impl Renderer for CountingRenderer {
		fn media_type(&self) -> String {
			self.inner.media_type()
		}

		fn media_types(&self) -> Vec<String> {
			self.inner.media_types()
		}

		fn format(&self) -> Option<&str> {
			self.inner.format()
		}

		async fn render(
			&self,
			data: &Value,
			context: Option<&RendererContext>,
		) -> RenderResult<Bytes> {
			self.counter.fetch_add(1, Ordering::SeqCst);
			self.inner.render(data, context).await
		}
	}

	#[tokio::test]
	async fn test_cache_hit() {
		let counter = Arc::new(AtomicUsize::new(0));
		let counting_renderer = CountingRenderer::new(counter.clone());

		let config = CacheConfig::default();
		let cached_renderer = CachedRenderer::new(counting_renderer, config);

		let data = json!({"message": "hello"});

		// First rendering
		let result1 = cached_renderer.render(&data, None).await.unwrap();
		assert_eq!(counter.load(Ordering::SeqCst), 1);

		// Second time hits cache
		let result2 = cached_renderer.render(&data, None).await.unwrap();
		assert_eq!(counter.load(Ordering::SeqCst), 1); // Count does not increase

		assert_eq!(result1, result2);
	}

	#[tokio::test]
	async fn test_cache_miss_different_data() {
		let counter = Arc::new(AtomicUsize::new(0));
		let counting_renderer = CountingRenderer::new(counter.clone());

		let cached_renderer = CachedRenderer::new(counting_renderer, CacheConfig::default());

		let data1 = json!({"message": "hello"});
		let data2 = json!({"message": "world"});

		// Different data is cached separately
		cached_renderer.render(&data1, None).await.unwrap();
		assert_eq!(counter.load(Ordering::SeqCst), 1);

		cached_renderer.render(&data2, None).await.unwrap();
		assert_eq!(counter.load(Ordering::SeqCst), 2);
	}

	#[tokio::test]
	async fn test_cache_miss_different_context() {
		let counter = Arc::new(AtomicUsize::new(0));
		let counting_renderer = CountingRenderer::new(counter.clone());

		let cached_renderer = CachedRenderer::new(counting_renderer, CacheConfig::default());

		let data = json!({"message": "hello"});
		let context1 = RendererContext::new().with_request("GET", "/api/v1");
		let context2 = RendererContext::new().with_request("GET", "/api/v2");

		// Different contexts are cached separately
		cached_renderer
			.render(&data, Some(&context1))
			.await
			.unwrap();
		assert_eq!(counter.load(Ordering::SeqCst), 1);

		cached_renderer
			.render(&data, Some(&context2))
			.await
			.unwrap();
		assert_eq!(counter.load(Ordering::SeqCst), 2);
	}

	#[tokio::test]
	async fn test_cache_invalidation() {
		let counter = Arc::new(AtomicUsize::new(0));
		let counting_renderer = CountingRenderer::new(counter.clone());

		let cached_renderer = CachedRenderer::new(counting_renderer, CacheConfig::default());

		let data = json!({"message": "hello"});

		// First rendering
		cached_renderer.render(&data, None).await.unwrap();
		assert_eq!(counter.load(Ordering::SeqCst), 1);

		// Invalidate cache
		cached_renderer.invalidate().await;

		// Re-render after invalidation
		cached_renderer.render(&data, None).await.unwrap();
		assert_eq!(counter.load(Ordering::SeqCst), 2);
	}

	#[tokio::test]
	async fn test_cache_key_invalidation() {
		let counter = Arc::new(AtomicUsize::new(0));
		let counting_renderer = CountingRenderer::new(counter.clone());

		let cached_renderer = CachedRenderer::new(counting_renderer, CacheConfig::default());

		let data = json!({"message": "hello"});

		// First rendering
		cached_renderer.render(&data, None).await.unwrap();
		assert_eq!(counter.load(Ordering::SeqCst), 1);

		// Invalidate specific key
		cached_renderer.invalidate_key(&data, None).await;

		// Re-render after invalidation
		cached_renderer.render(&data, None).await.unwrap();
		assert_eq!(counter.load(Ordering::SeqCst), 2);
	}

	#[tokio::test]
	async fn test_entry_count() {
		let cached_renderer = CachedRenderer::new(JSONRenderer::new(), CacheConfig::default());

		assert_eq!(cached_renderer.entry_count(), 0);

		let data1 = json!({"key": "value1"});
		let data2 = json!({"key": "value2"});

		cached_renderer.render(&data1, None).await.unwrap();
		// Execute pending tasks as moka processes cache asynchronously
		cached_renderer.cache.run_pending_tasks().await;
		assert_eq!(cached_renderer.entry_count(), 1);

		cached_renderer.render(&data2, None).await.unwrap();
		cached_renderer.cache.run_pending_tasks().await;
		assert_eq!(cached_renderer.entry_count(), 2);

		// Re-rendering the same data does not increase the count
		cached_renderer.render(&data1, None).await.unwrap();
		cached_renderer.cache.run_pending_tasks().await;
		assert_eq!(cached_renderer.entry_count(), 2);
	}

	#[tokio::test]
	async fn test_cache_ttl() {
		let counter = Arc::new(AtomicUsize::new(0));
		let counting_renderer = CountingRenderer::new(counter.clone());

		let config = CacheConfig::new().with_ttl(Duration::from_millis(100));
		let cached_renderer = CachedRenderer::new(counting_renderer, config);

		let data = json!({"message": "hello"});

		// First rendering
		cached_renderer.render(&data, None).await.unwrap();
		assert_eq!(counter.load(Ordering::SeqCst), 1);

		// Cache hit before TTL expires
		tokio::time::sleep(Duration::from_millis(50)).await;
		cached_renderer.render(&data, None).await.unwrap();
		assert_eq!(counter.load(Ordering::SeqCst), 1);

		// Cache miss after TTL expires
		tokio::time::sleep(Duration::from_millis(100)).await;
		cached_renderer.render(&data, None).await.unwrap();
		assert_eq!(counter.load(Ordering::SeqCst), 2);
	}

	#[tokio::test]
	async fn test_config_builder() {
		let config = CacheConfig::new()
			.with_ttl(Duration::from_secs(600))
			.with_max_capacity(5000);

		assert_eq!(config.ttl, Duration::from_secs(600));
		assert_eq!(config.max_capacity, 5000);
	}

	#[tokio::test]
	async fn test_renderer_trait_methods() {
		let cached_renderer = CachedRenderer::new(JSONRenderer::new(), CacheConfig::default());

		assert_eq!(
			cached_renderer.media_type(),
			"application/json; charset=utf-8"
		);
		assert_eq!(cached_renderer.format(), Some("json"));
		assert!(!cached_renderer.media_types().is_empty());
	}

	#[tokio::test]
	async fn test_inner_and_config_accessors() {
		let json_renderer = JSONRenderer::new().pretty(true);
		let config = CacheConfig::new().with_ttl(Duration::from_secs(600));

		let cached_renderer = CachedRenderer::new(json_renderer, config.clone());

		assert!(cached_renderer.inner().pretty);
		assert_eq!(cached_renderer.config().ttl, Duration::from_secs(600));
	}
}
