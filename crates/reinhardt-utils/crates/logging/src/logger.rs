use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait LogHandler: Send + Sync {
	async fn handle(&self, record: &LogRecord);
	fn level(&self) -> LogLevel;
	fn set_level(&mut self, level: LogLevel);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
	Debug,
	Info,
	Warning,
	Error,
}

#[derive(Debug, Clone)]
pub struct LogRecord {
	pub level: LogLevel,
	pub logger_name: String,
	pub message: String,
	pub extra: HashMap<String, serde_json::Value>,
}

impl LogRecord {
	pub fn new(level: LogLevel, logger_name: impl Into<String>, message: impl Into<String>) -> Self {
		Self {
			level,
			logger_name: logger_name.into(),
			message: message.into(),
			extra: HashMap::new(),
		}
	}
}

pub struct Logger {
	name: String,
	handlers: Arc<Mutex<Vec<Arc<dyn LogHandler>>>>,
	level: Arc<Mutex<LogLevel>>,
}

impl Logger {
	pub fn new(name: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			handlers: Arc::new(Mutex::new(Vec::new())),
			level: Arc::new(Mutex::new(LogLevel::Debug)),
		}
	}

	pub async fn add_handler(&self, handler: Arc<dyn LogHandler>) {
		self.handlers.lock().unwrap().push(handler);
	}

	pub async fn set_level(&self, level: LogLevel) {
		*self.level.lock().unwrap() = level;
	}

	pub async fn log_record(&self, record: &LogRecord) {
		// Clone Arc references to handlers before releasing the lock
		let handlers: Vec<Arc<dyn LogHandler>> = {
			let handlers_guard = self.handlers.lock().unwrap();
			handlers_guard.clone()
		};

		// Now we can iterate and await without holding the lock
		for handler in handlers {
			handler.handle(record).await;
		}
	}

	async fn log(&self, level: LogLevel, message: impl Into<String>) {
		let current_level = *self.level.lock().unwrap();
		if level < current_level {
			return;
		}

		let record = LogRecord::new(level, &self.name, message);

		// Clone Arc references to handlers before releasing the lock
		let handlers: Vec<Arc<dyn LogHandler>> = {
			let handlers_guard = self.handlers.lock().unwrap();
			handlers_guard.clone()
		};

		// Now we can iterate and await without holding the lock
		for handler in handlers {
			handler.handle(&record).await;
		}
	}

	pub async fn debug(&self, message: impl Into<String>) {
		self.log(LogLevel::Debug, message).await;
	}

	pub async fn info(&self, message: impl Into<String>) {
		self.log(LogLevel::Info, message).await;
	}

	pub async fn warning(&self, message: impl Into<String>) {
		self.log(LogLevel::Warning, message).await;
	}

	/// Synchronous version of warning log
	///
	/// This method provides a synchronous interface for logging warnings.
	/// Note that if handlers perform async operations, they will be blocked.
	/// Prefer using the async `warning` method when possible.
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_logging::Logger;
	///
	/// let logger = Logger::new("my_logger");
	/// logger.warning_sync("This is a synchronous warning");
	/// ```
	pub fn warning_sync(&self, message: impl Into<String>) {
		self.log_sync(LogLevel::Warning, message);
	}

	fn log_sync(&self, level: LogLevel, message: impl Into<String>) {
		let current_level = *self.level.lock().unwrap();
		if level < current_level {
			return;
		}

		let record = LogRecord::new(level, &self.name, message);

		// Clone Arc references to handlers before releasing the lock
		let handlers: Vec<Arc<dyn LogHandler>> = {
			let handlers_guard = self.handlers.lock().unwrap();
			handlers_guard.clone()
		};

		// Synchronously process handlers using tokio runtime
		let rt = tokio::runtime::Handle::try_current().ok().or_else(|| {
			// If no runtime exists, create a temporary one
			tokio::runtime::Runtime::new()
				.ok()
				.map(|rt| rt.handle().clone())
		});

		if let Some(handle) = rt {
			for handler in handlers.iter() {
				// Block on async handler using tokio runtime
				handle.block_on(handler.handle(&record));
			}
		} else {
			// Fallback: skip async operations if no runtime available
			// This should rarely happen in practice
			eprintln!("Warning: No tokio runtime available for synchronous logging");
		}
	}

	pub async fn error(&self, message: impl Into<String>) {
		self.log(LogLevel::Error, message).await;
	}
}
