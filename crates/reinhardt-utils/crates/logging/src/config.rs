pub struct HandlerConfig;
pub struct LoggerConfig;

#[derive(Clone)]
pub struct LoggingConfig;

impl Default for LoggingConfig {
	fn default() -> Self {
		Self
	}
}

pub struct LoggingManager {
	#[allow(dead_code)]
	config: LoggingConfig,
}

impl LoggingManager {
	pub fn new(config: LoggingConfig) -> Self {
		Self { config }
	}

	pub fn get_logger(&self, name: &str) -> std::sync::Arc<crate::logger::Logger> {
		std::sync::Arc::new(crate::logger::Logger::new(name.to_string()))
	}
}
