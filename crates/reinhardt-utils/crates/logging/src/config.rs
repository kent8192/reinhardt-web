pub struct HandlerConfig;
pub struct LoggerConfig;

#[derive(Clone)]
pub struct LoggingConfig;

impl Default for LoggingConfig {
	fn default() -> Self {
		Self
	}
}

// TODO: LoggingManager.config field was removed as it was unused
// LoggingConfig is currently empty and not utilized in get_logger()
// If configuration-based logger behavior is needed in the future,
// LoggingConfig should be properly defined and utilized
pub struct LoggingManager;

impl LoggingManager {
	pub fn new(_config: LoggingConfig) -> Self {
		Self
	}

	pub fn get_logger(&self, name: &str) -> std::sync::Arc<crate::logger::Logger> {
		std::sync::Arc::new(crate::logger::Logger::new(name.to_string()))
	}
}
