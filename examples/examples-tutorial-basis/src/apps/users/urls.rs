//! URL configuration for the users application.
//!
//! This module is target-neutral so native and WASM builds share one route
//! table and reverse-helper surface.

use reinhardt::{ClientRouter, ServerRouter};

use super::pages;

/// Server-side app router.
pub fn server_url_patterns() -> ServerRouter {
	#[cfg(server)]
	{
		super::server::urls::server_url_patterns()
	}
	#[cfg(not(server))]
	{
		ServerRouter::new()
	}
}

/// Client-side routes for login/logout/signup pages.
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
		.route("login", "/login/", pages::login_page)
		.route("logout", "/logout/", pages::logout_page)
		.route("signup", "/signup/", pages::signup_page)
}

/// Reverse a named users client route.
pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
	client_url_patterns()
		.reverse(name, params)
		.unwrap_or_else(|error| panic!("failed to reverse users client route `{name}`: {error}"))
}
