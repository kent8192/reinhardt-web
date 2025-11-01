//! Response caching support for ViewSets
//!
//! Provides automatic caching for read-only operations (list and retrieve).
//! Supports TTL-based expiration and cache invalidation.

use async_trait::async_trait;
use reinhardt_apps::{Request, Response, Result};
use reinhardt_cache::Cache;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Cache configuration for ViewSets
#[derive(Debug, Clone)]
pub struct CacheConfig {
	/// Cache key prefix
	pub key_prefix: String,
	/// Time-to-live for cached responses
	pub ttl: Option<Duration>,
	/// Whether to cache list() responses
	pub cache_list: bool,
	/// Whether to cache retrieve() responses
	pub cache_retrieve: bool,
}

impl CacheConfig {
	/// Create a new cache configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_viewsets::CacheConfig;
	/// use std::time::Duration;
	///
	/// let config = CacheConfig::new("users")
	///     .with_ttl(Duration::from_secs(300))
	///     .cache_all();
	///
	/// assert_eq!(config.key_prefix, "users");
	/// assert!(config.cache_list);
	/// assert!(config.cache_retrieve);
	/// ```
	pub fn new(key_prefix: impl Into<String>) -> Self {
		Self {
			key_prefix: key_prefix.into(),
			ttl: None,
			cache_list: false,
			cache_retrieve: false,
		}
	}

	/// Set TTL for cached responses
	pub fn with_ttl(mut self, ttl: Duration) -> Self {
		self.ttl = Some(ttl);
		self
	}

	/// Enable caching for list() operations
	pub fn cache_list_only(mut self) -> Self {
		self.cache_list = true;
		self.cache_retrieve = false;
		self
	}

	/// Enable caching for retrieve() operations
	pub fn cache_retrieve_only(mut self) -> Self {
		self.cache_list = false;
		self.cache_retrieve = true;
		self
	}

	/// Enable caching for all read operations
	pub fn cache_all(mut self) -> Self {
		self.cache_list = true;
		self.cache_retrieve = true;
		self
	}
}

impl Default for CacheConfig {
	fn default() -> Self {
		Self::new("viewset").cache_all()
	}
}

/// Cached response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
	/// HTTP status code
	pub status: u16,
	/// Response body
	pub body: Vec<u8>,
	/// Response headers (simplified)
	pub headers: Vec<(String, String)>,
}

impl CachedResponse {
	/// Create from a Response
	pub fn from_response(response: &Response) -> Self {
		let headers = response
			.headers
			.iter()
			.map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
			.collect();

		Self {
			status: response.status.as_u16(),
			body: response.body.to_vec(),
			headers,
		}
	}

	/// Convert to Response
	pub fn to_response(&self) -> Response {
		use hyper::StatusCode;

		let mut response =
			Response::new(StatusCode::from_u16(self.status).unwrap_or(StatusCode::OK));

		response.body = self.body.clone().into();

		for (key, value) in &self.headers {
			if let Ok(header_name) = hyper::header::HeaderName::from_bytes(key.as_bytes())
				&& let Ok(header_value) = hyper::header::HeaderValue::from_str(value) {
					response.headers.insert(header_name, header_value);
				}
		}

		response
	}
}

/// Cached ViewSet wrapper
///
/// # Example
///
/// ```
/// use reinhardt_viewsets::{CachedViewSet, CacheConfig, ModelViewSet};
/// use reinhardt_cache::InMemoryCache;
/// use std::time::Duration;
///
/// #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// #[derive(Debug, Clone)]
/// struct UserSerializer;
///
/// # async fn example() {
/// let cache = InMemoryCache::new();
/// let inner_viewset = ModelViewSet::<User, UserSerializer>::new("users");
/// let config = CacheConfig::new("users")
///     .with_ttl(Duration::from_secs(300))
///     .cache_all();
///
/// let cached_viewset = CachedViewSet::new(inner_viewset, cache, config);
/// # }
/// ```
pub struct CachedViewSet<V, C> {
	/// Inner ViewSet
	inner: Arc<V>,
	/// Cache backend
	cache: Arc<C>,
	/// Cache configuration
	config: CacheConfig,
}

