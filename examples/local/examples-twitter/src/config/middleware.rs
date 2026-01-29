//! Server middleware
//!
//! Production-ready middleware stack for the Twitter clone example.

use reinhardt::middleware::cors::CorsConfig;
use reinhardt::middleware::request_id::RequestIdConfig;
use reinhardt::middleware::{CorsMiddleware, LoggingMiddleware, RequestIdMiddleware};
use reinhardt::prelude::*;
use std::sync::Arc;

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

/// Create a production-ready middleware stack for the Twitter clone.
///
/// Stack includes:
/// - CORS for cross-origin API access
/// - Request ID for distributed tracing
/// - Logging for request/response auditing
pub fn create_middleware_stack() -> Vec<Arc<dyn Middleware>> {
	let request_id_config = RequestIdConfig::default();

	vec![
		Arc::new(RequestIdMiddleware::new(request_id_config)),
		Arc::new(LoggingMiddleware::new()),
		Arc::new(create_cors_middleware()),
	]
}
