//! Tests for log level filtering functionality
//! Based on Django's DefaultLoggingTests

use reinhardt_logging::{LogLevel, Logger, LoggingConfig, LoggingManager, handlers::MemoryHandler};
use std::sync::Arc;

#[tokio::test]
async fn test_logger_basic_levels() {
	// Test that different log levels are properly recorded
	let logger = Logger::new("test".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger
		.add_handler(Box::new(handler_clone.as_ref().clone() as MemoryHandler))
		.await;
	logger.set_level(LogLevel::Debug).await;

	logger.error("Hey, this is an error.".to_string()).await;
	logger.warning("warning".to_string()).await;
	logger.info("info".to_string()).await;
	logger.debug("debug".to_string()).await;

	let records = handler.get_records();
	assert_eq!(records.len(), 4);
	assert_eq!(records[0].level, LogLevel::Error);
	assert_eq!(records[1].level, LogLevel::Warning);
	assert_eq!(records[2].level, LogLevel::Info);
	assert_eq!(records[3].level, LogLevel::Debug);
}

#[tokio::test]
async fn test_logger_only_outputs_when_level_allows() {
	// The logger only outputs when the log level is at or above the configured level
	let logger = Logger::new("test".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger
		.add_handler(Box::new(handler_clone.as_ref().clone() as MemoryHandler))
		.await;
	logger.set_level(LogLevel::Info).await;

	logger.error("Hey, this is an error.".to_string()).await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].message, "Hey, this is an error.");

	handler.clear();

	// Now set to DEBUG level
	logger.set_level(LogLevel::Debug).await;
	logger.error("Hey, this is an error.".to_string()).await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].message, "Hey, this is an error.");
}

#[tokio::test]
async fn test_logger_warning() {
	let logger = Logger::new("test".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger
		.add_handler(Box::new(handler_clone.as_ref().clone() as MemoryHandler))
		.await;
	logger.set_level(LogLevel::Debug).await;

	logger.warning("warning".to_string()).await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].message, "warning");
	assert_eq!(records[0].level, LogLevel::Warning);
}

#[tokio::test]
async fn test_logger_info() {
	let logger = Logger::new("test".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger
		.add_handler(Box::new(handler_clone.as_ref().clone() as MemoryHandler))
		.await;
	logger.set_level(LogLevel::Debug).await;

	logger.info("info".to_string()).await;

	let records = handler.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].message, "info");
	assert_eq!(records[0].level, LogLevel::Info);
}

#[tokio::test]
async fn test_logger_debug_filtered_by_default() {
	// Debug logs are filtered when logger level is Info
	let logger = Logger::new("test".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Debug));
	let handler_clone = handler.clone();

	logger
		.add_handler(Box::new(handler_clone.as_ref().clone() as MemoryHandler))
		.await;
	logger.set_level(LogLevel::Info).await;

	logger.debug("debug".to_string()).await;

	let records = handler.get_records();
	assert_eq!(records.len(), 0);
}

#[tokio::test]
async fn test_handler_level_filtering() {
	// Test that handlers only process logs at or above their level
	let logger = Logger::new("test".to_string());
	let handler = Arc::new(MemoryHandler::new(LogLevel::Warning));
	let handler_clone = handler.clone();

	logger
		.add_handler(Box::new(handler_clone.as_ref().clone() as MemoryHandler))
		.await;
	logger.set_level(LogLevel::Debug).await;

	logger.debug("Should not log".to_string()).await;
	logger.info("Should not log".to_string()).await;
	logger.warning("Should log".to_string()).await;
	logger.error("Should log".to_string()).await;

	let records = handler.get_records();
	assert_eq!(records.len(), 2);
	assert_eq!(records[0].level, LogLevel::Warning);
	assert_eq!(records[1].level, LogLevel::Error);
}

#[tokio::test]
async fn test_multiple_handlers_different_levels() {
	// Test multiple handlers with different log levels
	let logger = Logger::new("test".to_string());

	let handler1 = Arc::new(MemoryHandler::new(LogLevel::Info));
	let handler2 = Arc::new(MemoryHandler::new(LogLevel::Error));

	let handler1_clone = handler1.clone();
	let handler2_clone = handler2.clone();

	logger
		.add_handler(Box::new(handler1_clone.as_ref().clone() as MemoryHandler))
		.await;
	logger
		.add_handler(Box::new(handler2_clone.as_ref().clone() as MemoryHandler))
		.await;
	logger.set_level(LogLevel::Debug).await;

	logger.debug("debug".to_string()).await;
	logger.info("info".to_string()).await;
	logger.warning("warning".to_string()).await;
	logger.error("error".to_string()).await;

	let records1 = handler1.get_records();
	let records2 = handler2.get_records();

	// Handler 1 should get info, warning, and error
	assert_eq!(records1.len(), 3);
	assert_eq!(records1[0].level, LogLevel::Info);
	assert_eq!(records1[1].level, LogLevel::Warning);
	assert_eq!(records1[2].level, LogLevel::Error);

	// Handler 2 should only get error
	assert_eq!(records2.len(), 1);
	assert_eq!(records2[0].level, LogLevel::Error);
}
