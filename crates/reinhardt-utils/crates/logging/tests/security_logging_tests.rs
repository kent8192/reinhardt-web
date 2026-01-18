//! Security Logging Integration Tests
//!
//! Tests for logging security-related events and exceptions.
//! Based on Django's security logging tests.

use reinhardt_utils::logging::handlers::MemoryHandler;
use reinhardt_utils::logging::{LogLevel, Logger, SecurityError, SecurityLogger};
use std::sync::Arc;

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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
	// Security logger should log at appropriate levels based on event type
	let logger = Arc::new(Logger::new("reinhardt.security".to_string()));
	let handler = MemoryHandler::new(LogLevel::Debug);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Debug).await;

	let sec_logger = SecurityLogger::new(logger.clone());

	// Log at ERROR level (security violation)
	let error = SecurityError::SuspiciousOperation("Test".to_string());
	sec_logger.log_security_error(&error).await;

	// Log at WARNING level (auth failure)
	sec_logger
		.log_auth_event("hacker", false, Some("10.0.0.1"))
		.await;

	// Log at INFO level (successful auth)
	sec_logger
		.log_auth_event("admin", true, Some("192.168.1.1"))
		.await;

	let records = memory.get_records();
	assert_eq!(records.len(), 3);
	assert_eq!(records[0].level, LogLevel::Error);
	assert_eq!(records[1].level, LogLevel::Warning);
	assert_eq!(records[2].level, LogLevel::Info);
}

#[tokio::test]
async fn test_security_info_filtered_by_level() {
	// INFO level security events should be filtered when logger level is WARNING
	let logger = Arc::new(Logger::new("reinhardt.security".to_string()));
	let handler = MemoryHandler::new(LogLevel::Warning);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Warning).await;

	let sec_logger = SecurityLogger::new(logger.clone());

	// Log at INFO level (should be filtered)
	sec_logger
		.log_auth_event("admin", true, Some("192.168.1.1"))
		.await;

	// Log at WARNING level (should appear)
	sec_logger
		.log_auth_event("hacker", false, Some("10.0.0.1"))
		.await;

	// Log at ERROR level (should appear)
	sec_logger.log_csrf_violation("/api/transfer").await;

	let records = memory.get_records();
	assert_eq!(records.len(), 2);
	assert_eq!(records[0].level, LogLevel::Warning);
	assert_eq!(records[1].level, LogLevel::Error);
}

#[tokio::test]
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
		SecurityError::AuthenticationFailed("invalid password".to_string()),
		SecurityError::AuthorizationDenied("no permission".to_string()),
		SecurityError::RateLimitExceeded("user123".to_string()),
		SecurityError::CsrfViolation("missing token".to_string()),
	];

	for error in errors {
		sec_logger.log_security_error(&error).await;
	}

	let records = memory.get_records();
	assert_eq!(records.len(), 7);
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
	assert_eq!(
		records[3].message,
		"Security Error: AuthenticationFailed: invalid password"
	);
	assert_eq!(
		records[4].message,
		"Security Error: AuthorizationDenied: no permission"
	);
	assert_eq!(
		records[5].message,
		"Security Error: RateLimitExceeded: user123"
	);
	assert_eq!(
		records[6].message,
		"Security Error: CsrfViolation: missing token"
	);
}

#[tokio::test]
async fn test_rate_limit_exceeded_logged() {
	// Rate limit exceeded should be logged at WARNING level
	let logger = Arc::new(Logger::new("reinhardt.security".to_string()));
	let handler = MemoryHandler::new(LogLevel::Debug);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Debug).await;

	let sec_logger = SecurityLogger::new(logger.clone());
	sec_logger
		.log_rate_limit_exceeded("192.168.1.100", 1000)
		.await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Warning);
	assert!(
		records[0]
			.message
			.contains("Rate limit exceeded for '192.168.1.100'")
	);
	assert!(records[0].message.contains("1000"));
}

#[tokio::test]
async fn test_csrf_violation_logged() {
	// CSRF violation should be logged at ERROR level
	let logger = Arc::new(Logger::new("reinhardt.security".to_string()));
	let handler = MemoryHandler::new(LogLevel::Debug);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Debug).await;

	let sec_logger = SecurityLogger::new(logger.clone());
	sec_logger.log_csrf_violation("/api/bank/transfer").await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Error);
	assert!(
		records[0]
			.message
			.contains("CSRF validation failed for request")
	);
	assert!(records[0].message.contains("/api/bank/transfer"));
}

#[tokio::test]
async fn test_auth_event_with_unknown_ip() {
	// Auth event should handle unknown IP gracefully
	let logger = Arc::new(Logger::new("reinhardt.security".to_string()));
	let handler = MemoryHandler::new(LogLevel::Debug);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Debug).await;

	let sec_logger = SecurityLogger::new(logger.clone());
	sec_logger.log_auth_event("admin", true, None).await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert!(records[0].message.contains("from IP unknown"));
}
