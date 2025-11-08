//! Test: No trailing comma should also work

use reinhardt_macros::installed_apps;

fn main() {
	// No trailing comma should also work
	installed_apps! {
		auth: "reinhardt.contrib.auth",
		sessions: "reinhardt.contrib.sessions"
	}

	let apps = InstalledApp::all_apps();
	assert_eq!(apps.len(), 2);

	println!("Test passed!");
}
