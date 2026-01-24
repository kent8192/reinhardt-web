//! Server middleware
//!
//! Production-ready middleware stack for the Twitter clone example.

use reinhardt::prelude::*;
use std::sync::Arc;

/// Create a production-ready middleware stack for the Twitter clone.
///
/// Stack includes:
/// - CORS for cross-origin API access
/// - Request ID for distributed tracing
/// - Logging for request/response auditing
pub fn create_middleware_stack() -> Vec<Arc<dyn Middleware>> {
	use reinhardt::middleware::cors::CorsConfig;
	use reinhardt::middleware::request_id::RequestIdConfig;
	use reinhardt::middleware::{CorsMiddleware, LoggingMiddleware, RequestIdMiddleware};

	let cors_config = CorsConfig {
		allow_origins: vec!["*".to_string()],
		allow_methods: vec![
			"GET".to_string(),
			"POST".to_string(),
			"PUT".to_string(),
			"DELETE".to_string(),
		],
		allow_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
		allow_credentials: true,
		max_age: Some(3600),
	};

	let request_id_config = RequestIdConfig::default();

	vec![
		Arc::new(RequestIdMiddleware::new(request_id_config)),
		Arc::new(LoggingMiddleware::new()),
		Arc::new(CorsMiddleware::new(cors_config)),
	]
}
