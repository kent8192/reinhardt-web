//! ORM Pool Logging Integration Tests
//!
//! Tests for logging connection pool events and statistics.
//! Based on SQLAlchemy's pool logging tests.

use reinhardt_logging::handlers::MemoryHandler;
use reinhardt_logging::{LogLevel, Logger};
use std::sync::Arc;
use std::time::Duration;

/// Mock connection pool with logging
pub struct LoggingPool {
	logger: Arc<Logger>,
	pool_size: usize,
	max_overflow: usize,
	current_connections: usize,
	pool_recycle: Option<Duration>,
}

impl LoggingPool {
	pub fn new(logger: Arc<Logger>, pool_size: usize, max_overflow: usize) -> Self {
		Self {
			logger,
			pool_size,
			max_overflow,
			current_connections: 0,
			pool_recycle: None,
		}
	}

	pub fn with_recycle(mut self, recycle: Duration) -> Self {
		self.pool_recycle = Some(recycle);
		self
	}

	pub async fn acquire(&mut self) -> Result<(), PoolError> {
		if self.current_connections < self.pool_size {
			self.current_connections += 1;
			self.logger
				.debug(format!(
					"Pool connection acquired ({}/{})",
					self.current_connections, self.pool_size
				))
				.await;
			Ok(())
		} else if self.current_connections < self.pool_size + self.max_overflow {
			self.current_connections += 1;
			self.logger
				.warning(format!(
					"Pool overflow: {} connections (max: {}, overflow: {})",
					self.current_connections, self.pool_size, self.max_overflow
				))
				.await;
			Ok(())
		} else {
			self.logger
				.error("Pool connection timeout: max connections exceeded".to_string())
				.await;
			Err(PoolError::Timeout)
		}
	}

	pub async fn release(&mut self) {
		if self.current_connections > 0 {
			self.current_connections -= 1;
			self.logger
				.debug(format!(
					"Pool connection released ({}/{})",
					self.current_connections, self.pool_size
				))
				.await;
		}
	}

	pub async fn recycle(&self) {
		if let Some(duration) = self.pool_recycle {
			self.logger
				.info(format!(
					"Pool recycle event: recycling connections older than {:?}",
					duration
				))
				.await;
		}
	}

	pub async fn dispose(&self) {
		self.logger
			.info("Pool dispose: closing all connections".to_string())
			.await;
	}

	pub async fn log_statistics(&self) {
		self.logger
			.info(format!(
				"Pool statistics: size={}, overflow={}, current={}",
				self.pool_size, self.max_overflow, self.current_connections
			))
			.await;
	}

	pub async fn validate_connection(&self, is_valid: bool) -> Result<(), PoolError> {
		if !is_valid {
			self.logger
				.error("Connection validation failed: connection is stale".to_string())
				.await;
			Err(PoolError::ValidationFailed)
		} else {
			Ok(())
		}
	}
}

#[derive(Debug)]
pub enum PoolError {
	Timeout,
	ValidationFailed,
}

#[tokio::test]
async fn test_pool_connection_logging() {
	// Pool should log connection acquire/release at DEBUG level
	let logger = Arc::new(Logger::new("reinhardt.orm.pool".to_string()));
	let handler = MemoryHandler::new(LogLevel::Debug);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Debug).await;

	let mut pool = LoggingPool::new(logger.clone(), 5, 2);

	pool.acquire().await.unwrap();
	pool.acquire().await.unwrap();
	pool.release().await;

	let records = memory.get_records();
	assert_eq!(records.len(), 3);
	assert_eq!(records[0].message, "Pool connection acquired (1/5)");
	assert_eq!(records[1].message, "Pool connection acquired (2/5)");
	assert_eq!(records[2].message, "Pool connection released (1/5)");
}

#[tokio::test]
async fn test_pool_overflow_warnings() {
	// Pool should log WARNING when overflow connections are used
	let logger = Arc::new(Logger::new("reinhardt.orm.pool".to_string()));
	let handler = MemoryHandler::new(LogLevel::Debug);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Debug).await;

	let mut pool = LoggingPool::new(logger.clone(), 2, 3);

	// Fill regular pool
	pool.acquire().await.unwrap();
	pool.acquire().await.unwrap();

	// Use overflow
	pool.acquire().await.unwrap();

	let records = memory.get_records();
	assert_eq!(records.len(), 3);
	assert_eq!(records[2].level, LogLevel::Warning);
	assert_eq!(
		records[2].message,
		"Pool overflow: 3 connections (max: 2, overflow: 3)"
	);
}

