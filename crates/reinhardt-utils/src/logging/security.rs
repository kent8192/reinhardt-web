//! Security event logging module
//!
//! Provides specialized logging for security-related events with
//! appropriate log levels based on event severity.
//!
//! # Event Severity Mapping
//!
//! - **ERROR**: Security violations (disallowed host, CSRF, file attacks)
//! - **WARNING**: Authentication failures, rate limiting, permission denied
//! - **INFO**: Successful authentication, session events
//!
//! # Examples
//!
//! ```
//! use reinhardt_utils::logging::{Logger, SecurityLogger};
//! use std::sync::Arc;
//!
//! # async fn example() {
//! let logger = Arc::new(Logger::new("security".to_string()));
//! let security_logger = SecurityLogger::new(logger);
//!
//! // Log authentication event
//! security_logger.log_auth_event("admin", true, Some("192.168.1.1")).await;
//!
//! // Log security violation
//! security_logger.log_disallowed_host("evil.com", "/admin/").await;
//! # }
//! ```

use super::logger::Logger;
use std::sync::Arc;

/// Security error types for categorized logging
#[derive(Debug, Clone)]
pub enum SecurityError {
	/// Suspicious operation detected (e.g., CSRF violation, invalid data)
	SuspiciousOperation(String),
	/// Disallowed host in request
	DisallowedHost(String),
	/// Suspicious file system operation attempt
	SuspiciousFileOperation(String),
	/// Authentication failure
	AuthenticationFailed(String),
	/// Authorization denied (permission error)
	AuthorizationDenied(String),
	/// Rate limit exceeded
	RateLimitExceeded(String),
	/// CSRF token validation failure
	CsrfViolation(String),
}

impl std::fmt::Display for SecurityError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SecurityError::SuspiciousOperation(msg) => {
				write!(f, "SuspiciousOperation: {}", msg)
			}
			SecurityError::DisallowedHost(host) => write!(f, "DisallowedHost: {}", host),
			SecurityError::SuspiciousFileOperation(path) => {
				write!(f, "SuspiciousFileOperation: {}", path)
			}
			SecurityError::AuthenticationFailed(reason) => {
				write!(f, "AuthenticationFailed: {}", reason)
			}
			SecurityError::AuthorizationDenied(reason) => {
				write!(f, "AuthorizationDenied: {}", reason)
			}
			SecurityError::RateLimitExceeded(identifier) => {
				write!(f, "RateLimitExceeded: {}", identifier)
			}
			SecurityError::CsrfViolation(details) => {
				write!(f, "CsrfViolation: {}", details)
			}
		}
	}
}

impl std::error::Error for SecurityError {}

/// Security logger for logging security-related events
///
/// Provides methods for logging various security events at appropriate
/// log levels based on severity.
///
/// # Log Level Guidelines
///
/// | Event Type | Log Level | Example |
/// |------------|-----------|---------|
/// | Security violations | ERROR | Disallowed host, CSRF, file attacks |
/// | Auth failures | WARNING | Failed login, permission denied |
/// | Rate limiting | WARNING | Request quota exceeded |
/// | Successful auth | INFO | User login, session start |
pub struct SecurityLogger {
	logger: Arc<Logger>,
}

impl SecurityLogger {
	/// Create a new security logger
	pub fn new(logger: Arc<Logger>) -> Self {
		Self { logger }
	}

	/// Log a security error at ERROR level
	///
	/// Use for serious security violations that require immediate attention.
	pub async fn log_security_error(&self, error: &SecurityError) {
		self.logger
			.error(format!("Security Error: {}", error))
			.await;
	}

	/// Log a security warning at WARNING level
	///
	/// Use for security events that are concerning but not critical.
	pub async fn log_security_warning(&self, message: &str) {
		self.logger
			.warning(format!("Security Warning: {}", message))
			.await;
	}

	/// Log a security info message at INFO level
	///
	/// Use for informational security events like successful authentication.
	pub async fn log_security_info(&self, message: &str) {
		self.logger
			.info(format!("Security Info: {}", message))
			.await;
	}

	/// Log an authentication event with appropriate level
	///
	/// - Success: INFO level
	/// - Failure: WARNING level
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_utils::logging::{Logger, SecurityLogger};
	/// use std::sync::Arc;
	///
	/// # async fn example() {
	/// let logger = Arc::new(Logger::new("security".to_string()));
	/// let security_logger = SecurityLogger::new(logger);
	///
	/// // Successful login
	/// security_logger.log_auth_event("admin", true, Some("192.168.1.1")).await;
	///
	/// // Failed login attempt
	/// security_logger.log_auth_event("hacker", false, Some("10.0.0.1")).await;
	/// # }
	/// ```
	pub async fn log_auth_event(&self, user: &str, success: bool, ip: Option<&str>) {
		let ip_str = ip.unwrap_or("unknown");
		if success {
			self.logger
				.info(format!(
					"Authentication success for user '{}' from IP {}",
					user, ip_str
				))
				.await;
		} else {
			self.logger
				.warning(format!(
					"Authentication failed for user '{}' from IP {}",
					user, ip_str
				))
				.await;
		}
	}

