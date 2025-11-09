//! Settings module for example-rest-api
//!
//! This module provides environment-specific settings configuration using TOML files.

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
use reinhardt_conf::settings::builder::SettingsBuilder;
#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
use reinhardt_conf::settings::profile::Profile;
#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
use reinhardt_conf::settings::sources::{DefaultSource, LowPriorityEnvSource, TomlFileSource};
#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
use reinhardt_core::Settings;
#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
use std::env;

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub mod base;
#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub mod local;
#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub mod staging;
#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub mod production;

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
#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub fn get_settings() -> Settings {
    let profile_str = env::var("REINHARDT_ENV").unwrap_or_else(|_| "local".to_string());
    let profile = Profile::from_str(&profile_str).unwrap_or(Profile::Development);

    // Get the project root directory
    let base_dir = env::current_dir().expect("Failed to get current directory");
    let settings_dir = base_dir.join("settings");

    // Build settings by merging sources in priority order
    let merged = SettingsBuilder::new()
        .profile(profile)
        // Lowest priority: Default values
        .add_source(
            DefaultSource::new()
                .with_value("debug", serde_json::Value::Bool(false))
                .with_value(
                    "language_code",
                    serde_json::Value::String("en-us".to_string()),
                )
                .with_value("time_zone", serde_json::Value::String("UTC".to_string()))
                .with_value("use_i18n", serde_json::Value::Bool(true))
                .with_value("use_tz", serde_json::Value::Bool(true))
                .with_value("append_slash", serde_json::Value::Bool(true))
                .with_value(
                    "default_auto_field",
                    serde_json::Value::String("reinhardt.db.models.BigAutoField".to_string()),
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
        .expect("Failed to build settings");

    // Convert MergedSettings to reinhardt_core::Settings
    merged
        .into_typed()
        .expect("Failed to convert settings to Settings struct")
}

#[cfg(any(reinhardt_unavailable, reinhardt_version_mismatch))]
pub fn get_settings() -> () {
    ()
}