#[tokio::test]
async fn test_connection_timeout_logging() {
	// Pool should log ERROR when max connections exceeded
	let logger = Arc::new(Logger::new("reinhardt.orm.pool".to_string()));
	let handler = MemoryHandler::new(LogLevel::Debug);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Debug).await;

	let mut pool = LoggingPool::new(logger.clone(), 2, 1);

	// Fill pool + overflow
	pool.acquire().await.unwrap();
	pool.acquire().await.unwrap();
	pool.acquire().await.unwrap();

	// This should timeout
	let result = pool.acquire().await;
	assert!(result.is_err());

	let records = memory.get_records();
	let error_record = records.iter().find(|r| r.level == LogLevel::Error);
	assert!(error_record.is_some());
	assert_eq!(
		error_record.unwrap().message,
		"Pool connection timeout: max connections exceeded"
	);
}

#[tokio::test]
async fn test_pool_recycle_events() {
	// Pool recycle events should be logged at INFO level
	let logger = Arc::new(Logger::new("reinhardt.orm.pool".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let pool = LoggingPool::new(logger.clone(), 5, 2).with_recycle(Duration::from_secs(3600));

	pool.recycle().await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(
		records[0].message,
		"Pool recycle event: recycling connections older than 3600s"
	);
}

#[tokio::test]
async fn test_pool_dispose_logging() {
	// Pool dispose should log at INFO level
	let logger = Arc::new(Logger::new("reinhardt.orm.pool".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let pool = LoggingPool::new(logger.clone(), 5, 2);

	pool.dispose().await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].message, "Pool dispose: closing all connections");
}

#[tokio::test]
async fn test_multiple_pools_different_configs() {
	// Multiple pools should log independently
	let logger1 = Arc::new(Logger::new("reinhardt.orm.pool.primary".to_string()));
	let handler1 = MemoryHandler::new(LogLevel::Debug);
	let memory1 = handler1.clone();
	logger1.add_handler(Arc::new(handler1)).await;
	logger1.set_level(LogLevel::Debug).await;

	let logger2 = Arc::new(Logger::new("reinhardt.orm.pool.replica".to_string()));
	let handler2 = MemoryHandler::new(LogLevel::Debug);
	let memory2 = handler2.clone();
	logger2.add_handler(Arc::new(handler2)).await;
	logger2.set_level(LogLevel::Debug).await;

	let mut pool1 = LoggingPool::new(logger1.clone(), 10, 5);
	let mut pool2 = LoggingPool::new(logger2.clone(), 5, 2);

	pool1.acquire().await.unwrap();
	pool2.acquire().await.unwrap();

	let records1 = memory1.get_records();
	let records2 = memory2.get_records();

	assert_eq!(records1.len(), 1);
	assert_eq!(records2.len(), 1);
	assert_eq!(records1[0].message, "Pool connection acquired (1/10)");
	assert_eq!(records2[0].message, "Pool connection acquired (1/5)");
	assert_eq!(records1[0].logger_name, "reinhardt.orm.pool.primary");
	assert_eq!(records2[0].logger_name, "reinhardt.orm.pool.replica");
}

#[tokio::test]
async fn test_pool_statistics_logging() {
	// Pool statistics should be logged at INFO level
	let logger = Arc::new(Logger::new("reinhardt.orm.pool".to_string()));
	let handler = MemoryHandler::new(LogLevel::Info);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let mut pool = LoggingPool::new(logger.clone(), 10, 5);

	pool.acquire().await.unwrap();
	pool.acquire().await.unwrap();
	pool.log_statistics().await;

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(
		records[0].message,
		"Pool statistics: size=10, overflow=5, current=2"
	);
}

#[tokio::test]
async fn test_connection_validation_errors() {
	// Connection validation failures should be logged at ERROR level
	let logger = Arc::new(Logger::new("reinhardt.orm.pool".to_string()));
	let handler = MemoryHandler::new(LogLevel::Error);
	let memory = handler.clone();

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Error).await;

	let pool = LoggingPool::new(logger.clone(), 5, 2);

	// Simulate validation failure
	let result = pool.validate_connection(false).await;
	assert!(result.is_err());

	let records = memory.get_records();
	assert_eq!(records.len(), 1);
	assert_eq!(records[0].level, LogLevel::Error);
	assert_eq!(
		records[0].message,
		"Connection validation failed: connection is stale"
	);
}
