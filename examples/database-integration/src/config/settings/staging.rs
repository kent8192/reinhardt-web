//! Staging environment settings for example-rest-api

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub use available::*;

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
mod available {
    use reinhardt_core::Settings;
    use std::env;
    use super::base::get_base_settings;

    /// Get staging environment settings
    pub fn get_settings() -> Settings {
        // Load secret key from environment
        let secret_key = env::var("SECRET_KEY")
            .expect("SECRET_KEY environment variable must be set in staging");

        let mut settings = get_base_settings(secret_key, false);

        // Staging-specific overrides
        settings.debug = false;

        // Load allowed hosts from environment
        let allowed_hosts = env::var("ALLOWED_HOSTS")
            .unwrap_or_else(|_| "".to_string());
        settings.allowed_hosts = allowed_hosts
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

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
