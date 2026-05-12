//! URL configuration for the {{ app_name }} application crate.
//!
//! Both submodules use `#[url_patterns(InstalledApp::{{ app_name }}, mode = ...)]`,
//! so the framework auto-registers them via inventory. The WASM entry point
//! looks up the client router through
//! `ClientLauncher::router_client(client_url_patterns)`, and the native
//! aggregator does not need to mount the server router explicitly.
//!
//! - `server_urls` — `#[url_patterns(..., mode = server)]` → `ServerRouter`
//! - `client_router` — `#[url_patterns(..., mode = client)]` → `ClientRouter`

#[cfg(server)]
pub mod server_urls;

#[cfg(client)]
pub mod client_router;
