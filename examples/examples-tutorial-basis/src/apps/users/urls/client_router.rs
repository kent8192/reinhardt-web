//! Client-side routes for login/logout/signup pages.

use reinhardt::ClientRouter;
#[cfg(not(client))]
use reinhardt::pages::component::Page;

#[cfg(client)]
use crate::apps::users::client::components;

/// Client-side routes for login/logout/signup pages.
pub fn client_url_patterns() -> ClientRouter {
	#[cfg(client)]
	{
		return ClientRouter::new()
			.component(components::login_page::login_page)
			.component(components::logout_page::logout_page)
			.component(components::signup_page::signup_page);
	}

	#[cfg(not(client))]
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
