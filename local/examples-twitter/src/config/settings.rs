//! Settings module for examples-twitter
//!
//! This module provides environment-specific settings configuration using TOML files.
//! ## Configuration Structure
//! Settings are loaded from TOML files in the `settings/` directory:
//! - `base.toml` - Common settings across all environments
//! - `local.toml` - Local development settings
//! - `staging.toml` - Staging environment settings
//! - `production.toml` - Production environment settings
//! ## Priority Order
//! Settings are merged with the following priority (highest to lowest):
//! 1. Environment-specific TOML file (e.g., `production.toml`)
//! 2. Base TOML file (`base.toml`)
//! 3. Environment variables with `REINHARDT_` prefix
//! 4. Default values
//! ## Environment Selection
//! The environment is determined by the `REINHARDT_ENV` environment variable:
//! - `local` or `development` → loads `local.toml`
//! - `staging` → loads `staging.toml`
//! - `production` → loads `production.toml`
//!   If `REINHARDT_ENV` is not set, it defaults to `local`.

use reinhardt::Settings;
use reinhardt::conf::settings::AdvancedSettings;
use reinhardt::conf::settings::builder::SettingsBuilder;
use reinhardt::conf::settings::profile::Profile;
use reinhardt::conf::settings::sources::{DefaultSource, LowPriorityEnvSource, TomlFileSource};
use reinhardt::core::serde::json;
use std::env;
/// Get settings based on environment variable
///
/// Reads the REINHARDT_ENV environment variable to determine which settings to load.
/// Defaults to "local" if not set.
/// # Examples
/// ```no_run
/// use examples_twitter::config::settings::get_settings;
/// let settings = get_settings();
/// println!("Debug mode: {}", settings.debug);
/// ```
/// # Panics
/// Panics if:
/// - Settings files cannot be read
/// - Settings cannot be deserialized
/// - Required settings are missing
pub fn get_settings() -> Settings {
	let profile_str = env::var("REINHARDT_ENV").unwrap_or_else(|_| "local".to_string());
	let profile = Profile::parse(&profile_str);
	// Get the project root directory (parent of src/)
	let base_dir = env::current_dir().expect("Failed to get current directory");
	let settings_dir = base_dir.join("settings");
	// Build settings by merging sources in priority order
	let merged = SettingsBuilder::new()
        .profile(profile)
        // Lowest priority: Default values
        .add_source(
            DefaultSource::new()
                .with_value(
                    "base_dir",
                    json::Value::String(
                        base_dir
                            .to_str()
                            .expect("base_dir contains invalid UTF-8")
                            .to_string(),
                    ),
                )
                .with_value("debug", json::Value::Bool(true))
                .with_value(
                    "secret_key",
                    json::Value::String("insecure-dev-key-change-in-production".to_string()),
                )
                .with_value("allowed_hosts", json::Value::Array(vec![]))
                .with_value("installed_apps", json::Value::Array(vec![]))
                .with_value("middleware", json::Value::Array(vec![]))
                .with_value(
                    "root_urlconf",
                    json::Value::String("config.urls".to_string()),
                )
                .with_value("databases", json::json!({}))
                .with_value("templates", json::Value::Array(vec![]))
                .with_value(
                    "static_url",
                    json::Value::String("/static/".to_string()),
                )
                .with_value("staticfiles_dirs", json::Value::Array(vec![]))
                .with_value(
                    "media_url",
                    json::Value::String("/media/".to_string()),
                )
                .with_value(
                    "language_code",
                    json::Value::String("en-us".to_string()),
                )
                .with_value("time_zone", json::Value::String("UTC".to_string()))
                .with_value("use_i18n", json::Value::Bool(false))
                .with_value("use_tz", json::Value::Bool(false))
                .with_value(
                    "default_auto_field",
                    json::Value::String("reinhardt.db.models.BigAutoField".to_string()),
                )
                .with_value("secure_ssl_redirect", json::Value::Bool(false))
                .with_value(
                    "secure_hsts_include_subdomains",
                    json::Value::Bool(false),
                )
                .with_value("secure_hsts_preload", json::Value::Bool(false))
                .with_value("session_cookie_secure", json::Value::Bool(false))
                .with_value("csrf_cookie_secure", json::Value::Bool(false))
                .with_value("append_slash", json::Value::Bool(false))
                .with_value("admins", json::Value::Array(vec![]))
                .with_value("managers", json::Value::Array(vec![])),
        )
        // Low priority: Environment variables (for container overrides)
        .add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
        // Medium priority: Base TOML file
        .add_source(TomlFileSource::new(settings_dir.join("base.toml")))
        // Highest priority: Environment-specific TOML file
        .add_source(TomlFileSource::new(
            settings_dir.join(format!("{}.toml", profile_str)),
        ))
        .build()
        .expect("Failed to build settings");
	// Convert MergedSettings to reinhardt_core::Settings
	merged
		.into_typed()
		.expect("Failed to convert settings to Settings struct")
}
/// Get advanced settings based on environment variable
///
/// Similar to `get_settings()`, but returns `AdvancedSettings` which provides
/// more detailed configuration options including cache, CORS, email, logging, and sessions.
///
/// # Examples
///
/// ```no_run
/// use examples_twitter::config::settings::get_advanced_settings;
///
/// let settings = get_advanced_settings();
/// println!("Debug mode: {}", settings.debug);
/// println!("Database URL: {}", settings.database.url);
/// println!("Cache backend: {}", settings.cache.backend);
/// ```
///
/// # Panics
///
/// Panics if:
/// - Settings files cannot be read
/// - Settings cannot be deserialized
/// - Required settings are missing
pub fn get_advanced_settings() -> AdvancedSettings {
	let profile_str = env::var("REINHARDT_ENV").unwrap_or_else(|_| "local".to_string());
	let profile = Profile::parse(&profile_str);

	let base_dir = env::current_dir().expect("Failed to get current directory");
	let settings_dir = base_dir.join("settings");

	// Build settings by merging sources in priority order
	let merged = SettingsBuilder::new()
		.profile(profile)
		// Lowest priority: Default values
		.add_source(
			DefaultSource::new()
				.with_value("debug", json::Value::Bool(false))
				.with_value(
					"secret_key",
					json::Value::String(
						"change-me-in-production-must-be-at-least-32-chars".to_string(),
					),
				)
				.with_value(
					"allowed_hosts",
					json::Value::Array(vec![
						json::Value::String("localhost".to_string()),
						json::Value::String("127.0.0.1".to_string()),
					]),
				),
		)
		// Low priority: Environment variables (for container overrides)
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		// Medium priority: Base TOML file
		.add_source(TomlFileSource::new(settings_dir.join("base.toml")))
		// Highest priority: Environment-specific TOML file
		.add_source(TomlFileSource::new(
			settings_dir.join(format!("{}.toml", profile_str)),
		))
		.build()
		.expect("Failed to build advanced settings");

	// Convert MergedSettings to AdvancedSettings
	merged
		.into_typed::<AdvancedSettings>()
		.expect("Failed to convert settings to AdvancedSettings struct")
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_get_settings() {
		// This test requires settings files to exist
		// In a real project, you would set up test fixtures
		let settings = get_settings();
		assert!(!settings.secret_key.is_empty());
	}

	#[test]
	fn test_get_advanced_settings() {
		// This test requires settings files to exist
		let settings = get_advanced_settings();
		assert!(!settings.secret_key.is_empty());
		// Verify default values from AdvancedSettings::default()
		assert!(!settings.database.url.is_empty());
		assert!(!settings.cache.backend.is_empty());
	}
}
