//! Async Handler Integration Tests
//!
//! Tests for QueueHandler and QueueListener for non-blocking async logging.
//! Based on Python's logging.handlers.QueueHandler.

use reinhardt_logging::handlers::MemoryHandler;
use reinhardt_logging::{LogHandler, LogLevel, LogRecord, Logger};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::mpsc;

/// Queue-based handler for non-blocking logging
pub struct QueueHandler {
	level: LogLevel,
	sender: mpsc::UnboundedSender<LogRecord>,
}

impl QueueHandler {
	pub fn new(level: LogLevel) -> (Self, mpsc::UnboundedReceiver<LogRecord>) {
		let (sender, receiver) = mpsc::unbounded_channel();
		(Self { level, sender }, receiver)
	}
}

#[async_trait::async_trait]
impl LogHandler for QueueHandler {
	async fn handle(&self, record: &LogRecord) {
		// Non-blocking send - if the channel is full or closed, just drop the record
		let _ = self.sender.send(record.clone());
	}

	fn level(&self) -> LogLevel {
		self.level
	}

	fn set_level(&mut self, level: LogLevel) {
		self.level = level;
	}
}

/// Queue listener that processes log records from a queue in the background
pub struct QueueListener {
	receiver: Arc<AsyncMutex<mpsc::UnboundedReceiver<LogRecord>>>,
	handlers: Vec<Arc<MemoryHandler>>,
}

impl QueueListener {
	pub fn new(receiver: mpsc::UnboundedReceiver<LogRecord>) -> Self {
		Self {
			receiver: Arc::new(AsyncMutex::new(receiver)),
			handlers: Vec::new(),
		}
	}

	pub fn add_handler(&mut self, handler: Arc<MemoryHandler>) {
		self.handlers.push(handler);
	}

	/// Start processing records from the queue
	pub async fn start(&self) {
		let receiver = self.receiver.clone();
		let handlers = self.handlers.clone();

		tokio::spawn(async move {
			let mut rx = receiver.lock().await;
			while let Some(record) = rx.recv().await {
				for handler in &handlers {
					// Respect handler level filtering
					if record.level >= handler.level() {
						handler.handle(&record).await;
					}
				}
			}
		});
	}

	/// Process a single batch of records (for testing)
	pub async fn process_batch(&self, max_records: usize) -> usize {
		let mut rx = self.receiver.lock().await;
		let mut count = 0;

		for _ in 0..max_records {
			if let Ok(record) = rx.try_recv() {
				for handler in &self.handlers {
					// Respect handler level filtering
					if record.level >= handler.level() {
						handler.handle(&record).await;
					}
				}
				count += 1;
			} else {
				break;
			}
		}

		count
	}
}

#[tokio::test]
async fn test_queue_handler_enqueues_records() {
	// QueueHandler should enqueue log records without blocking
	let logger = Logger::new("test.queue".to_string());
	let (handler, mut receiver) = QueueHandler::new(LogLevel::Info);

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	logger.info("Test message 1".to_string()).await;
	logger.info("Test message 2".to_string()).await;

	// Wait for records to be enqueued
	tokio::time::sleep(Duration::from_millis(100)).await;

	// Check that records were enqueued
	let record1 = receiver.try_recv().unwrap();
	let record2 = receiver.try_recv().unwrap();

	assert_eq!(record1.message, "Test message 1");
	assert_eq!(record2.message, "Test message 2");
}

