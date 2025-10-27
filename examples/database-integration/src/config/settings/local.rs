//! Local development settings for example-database-integration

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub use available::*;

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
mod available {
    use reinhardt_core::Settings;
    use super::base::get_base_settings;

    /// Get local development settings
    pub fn get_settings() -> Settings {
        use reinhardt_core::DatabaseConfig;
        use std::env;

        // Use a simple secret key for development
        // WARNING: Never use this in production!
        let secret_key = "dev-secret-key-not-for-production".to_string();

        let mut settings = get_base_settings(secret_key, true);

        // Development-specific overrides
        settings.debug = true;

        // Allow all hosts in development
        settings.allowed_hosts = vec!["*".to_string()];

        // Database configuration for local development
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://reinhardt:reinhardt_dev@localhost:5432/reinhardt_examples".to_string()
        });

        settings.database = Some(DatabaseConfig {
            url: database_url,
            max_connections: 10,
            min_connections: 1,
            connect_timeout: std::time::Duration::from_secs(30),
            idle_timeout: Some(std::time::Duration::from_secs(600)),
        });

        settings
    }
}

#[cfg(any(reinhardt_unavailable, reinhardt_version_mismatch))]
pub use unavailable::*;

#[cfg(any(reinhardt_unavailable, reinhardt_version_mismatch))]
mod unavailable {
    pub fn get_settings() -> () {
        ()
    }
}
