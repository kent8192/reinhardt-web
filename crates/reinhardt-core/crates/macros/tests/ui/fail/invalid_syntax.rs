//! Test: Invalid syntax should fail to compile

use reinhardt_macros::installed_apps;

fn main() {
	// This should fail because of invalid syntax (missing colon)
	installed_apps! {
		auth "reinhardt.contrib.auth",
	}
}
