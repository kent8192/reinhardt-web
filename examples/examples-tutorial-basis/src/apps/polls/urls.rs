//! URL configuration for the polls application.
//!
//! This module is intentionally target-neutral. Native builds aggregate the
//! split app-local router modules, while WASM builds use the same client route
//! table and reverse helpers.

use reinhardt::{ClientRouter, ServerRouter};

#[cfg(client)]
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
	#[cfg(client)]
	{
		client_router::client_url_patterns()
	}
	#[cfg(not(client))]
	{
		ClientRouter::new()
	}
}

/// Reverse a named polls client route.
pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
	#[cfg(client)]
	{
		client_router::reverse(name, params)
	}
	#[cfg(not(client))]
	{
		ClientRouter::new()
			.reverse(name, params)
			.unwrap_or_else(|error| {
				panic!("failed to reverse polls client route `{name}`: {error}")
			})
	}
}
