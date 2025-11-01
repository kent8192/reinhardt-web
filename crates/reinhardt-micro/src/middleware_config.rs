//! Middleware Configuration Helpers
//!
//! This module provides convenient builder-style methods for configuring middleware
//! in reinhardt-micro applications. It wraps the underlying reinhardt-middleware
//! configurations with a more ergonomic API.

use std::time::Duration;

// Re-export configuration types from reinhardt-middleware (with feature gates)
#[cfg(feature = "cors")]
pub use reinhardt_middleware::cors::CorsConfig;

#[cfg(feature = "compression")]
pub use reinhardt_middleware::gzip::GZipConfig as CompressionConfig;

pub use reinhardt_middleware::metrics::MetricsConfig;

#[cfg(feature = "rate-limit")]
pub use reinhardt_middleware::rate_limit::RateLimitConfig;

/// Logging middleware configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
	/// Enable request logging
	pub log_requests: bool,
	/// Enable response logging
	pub log_responses: bool,
	/// Log level for requests
	pub request_log_level: LogLevel,
	/// Log level for responses
	pub response_log_level: LogLevel,
}

/// Log level for logging configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
	/// Error level
	Error,
	/// Warning level
	Warn,
	/// Info level
	Info,
	/// Debug level
	Debug,
	/// Trace level
	Trace,
}

impl Default for LoggingConfig {
	fn default() -> Self {
		Self {
			log_requests: true,
			log_responses: true,
			request_log_level: LogLevel::Info,
			response_log_level: LogLevel::Info,
		}
	}
}

impl LoggingConfig {
	/// Create a quiet logging configuration (only errors)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_micro::middleware_config::LoggingConfig;
	///
	/// let config = LoggingConfig::quiet();
	/// assert!(config.log_requests);
	/// assert!(config.log_responses);
	/// ```
	pub fn quiet() -> Self {
		Self {
			log_requests: true,
			log_responses: true,
			request_log_level: LogLevel::Error,
			response_log_level: LogLevel::Error,
		}
	}

	/// Create a verbose logging configuration (debug level)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_micro::middleware_config::{LoggingConfig, LogLevel};
	///
	/// let config = LoggingConfig::verbose();
	/// assert_eq!(config.request_log_level, LogLevel::Debug);
	/// ```
	pub fn verbose() -> Self {
		Self {
			log_requests: true,
			log_responses: true,
			request_log_level: LogLevel::Debug,
			response_log_level: LogLevel::Debug,
		}
	}
}

/// CORS configuration helper functions
#[cfg(feature = "cors")]
pub mod cors {
	use super::CorsConfig;

	/// Create a permissive CORS configuration (allows all origins)
	///
	/// This is useful for development but should be used with caution in production.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_micro::middleware_config;
	///
	/// let config = middleware_config::cors::permissive();
	/// assert_eq!(config.allow_origins, vec!["*"]);
	/// assert!(config.allow_methods.contains(&"GET".to_string()));
	/// ```
	pub fn permissive() -> CorsConfig {
		CorsConfig::default()
	}

	/// Create a restrictive CORS configuration for production
	///
	/// # Arguments
	///
	/// * `allowed_origin` - Single allowed origin (e.g., "https://example.com")
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_micro::middleware_config;
	///
	/// let config = middleware_config::cors::restrictive("https://example.com");
	/// assert_eq!(config.allow_origins, vec!["https://example.com"]);
	/// assert_eq!(config.allow_credentials, true);
	/// ```
	pub fn restrictive(allowed_origin: &str) -> CorsConfig {
		CorsConfig {
			allow_origins: vec![allowed_origin.to_string()],
			allow_methods: vec!["GET".to_string(), "POST".to_string()],
			allow_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
			allow_credentials: true,
			max_age: Some(3600),
		}
	}
}

/// Compression configuration helper functions
#[cfg(feature = "compression")]
pub mod compression {
	use super::CompressionConfig;

	/// Create compression config optimized for JSON APIs
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_micro::middleware_config;
	///
	/// let config = middleware_config::compression::for_json();
	/// assert!(config.compressible_types.contains(&"application/json".to_string()));
	/// ```
	pub fn for_json() -> CompressionConfig {
		CompressionConfig {
			min_length: 512,
			compression_level: 6,
			compressible_types: vec![
				"application/json".to_string(),
				"application/xml".to_string(),
				"text/plain".to_string(),
			],
		}
	}
}

/// Rate limit configuration helper functions
#[cfg(feature = "rate-limit")]
pub mod rate_limit {
	use super::RateLimitConfig;

	/// Create a lenient rate limit configuration (100 requests per minute)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_micro::middleware_config;
	///
	/// let config = middleware_config::rate_limit::lenient();
	/// assert_eq!(config.capacity, 100.0);
	/// ```
	pub fn lenient() -> RateLimitConfig {
		RateLimitConfig::default()
	}

