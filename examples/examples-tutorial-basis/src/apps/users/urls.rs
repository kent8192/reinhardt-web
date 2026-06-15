//! URL configuration for the users application.
//!
//! - `server_url_patterns()` — server-side app router.
//! - `client_url_patterns()` — client-side app router.

pub mod client_router;

pub fn server_url_patterns() -> reinhardt::ServerRouter {
	crate::native_runtime::users_server_url_patterns()
}

pub fn client_url_patterns() -> reinhardt::ClientRouter {
	client_router::client_url_patterns()
}
