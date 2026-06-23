//! Client-side routes for login/logout/signup pages.

use reinhardt::ClientRouter;

use crate::apps::users::pages;

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
