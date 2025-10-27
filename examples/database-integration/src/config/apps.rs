//! Application configuration for example-rest-api
//!
//! This module defines the installed applications using compile-time validation.

#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
use reinhardt_macros::installed_apps;

// Define installed applications with compile-time validation
// The macro will fail to compile if any referenced reinhardt.contrib.* app doesn't exist
#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
installed_apps! {
    auth: "reinhardt.contrib.auth",
    contenttypes: "reinhardt.contrib.contenttypes",
    sessions: "reinhardt.contrib.sessions",
    drf: "reinhardt.drf",
}

/// Get the list of installed applications
#[cfg(not(any(reinhardt_unavailable, reinhardt_version_mismatch)))]
pub fn get_installed_apps() -> Vec<String> {
    InstalledApp::all_apps()
}

#[cfg(any(reinhardt_unavailable, reinhardt_version_mismatch))]
pub fn get_installed_apps() -> Vec<String> {
    vec![]
}
