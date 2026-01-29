//! Server middleware
//!
//! Production-ready middleware stack for the Twitter clone example.

use reinhardt::middleware::cors::CorsConfig;
use reinhardt::middleware::security_middleware::{SecurityConfig, SecurityMiddleware};
use reinhardt::middleware::{CorsMiddleware, LoggingMiddleware};
use reinhardt::prelude::*;
use reinhardt::utils::staticfiles::caching::{
	CacheControlConfig, CacheControlMiddleware, CacheDirective, CachePolicy,
};
use std::sync::Arc;
use std::time::Duration;

/// Create CORS middleware with credentials support for the Twitter clone.
///
/// Enables cross-origin requests with authentication support:
/// - Allows credentials for cookie/session-based auth
/// - Supports all standard HTTP methods including OPTIONS
/// - Includes CSRF token header for security
pub fn create_cors_middleware() -> CorsMiddleware {
	let config = CorsConfig {
		allow_origins: vec!["*".to_string()], // Development
		allow_methods: vec![
			"GET".to_string(),
			"POST".to_string(),
			"PUT".to_string(),
			"DELETE".to_string(),
			"OPTIONS".to_string(),
		],
		allow_headers: vec![
			"Content-Type".to_string(),
			"Authorization".to_string(),
			"X-CSRF-Token".to_string(),
		],
		allow_credentials: true,
		max_age: Some(3600),
	};
	CorsMiddleware::new(config)
}

/// Create SecurityMiddleware with default configuration.
///
/// Provides security headers for the application:
/// - HSTS (HTTP Strict Transport Security)
/// - X-Content-Type-Options: nosniff
/// - Referrer-Policy: same-origin
pub fn create_security_middleware() -> SecurityMiddleware {
	SecurityMiddleware::with_config(SecurityConfig::default())
}

/// Create CacheControlMiddleware with optimized settings for API responses.
///
/// Configuration:
/// - public: true (responses can be cached by any cache)
/// - max_age: 3600 seconds (1 hour)
/// - s_maxage: 86400 seconds (1 day for shared caches/CDNs)
pub fn create_cache_control_middleware() -> CacheControlMiddleware {
	let cache_policy = CachePolicy::new()
		.with_directive(CacheDirective::Public)
		.with_max_age(Duration::from_secs(3600))
		.with_s_maxage(Duration::from_secs(86400));

	let cache_config = CacheControlConfig::new().with_default_policy(cache_policy);

	CacheControlMiddleware::new(cache_config)
}

/// Create a production-ready middleware stack for the Twitter clone.
///
/// Stack order (execution order for requests):
/// 1. LoggingMiddleware - Log all incoming requests
/// 2. SecurityMiddleware - Apply security headers
/// 3. CorsMiddleware - Handle cross-origin requests
/// 4. CacheControlMiddleware - Set cache headers for responses
pub fn create_middleware_stack() -> Vec<Arc<dyn Middleware>> {
	vec![
		Arc::new(LoggingMiddleware::new()),
		Arc::new(create_security_middleware()),
		Arc::new(create_cors_middleware()),
		Arc::new(create_cache_control_middleware()),
	]
}
