//! Rate limiting permission for IP/user-based request control
//!
//! Provides permission checking based on rate limits, integrating with
//! the throttling backend for distributed rate limiting support.

use crate::{Permission, PermissionContext};
use async_trait::async_trait;
use reinhardt_throttling::ThrottleBackend;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;

// Type alias to simplify custom key extraction function signature
/// Custom key extraction function that takes a PermissionContext and returns an optional key string
pub type CustomKeyFn = Arc<dyn Fn(&PermissionContext) -> Option<String> + Send + Sync>;

/// Strategy for generating rate limit keys
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitKeyStrategy {
	/// Use client IP address as key
	IpAddress,
	/// Use authenticated user ID as key (requires authentication)
	UserId,
	/// Combine IP and user ID (requires authentication)
	IpAndUser,
	/// Custom key extraction (provided via closure)
	Custom,
}

/// Configuration for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
	/// Maximum number of requests allowed
	pub rate: usize,
	/// Time window in seconds
	pub window: u64,
	/// Key generation strategy
	pub strategy: RateLimitKeyStrategy,
	/// Allow requests on backend errors (fail-open)
	pub allow_on_error: bool,
	/// Scope identifier for namespacing rate limits
	pub scope: Option<String>,
}

impl RateLimitConfig {
	/// Creates a new rate limit configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::{RateLimitConfig, RateLimitKeyStrategy};
	///
	/// let config = RateLimitConfig::new(100, 60, RateLimitKeyStrategy::IpAddress);
	/// assert_eq!(config.rate, 100);
	/// assert_eq!(config.window, 60);
	/// ```
	pub fn new(rate: usize, window: u64, strategy: RateLimitKeyStrategy) -> Self {
		Self {
			rate,
			window,
			strategy,
			allow_on_error: false,
			scope: None,
		}
	}

	/// Creates a builder for fluent configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::{RateLimitConfig, RateLimitKeyStrategy};
	///
	/// let config = RateLimitConfig::builder()
	///     .rate(100)
	///     .window(60)
	///     .strategy(RateLimitKeyStrategy::IpAddress)
	///     .allow_on_error(true)
	///     .scope("api".to_string())
	///     .build();
	///
	/// assert_eq!(config.rate, 100);
	/// assert!(config.allow_on_error);
	/// ```
	pub fn builder() -> RateLimitConfigBuilder {
		RateLimitConfigBuilder::default()
	}
}

/// Builder for RateLimitConfig
#[derive(Debug, Default)]
pub struct RateLimitConfigBuilder {
	rate: Option<usize>,
	window: Option<u64>,
	strategy: Option<RateLimitKeyStrategy>,
	allow_on_error: bool,
	scope: Option<String>,
}

impl RateLimitConfigBuilder {
	/// Set the maximum number of requests allowed
	pub fn rate(mut self, rate: usize) -> Self {
		self.rate = Some(rate);
		self
	}

	/// Set the time window in seconds
	pub fn window(mut self, window: u64) -> Self {
		self.window = Some(window);
		self
	}

	/// Set the key generation strategy
	pub fn strategy(mut self, strategy: RateLimitKeyStrategy) -> Self {
		self.strategy = Some(strategy);
		self
	}

	/// Allow requests on backend errors
	pub fn allow_on_error(mut self, allow: bool) -> Self {
		self.allow_on_error = allow;
		self
	}

	/// Set scope identifier for namespacing
	pub fn scope(mut self, scope: String) -> Self {
		self.scope = Some(scope);
		self
	}

	/// Build the configuration
	///
	/// # Panics
	///
	/// Panics if rate, window, or strategy are not set
	pub fn build(self) -> RateLimitConfig {
		RateLimitConfig {
			rate: self.rate.expect("rate must be set"),
			window: self.window.expect("window must be set"),
			strategy: self.strategy.expect("strategy must be set"),
			allow_on_error: self.allow_on_error,
			scope: self.scope,
		}
	}
}

