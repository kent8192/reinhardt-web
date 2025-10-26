//! Integration tests for Settings System
//!
//! These tests verify that reinhardt-settings works correctly with loading,
//! validation, and access patterns.

use reinhardt_settings::{DatabaseConfig, Settings, TemplateConfig};
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// Settings Loading Tests
// ============================================================================

#[test]
fn test_load_default_settings() {
    let settings = Settings::default();

    // Verify default values
    assert_eq!(settings.debug, true);
    assert_eq!(settings.language_code, "en-us");
    assert_eq!(settings.time_zone, "UTC");
    assert_eq!(settings.use_i18n, true);
    assert_eq!(settings.use_tz, true);
    assert_eq!(settings.static_url, "/static/");
    assert_eq!(settings.media_url, "/media/");
    assert_eq!(settings.append_slash, true);
}

#[test]
fn test_load_custom_settings() {
    let settings = Settings::new(PathBuf::from("/app"), "super-secret-key".to_string());

    assert_eq!(settings.base_dir, PathBuf::from("/app"));
    assert_eq!(settings.secret_key, "super-secret-key");
    assert_eq!(settings.debug, true); // Still defaults to true
}

#[test]
fn test_settings_with_root_urlconf() {
    let settings = Settings::default().with_root_urlconf("myproject.urls");

    assert_eq!(settings.root_urlconf, "myproject.urls");
}

// ============================================================================
// Settings Modification Tests
// ============================================================================

#[test]
fn test_add_installed_app() {
    let mut settings = Settings::default();

    let initial_count = settings.installed_apps.len();
    settings.add_app("myapp");

    assert_eq!(settings.installed_apps.len(), initial_count + 1);
    assert!(settings.installed_apps.contains(&"myapp".to_string()));
}

#[test]
fn test_add_middleware() {
    let mut settings = Settings::default();

    let initial_count = settings.middleware.len();
    settings.add_middleware("myapp.middleware.CustomMiddleware");

    assert_eq!(settings.middleware.len(), initial_count + 1);
    assert!(settings
        .middleware
        .contains(&"myapp.middleware.CustomMiddleware".to_string()));
}

#[test]
fn test_modify_security_settings() {
    let mut settings = Settings::default();

    // Modify security settings
    settings.debug = false;
    settings.secure_ssl_redirect = true;
    settings.secure_hsts_seconds = Some(31536000);
    settings.session_cookie_secure = true;
    settings.csrf_cookie_secure = true;

    // Verify security configuration
    assert_eq!(settings.debug, false);
    assert_eq!(settings.secure_ssl_redirect, true);
    assert_eq!(settings.secure_hsts_seconds, Some(31536000));
    assert_eq!(settings.session_cookie_secure, true);
    assert_eq!(settings.csrf_cookie_secure, true);
}

// ============================================================================
// Database Configuration Tests
// ============================================================================
// NOTE: Basic DatabaseConfig initialization tests (sqlite, postgresql, mysql)
// are in crates/reinhardt-apps/tests/test_settings.rs as unit tests.
// These single-crate tests don't require integration testing.

#[test]
fn test_multiple_database_configs() {
    let mut settings = Settings::default();

    // Add additional databases
    settings
        .databases
        .insert("cache".to_string(), DatabaseConfig::sqlite("cache.db"));
    settings.databases.insert(
        "analytics".to_string(),
        DatabaseConfig::postgresql("analytics", "user", "pass", "localhost", 5432),
    );

    assert_eq!(settings.databases.len(), 3); // default + cache + analytics
    assert!(settings.databases.contains_key("default"));
    assert!(settings.databases.contains_key("cache"));
    assert!(settings.databases.contains_key("analytics"));
}

// ============================================================================
// Template Configuration Tests
// ============================================================================

#[test]
fn test_template_config_default() {
    let config = TemplateConfig::default();

    assert_eq!(config.backend, "reinhardt.template.backends.jinja2.Jinja2");
    assert_eq!(config.app_dirs, true);
    assert!(config.dirs.is_empty());
    assert!(config.options.contains_key("context_processors"));
}

