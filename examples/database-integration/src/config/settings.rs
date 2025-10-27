//! Settings module for example-rest-api
//!
//! This module provides environment-specific settings configuration.

pub mod base;
pub mod local;
pub mod staging;
pub mod production;

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
use std::env;

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
use reinhardt_core::Settings;

/// Get settings based on environment variable
///
/// Reads the REINHARDT_ENV environment variable to determine which settings to load.
/// Defaults to "local" if not set.
#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub fn get_settings() -> Settings {
    let env = env::var("REINHARDT_ENV").unwrap_or_else(|_| "local".to_string());

    match env.as_str() {
        "production" => production::get_settings(),
        "staging" => staging::get_settings(),
        "local" | _ => local::get_settings(),
    }
}

#[cfg(any(reinhardt_unavailable, reinhardt_version_mismatch))]
pub fn get_settings() -> () {
    ()
}
