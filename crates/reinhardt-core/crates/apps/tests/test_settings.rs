//! Settings Module Tests
//!
//! Tests inspired by Django Rest Framework and Django settings tests

use reinhardt_apps::{DatabaseConfig, MiddlewareConfig, Settings, TemplateConfig};
use serde_json::json;
use std::path::PathBuf;

#[test]
fn test_apps_settings_default() {
	let settings = Settings::default();
	assert_eq!(settings.debug, true);
	assert_eq!(settings.language_code, "en-us");
	assert_eq!(settings.time_zone, "UTC");
	assert_eq!(settings.use_i18n, true);
	assert_eq!(settings.use_tz, true);
}

#[test]
fn test_settings_new() {
	let base_dir = PathBuf::from("/project");
	let secret_key = "test-secret-key".to_string();
	let settings = Settings::new(base_dir.clone(), secret_key.clone());

	assert_eq!(settings.base_dir, base_dir);
	assert_eq!(settings.secret_key, secret_key);
}

// This test is covered in integration tests
// See: tests/integration/tests/settings_system_integration.rs::test_settings_with_root_urlconf

// This test is covered in integration tests
// See: tests/integration/tests/settings_system_integration.rs::test_add_installed_app

// This test is covered in integration tests
// See: tests/integration/tests/settings_system_integration.rs::test_add_middleware

#[test]
fn test_database_config_sqlite() {
	let db = DatabaseConfig::sqlite("test.db");
	assert_eq!(db.engine, "reinhardt.db.backends.sqlite3");
	assert_eq!(db.name, "test.db");
	assert!(db.user.is_none());
	assert!(db.password.is_none());
	assert!(db.host.is_none());
	assert!(db.port.is_none());
}

#[test]
fn test_database_config_postgresql() {
	let db = DatabaseConfig::postgresql("testdb", "user", "pass", "localhost", 5432);
	assert_eq!(db.engine, "reinhardt.db.backends.postgresql");
	assert_eq!(db.name, "testdb");
	assert_eq!(db.user, Some("user".to_string()));
	assert_eq!(db.password, Some("pass".to_string()));
	assert_eq!(db.host, Some("localhost".to_string()));
	assert_eq!(db.port, Some(5432));
}

#[test]
fn test_database_config_mysql() {
	let db = DatabaseConfig::mysql("testdb", "user", "pass", "localhost", 3306);
	assert_eq!(db.engine, "reinhardt.db.backends.mysql");
	assert_eq!(db.name, "testdb");
	assert_eq!(db.user, Some("user".to_string()));
	assert_eq!(db.password, Some("pass".to_string()));
	assert_eq!(db.host, Some("localhost".to_string()));
	assert_eq!(db.port, Some(3306));
}

#[test]
fn test_template_config_default() {
	let config = TemplateConfig::default();
	assert!(config.app_dirs);
	assert_eq!(config.backend, "reinhardt.template.backends.jinja2.Jinja2");
	assert!(config.dirs.is_empty());
}

#[test]
fn test_template_config_new() {
	let config = TemplateConfig::new("reinhardt.template.backends.custom.Custom");
	assert_eq!(config.backend, "reinhardt.template.backends.custom.Custom");
	assert!(config.app_dirs);
}

#[test]
fn test_template_config_add_dir() {
	let config = TemplateConfig::default()
		.add_dir("/templates")
		.add_dir("/other_templates");

	assert_eq!(config.dirs.len(), 2);
	assert_eq!(config.dirs[0], PathBuf::from("/templates"));
	assert_eq!(config.dirs[1], PathBuf::from("/other_templates"));
}

#[test]
fn test_middleware_config_new() {
	let middleware = MiddlewareConfig::new("reinhardt.middleware.TestMiddleware");
	assert_eq!(middleware.path, "reinhardt.middleware.TestMiddleware");
	assert!(middleware.options.is_empty());
}

#[test]
fn test_middleware_config_with_option() {
	let middleware = MiddlewareConfig::new("reinhardt.middleware.TestMiddleware")
		.with_option("enabled", json!(true))
		.with_option("debug", json!(false));

	assert_eq!(middleware.path, "reinhardt.middleware.TestMiddleware");
	assert_eq!(middleware.options.get("enabled"), Some(&json!(true)));
	assert_eq!(middleware.options.get("debug"), Some(&json!(false)));
}

#[test]
fn test_settings_installed_apps_contains_defaults() {
	let settings = Settings::default();

	// Check that default apps are installed
	assert!(
		settings
			.installed_apps
			.contains(&"reinhardt.contrib.admin".to_string())
	);
	assert!(
		settings
			.installed_apps
			.contains(&"reinhardt.contrib.auth".to_string())
	);
	assert!(
		settings
			.installed_apps
			.contains(&"reinhardt.contrib.contenttypes".to_string())
	);
}

#[test]
fn test_settings_middleware_contains_defaults() {
	let settings = Settings::default();

	// Check that default middleware is configured
	assert!(
		settings
			.middleware
			.contains(&"reinhardt.middleware.security.SecurityMiddleware".to_string())
	);
	assert!(
		settings
			.middleware
			.contains(&"reinhardt.middleware.csrf.CsrfViewMiddleware".to_string())
	);
}

#[test]
fn test_settings_static_url() {
	let settings = Settings::default();
	assert_eq!(settings.static_url, "/static/");
	assert!(settings.static_root.is_none());
}

#[test]
fn test_settings_media_url() {
	let settings = Settings::default();
	assert_eq!(settings.media_url, "/media/");
	assert!(settings.media_root.is_none());
}

#[test]
fn test_settings_databases_has_default() {
	let settings = Settings::default();
	assert!(settings.databases.contains_key("default"));

	let default_db = settings.databases.get("default").unwrap();
	assert_eq!(default_db.engine, "reinhardt.db.backends.sqlite3");
	assert_eq!(default_db.name, "db.sqlite3");
}

#[test]
fn test_settings_templates_has_default() {
	let settings = Settings::default();
	assert_eq!(settings.templates.len(), 1);

	let template = &settings.templates[0];
	assert_eq!(
		template.backend,
		"reinhardt.template.backends.jinja2.Jinja2"
	);
}

#[test]
fn test_database_config_default() {
	let db = DatabaseConfig::default();
	assert_eq!(db.engine, "reinhardt.db.backends.sqlite3");
	assert_eq!(db.name, "db.sqlite3");
}

#[test]
fn test_settings_secret_key_warning() {
	let settings = Settings::default();
	// In production, secret key should not be the default insecure value
	assert_eq!(settings.secret_key, "insecure-change-this-in-production");
}

#[test]
fn test_settings_debug_mode_default() {
	let settings = Settings::default();
	// Debug should be true by default for development
	assert_eq!(settings.debug, true);
}

#[test]
fn test_settings_allowed_hosts_empty_by_default() {
	let settings = Settings::default();
	// Allowed hosts should be empty by default
	assert!(settings.allowed_hosts.is_empty());
}

#[test]
fn test_settings_default_auto_field() {
	let settings = Settings::default();
	assert_eq!(
		settings.default_auto_field,
		"reinhardt.db.models.BigAutoField"
	);
}

#[test]
fn test_template_config_context_processors() {
	let config = TemplateConfig::default();

	// Check that default context processors are configured
	assert!(config.options.contains_key("context_processors"));

	let processors = config.options.get("context_processors").unwrap();
	assert!(processors.is_array());
}
