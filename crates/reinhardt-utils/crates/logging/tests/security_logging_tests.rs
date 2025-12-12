//! Security Logging Integration Tests
//!
//! Tests for logging security-related events and exceptions.
//! Based on Django's security logging tests.

use reinhardt_logging::handlers::MemoryHandler;
use reinhardt_logging::{LogLevel, Logger};
use std::sync::Arc;

/// Security exception types for testing
#[derive(Debug)]
pub enum SecurityError {
	SuspiciousOperation(String),
	DisallowedHost(String),
	SuspiciousFileOperation(String),
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
		}
	}
}

impl std::error::Error for SecurityError {}

/// Security logger wrapper that logs security events
pub struct SecurityLogger {
	logger: Arc<Logger>,
}

impl SecurityLogger {
	pub fn new(logger: Arc<Logger>) -> Self {
		Self { logger }
	}

	pub async fn log_security_error(&self, error: &SecurityError) {
		self.logger
			.error(format!("Security Error: {}", error))
			.await;
	}

	pub async fn log_disallowed_host(&self, host: &str, request_path: &str) {
		self.logger
			.error(format!(
				"Invalid HTTP_HOST header: '{}'. You may need to add '{}' to ALLOWED_HOSTS. Request path: {}",
				host, host, request_path
			))
			.await;
	}

	pub async fn log_suspicious_file_operation(&self, operation: &str, path: &str) {
		self.logger
			.error(format!(
				"Attempted access to '{}' denied. Operation: {}",
				path, operation
			))
			.await;
	}
}

#[allow(dead_code)]
async fn test_suspicious_operation_logged() {
	// SuspiciousOperation should be logged at ERROR level
	let logger = Arc::new(Logger::new("reinhardt.security".to_string()));
	let handler = MemoryHandler::new(LogLevel::Error);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Error).await;

	let sec_logger = SecurityLogger::new(logger.clone());
	let error = SecurityError::SuspiciousOperation("Invalid data in request".to_string());

	sec_logger.log_security_error(&error).await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Error);
	assert_eq!(
		records[0].message,
		"Security Error: SuspiciousOperation: Invalid data in request"
	);
}

#[allow(dead_code)]
async fn test_disallowed_host_logged() {
	// DisallowedHost errors should be logged with helpful message
	let logger = Arc::new(Logger::new("reinhardt.security.DisallowedHost".to_string()));
	let handler = MemoryHandler::new(LogLevel::Error);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Error).await;

	let sec_logger = SecurityLogger::new(logger.clone());
	sec_logger.log_disallowed_host("evil.com", "/admin/").await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(
		records[0].message,
		"Invalid HTTP_HOST header: 'evil.com'. You may need to add 'evil.com' to ALLOWED_HOSTS. Request path: /admin/"
	);
}

#[allow(dead_code)]
async fn test_suspicious_file_operation_logged() {
	// Suspicious file operations should be logged
	let logger = Arc::new(Logger::new(
		"reinhardt.security.SuspiciousFileOperation".to_string(),
	));
	let handler = MemoryHandler::new(LogLevel::Error);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Error).await;

	let sec_logger = SecurityLogger::new(logger.clone());
	sec_logger
		.log_suspicious_file_operation("read", "../../../etc/passwd")
		.await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(
		records[0].message,
		"Attempted access to '../../../etc/passwd' denied. Operation: read"
	);
}

