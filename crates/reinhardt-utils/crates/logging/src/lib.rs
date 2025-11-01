//! Logging framework for Reinhardt

pub mod config;
pub mod filters;
pub mod formatters;
pub mod handlers;
pub mod logger;
pub mod params;

pub use config::{HandlerConfig, LoggerConfig, LoggingConfig, LoggingManager};
pub use filters::{CallbackFilter, Filter, RequireDebugFalse, RequireDebugTrue};
pub use formatters::{Formatter, ServerFormatter, StandardFormatter, escape_control_chars};
pub use handlers::{ConsoleHandler, FileHandler, JsonHandler, MemoryHandler};
pub use logger::{Handler, LogLevel, LogRecord, Logger};
pub use params::{ReprParamsConfig, repr_params, truncate_param};

// ----------------------------------------------------------------------------
// Global logging manager and convenience APIs
// ----------------------------------------------------------------------------

use once_cell::sync::OnceCell;
use std::sync::Arc;

static GLOBAL_MANAGER: OnceCell<LoggingManager> = OnceCell::new();

/// Initialize the global logging manager. Subsequent calls are no-ops.
pub fn init_global_logging(config: LoggingConfig) {
	let _ = GLOBAL_MANAGER.set(LoggingManager::new(config));
}

fn global_manager() -> &'static LoggingManager {
	GLOBAL_MANAGER.get_or_init(|| LoggingManager::new(LoggingConfig::default()))
}

/// Get a logger by name from the global manager.
pub fn get_logger(name: &str) -> Arc<Logger> {
	global_manager().get_logger(name)
}

/// Emit a warning message via the named logger (global).
pub fn emit_warning(logger_name: &str, message: impl Into<String>) {
	let logger = get_logger(logger_name);
	logger.warning_sync(&message.into());
}

/// Attach a `MemoryHandler` to the named logger (useful for tests).
pub async fn attach_memory_handler(logger_name: &str, level: LogLevel) -> handlers::MemoryHandler {
	let handler = handlers::MemoryHandler::new(level);
	let logger = get_logger(logger_name);
	logger.add_handler(Box::new(handler.clone())).await;
	handler
}