/// Rate limiting permission
///
/// Checks if a request should be allowed based on rate limits.
/// Integrates with throttling backends for distributed rate limiting.
///
/// # Examples
///
/// ```
/// use reinhardt_auth::{RateLimitPermission, RateLimitConfig, RateLimitKeyStrategy};
/// use reinhardt_throttling::MemoryBackend;
/// use std::sync::Arc;
///
/// let backend = Arc::new(MemoryBackend::new());
/// let config = RateLimitConfig::new(100, 60, RateLimitKeyStrategy::IpAddress);
/// let permission = RateLimitPermission::new(backend, config);
///
/// // Permission will now enforce 100 requests per 60 seconds per IP
/// ```
pub struct RateLimitPermission<B: ThrottleBackend> {
	backend: Arc<B>,
	config: RateLimitConfig,
	custom_key_fn: Option<CustomKeyFn>,
}

impl<B: ThrottleBackend> RateLimitPermission<B> {
	/// Creates a new rate limit permission
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::{RateLimitPermission, RateLimitConfig, RateLimitKeyStrategy};
	/// use reinhardt_throttling::MemoryBackend;
	/// use std::sync::Arc;
	///
	/// let backend = Arc::new(MemoryBackend::new());
	/// let config = RateLimitConfig::new(1000, 3600, RateLimitKeyStrategy::UserId);
	/// let permission = RateLimitPermission::new(backend, config);
	/// ```
	pub fn new(backend: Arc<B>, config: RateLimitConfig) -> Self {
		Self {
			backend,
			config,
			custom_key_fn: None,
		}
	}

	/// Creates a builder for fluent configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::{RateLimitPermission, RateLimitConfig, RateLimitKeyStrategy};
	/// use reinhardt_throttling::MemoryBackend;
	/// use std::sync::Arc;
	///
	/// let backend = Arc::new(MemoryBackend::new());
	/// let config = RateLimitConfig::new(100, 60, RateLimitKeyStrategy::IpAddress);
	///
	/// let permission = RateLimitPermission::builder()
	///     .backend(backend)
	///     .config(config)
	///     .build();
	/// ```
	pub fn builder() -> RateLimitPermissionBuilder<B> {
		RateLimitPermissionBuilder {
			backend: None,
			config: None,
			custom_key_fn: None,
		}
	}

	/// Set custom key extraction function
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_auth::{RateLimitPermission, RateLimitConfig, RateLimitKeyStrategy};
	/// use reinhardt_throttling::MemoryBackend;
	/// use std::sync::Arc;
	///
	/// let backend = Arc::new(MemoryBackend::new());
	/// let config = RateLimitConfig::new(100, 60, RateLimitKeyStrategy::Custom);
	///
	/// let permission = RateLimitPermission::new(backend, config)
	///     .with_custom_key(|ctx| {
	///         // Extract custom key from request
	///         Some("custom_key".to_string())
	///     });
	/// ```
	pub fn with_custom_key<F>(mut self, f: F) -> Self
	where
		F: Fn(&PermissionContext) -> Option<String> + Send + Sync + 'static,
	{
		self.custom_key_fn = Some(Arc::new(f));
		self
	}

	/// Extract IP address from request
	fn extract_ip(&self, context: &PermissionContext) -> Option<String> {
		// Try X-Forwarded-For header first
		if let Some(forwarded) = context.request.headers.get("X-Forwarded-For")
			&& let Ok(forwarded_str) = forwarded.to_str()
		{
			// Take the first IP in the chain
			if let Some(first_ip) = forwarded_str.split(',').next()
				&& let Ok(ip) = IpAddr::from_str(first_ip.trim())
			{
				return Some(ip.to_string());
			}
		}

		// Try X-Real-IP header
		if let Some(real_ip) = context.request.headers.get("X-Real-IP")
			&& let Ok(ip_str) = real_ip.to_str()
			&& let Ok(ip) = IpAddr::from_str(ip_str.trim())
		{
			return Some(ip.to_string());
		}

		// Extract from socket address if available
		if let Some(remote_addr) = context.request.remote_addr {
			return Some(remote_addr.ip().to_string());
		}

		None
	}

	/// Extract user ID from context
	fn extract_user_id(&self, context: &PermissionContext) -> Option<String> {
		context.user.as_ref().map(|user| user.id())
	}