impl<V, C> CachedViewSet<V, C>
where
	C: Cache,
{
	/// Create a new cached ViewSet
	pub fn new(inner: V, cache: C, config: CacheConfig) -> Self {
		Self {
			inner: Arc::new(inner),
			cache: Arc::new(cache),
			config,
		}
	}

	/// Get the cache key for a list operation
	fn list_cache_key(&self, query_string: &str) -> String {
		format!("{}:list:{}", self.config.key_prefix, query_string)
	}

	/// Get the cache key for a retrieve operation
	fn retrieve_cache_key(&self, id: &str) -> String {
		format!("{}:retrieve:{}", self.config.key_prefix, id)
	}

	/// Get the inner ViewSet
	pub fn inner(&self) -> Arc<V> {
		self.inner.clone()
	}

	/// Get the cache backend
	pub fn cache(&self) -> Arc<C> {
		self.cache.clone()
	}

	/// Invalidate all cached responses
	pub async fn invalidate_all(&self) -> Result<()> {
		// Note: This is a simplified implementation
		// A production implementation would use cache tags or patterns
		self.cache.clear().await?;
		Ok(())
	}

	/// Invalidate cached response for a specific item
	pub async fn invalidate_item(&self, id: &str) -> Result<()> {
		let key = self.retrieve_cache_key(id);
		self.cache.delete(&key).await?;
		Ok(())
	}
}

/// Trait for cached read operations
#[async_trait]
pub trait CachedViewSetTrait: Send + Sync {
	/// Cached list operation
	async fn cached_list(&self, request: Request) -> Result<Response>;

	/// Cached retrieve operation
	async fn cached_retrieve(&self, request: Request, id: String) -> Result<Response>;

	/// Invalidate cache for a specific item
	async fn invalidate(&self, id: &str) -> Result<()>;

	/// Invalidate all cached items
	async fn invalidate_all(&self) -> Result<()>;
}

