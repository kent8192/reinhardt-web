//! Application configuration for examples-twitter
//!
//! This module defines the installed applications using compile-time validation.

use reinhardt::installed_apps;

// Define installed applications with compile-time validation
// The macro will fail to compile if any referenced reinhardt.contrib.* app doesn't exist
installed_apps! {
	auth: "reinhardt.contrib.auth",
	contenttypes: "reinhardt.contrib.contenttypes",
	sessions: "reinhardt.contrib.sessions",
	drf: "reinhardt.drf",
	tweet: "tweet",
}

/// Get the list of installed applications
pub fn get_installed_apps() -> Vec<String> {
	InstalledApp::all_apps()
}
