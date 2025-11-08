//! Test: Single app should compile successfully

use reinhardt_macros::installed_apps;

fn main() {
	installed_apps! {
		auth: "reinhardt.contrib.auth",
	}

	let app = InstalledApp::auth;
	assert_eq!(app.path(), "reinhardt.contrib.auth");

	println!("Test passed!");
}