#[allow(dead_code)]
async fn test_security_logger_separation() {
	// Security logger should be separate from main logger
	let main_logger = Arc::new(Logger::new("myapp.main".to_string()));
	let main_handler = MemoryHandler::new(LogLevel::Info);
	let main_memory = main_handler.clone();
	main_logger.add_handler(Arc::new(main_handler)).await;
	main_logger.set_level(LogLevel::Info).await;

	let sec_logger_inner = Arc::new(Logger::new("myapp.security".to_string()));
	let sec_handler = MemoryHandler::new(LogLevel::Error);
	let sec_memory = sec_handler.clone();
	sec_logger_inner.add_handler(Arc::new(sec_handler)).await;
	sec_logger_inner.set_level(LogLevel::Error).await;

	let sec_logger = SecurityLogger::new(sec_logger_inner.clone());

	// Log to main logger
	main_logger.info("Normal operation".to_string()).await;

	// Log security event
	let error = SecurityError::SuspiciousOperation("Attack detected".to_string());
	sec_logger.log_security_error(&error).await;

	// Each logger should have its own records
	let main_records = main_memory.get_records();
	let sec_records = sec_memory.get_records();

	assert_eq!(main_records.len(), 1);
	assert_eq!(sec_records.len(), 1);
	assert_eq!(main_records[0].message, "Normal operation");
	assert_eq!(
		sec_records[0].message,
		"Security Error: SuspiciousOperation: Attack detected"
	);
}

#[allow(dead_code)]
async fn test_multiple_security_violations() {
	// Multiple security violations should all be logged
	let logger = Arc::new(Logger::new("reinhardt.security".to_string()));
	let handler = MemoryHandler::new(LogLevel::Error);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Error).await;

	let sec_logger = SecurityLogger::new(logger.clone());

	// Log multiple violations
	sec_logger.log_disallowed_host("badhost1.com", "/").await;
	sec_logger
		.log_disallowed_host("badhost2.com", "/admin/")
		.await;
	sec_logger
		.log_suspicious_file_operation("write", "/etc/shadow")
		.await;

	let error = SecurityError::SuspiciousOperation("CSRF token missing".to_string());
	sec_logger.log_security_error(&error).await;

	let records = memory.get_records();
	assert_eq!(records.len(), 4);
	assert_eq!(
		records[0].message,
		"Invalid HTTP_HOST header: 'badhost1.com'. You may need to add 'badhost1.com' to ALLOWED_HOSTS. Request path: /"
	);
	assert_eq!(
		records[1].message,
		"Invalid HTTP_HOST header: 'badhost2.com'. You may need to add 'badhost2.com' to ALLOWED_HOSTS. Request path: /admin/"
	);
	assert_eq!(
		records[2].message,
		"Attempted access to '/etc/shadow' denied. Operation: write"
	);
	assert_eq!(
		records[3].message,
		"Security Error: SuspiciousOperation: CSRF token missing"
	);
}

#[tokio::test]
async fn test_security_logger_with_different_levels() {
	// Security logger should respect different log levels
	let logger = Arc::new(Logger::new("reinhardt.security".to_string()));
	let handler = MemoryHandler::new(LogLevel::Debug);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Debug).await;

	let sec_logger = SecurityLogger::new(logger.clone());

	// Log at ERROR level (should appear)
	let error = SecurityError::SuspiciousOperation("Test".to_string());
	sec_logger.log_security_error(&error).await;

	// If we had lower-severity security events, they would also be logged
	// TODO: For now, just verify ERROR level works

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Error);
}

#[allow(dead_code)]
async fn test_security_error_types() {
	// Test all security error types can be logged
	let logger = Arc::new(Logger::new("reinhardt.security".to_string()));
	let handler = MemoryHandler::new(LogLevel::Error);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Error).await;

	let sec_logger = SecurityLogger::new(logger.clone());

	// Test each error type
	let errors = vec![
		SecurityError::SuspiciousOperation("Test operation".to_string()),
		SecurityError::DisallowedHost("test.com".to_string()),
		SecurityError::SuspiciousFileOperation("/test/path".to_string()),
	];

	for error in errors {
		sec_logger.log_security_error(&error).await;
	}

	let records = memory.get_records();
	assert_eq!(records.len(), 3);
	assert_eq!(
		records[0].message,
		"Security Error: SuspiciousOperation: Test operation"
	);
	assert_eq!(
		records[1].message,
		"Security Error: DisallowedHost: test.com"
	);
	assert_eq!(
		records[2].message,
		"Security Error: SuspiciousFileOperation: /test/path"
	);
}
