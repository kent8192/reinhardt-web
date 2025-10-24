//! Staging environment settings for {{ project_name }}
//!
//! This configuration is used for the staging environment.
//! It imports and overrides base settings with staging-specific configuration.

use super::base::{get_base_settings, base_dir};
use reinhardt_core::{Settings, DatabaseConfig};
use std::env;

/// Get settings for staging environment
pub fn get_settings() -> Settings {
    // Import all base settings with debug=false
    let secret_key = env::var("SECRET_KEY")
        .expect("SECRET_KEY environment variable must be set in staging");
    let mut settings = get_base_settings(secret_key, false);

    // Override: Staging-specific allowed hosts
    settings.allowed_hosts = vec![
        "staging.{{ project_name }}.com".to_string(),
        "*.{{ project_name }}-staging.com".to_string(),
    ];

    // Override: Use PostgreSQL for staging
    let db_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| {
            "postgresql://user:password@localhost:5432/{{ project_name }}_staging".to_string()
        });

    settings.databases.insert(
        "default".to_string(),
        DatabaseConfig::postgresql(
            env::var("DB_NAME").unwrap_or_else(|_| "{{ project_name }}_staging".to_string()),
            env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
            env::var("DB_PASSWORD").unwrap_or_else(|_| "password".to_string()),
            env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            env::var("DB_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .unwrap_or(5432),
        ),
    );

    // Staging-specific: Enable some debugging but with security measures
    settings.debug = false;

    // Override: Static files configuration for staging
    settings.static_root = Some(base_dir().join("staticfiles"));

    settings
}
