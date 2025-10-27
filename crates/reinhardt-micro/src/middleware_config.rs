//! Middleware Configuration Helpers
//!
//! This module provides convenient builder-style methods for configuring middleware
//! in reinhardt-micro applications. It wraps the underlying reinhardt-middleware
//! configurations with a more ergonomic API.

use std::time::Duration;

// Re-export configuration types from reinhardt-middleware
pub use reinhardt_middleware::cors::CorsConfig;
pub use reinhardt_middleware::gzip::GZipConfig as CompressionConfig;
pub use reinhardt_middleware::metrics::MetricsConfig;
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

impl CorsConfig {
    /// Create a permissive CORS configuration (allows all origins)
    ///
    /// This is useful for development but should be used with caution in production.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_micro::middleware_config::CorsConfig;
    ///
    /// let config = CorsConfig::permissive();
    /// assert_eq!(config.allow_origins, vec!["*"]);
    /// assert!(config.allow_methods.contains(&"GET".to_string()));
    /// ```
    pub fn permissive() -> Self {
        Self::default()
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
    /// use reinhardt_micro::middleware_config::CorsConfig;
    ///
    /// let config = CorsConfig::restrictive("https://example.com");
    /// assert_eq!(config.allow_origins, vec!["https://example.com"]);
    /// assert_eq!(config.allow_credentials, true);
    /// ```
    pub fn restrictive(allowed_origin: &str) -> Self {
        Self {
            allow_origins: vec![allowed_origin.to_string()],
            allow_methods: vec!["GET".to_string(), "POST".to_string()],
            allow_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
            allow_credentials: true,
            max_age: Some(3600),
        }
    }
}

impl CompressionConfig {
    /// Create compression config optimized for JSON APIs
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_micro::middleware_config::CompressionConfig;
    ///
    /// let config = CompressionConfig::for_json();
    /// assert!(config.compressible_types.contains(&"application/json".to_string()));
    /// ```
    pub fn for_json() -> Self {
        Self {
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

impl RateLimitConfig {
    /// Create a lenient rate limit configuration (100 requests per minute)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_micro::middleware_config::RateLimitConfig;
    ///
    /// let config = RateLimitConfig::lenient();
    /// assert_eq!(config.capacity, 100.0);
    /// ```
    pub fn lenient() -> Self {
        Self {
            strategy: reinhardt_middleware::rate_limit::RateLimitStrategy::TokenBucket,
            capacity: 100.0,
            refill_rate: 100.0 / 60.0,
            cost_per_request: 1.0,
            exclude_paths: vec![],
            error_message: None,
        }
    }

    /// Create a strict rate limit configuration (10 requests per minute)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_micro::middleware_config::RateLimitConfig;
    ///
    /// let config = RateLimitConfig::strict();
    /// assert_eq!(config.capacity, 10.0);
    /// ```
    pub fn strict() -> Self {
        Self {
            strategy: reinhardt_middleware::rate_limit::RateLimitStrategy::TokenBucket,
            capacity: 10.0,
            refill_rate: 10.0 / 60.0,
            cost_per_request: 1.0,
            exclude_paths: vec![],
            error_message: Some("Rate limit exceeded. Please try again later.".to_string()),
        }
    }
}

impl MetricsConfig {
    /// Create metrics config with custom endpoint
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Endpoint path for metrics (e.g., "/metrics")
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_micro::middleware_config::MetricsConfig;
    ///
    /// let config = MetricsConfig::with_endpoint("/custom-metrics");
    /// assert_eq!(config.metrics_endpoint, "/custom-metrics");
    /// assert!(config.track_response_time);
    /// ```
    pub fn with_endpoint(endpoint: &str) -> Self {
        Self {
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
    fn test_cors_config_permissive() {
        let config = CorsConfig::permissive();
        assert_eq!(config.allow_origins, vec!["*"]);
        assert!(config.allow_methods.contains(&"GET".to_string()));
        assert!(config.allow_methods.contains(&"POST".to_string()));
    }

    #[test]
    fn test_cors_config_restrictive() {
        let config = CorsConfig::restrictive("https://example.com");
        assert_eq!(config.allow_origins, vec!["https://example.com"]);
        assert_eq!(config.allow_credentials, true);
        assert_eq!(config.max_age, Some(3600));
    }

    #[test]
    fn test_compression_config_for_json() {
        let config = CompressionConfig::for_json();
        assert_eq!(config.min_length, 512);
        assert_eq!(config.compression_level, 6);
        assert!(config.compressible_types.contains(&"application/json".to_string()));
    }

    #[test]
    fn test_rate_limit_config_lenient() {
        let config = RateLimitConfig::lenient();
        assert_eq!(config.capacity, 100.0);
        assert_eq!(config.refill_rate, 100.0 / 60.0);
    }

    #[test]
    fn test_rate_limit_config_strict() {
        let config = RateLimitConfig::strict();
        assert_eq!(config.capacity, 10.0);
        assert!(config.error_message.is_some());
    }

    #[test]
    fn test_metrics_config_with_endpoint() {
        let config = MetricsConfig::with_endpoint("/custom-metrics");
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
