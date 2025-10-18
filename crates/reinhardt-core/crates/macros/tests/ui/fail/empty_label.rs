//! Test: Empty app label should fail to compile

use reinhardt_macros::installed_apps;

fn main() {
    // This should fail because labels cannot be empty
    installed_apps! {
        : "reinhardt.contrib.auth",
    }
}
