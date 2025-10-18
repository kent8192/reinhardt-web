//! Production environment settings for {{ project_name }}
//!
//! This configuration is used for the production environment.
//! It imports and overrides base settings with production-specific configuration.

use super::base::{get_base_settings, base_dir};
use reinhardt_core::{Settings, DatabaseConfig};
use std::env;

/// Get settings for production environment
pub fn get_settings() -> Settings {
    // Import all base settings with debug=false
    let secret_key = env::var("SECRET_KEY")
        .expect("SECRET_KEY environment variable must be set in production!");
    let mut settings = get_base_settings(secret_key, false);

    // SECURITY: Restrict allowed hosts in production
    settings.allowed_hosts = vec![
        "{{ project_name }}.com".to_string(),
        "www.{{ project_name }}.com".to_string(),
        "api.{{ project_name }}.com".to_string(),
    ];

    // Override: Use PostgreSQL for production
    let db_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL environment variable must be set in production!");

    settings.databases.insert(
        "default".to_string(),
        DatabaseConfig::postgresql(
            env::var("DB_NAME").expect("DB_NAME must be set"),
            env::var("DB_USER").expect("DB_USER must be set"),
            env::var("DB_PASSWORD").expect("DB_PASSWORD must be set"),
            env::var("DB_HOST").expect("DB_HOST must be set"),
            env::var("DB_PORT")
                .expect("DB_PORT must be set")
                .parse()
                .expect("DB_PORT must be a valid port number"),
        ),
    );

    // SECURITY: Ensure debug is false in production
    settings.debug = false;

    // Override: Static and media files configuration for production
    settings.static_root = Some(PathBuf::from("/var/www/{{ project_name }}/static"));
    settings.media_root = Some(PathBuf::from("/var/www/{{ project_name }}/media"));

    // Production-specific middleware (e.g., security headers, HTTPS redirect)
    settings.middleware.insert(
        0,
        "reinhardt.middleware.security.StrictSecurityMiddleware".to_string(),
    );

    settings
}
