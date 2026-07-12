//! Client-side routes for login/logout/signup pages.

use crate::apps::users::client::components;
use reinhardt::ClientRouter;

/// Client-side routes for login/logout/signup pages.
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
		.component(components::login_page::login_page)
		.component(components::logout_page::logout_page)
		.component(components::signup_page::signup_page)
}

/// Reverse a named users client route.
pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
	client_url_patterns()
		.reverse(name, params)
		.unwrap_or_else(|error| panic!("failed to reverse users client route `{name}`: {error}"))
}
