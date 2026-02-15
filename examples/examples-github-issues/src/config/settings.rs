//! Settings module for examples-github-issues
//!
//! This module provides environment-specific settings configuration using TOML files.
//!
//! ## Configuration Structure
//!
//! Settings are loaded from TOML files in the `settings/` directory:
//! - `base.toml` - Common settings across all environments
//! - `local.toml` - Local development settings
//! - `staging.toml` - Staging environment settings
//! - `production.toml` - Production environment settings
//!
//! ## Priority Order
//!
//! Settings are merged with the following priority (highest to lowest):
//! 1. Environment-specific TOML file (e.g., `production.toml`)
//! 2. Base TOML file (`base.toml`)
//! 3. Environment variables with `REINHARDT_` prefix
//! 4. Default values
//!
//! ## Environment Selection
//!
//! The environment is determined by the `REINHARDT_ENV` environment variable:
//! - `local` or `development` → loads `local.toml`
//! - `staging` → loads `staging.toml`
//! - `production` → loads `production.toml`
//!
//! If `REINHARDT_ENV` is not set, it defaults to `local`.

use reinhardt::Settings;
use reinhardt::conf::settings::builder::SettingsBuilder;
use reinhardt::conf::settings::profile::Profile;
use reinhardt::conf::settings::sources::{DefaultSource, LowPriorityEnvSource, TomlFileSource};
use std::env;

/// Get settings based on environment variable
///
/// Reads the REINHARDT_ENV environment variable to determine which settings to load.
/// Defaults to "local" if not set.
///
/// # Examples
///
/// ```no_run
/// use examples_github_issues::config::settings::get_settings;
///
/// let settings = get_settings();
/// println!("Debug mode: {}", settings.debug);
/// ```
///
/// # Panics
///
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
				// Core settings
				.with_value(
					"base_dir",
					serde_json::Value::String(base_dir.to_string_lossy().to_string()),
				)
				.with_value(
					"secret_key",
					serde_json::Value::String("test-secret-key-for-development-only".to_string()),
				)
				.with_value("debug", serde_json::Value::Bool(false))
				.with_value("allowed_hosts", serde_json::Value::Array(vec![]))
				.with_value("installed_apps", serde_json::Value::Array(vec![]))
				.with_value("middleware", serde_json::Value::Array(vec![]))
				.with_value("root_urlconf", serde_json::Value::String(String::new()))
				.with_value(
					"databases",
					serde_json::Value::Object(serde_json::Map::new()),
				)
				.with_value("templates", serde_json::Value::Array(vec![]))
				// Static/Media files
				.with_value(
					"static_url",
					serde_json::Value::String("/static/".to_string()),
				)
				.with_value("staticfiles_dirs", serde_json::Value::Array(vec![]))
				.with_value(
					"media_url",
					serde_json::Value::String("/media/".to_string()),
				)
				// Internationalization
				.with_value(
					"language_code",
					serde_json::Value::String("en-us".to_string()),
				)
				.with_value("time_zone", serde_json::Value::String("UTC".to_string()))
				.with_value("use_i18n", serde_json::Value::Bool(true))
				.with_value("use_tz", serde_json::Value::Bool(true))
				.with_value(
					"default_auto_field",
					serde_json::Value::String("BigAutoField".to_string()),
				)
				// Security settings
				.with_value("secure_ssl_redirect", serde_json::Value::Bool(false))
				.with_value(
					"secure_hsts_include_subdomains",
					serde_json::Value::Bool(false),
				)
				.with_value("secure_hsts_preload", serde_json::Value::Bool(false))
				.with_value("session_cookie_secure", serde_json::Value::Bool(false))
				.with_value("csrf_cookie_secure", serde_json::Value::Bool(false))
				.with_value("append_slash", serde_json::Value::Bool(true))
				// Contact lists
				.with_value("admins", serde_json::Value::Array(vec![]))
				.with_value("managers", serde_json::Value::Array(vec![])),
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
}
