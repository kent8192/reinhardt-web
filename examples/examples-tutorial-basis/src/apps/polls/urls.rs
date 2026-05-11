//! URL configuration for the polls application.
//!
//! Both submodules use `#[url_patterns(InstalledApp::polls, mode = ...)]`,
//! so the framework auto-registers them via inventory. The WASM entry
//! point looks up the client router through
//! `ClientLauncher::router_client(client_url_patterns)`, and the native
//! aggregator does not need to mount the server router explicitly.
//!
//! - `server_urls` — `#[url_patterns(..., mode = server)]` → `ServerRouter`
//! - `client_router` — `#[url_patterns(..., mode = client)]` → `ClientRouter`

#[cfg(native)]
pub mod server_urls;

#[cfg(wasm)]
pub mod client_router;
