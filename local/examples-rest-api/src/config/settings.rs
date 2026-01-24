//! Settings module for example-rest-api
//!
//! This module provides environment-specific settings configuration using TOML files.

use reinhardt::core::serde::json;
use reinhardt::{
	DefaultSource, LowPriorityEnvSource, Profile, Settings, SettingsBuilder, TomlFileSource,
};
use std::env;

/// Get settings based on environment variable
///
/// Reads the REINHARDT_ENV environment variable to determine which settings to load.
/// Defaults to "local" if not set.
///
/// Priority order (highest to lowest):
/// 1. Environment-specific TOML file (e.g., `production.toml`)
/// 2. Base TOML file (`base.toml`)
/// 3. Environment variables with `REINHARDT_` prefix
/// 4. Default values
pub fn get_settings() -> Settings {
	let profile_str = env::var("REINHARDT_ENV").unwrap_or_else(|_| "local".to_string());
	let profile = Profile::parse(&profile_str);

	// Get the project root directory
	let base_dir = env::current_dir().expect("Failed to get current directory");
	let settings_dir = base_dir.join("settings");

	let merged = SettingsBuilder::new()
		.profile(profile)
		.add_source(
			DefaultSource::new()
				.with_value("debug", json::Value::Bool(true))
				.with_value("language_code", json::Value::String("en-us".to_string()))
				.with_value("time_zone", json::Value::String("UTC".to_string())),
		)
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.add_source(TomlFileSource::new(settings_dir.join("base.toml")))
		.add_source(TomlFileSource::new(
			settings_dir.join(format!("{}.toml", profile_str)),
		))
		.build()
		.expect("Failed to build settings");

	merged
		.into_typed()
		.expect("Failed to convert settings to Settings struct")
}
