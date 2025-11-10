//! Tests for HTTP request/response logging
//! Based on Django's HandlerLoggingTests and LogResponseRealLoggerTests

use reinhardt_logging::{
	LogLevel, LogRecord, Logger, escape_control_chars, handlers::MemoryHandler,
};
use std::sync::Arc;

/// Log a response with status code
async fn log_response(logger: &Logger, message: &str, status_code: u16, path: &str) {
	let mut record = LogRecord::new(
		if status_code >= 500 {
			LogLevel::Error
		} else if status_code >= 400 {
			LogLevel::Warning
		} else {
			LogLevel::Info
		},
		"reinhardt.request".to_string(),
		message.to_string(),
	);

	record
		.extra
		.insert("status_code".to_string(), serde_json::json!(status_code));
	record
		.extra
		.insert("path".to_string(), serde_json::json!(path));

	logger.log_record(&record).await;
}

#[tokio::test]
async fn test_page_not_found_warning() {
	let logger = Logger::new("reinhardt.request".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger.add_handler(handler_clone).await;
	logger.set_level(LogLevel::Debug).await;

	log_response(
		&logger,
		"Not Found: /does_not_exist/",
		404,
		"/does_not_exist/",
	)
	.await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Warning);
	assert_eq!(records[0].message, "Not Found: /does_not_exist/");

	let status_code = records[0].extra.get("status_code").and_then(|v| v.as_u64());
	assert_eq!(status_code, Some(404));
}

#[allow(dead_code)]
async fn test_control_chars_escaped() {
	let logger = Logger::new("reinhardt.request".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger.add_handler(handler_clone).await;
	logger.set_level(LogLevel::Debug).await;

	// Use actual ESC character (\x1b) instead of URL-encoded version
	let path = "/\x1b[1;31mNOW IN RED!!!1B[0m/";
	let escaped_path = escape_control_chars(path);
	let message = format!("Not Found: {}", escaped_path);

	log_response(&logger, &message, 404, path).await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	// Check that the control characters were properly escaped
	assert_eq!(
		records[0].message,
		"Not Found: /\\x1b[1;31mNOW IN RED!!!1B[0m/"
	);
}

#[allow(dead_code)]
async fn test_permission_denied() {
	let logger = Logger::new("reinhardt.request".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger.add_handler(handler_clone).await;
	logger.set_level(LogLevel::Debug).await;

	log_response(
		&logger,
		"Forbidden (Permission denied): /permission_denied/",
		403,
		"/permission_denied/",
	)
	.await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Warning);
	assert_eq!(
		records[0].message,
		"Forbidden (Permission denied): /permission_denied/"
	);
}

#[allow(dead_code)]
async fn test_internal_server_error() {
	let logger = Logger::new("reinhardt.request".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger.add_handler(handler_clone).await;
	logger.set_level(LogLevel::Debug).await;

	log_response(
		&logger,
		"Internal Server Error: /internal_server_error/",
		500,
		"/internal_server_error/",
	)
	.await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Error);
	assert_eq!(
		records[0].message,
		"Internal Server Error: /internal_server_error/"
	);
}

#[tokio::test]
async fn test_internal_server_error_599() {
	let logger = Logger::new("reinhardt.request".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger.add_handler(handler_clone).await;
	logger.set_level(LogLevel::Debug).await;

	log_response(
		&logger,
		"Unknown Status Code: /internal_server_error/",
		599,
		"/internal_server_error/",
	)
	.await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Error);

	let status_code = records[0].extra.get("status_code").and_then(|v| v.as_u64());
	assert_eq!(status_code, Some(599));
}

#[tokio::test]
async fn test_logs_5xx_as_error() {
	let logger = Logger::new("reinhardt.request".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger.add_handler(handler_clone).await;
	logger.set_level(LogLevel::Debug).await;

	log_response(&logger, "Server error occurred", 508, "/test-path/").await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Error);
	assert_eq!(records[0].message, "Server error occurred");
}

#[tokio::test]
async fn test_logs_4xx_as_warning() {
	let logger = Logger::new("reinhardt.request".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger.add_handler(handler_clone).await;
	logger.set_level(LogLevel::Debug).await;

	log_response(&logger, "This is a teapot!", 418, "/test-path/").await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Warning);
	assert_eq!(records[0].message, "This is a teapot!");
}

#[tokio::test]
async fn test_logs_2xx_as_info() {
	let logger = Logger::new("reinhardt.request".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger.add_handler(handler_clone).await;
	logger.set_level(LogLevel::Debug).await;

	log_response(&logger, "OK response", 201, "/test-path/").await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Info);
	assert_eq!(records[0].message, "OK response");
}

#[allow(dead_code)]
async fn test_unicode_paths() {
	let logger = Logger::new("reinhardt.request".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger.add_handler(handler_clone).await;
	logger.set_level(LogLevel::Debug).await;

	let path = "/café/test/路径";
	let escaped = escape_control_chars(path);
	let message = format!("Not Found: {}", escaped);

	log_response(&logger, &message, 404, path).await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	// Unicode should be escaped - verify exact escaped format
	assert_eq!(
		records[0].message,
		"Not Found: /caf\\xe9/test/\\xe8\\xb7\\xaf\\xe5\\xbe\\x84"
	);
}

#[tokio::test]
async fn test_multi_part_parser_error() {
	let logger = Logger::new("reinhardt.request".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger.add_handler(handler_clone).await;
	logger.set_level(LogLevel::Debug).await;

	log_response(
		&logger,
		"Bad request (Unable to parse request body): /multi_part_parser_error/",
		400,
		"/multi_part_parser_error/",
	)
	.await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Warning);
	assert!(records[0].message.contains("Bad request"));
	assert!(records[0].message.contains("Unable to parse request body"));
}

#[tokio::test]
async fn test_format_args_are_applied() {
	let logger = Logger::new("reinhardt.request".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger.add_handler(handler_clone).await;
	logger.set_level(LogLevel::Debug).await;

	let message = format!("Something went wrong: {} ({})", "DB error", 42);
	log_response(&logger, &message, 500, "/test/").await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].message, "Something went wrong: DB error (42)");
	assert_eq!(records[0].level, LogLevel::Error);
}
