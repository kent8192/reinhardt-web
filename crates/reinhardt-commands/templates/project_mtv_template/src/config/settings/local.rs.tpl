//! Local/Development settings for {{ project_name }}
//!
//! This configuration is used for local development.
//! It imports and overrides base settings with development-specific configuration.

use super::base::{get_base_settings, base_dir};
use reinhardt_core::{Settings, DatabaseConfig};

/// Secret key for local development
/// SECURITY WARNING: This is only for development! Override in production!
const LOCAL_SECRET_KEY: &str = "{{ secret_key }}";

/// Get settings for local development
pub fn get_settings() -> Settings {
    // Import all base settings with debug=true
    let mut settings = get_base_settings(LOCAL_SECRET_KEY.to_string(), true);

    // Override: Allow all hosts in development
    settings.allowed_hosts = vec!["*".to_string()];

    // Override: Use SQLite for local development
    settings.databases.insert(
        "default".to_string(),
        DatabaseConfig::sqlite(base_dir().join("db.sqlite3").to_string_lossy().to_string()),
    );

    // Development-specific middleware (e.g., debug toolbar)
    // settings.middleware.push("reinhardt.middleware.debug.DebugToolbarMiddleware".to_string());

    // CORS settings for local API development
    // settings.allowed_hosts.push("localhost:3000".to_string()); // React/Vue dev server

    settings
}
