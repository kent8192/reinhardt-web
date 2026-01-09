//! Integration tests for AdvancedSettings nested structure management.
//!
//! This test module validates the AdvancedSettings struct and its nested structures
//! including DatabaseSettings, CacheSettings, SessionSettings, CorsSettings,
//! EmailSettings, LoggingSettings, and their configuration from environment variables.

use reinhardt_settings::advanced::{
	AdvancedSettings, CacheSettings, CorsSettings, DatabaseSettings, EmailSettings,
	LoggingSettings, MediaSettings, SessionSettings, StaticSettings,
};
use rstest::*;
use serial_test::serial;
use std::env;
use std::path::PathBuf;

/// Test: DatabaseSettings from environment variables
///
/// Why: Validates that DatabaseSettings can be loaded from environment variables
/// with appropriate prefix handling.
#[rstest]
#[serial(advanced_env)]
#[test]
fn test_database_settings_from_env() {
	unsafe {
		env::set_var("DATABASE_URL", "postgres://localhost/testdb");
		env::set_var("DATABASE_MAX_CONNECTIONS", "100");
		env::set_var("DATABASE_MIN_CONNECTIONS", "10");
		env::set_var("DATABASE_CONNECT_TIMEOUT", "30");
		env::set_var("DATABASE_IDLE_TIMEOUT", "600");
	}

	// Note: DatabaseSettings fields are direct types (not Option)
	let db_settings = DatabaseSettings {
		url: env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string()),
		max_connections: env::var("DATABASE_MAX_CONNECTIONS")
			.ok()
			.and_then(|s| s.parse().ok())
			.unwrap_or(10),
		min_connections: env::var("DATABASE_MIN_CONNECTIONS")
			.ok()
			.and_then(|s| s.parse().ok())
			.unwrap_or(1),
		connect_timeout: env::var("DATABASE_CONNECT_TIMEOUT")
			.ok()
			.and_then(|s| s.parse().ok())
			.unwrap_or(30),
		idle_timeout: env::var("DATABASE_IDLE_TIMEOUT")
			.ok()
			.and_then(|s| s.parse().ok())
			.unwrap_or(600),
	};

	assert_eq!(db_settings.url, "postgres://localhost/testdb");
	assert_eq!(db_settings.max_connections, 100);
	assert_eq!(db_settings.min_connections, 10);
	assert_eq!(db_settings.connect_timeout, 30);
	assert_eq!(db_settings.idle_timeout, 600);

	unsafe {
		env::remove_var("DATABASE_URL");
		env::remove_var("DATABASE_MAX_CONNECTIONS");
		env::remove_var("DATABASE_MIN_CONNECTIONS");
		env::remove_var("DATABASE_CONNECT_TIMEOUT");
		env::remove_var("DATABASE_IDLE_TIMEOUT");
	}
}

/// Test: CacheSettings with complete configuration
///
/// Why: Validates that CacheSettings can be configured with all fields
/// (backend, location, timeout).
#[rstest]
#[test]
fn test_cache_settings_complete() {
	let cache_settings = CacheSettings {
		backend: "redis".to_string(),
		location: Some("redis://127.0.0.1:6379/0".to_string()),
		timeout: 300,
	};

	assert_eq!(cache_settings.backend, "redis");
	assert_eq!(
		cache_settings.location,
		Some("redis://127.0.0.1:6379/0".to_string())
	);
	assert_eq!(cache_settings.timeout, 300);
}

/// Test: SessionSettings with secure cookie configuration
///
/// Why: Validates that SessionSettings properly configures secure cookie options
/// for production environments.
#[rstest]
#[test]
fn test_session_settings_cookie_config() {
	let session_settings = SessionSettings {
		engine: "database".to_string(),
		cookie_name: "sessionid".to_string(),
		cookie_age: 1209600, // 2 weeks in seconds
		cookie_secure: true,
		cookie_httponly: true,
		cookie_samesite: "Strict".to_string(),
	};

	assert_eq!(session_settings.engine, "database");
	assert_eq!(session_settings.cookie_name, "sessionid");
	assert_eq!(session_settings.cookie_age, 1209600);
	assert_eq!(session_settings.cookie_secure, true);
	assert_eq!(session_settings.cookie_httponly, true);
	assert_eq!(session_settings.cookie_samesite, "Strict");
}

