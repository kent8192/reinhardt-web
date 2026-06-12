//! Client-side routing for the users application (login/logout pages).
//!
//! Each route is registered with a stable name (`users:login`,
//! `users:logout`) so callers can resolve URLs via the URL reverser.

use reinhardt::ClientRouter;

use crate::client::pages::{login_page, logout_page, signup_page};

pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
		.route("login", "/login/", login_page)
		.route("logout", "/logout/", logout_page)
		.route("signup", "/signup/", signup_page)
}

pub fn reverse(name: &str, params: &[(&str, &str)]) -> String {
	client_url_patterns()
		.reverse(name, params)
		.unwrap_or_else(|error| panic!("failed to reverse users client route `{name}`: {error}"))
}
