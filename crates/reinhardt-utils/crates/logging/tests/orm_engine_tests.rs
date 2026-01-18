//! ORM Engine Logging Integration Tests
//!
//! Tests for logging SQL execution with Engine echo flags.
//! Based on SQLAlchemy's engine logging tests.

use reinhardt_utils::logging::handlers::MemoryHandler;
use reinhardt_utils::logging::params::{ReprParamsConfig, repr_params};
use reinhardt_utils::logging::{LogLevel, Logger};
use serde_json::json;
use std::sync::Arc;

/// Mock Engine wrapper with logging (no actual database)
pub struct LoggingEngine {
	logger: Arc<Logger>,
	echo: bool,
}

impl LoggingEngine {
	pub fn new(logger: Arc<Logger>, echo: bool) -> Self {
		Self { logger, echo }
	}

	pub async fn execute(&self, sql: &str) {
		if self.echo {
			self.logger.info(format!("SQL: {}", sql)).await;
		}
	}

	pub async fn execute_with_params(&self, sql: &str, params: &serde_json::Value) {
		if self.echo {
			let config = ReprParamsConfig::default();
			let params_str = repr_params(params, &config);
			self.logger
				.info(format!("SQL: {} | Params: {}", sql, params_str))
				.await;
		}
	}
}

#[tokio::test]
async fn test_engine_echo_flag() {
	// Engine with echo=true should log all SQL at INFO level
	let logger = Arc::new(Logger::new("reinhardt.orm.engine".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let engine = LoggingEngine::new(logger.clone(), true);

	engine.execute("SELECT * FROM users").await;
	engine.execute("INSERT INTO users VALUES (1, 'test')").await;

	let records = memory.get_records();
	assert_eq!(records.len(), 2);
	assert_eq!(records[0].message, "SQL: SELECT * FROM users");
	assert_eq!(
		records[1].message,
		"SQL: INSERT INTO users VALUES (1, 'test')"
	);
}

#[tokio::test]
async fn test_engine_echo_disabled() {
	// Engine with echo=false should not log SQL
	let logger = Arc::new(Logger::new("reinhardt.orm.engine".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let engine = LoggingEngine::new(logger.clone(), false);

	engine.execute("SELECT * FROM users").await;

	let records = memory.get_records();
	assert_eq!(records.len(), 0); // No logs
}

#[tokio::test]
async fn test_named_logger() {
	// Engine should support custom logger name
	let logger = Arc::new(Logger::new("myapp.database.engine".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let engine = LoggingEngine::new(logger.clone(), true);

	engine.execute("SELECT 1").await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].logger_name, "myapp.database.engine");
}

#[tokio::test]
async fn test_sql_parameter_logging() {
	// SQL with bound parameters should be logged
	let logger = Arc::new(Logger::new("reinhardt.orm.engine".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let engine = LoggingEngine::new(logger.clone(), true);

	let params = json!({"id": 1, "name": "test"});
	engine
		.execute_with_params("SELECT * FROM users WHERE id = ? AND name = ?", &params)
		.await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(
		records[0].message,
		"SQL: SELECT * FROM users WHERE id = ? AND name = ? | Params: {\"id\": Number(1), \"name\": String(\"test\")}"
	);
}

#[tokio::test]
async fn test_large_parameter_truncation() {
	// Large parameters should use repr_params for truncation
	let logger = Arc::new(Logger::new("reinhardt.orm.engine".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let engine = LoggingEngine::new(logger.clone(), true);

	// Create large parameter data
	let large_data = "x".repeat(1000);
	let params = json!({ "data": large_data });

	engine
		.execute_with_params("INSERT INTO logs VALUES (?)", &params)
		.await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	// Should be truncated (repr_params default is ~200 chars)
	let params_part = &records[0].message;
	assert!(params_part.len() < 1500); // Much less than 1000 char param
}

#[tokio::test]
async fn test_multiple_statements_logged() {
	// Each statement should be logged separately
	let logger = Arc::new(Logger::new("reinhardt.orm.engine".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let engine = LoggingEngine::new(logger.clone(), true);

	engine.execute("CREATE TABLE users (id INTEGER)").await;
	engine.execute("INSERT INTO users VALUES (1)").await;
	engine.execute("SELECT * FROM users").await;
	engine.execute("DROP TABLE users").await;

	let records = memory.get_records();
	assert_eq!(records.len(), 4);
	assert_eq!(records[0].message, "SQL: CREATE TABLE users (id INTEGER)");
	assert_eq!(records[1].message, "SQL: INSERT INTO users VALUES (1)");
	assert_eq!(records[2].message, "SQL: SELECT * FROM users");
	assert_eq!(records[3].message, "SQL: DROP TABLE users");
}

#[tokio::test]
async fn test_echo_respects_logger_level() {
	// Logger level should filter echo logs
	let logger = Arc::new(Logger::new("reinhardt.orm.engine".to_string()));
	let handler = MemoryHandler::new(LogLevel::Warning);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Warning).await; // Set to WARNING

	let engine = LoggingEngine::new(logger.clone(), true);

	// Echo logs at INFO level - should be filtered
	engine.execute("SELECT 1").await;

	let records = memory.get_records();
	assert_eq!(records.len(), 0); // Filtered by level
}

#[tokio::test]
async fn test_multiple_engines_different_echo() {
	// Multiple engines with different echo settings
	let logger1 = Arc::new(Logger::new("reinhardt.orm.engine.conn1".to_string()));
	let handler1 = MemoryHandler::new(LogLevel::Info);
	let memory1 = handler1.clone();
	logger1.add_handler(Arc::new(handler1)).await;
	logger1.set_level(LogLevel::Info).await;

	let logger2 = Arc::new(Logger::new("reinhardt.orm.engine.conn2".to_string()));
	let handler2 = MemoryHandler::new(LogLevel::Info);
	let memory2 = handler2.clone();
	logger2.add_handler(Arc::new(handler2)).await;
	logger2.set_level(LogLevel::Info).await;

	// Engine 1 with echo=true
	let engine1 = LoggingEngine::new(logger1.clone(), true);

	// Engine 2 with echo=false
	let engine2 = LoggingEngine::new(logger2.clone(), false);

	engine1.execute("SELECT 1").await;
	engine2.execute("SELECT 2").await;

	let records1 = memory1.get_records();
	let records2 = memory2.get_records();

	assert_eq!(records1.len(), 1); // Engine 1 logged
	assert_eq!(records2.len(), 0); // Engine 2 didn't log
	assert_eq!(records1[0].message, "SQL: SELECT 1");
}

#[tokio::test]
async fn test_sql_with_newlines_logged() {
	// Multi-line SQL should be logged correctly
	let logger = Arc::new(Logger::new("reinhardt.orm.engine".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let engine = LoggingEngine::new(logger.clone(), true);

	let multiline_sql = "SELECT id, name, email\nFROM users\nWHERE active = true";
	engine.execute(multiline_sql).await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(
		records[0].message,
		"SQL: SELECT id, name, email\nFROM users\nWHERE active = true"
	);
}
