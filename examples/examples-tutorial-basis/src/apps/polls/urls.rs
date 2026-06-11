//! URL configuration for the polls application.
//!
//! - `server_urls` — server-side `#[server_fn]` registration for the app.
//! - `client_router` — `ClientRouter` mounted by `src/config/urls.rs` via
//!   `client_url_patterns()`.

#[cfg(native)]
pub mod server_urls;

#[cfg(native)]
pub fn server_url_patterns() -> reinhardt::ServerRouter {
	server_urls::server_url_patterns()
}

#[cfg(wasm)]
pub mod client_router;

#[cfg(wasm)]
pub fn client_url_patterns() -> reinhardt::ClientRouter {
	client_router::client_url_patterns()
}
