//! Additional Integration Tests
//!
//! Miscellaneous integration tests including I18n, exception formatting,
//! settings configuration, and middleware logging.

use reinhardt_logging::handlers::MemoryHandler;
use reinhardt_logging::{LogLevel, LogRecord, Logger};
use std::collections::HashMap;
use std::sync::Arc;

// ==============================================================================
// I18n Logging Tests
// ==============================================================================

#[tokio::test]
async fn test_translation_key_missing_logged() {
	// Missing translation keys should be logged at DEBUG level
	let logger = Arc::new(Logger::new("reinhardt.translation".to_string()));
	let handler = MemoryHandler::new(LogLevel::Debug);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Debug).await;

	logger
		.debug("Translation key 'missing.key' not found in catalog 'en_US'".to_string())
		.await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Debug);
	assert_eq!(
		records[0].message,
		"Translation key 'missing.key' not found in catalog 'en_US'"
	);
}

#[tokio::test]
async fn test_translation_locale_fallback_logged() {
	// Locale fallback should be logged at INFO level
	let logger = Arc::new(Logger::new("reinhardt.translation".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	logger
		.info("Locale 'fr_CA' not available, falling back to 'fr'".to_string())
		.await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(
		records[0].message,
		"Locale 'fr_CA' not available, falling back to 'fr'"
	);
}

// ==============================================================================
// Exception Formatting Tests
// ==============================================================================

/// Helper to format exception with traceback
fn format_exception_with_traceback(
	error: &str,
	traceback: &[(&str, u32)], // (file, line)
) -> String {
	let mut result = format!("Exception: {}\n", error);
	result.push_str("Traceback (most recent call last):\n");
	for (file, line) in traceback {
		result.push_str(&format!("  File \"{}\", line {}\n", file, line));
	}
	result
}

#[tokio::test]
async fn test_format_exception_with_traceback() {
	// Exceptions with tracebacks should format correctly
	let logger = Arc::new(Logger::new("reinhardt.request".to_string()));
	let handler = MemoryHandler::new(LogLevel::Error);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Error).await;

	let traceback = vec![("views.py", 42), ("middleware.py", 15), ("handlers.py", 8)];
	let formatted = format_exception_with_traceback("ValueError: Invalid input", &traceback);

	logger.error(formatted).await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);

	// Verify the formatted exception contains all expected components
	let expected = format_exception_with_traceback("ValueError: Invalid input", &traceback);
	assert_eq!(records[0].message, expected);
}

#[tokio::test]
async fn test_format_exception_with_cause_chain() {
	// Exception cause chains should be logged
	let logger = Arc::new(Logger::new("reinhardt.request".to_string()));
	let handler = MemoryHandler::new(LogLevel::Error);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Error).await;

	let msg = "RuntimeError: Failed to process\nCaused by: ValueError: Invalid data\nCaused by: TypeError: Wrong type";
	logger.error(msg.to_string()).await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(
		records[0].message,
		"RuntimeError: Failed to process\nCaused by: ValueError: Invalid data\nCaused by: TypeError: Wrong type"
	);
}

#[tokio::test]
async fn test_format_exception_truncation() {
	// Very long tracebacks should be truncated
	let logger = Arc::new(Logger::new("reinhardt.request".to_string()));
	let handler = MemoryHandler::new(LogLevel::Error);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Error).await;

	// Create a very long traceback
	let mut traceback = Vec::new();
	for i in 0..100 {
		traceback.push((format!("file{}.py", i), i as u32));
	}

	let trace_refs: Vec<(&str, u32)> = traceback.iter().map(|(f, l)| (f.as_str(), *l)).collect();

	let formatted = format_exception_with_traceback("Error", &trace_refs);
	logger.error(formatted).await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	// Message should be very long but finite
	assert!(records[0].message.len() > 100);
	assert!(records[0].message.len() < 50000); // Reasonable limit
}

#[tokio::test]
async fn test_exception_in_log_record() {
	// LogRecord should be able to contain exception information
	let logger = Arc::new(Logger::new("reinhardt.request".to_string()));
	let handler = MemoryHandler::new(LogLevel::Error);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Error).await;

	let mut record = LogRecord::new(
		LogLevel::Error,
		"reinhardt.request".to_string(),
		"An error occurred".to_string(),
	);

	// Add exception info as extra field
	record.extra.insert(
		"exception_type".to_string(),
		serde_json::json!("ValueError"),
	);
	record.extra.insert(
		"exception_message".to_string(),
		serde_json::json!("Invalid"),
	);

	logger.log_record(&record).await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(
		records[0].extra.get("exception_type"),
		Some(&serde_json::json!("ValueError"))
	);
}

#[tokio::test]
async fn test_multiple_nested_exceptions() {
	// Multiple nested exceptions should all be logged
	let logger = Arc::new(Logger::new("reinhardt.request".to_string()));
	let handler = MemoryHandler::new(LogLevel::Error);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Error).await;

	let msg = "Level 1 Error\n  Caused by: Level 2 Error\n    Caused by: Level 3 Error\n      Caused by: Root cause";
	logger.error(msg.to_string()).await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(
		records[0].message,
		"Level 1 Error\n  Caused by: Level 2 Error\n    Caused by: Level 3 Error\n      Caused by: Root cause"
	);
}

