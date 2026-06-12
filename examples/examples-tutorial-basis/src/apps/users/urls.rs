//! URL configuration for the users application.
//!
//! - `server_urls` — server-side `#[server_fn]` registration for the app.
//! - `client_router` — client-side routes mounted by `src/config/urls.rs`.

#[cfg(server)]
pub mod server_urls;

#[cfg(client)]
pub mod client_router;
