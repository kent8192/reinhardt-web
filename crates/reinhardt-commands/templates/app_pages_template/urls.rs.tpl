//! URL configuration for the {{ app_name }} application.
//!
//! - `server_urls` — `ServerRouter`
//! - `client_router` — `ClientRouter`

#[cfg(server)]
pub mod server_urls;

#[cfg(client)]
pub mod client_router;
