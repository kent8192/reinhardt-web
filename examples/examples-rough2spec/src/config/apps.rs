//! Application configuration for examples-rough2spec

use reinhardt::installed_apps;

installed_apps! {
    generate: "generate",
}

pub fn get_installed_apps() -> Vec<String> {
    InstalledApp::all_apps()
}
