//! URL configuration for the polls application.
//!
//! - `server_urls` — server-side `#[server_fn]` registration for the app.
//! - `client_router` — client-side routes mounted by `src/config/urls.rs`.

#[cfg(native)]
pub mod server_urls;

#[cfg(wasm)]
pub mod client_router;
