//! Configuration module for {{ project_name }}.

// `apps` declares `InstalledApp`, which `#[url_patterns(InstalledApp::<app>, ...)]`
// references from both the server-side router (`apps/<app>/urls/server_urls.rs`)
// and the client-side router (`apps/<app>/urls/client_router.rs`), so it must
// be available on both targets.
pub mod apps;
#[cfg(server)]
pub mod settings;
pub mod urls;
#[cfg(server)]
pub mod wasm;
