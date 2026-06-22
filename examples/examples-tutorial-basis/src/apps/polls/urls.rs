//! URL configuration for the polls application.
//!
//! This module is intentionally target-neutral. Native builds aggregate the
//! split app-local router modules, while WASM builds use the same client route
//! table and reverse helpers.

use reinhardt::{ClientRouter, ServerRouter};

pub mod client_router;

#[cfg(server)]
pub mod server_router;

/// Server-side app router.
pub fn server_url_patterns() -> ServerRouter {
	#[cfg(server)]
	{
		server_router::server_url_patterns()
	}
	#[cfg(not(server))]
	{
		ServerRouter::new()
	}
}

/// Client-side routing for the polls SPA.
pub fn client_url_patterns() -> ClientRouter {
	client_router::client_url_patterns()
}

/// Reverse a named polls client route.
pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
	client_router::reverse(name, params)
}
