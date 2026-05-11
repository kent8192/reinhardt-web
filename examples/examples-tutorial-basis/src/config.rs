//! Configuration module for examples-tutorial-basis

// `apps` declares `InstalledApp`, which `#[url_patterns(InstalledApp::polls, ...)]`
// references from both the server-side router (`apps/polls/urls/server_urls.rs`)
// and the client-side router (`apps/polls/urls/client_router.rs`), so it must
// be available on both targets.
pub mod apps;
#[cfg(native)]
pub mod settings;
pub mod urls;
#[cfg(native)]
pub mod wasm;