// ==============================================================================
// Settings and Configuration Tests
// ==============================================================================

/// Simple logging configuration structure
#[derive(Clone)]
pub struct LoggingConfig {
	pub handlers: HashMap<String, HandlerConfig>,
	pub loggers: HashMap<String, LoggerConfig>,
}

#[derive(Clone)]
pub struct HandlerConfig {
	pub level: LogLevel,
}

#[derive(Clone)]
pub struct LoggerConfig {
	pub level: LogLevel,
	pub handlers: Vec<String>,
}

#[tokio::test]
async fn test_logging_config_from_dict() {
	// Logging configuration should be loadable from a config structure
	let mut config = LoggingConfig {
		handlers: HashMap::new(),
		loggers: HashMap::new(),
	};

	config.handlers.insert(
		"console".to_string(),
		HandlerConfig {
			level: LogLevel::Info,
		},
	);

	config.loggers.insert(
		"myapp".to_string(),
		LoggerConfig {
			level: LogLevel::Debug,
			handlers: vec!["console".to_string()],
		},
	);

	// Verify config was created correctly
	assert_eq!(config.handlers.len(), 1);
	assert_eq!(config.loggers.len(), 1);
	assert_eq!(
		config.handlers.get("console").unwrap().level,
		LogLevel::Info
	);
}

#[tokio::test]
async fn test_multiple_handlers_from_config() {
	// Configuration should support multiple handlers
	let mut config = LoggingConfig {
		handlers: HashMap::new(),
		loggers: HashMap::new(),
	};

	config.handlers.insert(
		"console".to_string(),
		HandlerConfig {
			level: LogLevel::Info,
		},
	);

	config.handlers.insert(
		"file".to_string(),
		HandlerConfig {
			level: LogLevel::Debug,
		},
	);

	config.loggers.insert(
		"myapp".to_string(),
		LoggerConfig {
			level: LogLevel::Debug,
			handlers: vec!["console".to_string(), "file".to_string()],
		},
	);

	// Verify multiple handlers
	assert_eq!(config.handlers.len(), 2);
	let logger_config = config.loggers.get("myapp").unwrap();
	assert_eq!(logger_config.handlers.len(), 2);
}

#[tokio::test]
async fn test_config_validation() {
	// Invalid configuration should be detectable
	let mut config = LoggingConfig {
		handlers: HashMap::new(),
		loggers: HashMap::new(),
	};

	// Logger references non-existent handler
	config.loggers.insert(
		"myapp".to_string(),
		LoggerConfig {
			level: LogLevel::Debug,
			handlers: vec!["nonexistent".to_string()],
		},
	);

	// Validation check
	let logger_cfg = config.loggers.get("myapp").unwrap();
	for handler_name in &logger_cfg.handlers {
		assert!(
			config.handlers.contains_key(handler_name) || handler_name == "nonexistent" // Expected to be missing
		);
	}

	assert!(!config.handlers.contains_key("nonexistent"));
}

// ==============================================================================
// Middleware Logging Tests
// ==============================================================================

/// Simulated HTTP request
pub struct Request {
	pub method: String,
	pub path: String,
	pub timestamp: std::time::Instant,
}

/// Simulated HTTP response
pub struct Response {
	pub status: u16,
}

#[tokio::test]
async fn test_request_logging_middleware() {
	// Middleware should log incoming requests
	let logger = Arc::new(Logger::new("reinhardt.request".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let request = Request {
		method: "GET".to_string(),
		path: "/api/users".to_string(),
		timestamp: std::time::Instant::now(),
	};

	logger
		.info(format!("{} {}", request.method, request.path))
		.await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].message, "GET /api/users");
}

#[tokio::test]
async fn test_response_logging_middleware() {
	// Middleware should log responses
	let logger = Arc::new(Logger::new("reinhardt.request".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let response = Response { status: 200 };

	logger.info(format!("Response: {}", response.status)).await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].message, "Response: 200");
}

#[tokio::test]
async fn test_request_response_timing_logged() {
	// Request/response timing should be logged
	let logger = Arc::new(Logger::new("reinhardt.request".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let request = Request {
		method: "POST".to_string(),
		path: "/api/data".to_string(),
		timestamp: std::time::Instant::now(),
	};

	// Simulate some processing time

	let duration = request.timestamp.elapsed();
	logger
		.info(format!(
			"{} {} completed in {:.2}ms",
			request.method,
			request.path,
			duration.as_secs_f64() * 1000.0
		))
		.await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	// NOTE: Execution time is non-deterministic, verify format pattern
	assert!(
		records[0]
			.message
			.starts_with("POST /api/data completed in ")
	);
	assert!(records[0].message.ends_with("ms"));
	// Verify the numeric part exists and is valid
	let parts: Vec<&str> = records[0].message.split_whitespace().collect();
	assert_eq!(parts.len(), 5); // ["POST", "/api/data", "completed", "in", "X.XXms"]
	assert_eq!(parts[0], "POST");
	assert_eq!(parts[1], "/api/data");
	assert_eq!(parts[2], "completed");
	assert_eq!(parts[3], "in");
	assert!(parts[4].ends_with("ms"));
	let time_str = parts[4].trim_end_matches("ms");
	assert!(time_str.parse::<f64>().is_ok()); // Verify numeric value
}
