//! Tests for the installed_apps! macro

use reinhardt_macros::installed_apps;
use rstest::rstest;

#[rstest]
fn test_installed_apps_basic() {
	installed_apps! {
		auth: "reinhardt.contrib.auth",
		contenttypes: "reinhardt.contrib.contenttypes",
	}

	let apps = InstalledApp::all_apps();
	assert_eq!(apps.len(), 2);
	assert!(apps.contains(&"reinhardt.contrib.auth".to_string()));
	assert!(apps.contains(&"reinhardt.contrib.contenttypes".to_string()));
}

#[rstest]
fn test_installed_apps_enum() {
	installed_apps! {
		auth: "reinhardt.contrib.auth",
		sessions: "reinhardt.contrib.sessions",
	}

	assert_eq!(InstalledApp::auth.path(), "reinhardt.contrib.auth");
	assert_eq!(InstalledApp::sessions.path(), "reinhardt.contrib.sessions");
}

#[rstest]
fn test_installed_apps_display() {
	installed_apps! {
		auth: "reinhardt.contrib.auth",
	}

	let app = InstalledApp::auth;
	assert_eq!(format!("{}", app), "reinhardt.contrib.auth");
}

#[rstest]
fn test_installed_apps_from_str() {
	installed_apps! {
		auth: "reinhardt.contrib.auth",
		sessions: "reinhardt.contrib.sessions",
	}

	use std::str::FromStr;

	let auth = InstalledApp::from_str("reinhardt.contrib.auth");
	assert!(auth.is_ok());
	assert_eq!(auth.unwrap(), InstalledApp::auth);

	let invalid = InstalledApp::from_str("invalid.app");
	assert!(invalid.is_err());
}

#[rstest]
fn test_installed_apps_with_user_apps() {
	installed_apps! {
		auth: "reinhardt.contrib.auth",
		myapp: "apps.myapp",
		another: "custom.another",
	}

	let apps = InstalledApp::all_apps();
	assert_eq!(apps.len(), 3);
	assert!(apps.contains(&"apps.myapp".to_string()));
	assert!(apps.contains(&"custom.another".to_string()));
}

#[rstest]
fn test_installed_apps_single_app() {
	installed_apps! {
		auth: "reinhardt.contrib.auth",
	}

	let apps = InstalledApp::all_apps();
	assert_eq!(apps.len(), 1);
}

#[rstest]
fn test_installed_apps_equality() {
	installed_apps! {
		auth: "reinhardt.contrib.auth",
		sessions: "reinhardt.contrib.sessions",
	}

	let app1 = InstalledApp::auth;
	let app2 = InstalledApp::auth;
	let app3 = InstalledApp::sessions;

	assert_eq!(app1, app2);
	assert_ne!(app1, app3);
}

#[rstest]
fn test_installed_apps_debug() {
	installed_apps! {
		auth: "reinhardt.contrib.auth",
	}

	let app = InstalledApp::auth;
	let debug_str = format!("{:?}", app);
	assert!(debug_str.contains("auth"));
}