	/// Create a strict rate limit configuration (10 requests per minute)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_micro::middleware_config;
	///
	/// let config = middleware_config::rate_limit::strict();
	/// assert_eq!(config.capacity, 10.0);
	/// ```
	pub fn strict() -> RateLimitConfig {
		RateLimitConfig {
			capacity: 10.0,
			refill_rate: 10.0 / 60.0,
			cost_per_request: 1.0,
			exclude_paths: vec![],
			error_message: Some("Rate limit exceeded. Please try again later.".to_string()),
			..Default::default()
		}
	}
}

/// Metrics configuration helper functions
pub mod metrics {
	use super::MetricsConfig;

	/// Create metrics config with custom endpoint
	///
	/// # Arguments
	///
	/// * `endpoint` - Endpoint path for metrics (e.g., "/metrics")
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_micro::middleware_config;
	///
	/// let config = middleware_config::metrics::with_endpoint("/custom-metrics");
	/// assert_eq!(config.metrics_endpoint, "/custom-metrics");
	/// assert!(config.track_response_time);
	/// ```
	pub fn with_endpoint(endpoint: &str) -> MetricsConfig {
		MetricsConfig {
			metrics_endpoint: endpoint.to_string(),
			track_response_time: true,
			exclude_paths: vec![],
		}
	}
}

/// Timeout configuration
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
	/// Request timeout duration
	pub duration: Duration,
}

impl TimeoutConfig {
	/// Create a new timeout configuration
	///
	/// # Arguments
	///
	/// * `duration` - Timeout duration
	///
	/// # Examples
	///
	/// ```
	/// use std::time::Duration;
	/// use reinhardt_micro::middleware_config::TimeoutConfig;
	///
	/// let config = TimeoutConfig::new(Duration::from_secs(30));
	/// assert_eq!(config.duration, Duration::from_secs(30));
	/// ```
	pub fn new(duration: Duration) -> Self {
		Self { duration }
	}
}

impl Default for TimeoutConfig {
	fn default() -> Self {
		Self {
			duration: Duration::from_secs(30),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_logging_config_default() {
		let config = LoggingConfig::default();
		assert!(config.log_requests);
		assert!(config.log_responses);
		assert_eq!(config.request_log_level, LogLevel::Info);
	}

	#[test]
	fn test_logging_config_quiet() {
		let config = LoggingConfig::quiet();
		assert_eq!(config.request_log_level, LogLevel::Error);
		assert_eq!(config.response_log_level, LogLevel::Error);
	}

	#[test]
	fn test_logging_config_verbose() {
		let config = LoggingConfig::verbose();
		assert_eq!(config.request_log_level, LogLevel::Debug);
		assert_eq!(config.response_log_level, LogLevel::Debug);
	}

	#[test]
	#[cfg(feature = "cors")]
	fn test_cors_config_permissive() {
		let config = cors::permissive();
		assert_eq!(config.allow_origins, vec!["*"]);
		assert!(config.allow_methods.contains(&"GET".to_string()));
		assert!(config.allow_methods.contains(&"POST".to_string()));
	}

	#[test]
	#[cfg(feature = "cors")]
	fn test_cors_config_restrictive() {
		let config = cors::restrictive("https://example.com");
		assert_eq!(config.allow_origins, vec!["https://example.com"]);
		assert_eq!(config.allow_credentials, true);
		assert_eq!(config.max_age, Some(3600));
	}

	#[test]
	#[cfg(feature = "compression")]
	fn test_compression_config_for_json() {
		let config = compression::for_json();
		assert_eq!(config.min_length, 512);
		assert_eq!(config.compression_level, 6);
		assert!(
			config
				.compressible_types
				.contains(&"application/json".to_string())
		);
	}

	#[test]
	#[cfg(feature = "rate-limit")]
	fn test_rate_limit_config_lenient() {
		let config = rate_limit::lenient();
		assert_eq!(config.capacity, 100.0);
	}

	#[test]
	#[cfg(feature = "rate-limit")]
	fn test_rate_limit_config_strict() {
		let config = rate_limit::strict();
		assert_eq!(config.capacity, 10.0);
		assert!(config.error_message.is_some());
	}

	#[test]
	fn test_metrics_config_with_endpoint() {
		let config = metrics::with_endpoint("/custom-metrics");
		assert_eq!(config.metrics_endpoint, "/custom-metrics");
		assert!(config.track_response_time);
	}

	#[test]
	fn test_timeout_config_new() {
		let config = TimeoutConfig::new(Duration::from_secs(60));
		assert_eq!(config.duration, Duration::from_secs(60));
	}

	#[test]
	fn test_timeout_config_default() {
		let config = TimeoutConfig::default();
		assert_eq!(config.duration, Duration::from_secs(30));
	}
}
