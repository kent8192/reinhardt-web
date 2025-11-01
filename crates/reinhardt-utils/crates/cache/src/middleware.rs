//! Cache middleware for HTTP responses
//!
//! Provides middleware for caching HTTP responses.

use crate::{Cache, CacheKeyBuilder};
use async_trait::async_trait;
use bytes::Bytes;
use reinhardt_apps::{Handler, Middleware, Request, Response, Result};
use std::sync::Arc;
use std::time::Duration;

/// Cache middleware configuration
#[derive(Debug, Clone)]
pub struct CacheMiddlewareConfig {
	/// Default cache timeout
	pub default_timeout: Duration,

	/// Cache key prefix
	pub key_prefix: String,

	/// Cache only GET requests
	pub cache_get_only: bool,

	/// Cache only successful responses (2xx status codes)
	pub cache_success_only: bool,

	/// Header to check for cache control
	pub cache_control_header: String,
}

impl Default for CacheMiddlewareConfig {
	fn default() -> Self {
		Self {
			default_timeout: Duration::from_secs(300),
			key_prefix: "view_cache".to_string(),
			cache_get_only: true,
			cache_success_only: true,
			cache_control_header: "Cache-Control".to_string(),
		}
	}
}

impl CacheMiddlewareConfig {
	/// Create new cache middleware configuration with default settings
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::CacheMiddlewareConfig;
	/// use std::time::Duration;
	///
	/// let config = CacheMiddlewareConfig::new();
	/// assert_eq!(config.default_timeout, Duration::from_secs(300));
	/// assert_eq!(config.key_prefix, "view_cache");
	/// assert!(config.cache_get_only);
	/// ```
	pub fn new() -> Self {
		Self::default()
	}
	/// Set default timeout for cached responses
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::CacheMiddlewareConfig;
	/// use std::time::Duration;
	///
	/// let config = CacheMiddlewareConfig::new()
	///     .with_default_timeout(Duration::from_secs(600));
	/// assert_eq!(config.default_timeout, Duration::from_secs(600));
	/// ```
	pub fn with_default_timeout(mut self, timeout: Duration) -> Self {
		self.default_timeout = timeout;
		self
	}
	/// Set key prefix for cache namespacing
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::CacheMiddlewareConfig;
	///
	/// let config = CacheMiddlewareConfig::new()
	///     .with_key_prefix("api_cache");
	/// assert_eq!(config.key_prefix, "api_cache");
	/// ```
	pub fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self {
		self.key_prefix = prefix.into();
		self
	}
	/// Cache all request methods (not just GET)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::CacheMiddlewareConfig;
	///
	/// let config = CacheMiddlewareConfig::new()
	///     .cache_all_methods();
	/// assert!(!config.cache_get_only);
	/// ```
	pub fn cache_all_methods(mut self) -> Self {
		self.cache_get_only = false;
		self
	}
	/// Cache all responses (not just successful ones)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::CacheMiddlewareConfig;
	///
	/// let config = CacheMiddlewareConfig::new()
	///     .cache_all_responses();
	/// assert!(!config.cache_success_only);
	/// ```
	pub fn cache_all_responses(mut self) -> Self {
		self.cache_success_only = false;
		self
	}
}

/// HTTP response cache middleware
///
/// Caches HTTP responses based on request path and query parameters.
pub struct CacheMiddleware<C: Cache> {
	cache: Arc<C>,
	config: CacheMiddlewareConfig,
	key_builder: CacheKeyBuilder,
}

impl<C: Cache> CacheMiddleware<C> {
	/// Create new cache middleware with default configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::{CacheMiddleware, InMemoryCache};
	/// use std::sync::Arc;
	///
	/// let cache = Arc::new(InMemoryCache::new());
	/// let middleware = CacheMiddleware::new(cache);
	// Middleware is now ready to cache HTTP responses
	/// ```
	pub fn new(cache: Arc<C>) -> Self {
		Self {
			cache,
			config: CacheMiddlewareConfig::default(),
			key_builder: CacheKeyBuilder::new("view_cache"),
		}
	}
	/// Create with custom configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_cache::{CacheMiddleware, InMemoryCache, CacheMiddlewareConfig};
	/// use std::sync::Arc;
	/// use std::time::Duration;
	///
	/// let cache = Arc::new(InMemoryCache::new());
	/// let config = CacheMiddlewareConfig::new()
	///     .with_default_timeout(Duration::from_secs(600))
	///     .with_key_prefix("api");
	/// let middleware = CacheMiddleware::with_config(cache, config);
	// Middleware now uses custom configuration
	/// ```
	pub fn with_config(cache: Arc<C>, config: CacheMiddlewareConfig) -> Self {
		let key_builder = CacheKeyBuilder::new(&config.key_prefix);
		Self {
			cache,
			config,
			key_builder,
		}
	}

	/// Build cache key from request
	fn build_cache_key(&self, request: &Request) -> String {
		let path = request.uri.path();
		let query = request.uri.query().unwrap_or("");

		if query.is_empty() {
			self.key_builder.build(path)
		} else {
			self.key_builder.build(&format!("{}?{}", path, query))
		}
	}

