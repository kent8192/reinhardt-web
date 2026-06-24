//! Native-side users route metadata for route aggregation and reversing.

use reinhardt::ClientRouter;
use reinhardt::pages::component::Page;

/// Client route names and paths without WASM component bodies.
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
		.route("login", "/login/", Page::empty)
		.route("logout", "/logout/", Page::empty)
		.route("signup", "/signup/", Page::empty)
}

/// Reverse a named users client route.
pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
	client_url_patterns()
		.reverse(name, params)
		.unwrap_or_else(|error| panic!("failed to reverse users client route `{name}`: {error}"))
}
