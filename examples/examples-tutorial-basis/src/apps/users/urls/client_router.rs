//! Client-side routing for the users application (login/logout pages).
//!
//! Routes are auto-prefixed with `/users/` from `InstalledApp::users`, so the
//! relative path `/login/` resolves to `/users/login/` at runtime. Each
//! route is registered with a stable name (`users:login`, `users:logout`)
//! so callers can resolve URLs via `ResolvedUrls::resolve_client_url(...)`.
use crate::client::pages::{login_page, logout_page, signup_page};
use crate::config::apps::InstalledApp;
use reinhardt::ClientRouter;
use reinhardt::url_patterns;
#[url_patterns(InstalledApp::users, mode = client)]
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
		.named_route("login", "/login/", login_page)
		.named_route("logout", "/logout/", logout_page)
		.named_route("signup", "/signup/", signup_page)
}