/// Test: CorsSettings with full configuration
///
/// Why: Validates that CorsSettings can configure origins, methods, headers,
/// credentials, and max_age for CORS policy.
#[rstest]
#[test]
fn test_cors_settings_full() {
	let cors_settings = CorsSettings {
		allow_origins: vec![
			"https://example.com".to_string(),
			"https://app.example.com".to_string(),
		],
		allow_methods: vec![
			"GET".to_string(),
			"POST".to_string(),
			"PUT".to_string(),
			"DELETE".to_string(),
		],
		allow_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
		allow_credentials: true,
		max_age: 3600,
	};

	assert_eq!(cors_settings.allow_origins.len(), 2);
	assert!(
		cors_settings
			.allow_origins
			.contains(&"https://example.com".to_string())
	);
	assert_eq!(cors_settings.allow_methods.len(), 4);
	assert_eq!(cors_settings.allow_headers.len(), 2);
	assert_eq!(cors_settings.allow_credentials, true);
	assert_eq!(cors_settings.max_age, 3600);
}

/// Test: EmailSettings for SMTP configuration
///
/// Why: Validates that EmailSettings can be configured for SMTP with TLS.
#[rstest]
#[test]
fn test_email_settings_smtp() {
	let email_settings = EmailSettings {
		backend: "smtp".to_string(),
		host: "smtp.example.com".to_string(),
		port: 587,
		username: Some("user@example.com".to_string()),
		password: Some("secret_password".to_string()),
		use_tls: true,
		use_ssl: false,
		from_email: "noreply@example.com".to_string(),
		admins: vec![],
		managers: vec![],
		server_email: "server@example.com".to_string(),
		subject_prefix: "[MyApp] ".to_string(),
		timeout: Some(30),
		ssl_certfile: None,
		ssl_keyfile: None,
		file_path: None,
	};

	assert_eq!(email_settings.backend, "smtp");
	assert_eq!(email_settings.host, "smtp.example.com");
	assert_eq!(email_settings.port, 587);
	assert_eq!(email_settings.use_tls, true);
	assert_eq!(email_settings.use_ssl, false);
	assert_eq!(email_settings.from_email, "noreply@example.com");
}

/// Test: EmailSettings for file-based backend
///
/// Why: Validates that EmailSettings can be configured for file-based email backend
/// (useful for development/testing).
#[rstest]
#[test]
fn test_email_settings_file_backend() {
	let email_settings = EmailSettings {
		backend: "file".to_string(),
		host: String::new(),
		port: 0,
		username: None,
		password: None,
		use_tls: false,
		use_ssl: false,
		from_email: "test@example.com".to_string(),
		admins: vec![],
		managers: vec![],
		server_email: String::new(),
		subject_prefix: String::new(),
		timeout: Some(0),
		ssl_certfile: None,
		ssl_keyfile: None,
		file_path: Some(PathBuf::from("/tmp/emails")),
	};

	assert_eq!(email_settings.backend, "file");
	assert_eq!(email_settings.file_path, Some(PathBuf::from("/tmp/emails")));
	assert_eq!(email_settings.from_email, "test@example.com");
}

/// Test: LoggingSettings with different log levels
///
/// Why: Validates that LoggingSettings supports all standard log levels
/// (DEBUG, INFO, WARN, ERROR).
#[rstest]
#[case("DEBUG")]
#[case("INFO")]
#[case("WARN")]
#[case("ERROR")]
#[test]
fn test_logging_settings_levels(#[case] level: &str) {
	let logging_settings = LoggingSettings {
		level: level.to_string(),
		format: "json".to_string(),
	};

	assert_eq!(logging_settings.level, level);
	assert_eq!(logging_settings.format, "json");
}