#[tokio::test]
async fn test_queue_listener_processes_records() {
	// QueueListener should consume and process queued records
	let logger = Logger::new("test.listener".to_string());
	let (handler, receiver) = QueueHandler::new(LogLevel::Info);

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	// Create listener with a memory handler
	let mut listener = QueueListener::new(receiver);
	let memory = Arc::new(MemoryHandler::new(LogLevel::Info));
	listener.add_handler(memory.clone());

	// Log some messages
	logger.info("Message 1".to_string()).await;
	logger.info("Message 2".to_string()).await;
	logger.info("Message 3".to_string()).await;

	// Wait for records to be enqueued
	tokio::time::sleep(Duration::from_millis(100)).await;

	// Process the queued records
	let processed = listener.process_batch(10).await;
	assert_eq!(processed, 3);

	// Wait for records to be processed by memory handler
	reinhardt_test::poll_until(
		Duration::from_millis(100),
		Duration::from_millis(5),
		|| async { memory.get_records().len() >= 3 },
	)
	.await
	.expect("Records should be processed within 100ms");

	// Check that records were handled
	let records = memory.get_records();
	assert_eq!(records.len(), 3);
	assert_eq!(records[0].message, "Message 1");
	assert_eq!(records[1].message, "Message 2");
	assert_eq!(records[2].message, "Message 3");
}

#[tokio::test]
async fn test_queue_handler_nonblocking() {
	// Queue handler should return immediately without blocking
	let logger = Logger::new("test.nonblock".to_string());
	let (handler, _receiver) = QueueHandler::new(LogLevel::Info);

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	// Log many messages - should complete quickly
	let start = std::time::Instant::now();

	for i in 0..1000 {
		logger.info(format!("Message {}", i)).await;
	}

	let duration = start.elapsed();

	// Should complete in well under 100ms since it's non-blocking
	assert!(duration.as_millis() < 100, "Took too long: {:?}", duration);
}

#[tokio::test]
async fn test_listener_respects_handler_levels() {
	// Listener should respect handler log levels
	let logger = Logger::new("test.levels".to_string());
	let (handler, receiver) = QueueHandler::new(LogLevel::Debug);

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Debug).await;

	// Create listener with handler at WARNING level
	let mut listener = QueueListener::new(receiver);
	let memory = Arc::new(MemoryHandler::new(LogLevel::Warning));
	listener.add_handler(memory.clone());

	// Log at different levels
	logger.debug("Debug message".to_string()).await;
	logger.info("Info message".to_string()).await;
	logger.warning("Warning message".to_string()).await;
	logger.error("Error message".to_string()).await;

	// Wait for records to be enqueued
	tokio::time::sleep(Duration::from_millis(100)).await;

	// Process all records
	listener.process_batch(10).await;

	// Wait for records to be processed (only WARNING and ERROR)
	reinhardt_test::poll_until(
		Duration::from_millis(100),
		Duration::from_millis(5),
		|| async { memory.get_records().len() >= 2 },
	)
	.await
	.expect("Records should be processed within 100ms");

	// Only WARNING and ERROR should be in memory handler
	let records = memory.get_records();
	assert_eq!(records.len(), 2);
	assert_eq!(records[0].level, LogLevel::Warning);
	assert_eq!(records[1].level, LogLevel::Error);
}

#[tokio::test]
async fn test_logging_multiple_listeners() {
	// Multiple listeners can consume from different queues
	let logger1 = Logger::new("test.queue1".to_string());
	let (handler1, receiver1) = QueueHandler::new(LogLevel::Info);
	logger1.add_handler(Arc::new(handler1)).await;
	logger1.set_level(LogLevel::Info).await;

	let logger2 = Logger::new("test.queue2".to_string());
	let (handler2, receiver2) = QueueHandler::new(LogLevel::Info);
	logger2.add_handler(Arc::new(handler2)).await;
	logger2.set_level(LogLevel::Info).await;

	// Create separate listeners
	let mut listener1 = QueueListener::new(receiver1);
	let memory1 = Arc::new(MemoryHandler::new(LogLevel::Info));
	listener1.add_handler(memory1.clone());

	let mut listener2 = QueueListener::new(receiver2);
	let memory2 = Arc::new(MemoryHandler::new(LogLevel::Info));
	listener2.add_handler(memory2.clone());

	// Log to different loggers
	logger1.info("Queue 1 message".to_string()).await;
	logger2.info("Queue 2 message".to_string()).await;

	// Wait for records to be enqueued in both queues
	tokio::time::sleep(Duration::from_millis(100)).await;

	// Process both queues
	listener1.process_batch(10).await;
	listener2.process_batch(10).await;

	// Wait for records to be processed in both memory handlers
	reinhardt_test::poll_until(
		Duration::from_millis(100),
		Duration::from_millis(5),
		|| async { memory1.get_records().len() >= 1 && memory2.get_records().len() >= 1 },
	)
	.await
	.expect("Records should be processed within 100ms");

	// Each memory handler should have its own records
	let records1 = memory1.get_records();
	let records2 = memory2.get_records();

	assert_eq!(records1.len(), 1);
	assert_eq!(records2.len(), 1);
	assert_eq!(records1[0].message, "Queue 1 message");
	assert_eq!(records2[0].message, "Queue 2 message");
}

