//! URL configuration for the users application.
//!
//! This module is target-neutral so native and WASM builds share one aggregate
//! surface for the split app-local router modules.

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

/// Client-side routes for login/logout/signup pages.
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

/// Reverse a named users client route.
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
				panic!("failed to reverse users client route `{name}`: {error}")
			})
	}
}