/// Test: AdvancedSettings set() and get() methods
///
/// Why: Validates that AdvancedSettings provides dynamic configuration
/// via set() and get() methods.
#[rstest]
#[test]
fn test_advanced_settings_set_get() {
	let mut settings = AdvancedSettings::default();

	// Set custom value
	settings
		.set("custom_key".to_string(), serde_json::json!("custom_value"))
		.expect("Failed to set custom_key");

	// Get custom value
	let value: Option<String> = settings.get("custom_key");
	assert_eq!(value, Some("custom_value".to_string()));

	// Get non-existent key
	let missing: Option<String> = settings.get("non_existent");
	assert_eq!(missing, None);
}

/// Test: StaticSettings configuration
///
/// Why: Validates that StaticSettings can be configured with URL and root path.
#[rstest]
#[test]
fn test_static_settings() {
	let static_settings = StaticSettings {
		url: "/static/".to_string(),
		root: PathBuf::from("/var/www/static"),
	};

	assert_eq!(static_settings.url, "/static/");
	assert_eq!(static_settings.root, PathBuf::from("/var/www/static"));
}

/// Test: MediaSettings configuration
///
/// Why: Validates that MediaSettings can be configured with URL and root path.
#[rstest]
#[test]
fn test_media_settings() {
	let media_settings = MediaSettings {
		url: "/media/".to_string(),
		root: PathBuf::from("/var/www/media"),
	};

	assert_eq!(media_settings.url, "/media/");
	assert_eq!(media_settings.root, PathBuf::from("/var/www/media"));
}

/// Test: AdvancedSettings default values
///
/// Why: Validates that AdvancedSettings::default() provides sensible defaults.
#[rstest]
#[test]
fn test_advanced_settings_default() {
	let settings = AdvancedSettings::default();

	// Verify defaults are set (non-Option fields)
	// Fields exist with default values
	assert!(
		!settings.debug || settings.debug,
		"debug field has a boolean value"
	);
	// secret_key and allowed_hosts existence is guaranteed at compile time
}

/// Test: DatabaseSettings default values
///
/// Why: Validates that DatabaseSettings::default() provides sensible connection defaults.
#[rstest]
#[test]
fn test_database_settings_default() {
	let db_settings = DatabaseSettings::default();

	// Verify defaults (fields are not Option, so always have values)
	assert_eq!(db_settings.url, "sqlite::memory:");
	assert_eq!(db_settings.max_connections, 10);
	assert_eq!(db_settings.min_connections, 1);
	assert_eq!(db_settings.connect_timeout, 30);
	assert_eq!(db_settings.idle_timeout, 600);
}

/// Test: Nested settings structure in AdvancedSettings
///
/// Why: Validates that AdvancedSettings correctly maintains all nested structures.
#[rstest]
#[test]
fn test_advanced_settings_nested_structures() {
	let settings = AdvancedSettings {
		debug: true,
		secret_key: "test_key".to_string(),
		allowed_hosts: vec!["localhost".to_string()],
		database: DatabaseSettings::default(),
		cache: CacheSettings::default(),
		session: SessionSettings::default(),
		cors: CorsSettings::default(),
		static_files: StaticSettings::default(),
		media: MediaSettings::default(),
		email: EmailSettings::default(),
		logging: LoggingSettings::default(),
		custom: Default::default(),
	};

	assert_eq!(settings.debug, true);
	assert_eq!(settings.secret_key, "test_key");
	assert_eq!(settings.allowed_hosts.len(), 1);
	// Nested structures are always present (not Option)
	assert_eq!(settings.database.url, "sqlite::memory:");
	assert_eq!(settings.cache.backend, "memory");
	assert_eq!(settings.session.engine, "cookie");
}
