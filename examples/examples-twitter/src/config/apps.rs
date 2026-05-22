//! Application configuration for examples-twitter
//!
//! This module defines the installed applications using compile-time validation.
use reinhardt::installed_apps;
installed_apps! {
	auth : "auth", tweet : "tweet", profile : "profile", relationship : "relationship",
	dm : "dm",
}
/// Get the list of installed applications
pub fn get_installed_apps() -> Vec<String> {
	InstalledApp::all_apps()
}
