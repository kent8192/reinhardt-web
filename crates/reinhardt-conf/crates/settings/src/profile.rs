//! Profile and environment support
//!
//! Provides Django-style environment profiles (development, staging, production)
//! with cascading configuration support.

use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;

/// Application profile/environment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    /// Development environment (default)
    Development,
    /// Staging/testing environment
    Staging,
    /// Production environment
    Production,
    /// Custom profile
    Custom,
}

impl Profile {
    /// Get the profile from environment variable
    ///
    /// Checks REINHARDT_ENV, REINHARDT_SETTINGS_MODULE, and ENVIRONMENT variables
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::profile::Profile;
    ///
    /// // Returns the profile from environment variables
    /// if let Some(profile) = Profile::from_env() {
    ///     println!("Running in {:?} mode", profile);
    /// }
    /// ```
    pub fn from_env() -> Option<Self> {
        // Try REINHARDT_ENV first
        if let Ok(env_val) = env::var("REINHARDT_ENV") {
            return Self::from_str(&env_val);
        }

        // Try ENVIRONMENT
        if let Ok(env_val) = env::var("ENVIRONMENT") {
            return Self::from_str(&env_val);
        }

        // Try to detect from REINHARDT_SETTINGS_MODULE
        if let Ok(settings_module) = env::var("REINHARDT_SETTINGS_MODULE") {
            if settings_module.contains("production") {
                return Some(Profile::Production);
            } else if settings_module.contains("staging") {
                return Some(Profile::Staging);
            } else if settings_module.contains("development") || settings_module.contains("dev") {
                return Some(Profile::Development);
            }
        }

        None
    }
    /// Parse profile from string
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::profile::Profile;
    ///
    /// assert_eq!(Profile::from_str("production"), Some(Profile::Production));
    /// assert_eq!(Profile::from_str("dev"), Some(Profile::Development));
    /// assert_eq!(Profile::from_str("staging"), Some(Profile::Staging));
    /// ```
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "development" | "dev" | "develop" => Some(Profile::Development),
            "staging" | "stage" | "test" => Some(Profile::Staging),
            "production" | "prod" => Some(Profile::Production),
            _ => Some(Profile::Custom),
        }
    }
    /// Get the profile name as a string
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::profile::Profile;
    ///
    /// assert_eq!(Profile::Development.as_str(), "development");
    /// assert_eq!(Profile::Production.as_str(), "production");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            Profile::Development => "development",
            Profile::Staging => "staging",
            Profile::Production => "production",
            Profile::Custom => "custom",
        }
    }
    /// Check if this is a production environment
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::profile::Profile;
    ///
    /// assert_eq!(Profile::Production.is_production(), true);
    /// assert_eq!(Profile::Development.is_production(), false);
    /// ```
    pub fn is_production(&self) -> bool {
        matches!(self, Profile::Production)
    }
    /// Check if this is a development environment
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::profile::Profile;
    ///
    /// assert_eq!(Profile::Development.is_development(), true);
    /// assert_eq!(Profile::Production.is_development(), false);
    /// ```
    pub fn is_development(&self) -> bool {
        matches!(self, Profile::Development)
    }
    /// Check if debug mode should be enabled by default
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::profile::Profile;
    ///
    /// assert_eq!(Profile::Development.default_debug(), true);
    /// assert_eq!(Profile::Production.default_debug(), false);
    /// ```
    pub fn default_debug(&self) -> bool {
        !self.is_production()
    }
    /// Get the .env file name for this profile
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_settings::profile::Profile;
    ///
    /// let profile = Profile::Development;
    /// assert_eq!(profile.env_file_name(), ".env.development");
    /// ```
    pub fn env_file_name(&self) -> String {
        match self {
            Profile::Custom => ".env".to_string(),
            _ => format!(".env.{}", self.as_str()),
        }
    }
}

impl Default for Profile {
    fn default() -> Self {
        Profile::Development
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_from_str() {
        assert_eq!(
            Profile::from_str("development").unwrap(),
            Profile::Development
        );
        assert_eq!(Profile::from_str("dev").unwrap(), Profile::Development);
        assert_eq!(Profile::from_str("staging").unwrap(), Profile::Staging);
        assert_eq!(Profile::from_str("stage").unwrap(), Profile::Staging);
        assert_eq!(
            Profile::from_str("production").unwrap(),
            Profile::Production
        );
        assert_eq!(Profile::from_str("prod").unwrap(), Profile::Production);
        assert_eq!(Profile::from_str("custom").unwrap(), Profile::Custom);
    }

    #[test]
    fn test_profile_as_str() {
        assert_eq!(Profile::Development.as_str(), "development");
        assert_eq!(Profile::Staging.as_str(), "staging");
        assert_eq!(Profile::Production.as_str(), "production");
    }

    #[test]
    fn test_profile_checks() {
        assert!(Profile::Production.is_production());
        assert!(!Profile::Development.is_production());

        assert!(Profile::Development.is_development());
        assert!(!Profile::Production.is_development());
    }

    #[test]
    fn test_default_debug() {
        assert_eq!(Profile::Development.default_debug(), true);
        assert_eq!(Profile::Staging.default_debug(), true);
        assert_eq!(Profile::Production.default_debug(), false);
    }

    #[test]
    fn test_env_file_name() {
        assert_eq!(Profile::Development.env_file_name(), ".env.development");
        assert_eq!(Profile::Staging.env_file_name(), ".env.staging");
        assert_eq!(Profile::Production.env_file_name(), ".env.production");
        assert_eq!(Profile::Custom.env_file_name(), ".env");
    }

    #[test]
    fn test_settings_profile_from_env() {
        unsafe {
            env::set_var("REINHARDT_ENV", "production");
        }
        assert_eq!(Profile::from_env().unwrap(), Profile::Production);
        unsafe {
            env::remove_var("REINHARDT_ENV");
        }

        unsafe {
            env::set_var("ENVIRONMENT", "development");
        }
        assert_eq!(Profile::from_env().unwrap(), Profile::Development);
        unsafe {
            env::remove_var("ENVIRONMENT");
        }
    }
}