	/// Generate rate limit key based on strategy
	fn generate_key(&self, context: &PermissionContext) -> Option<String> {
		let base_key = match self.config.strategy {
			RateLimitKeyStrategy::IpAddress => self.extract_ip(context),
			RateLimitKeyStrategy::UserId => self.extract_user_id(context),
			RateLimitKeyStrategy::IpAndUser => {
				if let (Some(ip), Some(user_id)) =
					(self.extract_ip(context), self.extract_user_id(context))
				{
					Some(format!("{}:{}", ip, user_id))
				} else {
					None
				}
			}
			RateLimitKeyStrategy::Custom => {
				if let Some(ref custom_fn) = self.custom_key_fn {
					custom_fn(context)
				} else {
					None
				}
			}
		};

		// Add scope prefix if configured
		base_key.map(|key| {
			if let Some(ref scope) = self.config.scope {
				format!("{}:{}", scope, key)
			} else {
				key
			}
		})
	}
}

/// Builder for RateLimitPermission
pub struct RateLimitPermissionBuilder<B: ThrottleBackend> {
	backend: Option<Arc<B>>,
	config: Option<RateLimitConfig>,
	custom_key_fn: Option<CustomKeyFn>,
}

impl<B: ThrottleBackend> RateLimitPermissionBuilder<B> {
	/// Set the throttle backend
	pub fn backend(mut self, backend: Arc<B>) -> Self {
		self.backend = Some(backend);
		self
	}

	/// Set the rate limit configuration
	pub fn config(mut self, config: RateLimitConfig) -> Self {
		self.config = Some(config);
		self
	}

	/// Set custom key extraction function
	pub fn custom_key<F>(mut self, f: F) -> Self
	where
		F: Fn(&PermissionContext) -> Option<String> + Send + Sync + 'static,
	{
		self.custom_key_fn = Some(Arc::new(f));
		self
	}

	/// Build the permission
	///
	/// # Panics
	///
	/// Panics if backend or config are not set
	pub fn build(self) -> RateLimitPermission<B> {
		RateLimitPermission {
			backend: self.backend.expect("backend must be set"),
			config: self.config.expect("config must be set"),
			custom_key_fn: self.custom_key_fn,
		}
	}
}

