//! URL configuration for the polls application.
//!
//! Both submodules use `#[url_patterns(InstalledApp::polls, mode = ...)]`,
//! so the framework auto-registers them via inventory. The WASM entry
//! point looks up the client router through
//! `ClientLauncher::router_client(client_url_patterns)`, and the native
//! aggregator does not need to mount the server router explicitly.
//!
//! - `server_urls` — `#[url_patterns(..., mode = server)]` → `ServerRouter`.
//!   The polls app exposes every dynamic data path through `#[server_fn]`,
//!   so this router is intentionally empty today; it is kept for symmetry
//!   with `users` and as a documented landing zone for any future
//!   native-only HTTP endpoints (CSV export, RSS, etc.). See
//!   `server_urls.rs` for the full rationale.
//! - `client_router` — `#[url_patterns(..., mode = client)]` → `ClientRouter`
//!   mounted by `src/config/urls.rs` via `client_url_patterns()` and
//!   discovered through the `inventory::submit!(ClientRouterRegistration)`
//!   emitted by `#[routes(..., client_inventory)]`.

#[cfg(native)]
pub mod server_urls;

#[cfg(wasm)]
pub mod client_router;
