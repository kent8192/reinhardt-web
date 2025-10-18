//! Test: Enum operations should work correctly

use reinhardt_macros::installed_apps;
use std::str::FromStr;

fn main() {
    installed_apps! {
        auth: "reinhardt.contrib.auth",
        sessions: "reinhardt.contrib.sessions",
    }

    // Test Display
    let app = InstalledApp::auth;
    assert_eq!(format!("{}", app), "reinhardt.contrib.auth");

    // Test FromStr
    let parsed = InstalledApp::from_str("reinhardt.contrib.auth").unwrap();
    assert_eq!(parsed, InstalledApp::auth);

    // Test equality
    assert_eq!(InstalledApp::auth, InstalledApp::auth);
    assert_ne!(InstalledApp::auth, InstalledApp::sessions);

    // Test Clone/Copy
    let app2 = app;
    assert_eq!(app, app2);

    println!("Test passed!");
}