	/// Log a disallowed host attempt at ERROR level
	///
	/// Based on Django's `DisallowedHost` exception logging.
	pub async fn log_disallowed_host(&self, host: &str, request_path: &str) {
		self.logger
			.error(format!(
				"Invalid HTTP_HOST header: '{}'. You may need to add '{}' to ALLOWED_HOSTS. Request path: {}",
				host, host, request_path
			))
			.await;
	}

	/// Log a suspicious file operation attempt at ERROR level
	pub async fn log_suspicious_file_operation(&self, operation: &str, path: &str) {
		self.logger
			.error(format!(
				"Attempted access to '{}' denied. Operation: {}",
				path, operation
			))
			.await;
	}

	/// Log rate limit exceeded at WARNING level
	pub async fn log_rate_limit_exceeded(&self, identifier: &str, limit: u32) {
		self.logger
			.warning(format!(
				"Rate limit exceeded for '{}'. Limit: {} requests",
				identifier, limit
			))
			.await;
	}

	/// Log CSRF violation at ERROR level
	pub async fn log_csrf_violation(&self, request_path: &str) {
		self.logger
			.error(format!(
				"CSRF validation failed for request: {}",
				request_path
			))
			.await;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::logging::handlers::MemoryHandler;
	use crate::logging::LogLevel;

	#[tokio::test]
	async fn test_security_error_display() {
		let errors = vec![
			(
				SecurityError::SuspiciousOperation("test".to_string()),
				"SuspiciousOperation: test",
			),
			(
				SecurityError::DisallowedHost("evil.com".to_string()),
				"DisallowedHost: evil.com",
			),
			(
				SecurityError::AuthenticationFailed("bad password".to_string()),
				"AuthenticationFailed: bad password",
			),
			(
				SecurityError::RateLimitExceeded("user123".to_string()),
				"RateLimitExceeded: user123",
			),
			(
				SecurityError::CsrfViolation("missing token".to_string()),
				"CsrfViolation: missing token",
			),
		];

		for (error, expected) in errors {
			assert_eq!(error.to_string(), expected);
		}
	}

	#[tokio::test]
	async fn test_auth_event_success_logged_at_info() {
		let logger = Arc::new(Logger::new("security".to_string()));
		let handler = MemoryHandler::new(LogLevel::Debug);
		let memory = handler.clone();

		logger.add_handler(Arc::new(handler)).await;
		logger.set_level(LogLevel::Debug).await;

		let security_logger = SecurityLogger::new(logger);
		security_logger
			.log_auth_event("admin", true, Some("192.168.1.1"))
			.await;

		let records = memory.get_records();
		assert_eq!(records.len(), 1);
		assert_eq!(records[0].level, LogLevel::Info);
		assert!(
			records[0]
				.message
				.contains("Authentication success for user 'admin'")
		);
	}

	#[tokio::test]
	async fn test_auth_event_failure_logged_at_warning() {
		let logger = Arc::new(Logger::new("security".to_string()));
		let handler = MemoryHandler::new(LogLevel::Debug);
		let memory = handler.clone();

		logger.add_handler(Arc::new(handler)).await;
		logger.set_level(LogLevel::Debug).await;

		let security_logger = SecurityLogger::new(logger);
		security_logger
			.log_auth_event("hacker", false, Some("10.0.0.1"))
			.await;

		let records = memory.get_records();
		assert_eq!(records.len(), 1);
		assert_eq!(records[0].level, LogLevel::Warning);
		assert!(
			records[0]
				.message
				.contains("Authentication failed for user 'hacker'")
		);
	}

	#[tokio::test]
	async fn test_rate_limit_logged_at_warning() {
		let logger = Arc::new(Logger::new("security".to_string()));
		let handler = MemoryHandler::new(LogLevel::Debug);
		let memory = handler.clone();

		logger.add_handler(Arc::new(handler)).await;
		logger.set_level(LogLevel::Debug).await;

		let security_logger = SecurityLogger::new(logger);
		security_logger
			.log_rate_limit_exceeded("user123", 100)
			.await;

		let records = memory.get_records();
		assert_eq!(records.len(), 1);
		assert_eq!(records[0].level, LogLevel::Warning);
		assert!(records[0].message.contains("Rate limit exceeded"));
		assert!(records[0].message.contains("100"));
	}

	#[tokio::test]
	async fn test_csrf_violation_logged_at_error() {
		let logger = Arc::new(Logger::new("security".to_string()));
		let handler = MemoryHandler::new(LogLevel::Debug);
		let memory = handler.clone();

		logger.add_handler(Arc::new(handler)).await;
		logger.set_level(LogLevel::Debug).await;

		let security_logger = SecurityLogger::new(logger);
		security_logger.log_csrf_violation("/api/transfer").await;

		let records = memory.get_records();
		assert_eq!(records.len(), 1);
		assert_eq!(records[0].level, LogLevel::Error);
		assert!(records[0].message.contains("CSRF validation failed"));
		assert!(records[0].message.contains("/api/transfer"));
	}
}