	/// Check if request should be cached
	fn should_cache_request(&self, request: &Request) -> bool {
		if self.config.cache_get_only {
			request.method == hyper::Method::GET
		} else {
			true
		}
	}

	/// Check if response should be cached
	fn should_cache_response(&self, response: &Response) -> bool {
		if self.config.cache_success_only {
			let status = response.status.as_u16();
			(200..300).contains(&status)
		} else {
			true
		}
	}

	/// Parse cache timeout from response headers
	fn parse_cache_timeout(&self, response: &Response) -> Option<Duration> {
		// Parse Cache-Control header
		let cache_control = response
			.headers
			.get(&self.config.cache_control_header)?
			.to_str()
			.ok()?;

		// Parse max-age directive
		for directive in cache_control.split(',') {
			let directive = directive.trim();
			if directive.starts_with("max-age=") {
				if let Some(age_str) = directive.strip_prefix("max-age=")
					&& let Ok(seconds) = age_str.parse::<u64>() {
						return Some(Duration::from_secs(seconds));
					}
			} else if directive == "no-cache" || directive == "no-store" {
				// If no-cache or no-store is set, return None to indicate
				// that this response should not be cached
				return None;
			}
		}

		None
	}
}

#[async_trait]
impl<C: Cache + 'static> Middleware for CacheMiddleware<C> {
	async fn process(&self, request: Request, next: Arc<dyn Handler>) -> Result<Response> {
		// Check if we should cache this request
		if !self.should_cache_request(&request) {
			return next.handle(request).await;
		}

		let cache_key = self.build_cache_key(&request);

		// Try to get from cache
		if let Ok(Some(cached_response)) = self.cache.get::<CachedResponse>(&cache_key).await {
			// Reconstruct the Response with headers and body
			let status = hyper::StatusCode::from_u16(cached_response.status)
				.unwrap_or(hyper::StatusCode::OK);

			let body = Bytes::from(cached_response.body);
			let mut response = Response::new(status).with_body(body);

			// Restore cached headers
			for (name, value) in cached_response.headers {
				if let (Ok(header_name), Ok(header_value)) = (
					name.parse::<hyper::header::HeaderName>(),
					hyper::header::HeaderValue::from_bytes(&value),
				) {
					response.headers.insert(header_name, header_value);
				}
			}

			return Ok(response);
		}

		// Not in cache, process request
		let response = next.handle(request).await?;

		// Check if we should cache the response
		if self.should_cache_response(&response) {
			// Determine cache timeout
			// If parse_cache_timeout returns None and the response has no-cache/no-store,
			// we skip caching. Otherwise, use the parsed timeout or the default.
			if let Some(timeout) = self
				.parse_cache_timeout(&response)
				.or(Some(self.config.default_timeout))
			{
				// Extract headers to cache
				let headers: Vec<(String, Vec<u8>)> = response
					.headers
					.iter()
					.map(|(name, value)| (name.to_string(), value.as_bytes().to_vec()))
					.collect();

				// Cache the response with full metadata
				let cached = CachedResponse {
					status: response.status.as_u16(),
					headers,
					body: response.body.to_vec(),
				};

				let _ = self.cache.set(&cache_key, &cached, Some(timeout)).await;
			}
		}

		Ok(response)
	}
}

/// Cached HTTP response
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CachedResponse {
	status: u16,
	headers: Vec<(String, Vec<u8>)>,
	body: Vec<u8>,
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::InMemoryCache;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Uri, Version};

	fn create_test_request(method: Method, path: &str) -> Request {
		Request::new(
			method,
			path.parse::<Uri>().unwrap(),
			Version::HTTP_11,
			HeaderMap::new(),
			Bytes::new(),
		)
	}

	#[test]
	fn test_cache_middleware_config() {
		let config = CacheMiddlewareConfig::new()
			.with_default_timeout(Duration::from_secs(600))
			.with_key_prefix("api_cache")
			.cache_all_methods();

		assert_eq!(config.default_timeout, Duration::from_secs(600));
		assert_eq!(config.key_prefix, "api_cache");
		assert!(!config.cache_get_only);
	}

	#[test]
	fn test_cache_key_building() {
		let cache = Arc::new(InMemoryCache::new());
		let middleware = CacheMiddleware::new(cache);

		let request = create_test_request(Method::GET, "/api/users?page=1");
		let key = middleware.build_cache_key(&request);

		assert!(key.contains("api/users"));
		assert!(key.contains("page=1"));
	}

	#[test]
	fn test_should_cache_request() {
		let cache = Arc::new(InMemoryCache::new());
		let middleware = CacheMiddleware::new(cache);

		let get_request = create_test_request(Method::GET, "/api/users");
		assert!(middleware.should_cache_request(&get_request));

		let post_request = create_test_request(Method::POST, "/api/users");
		assert!(!middleware.should_cache_request(&post_request));
	}

	#[test]
	fn test_should_cache_all_methods() {
		let cache = Arc::new(InMemoryCache::new());
		let config = CacheMiddlewareConfig::new().cache_all_methods();
		let middleware = CacheMiddleware::with_config(cache, config);

		let post_request = create_test_request(Method::POST, "/api/users");
		assert!(middleware.should_cache_request(&post_request));
	}
}
