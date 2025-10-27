//! Base settings for example-rest-api project (RESTful)
//!
//! This module contains base settings shared across all environments.
//! Environment-specific settings (local, staging, production) import and override these settings.

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub use available::*;

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
mod available {
    use reinhardt_core::{Settings, DatabaseConfig};
    use std::path::PathBuf;
    use crate::config::apps::get_installed_apps;

    /// Get the base directory of the project
    pub fn base_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// Create base settings
    ///
    /// These settings are shared across all environments and should contain
    /// configuration that doesn't change between development, staging, and production.
    pub fn get_base_settings(secret_key: String, debug: bool) -> Settings {
        let settings = Settings::new(base_dir(), secret_key)
            .with_validated_apps(get_installed_apps);

        let mut settings = settings;
        settings.debug = debug;

        settings.middleware = vec![
            "reinhardt.middleware.security.SecurityMiddleware".to_string(),
            "reinhardt.contrib.sessions.middleware.SessionMiddleware".to_string(),
            "reinhardt.middleware.common.CommonMiddleware".to_string(),
            "reinhardt.middleware.csrf.CsrfViewMiddleware".to_string(),
            "reinhardt.contrib.auth.middleware.AuthenticationMiddleware".to_string(),
            "reinhardt.middleware.clickjacking.XFrameOptionsMiddleware".to_string(),
        ];

        settings.root_urlconf = "config.urls".to_string();

        // Internationalization
        settings.language_code = "en-us".to_string();
        settings.time_zone = "UTC".to_string();
        settings.use_i18n = true;
        settings.use_tz = true;

        // Static files (CSS, JavaScript, Images)
        settings.static_url = "/static/".to_string();

        // Default primary key field type
        settings.default_auto_field = "reinhardt.db.models.BigAutoField".to_string();

        settings
    }
}

#[cfg(any(reinhardt_unavailable, reinhardt_version_mismatch))]
pub use unavailable::*;

#[cfg(any(reinhardt_unavailable, reinhardt_version_mismatch))]
mod unavailable {
    pub fn base_dir() -> () {
        ()
    }

    pub fn get_base_settings(_secret_key: String, _debug: bool) -> () {
        ()
    }
}
