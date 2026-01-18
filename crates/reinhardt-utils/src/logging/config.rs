pub struct HandlerConfig;
pub struct LoggerConfig;

#[derive(Clone)]
pub struct LoggingConfig;

impl Default for LoggingConfig {
	fn default() -> Self {
		Self
	}
}

pub struct LoggingManager;

impl LoggingManager {
	pub fn new(_config: LoggingConfig) -> Self {
		Self
	}

	pub fn get_logger(&self, name: &str) -> std::sync::Arc<crate::logging::logger::Logger> {
		std::sync::Arc::new(crate::logging::logger::Logger::new(name.to_string()))
	}
}
