//! Test: Missing path should fail to compile

use reinhardt_macros::installed_apps;

fn main() {
	// This should fail because path is missing
	installed_apps! {
		auth:,
	}
}