#[test]
fn test_template_config_custom_dirs() {
    let config = TemplateConfig::new("MyTemplateBackend")
        .add_dir("/app/templates")
        .add_dir("/app/custom_templates");

    assert_eq!(config.backend, "MyTemplateBackend");
    assert_eq!(config.dirs.len(), 2);
    assert_eq!(config.dirs[0], PathBuf::from("/app/templates"));
    assert_eq!(config.dirs[1], PathBuf::from("/app/custom_templates"));
}

// ============================================================================
// Settings Serialization Tests
// ============================================================================

#[test]
fn test_settings_serialization() {
    let settings = Settings::default();

    // Serialize to JSON
    let json = serde_json::to_string(&settings).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("debug"));
    assert!(json.contains("secret_key"));
}

#[test]
fn test_settings_deserialization() {
    let json = r#"{
        "base_dir": ".",
        "secret_key": "test-key",
        "debug": false,
        "allowed_hosts": ["example.com"],
        "installed_apps": ["app1", "app2"],
        "middleware": ["middleware1"],
        "root_urlconf": "urls",
        "databases": {},
        "templates": [],
        "static_url": "/static/",
        "static_root": null,
        "media_url": "/media/",
        "media_root": null,
        "language_code": "en",
        "time_zone": "UTC",
        "use_i18n": true,
        "use_tz": true,
        "default_auto_field": "AutoField",
        "secure_proxy_ssl_header": null,
        "secure_ssl_redirect": false,
        "secure_hsts_seconds": null,
        "secure_hsts_include_subdomains": false,
        "secure_hsts_preload": false,
        "session_cookie_secure": false,
        "csrf_cookie_secure": false,
        "append_slash": true
    }"#;

    let settings: Settings = serde_json::from_str(json).unwrap();
    assert_eq!(settings.debug, false);
    assert_eq!(settings.secret_key, "test-key");
    assert_eq!(settings.allowed_hosts, vec!["example.com"]);
}

// ============================================================================
// Settings Validation Tests
// ============================================================================

#[test]
fn test_production_settings_validation() {
    let mut settings = Settings::default();

    // Simulate production configuration
    settings.debug = false;
    settings.allowed_hosts = vec!["example.com".to_string(), "www.example.com".to_string()];
    settings.secure_ssl_redirect = true;
    settings.session_cookie_secure = true;
    settings.csrf_cookie_secure = true;

    // Verify production settings
    assert_eq!(settings.debug, false);
    assert!(!settings.allowed_hosts.is_empty());
    assert_eq!(settings.secure_ssl_redirect, true);
    assert_eq!(settings.session_cookie_secure, true);
    assert_eq!(settings.csrf_cookie_secure, true);
}

#[test]
fn test_required_settings_present() {
    let settings = Settings::default();

    // Verify required fields are present
    assert!(!settings.secret_key.is_empty());
    assert!(!settings.installed_apps.is_empty());
    assert!(!settings.middleware.is_empty());
    assert!(!settings.databases.is_empty());
}

// ============================================================================
// Settings Access Pattern Tests
// ============================================================================

#[test]
fn test_nested_settings_access() {
    let mut settings = Settings::default();

    // Access nested database configuration
    let default_db = settings.databases.get("default").unwrap();
    assert_eq!(default_db.engine, "reinhardt.db.backends.sqlite3");
    assert_eq!(default_db.name, "db.sqlite3");

    // Modify nested setting
    settings
        .databases
        .insert("test".to_string(), DatabaseConfig::sqlite("test.db"));
    assert_eq!(settings.databases.len(), 2);
}

#[test]
fn test_settings_immutability_pattern() {
    let settings = Settings::default();

    // Clone for immutability pattern
    let mut modified_settings = settings.clone();
    modified_settings.debug = false;

    // Original remains unchanged
    assert_eq!(settings.debug, true);
    assert_eq!(modified_settings.debug, false);
}