#[async_trait]
impl<V, C> CachedViewSetTrait for CachedViewSet<V, C>
where
	V: crate::ListMixin + crate::RetrieveMixin + Send + Sync + 'static,
	C: Cache + Send + Sync + 'static,
{
	async fn cached_list(&self, request: Request) -> Result<Response> {
		if !self.config.cache_list {
			// Caching disabled, passthrough to inner viewset
			return self.inner.list(request).await;
		}

		let query_string = request.uri.query().unwrap_or("");
		let cache_key = self.list_cache_key(query_string);

		// Try to get from cache
		if let Some(cached) = self.cache.get::<CachedResponse>(&cache_key).await? {
			return Ok(cached.to_response());
		}

		// Cache miss - call inner viewset and cache result
		let response = self.inner.list(request).await?;
		let cached = CachedResponse::from_response(&response);

		// Cache the response with configured TTL
		self.cache.set(&cache_key, &cached, self.config.ttl).await?;

		Ok(response)
	}

	async fn cached_retrieve(&self, request: Request, id: String) -> Result<Response> {
		if !self.config.cache_retrieve {
			// Caching disabled, passthrough to inner viewset
			return self.inner.retrieve(request, id).await;
		}

		let cache_key = self.retrieve_cache_key(&id);

		// Try to get from cache
		if let Some(cached) = self.cache.get::<CachedResponse>(&cache_key).await? {
			return Ok(cached.to_response());
		}

		// Cache miss - call inner viewset and cache result
		let response = self.inner.retrieve(request, id.clone()).await?;
		let cached = CachedResponse::from_response(&response);

		// Cache the response with configured TTL
		self.cache.set(&cache_key, &cached, self.config.ttl).await?;

		Ok(response)
	}

	async fn invalidate(&self, id: &str) -> Result<()> {
		self.invalidate_item(id).await
	}

	async fn invalidate_all(&self) -> Result<()> {
		self.invalidate_all().await
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, StatusCode, Uri, Version};
	use reinhardt_cache::InMemoryCache;

	#[test]
	fn test_cache_config_builder() {
		let config = CacheConfig::new("users")
			.with_ttl(Duration::from_secs(300))
			.cache_all();

		assert_eq!(config.key_prefix, "users");
		assert_eq!(config.ttl, Some(Duration::from_secs(300)));
		assert!(config.cache_list);
		assert!(config.cache_retrieve);
	}

	#[test]
	fn test_cache_config_list_only() {
		let config = CacheConfig::new("posts").cache_list_only();

		assert!(config.cache_list);
		assert!(!config.cache_retrieve);
	}

	#[test]
	fn test_cache_config_retrieve_only() {
		let config = CacheConfig::new("posts").cache_retrieve_only();

		assert!(!config.cache_list);
		assert!(config.cache_retrieve);
	}

	#[test]
	fn test_cached_response_conversion() {
		let mut original = Response::new(StatusCode::OK);
		original.body = Bytes::from("test body");
		let cached = CachedResponse::from_response(&original);

		assert_eq!(cached.status, 200);
		assert_eq!(cached.body, b"test body");

		let restored = cached.to_response();
		assert_eq!(restored.status, StatusCode::OK);
		assert_eq!(restored.body, Bytes::from("test body"));
	}

	#[test]
	fn test_cached_viewset_creation() {
		#[derive(Debug, Clone)]
		struct TestViewSet {
			#[allow(dead_code)]
			name: String,
		}

		let inner = TestViewSet {
			name: "users".to_string(),
		};
		let cache = InMemoryCache::new();
		let config = CacheConfig::new("users").cache_all();

		let cached_viewset = CachedViewSet::new(inner, cache, config);
		assert_eq!(cached_viewset.config.key_prefix, "users");
	}

	#[test]
	fn test_cache_keys() {
		#[derive(Debug, Clone)]
		struct TestViewSet;

		let inner = TestViewSet;
		let cache = InMemoryCache::new();
		let config = CacheConfig::new("users");

		let cached_viewset = CachedViewSet::new(inner, cache, config);

		let list_key = cached_viewset.list_cache_key("page=1&limit=10");
		assert_eq!(list_key, "users:list:page=1&limit=10");

		let retrieve_key = cached_viewset.retrieve_cache_key("123");
		assert_eq!(retrieve_key, "users:retrieve:123");
	}

	#[tokio::test]
	async fn test_invalidate_item() {
		#[derive(Debug, Clone)]
		struct TestViewSet;

		let inner = TestViewSet;
		let cache = InMemoryCache::new();
		let config = CacheConfig::new("users");

		let cached_viewset = CachedViewSet::new(inner, cache.clone(), config);

		// Set a cached value
		let cached_response = CachedResponse {
			status: 200,
			body: b"cached data".to_vec(),
			headers: vec![],
		};
		cache
			.set("users:retrieve:123", &cached_response, None)
			.await
			.unwrap();

		// Verify it exists
		let cached: Option<CachedResponse> = cache.get("users:retrieve:123").await.unwrap();
		assert!(cached.is_some());

		// Invalidate
		cached_viewset.invalidate_item("123").await.unwrap();

		// Verify it's gone
		let cached: Option<CachedResponse> = cache.get("users:retrieve:123").await.unwrap();
		assert!(cached.is_none());
	}

	#[tokio::test]
	async fn test_invalidate_all() {
		#[derive(Debug, Clone)]
		struct TestViewSet;

		let inner = TestViewSet;
		let cache = InMemoryCache::new();
		let config = CacheConfig::new("users");

		let cached_viewset = CachedViewSet::new(inner, cache.clone(), config);

		// Set multiple cached values
		let cached_response = CachedResponse {
			status: 200,
			body: b"cached data".to_vec(),
			headers: vec![],
		};

		cache
			.set("users:retrieve:123", &cached_response, None)
			.await
			.unwrap();
		cache
			.set("users:list:page=1", &cached_response, None)
			.await
			.unwrap();

		// Invalidate all
		cached_viewset.invalidate_all().await.unwrap();

		// Verify all are gone
		let cached1: Option<CachedResponse> = cache.get("users:retrieve:123").await.unwrap();
		let cached2: Option<CachedResponse> = cache.get("users:list:page=1").await.unwrap();
		assert!(cached1.is_none());
		assert!(cached2.is_none());
	}

	#[test]
	fn test_cache_config_default() {
		let config = CacheConfig::default();
		assert_eq!(config.key_prefix, "viewset");
		assert!(config.cache_list);
		assert!(config.cache_retrieve);
		assert_eq!(config.ttl, None);
	}
}
