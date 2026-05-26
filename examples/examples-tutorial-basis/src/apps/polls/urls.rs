//! URL configuration for the polls application.
//!
//! - `server_urls` — `ServerRouter`. The polls app exposes every dynamic
//!   data path through `#[server_fn]`, so this router is intentionally
//!   empty today; it is kept for symmetry with `users` and as a
//!   documented landing zone for any future native-only HTTP endpoints
//!   (CSV export, RSS, etc.). See `server_urls.rs` for the full rationale.
//! - `client_router` — `ClientRouter` mounted by `src/config/urls.rs` via
//!   `client_url_patterns()`.

#[cfg(native)]
pub mod server_urls;

#[cfg(wasm)]
pub mod client_router;