#[async_trait]
impl<B: ThrottleBackend> Permission for RateLimitPermission<B> {
	async fn has_permission(&self, context: &PermissionContext<'_>) -> bool {
		// Generate rate limit key
		let key = match self.generate_key(context) {
			Some(k) => k,
			None => {
				// No key could be generated (e.g., unauthenticated user with UserId strategy)
				return false;
			}
		};

		// Check rate limit using backend
		match self.backend.increment(&key, self.config.window).await {
			Ok(count) => {
				// Allow if under rate limit
				count <= self.config.rate
			}
			Err(_) => {
				// On error, use configured fail-open/fail-closed behavior
				self.config.allow_on_error
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bytes::Bytes;
	use hyper::{HeaderMap, Method, Uri, Version};
	use reinhardt_core::types::Request;
	use reinhardt_throttling::MemoryBackend;

	fn create_test_request(headers: HeaderMap) -> Request {
		Request::new(
			Method::GET,
			Uri::from_static("/test"),
			Version::HTTP_11,
			headers,
			Bytes::new(),
		)
	}

	#[tokio::test]
	async fn test_rate_limit_permission_ip_strategy() {
		let backend = Arc::new(MemoryBackend::new());
		let config = RateLimitConfig::new(2, 60, RateLimitKeyStrategy::IpAddress);
		let permission = RateLimitPermission::new(backend, config);

		let mut headers = HeaderMap::new();
		headers.insert("X-Forwarded-For", "192.168.1.100".parse().unwrap());

		let request = create_test_request(headers);
		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};

		// First two requests should be allowed
		assert!(permission.has_permission(&context).await);
		assert!(permission.has_permission(&context).await);

		// Third request should be denied
		assert!(!permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_rate_limit_permission_user_strategy() {
		use crate::SimpleUser;
		use uuid::Uuid;

		let backend = Arc::new(MemoryBackend::new());
		let config = RateLimitConfig::new(3, 60, RateLimitKeyStrategy::UserId);
		let permission = RateLimitPermission::new(backend, config);

		let headers = HeaderMap::new();
		let request = create_test_request(headers);

		let test_user = SimpleUser {
			id: Uuid::new_v4(),
			username: "testuser".to_string(),
			email: "test@example.com".to_string(),
			is_active: true,
			is_admin: false,
			is_staff: false,
			is_superuser: false,
		};

		let context = PermissionContext {
			request: &request,
			is_authenticated: true,
			is_admin: false,
			is_active: true,
			user: Some(Box::new(test_user)),
		};

		// First three requests should be allowed
		assert!(permission.has_permission(&context).await);
		assert!(permission.has_permission(&context).await);
		assert!(permission.has_permission(&context).await);

		// Fourth request should be denied
		assert!(!permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_rate_limit_permission_unauthenticated_user_strategy() {
		let backend = Arc::new(MemoryBackend::new());
		let config = RateLimitConfig::new(10, 60, RateLimitKeyStrategy::UserId);
		let permission = RateLimitPermission::new(backend, config);

		let headers = HeaderMap::new();
		let request = create_test_request(headers);
		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};

		// Should be denied for unauthenticated users
		assert!(!permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_rate_limit_permission_custom_strategy() {
		let backend = Arc::new(MemoryBackend::new());
		let config = RateLimitConfig::new(2, 60, RateLimitKeyStrategy::Custom);
		let permission = RateLimitPermission::new(backend, config)
			.with_custom_key(|_ctx| Some("custom_key".to_string()));

		let headers = HeaderMap::new();
		let request = create_test_request(headers);
		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};

		// First two requests should be allowed
		assert!(permission.has_permission(&context).await);
		assert!(permission.has_permission(&context).await);

		// Third request should be denied
		assert!(!permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_rate_limit_config_builder() {
		let config = RateLimitConfig::builder()
			.rate(100)
			.window(3600)
			.strategy(RateLimitKeyStrategy::IpAddress)
			.allow_on_error(true)
			.scope("api".to_string())
			.build();

		assert_eq!(config.rate, 100);
		assert_eq!(config.window, 3600);
		assert_eq!(config.strategy, RateLimitKeyStrategy::IpAddress);
		assert!(config.allow_on_error);
		assert_eq!(config.scope, Some("api".to_string()));
	}

	#[tokio::test]
	async fn test_rate_limit_permission_builder() {
		let backend = Arc::new(MemoryBackend::new());
		let config = RateLimitConfig::new(5, 60, RateLimitKeyStrategy::IpAddress);

		let _permission = RateLimitPermission::builder()
			.backend(backend)
			.config(config)
			.build();

		// Successfully built
	}

	#[tokio::test]
	async fn test_rate_limit_permission_with_scope() {
		let backend = Arc::new(MemoryBackend::new());
		let config = RateLimitConfig::builder()
			.rate(2)
			.window(60)
			.strategy(RateLimitKeyStrategy::IpAddress)
			.scope("api".to_string())
			.build();

		let permission = RateLimitPermission::new(backend, config);

		let mut headers = HeaderMap::new();
		headers.insert("X-Real-IP", "10.0.0.1".parse().unwrap());

		let request = create_test_request(headers);
		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};

		// Should work with scoped keys
		assert!(permission.has_permission(&context).await);
		assert!(permission.has_permission(&context).await);
		assert!(!permission.has_permission(&context).await);
	}

	#[tokio::test]
	async fn test_rate_limit_permission_x_real_ip_header() {
		let backend = Arc::new(MemoryBackend::new());
		let config = RateLimitConfig::new(1, 60, RateLimitKeyStrategy::IpAddress);
		let permission = RateLimitPermission::new(backend, config);

		let mut headers = HeaderMap::new();
		headers.insert("X-Real-IP", "172.16.0.1".parse().unwrap());

		let request = create_test_request(headers);
		let context = PermissionContext {
			request: &request,
			is_authenticated: false,
			is_admin: false,
			is_active: false,
			user: None,
		};

		// First request allowed
		assert!(permission.has_permission(&context).await);
		// Second request denied
		assert!(!permission.has_permission(&context).await);
	}

	#[test]
	fn test_rate_limit_key_strategy_equality() {
		assert_eq!(
			RateLimitKeyStrategy::IpAddress,
			RateLimitKeyStrategy::IpAddress
		);
		assert_ne!(
			RateLimitKeyStrategy::IpAddress,
			RateLimitKeyStrategy::UserId
		);
	}
}
