//! Client-side routing for the users application (login/logout pages).
//!
//! Each route is registered with a stable name (`users:login`,
//! `users:logout`) so callers can resolve URLs via
//! `ResolvedUrls::resolve_client_url(...)`.

use reinhardt::ClientRouter;

use crate::client::pages::{login_page, logout_page, signup_page};

pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
		.route("login", "/login/", login_page)
		.route("logout", "/logout/", logout_page)
		.route("signup", "/signup/", signup_page)
}