#[tokio::test]
async fn test_listener_with_multiple_handlers() {
	// Single listener can forward to multiple handlers
	let logger = Logger::new("test.multi".to_string());
	let (handler, receiver) = QueueHandler::new(LogLevel::Info);
	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	// Create listener with multiple memory handlers
	let mut listener = QueueListener::new(receiver);
	let memory1 = Arc::new(MemoryHandler::new(LogLevel::Info));
	let memory2 = Arc::new(MemoryHandler::new(LogLevel::Info));
	listener.add_handler(memory1.clone());
	listener.add_handler(memory2.clone());

	logger.info("Broadcast message".to_string()).await;

	// Wait for record to be enqueued
	tokio::time::sleep(Duration::from_millis(100)).await;

	listener.process_batch(10).await;

	// Wait for record to be processed by both handlers
	reinhardt_test::poll_until(
		Duration::from_millis(100),
		Duration::from_millis(5),
		|| async { memory1.get_records().len() >= 1 && memory2.get_records().len() >= 1 },
	)
	.await
	.expect("Record should be processed within 100ms");

	// Both handlers should have received the record
	let records1 = memory1.get_records();
	let records2 = memory2.get_records();

	assert_eq!(records1.len(), 1);
	assert_eq!(records2.len(), 1);
	assert_eq!(records1[0].message, "Broadcast message");
	assert_eq!(records2[0].message, "Broadcast message");
}

#[tokio::test]
async fn test_queue_handler_cloning_records() {
	// QueueHandler should clone records (not move them)
	let logger = Logger::new("test.clone".to_string());
	let (handler, mut receiver) = QueueHandler::new(LogLevel::Info);

	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	logger.info("Test message".to_string()).await;

	// Wait for first record
	tokio::time::sleep(Duration::from_millis(100)).await;

	let record1 = receiver.try_recv().unwrap();

	// Should be able to log again (handler still works)
	logger.info("Another message".to_string()).await;

	// Wait for second record
	tokio::time::sleep(Duration::from_millis(100)).await;

	let record2 = receiver.try_recv().unwrap();

	assert_eq!(record1.message, "Test message");
	assert_eq!(record2.message, "Another message");
}

#[tokio::test]
async fn test_listener_start_background() {
	// Listener start() should process records in the background
	let logger = Logger::new("test.background".to_string());
	let (handler, receiver) = QueueHandler::new(LogLevel::Info);
	logger.add_handler(Arc::new(handler)).await;
	logger.set_level(LogLevel::Info).await;

	let mut listener = QueueListener::new(receiver);
	let memory = Arc::new(MemoryHandler::new(LogLevel::Info));
	listener.add_handler(memory.clone());

	// Start background processing
	listener.start().await;

	// Log messages
	logger.info("Background message 1".to_string()).await;
	logger.info("Background message 2".to_string()).await;

	// Wait for background task to process records
	reinhardt_test::poll_until(
		Duration::from_millis(200),
		Duration::from_millis(10),
		|| async { memory.get_records().len() >= 2 },
	)
	.await
	.expect("Background task should process records within 200ms");

	// Records should be processed automatically
	let records = memory.get_records();
	assert_eq!(records.len(), 2);
	assert_eq!(records[0].message, "Background message 1");
	assert_eq!(records[1].message, "Background message 2");
}
