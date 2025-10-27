//! Production environment settings for example-rest-api

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub use available::*;

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
mod available {
    use reinhardt_core::Settings;
    use std::env;
    use super::base::get_base_settings;

    /// Get production environment settings
    pub fn get_settings() -> Settings {
        // Load secret key from environment
        let secret_key = env::var("SECRET_KEY")
            .expect("SECRET_KEY environment variable must be set in production");

        let mut settings = get_base_settings(secret_key, false);

        // Production-specific overrides
        settings.debug = false;

        // Load allowed hosts from environment (required in production)
        let allowed_hosts = env::var("ALLOWED_HOSTS")
            .expect("ALLOWED_HOSTS environment variable must be set in production");
        settings.allowed_hosts = allowed_hosts
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Additional production security settings
        settings.secure_ssl_redirect = true;
        settings.session_cookie_secure = true;
        settings.csrf_cookie_secure = true;

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
