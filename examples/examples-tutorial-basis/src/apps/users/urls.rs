//! URL configuration for the users application.
//!
//! - `server_url_patterns()` — server-side app router.
//! - `client_url_patterns()` — client-side app router.

#[cfg(server)]
pub mod server_urls;

#[cfg(client)]
pub mod client_router;

#[cfg(server)]
pub fn server_url_patterns() -> reinhardt::ServerRouter {
	server_urls::server_url_patterns()
}

#[cfg(client)]
pub fn client_url_patterns() -> reinhardt::ClientRouter {
	client_router::client_url_patterns()
}
