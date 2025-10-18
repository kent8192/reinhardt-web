//! Test: Trailing comma should be allowed

use reinhardt_macros::installed_apps;

fn main() {
    // Trailing comma should be allowed
    installed_apps! {
        auth: "reinhardt.contrib.auth",
        sessions: "reinhardt.contrib.sessions",
    }

    let apps = InstalledApp::all_apps();
    assert_eq!(apps.len(), 2);

    println!("Test passed!");
}
