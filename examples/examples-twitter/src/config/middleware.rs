use reinhardt::CorsSettings;
use reinhardt::SecuritySettings;
use reinhardt::middleware::security_middleware::SecurityMiddleware;
use reinhardt::middleware::session::{SessionConfig, SessionMiddleware};
use reinhardt::middleware::{
	CorsMiddleware, LoggingMiddleware, create_cors_middleware_from_settings,
};

/// Configure CORS middleware
/// Returns a configured CORS middleware instance
pub fn create_cors_middleware() -> CorsMiddleware {
	let mut config = CorsSettings::default();
	config.allow_origins = vec!["*".to_string()]; // Development
	config.allow_methods = vec![
		"GET".to_string(),
		"POST".to_string(),
		"PUT".to_string(),
		"DELETE".to_string(),
		"OPTIONS".to_string(),
	];
	config.allow_headers = vec![
		"Content-Type".to_string(),
		"Authorization".to_string(),
		"X-CSRF-Token".to_string(),
	];
	config.allow_credentials = true;
	config.max_age = 3600;
	create_cors_middleware_from_settings(&config)
}

/// Configure logging middleware
/// Returns a configured logging middleware instance
pub fn create_logging_middleware() -> LoggingMiddleware {
	LoggingMiddleware::new()
}

/// Configure session middleware
/// Returns a configured session middleware instance
pub fn create_session_middleware() -> SessionMiddleware {
	let config = SessionConfig::default();
	SessionMiddleware::new(config)
}

/// Configure security middleware
/// Returns a configured security middleware instance
pub fn create_security_middleware() -> SecurityMiddleware {
	let mut config = SecurityMiddlewareConfig::default();
	config.security_settings = SecuritySettings::default();
	SecurityMiddleware::new(config)
}
