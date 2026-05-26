//! URL configuration for the users application.

#[cfg(native)]
pub mod server_urls;

#[cfg(wasm)]
pub mod client_router;
