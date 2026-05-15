//! Client-side routing for the users application (login/logout pages).
//!
//! Routes are auto-prefixed with `/users/` from `InstalledApp::users`, so the
//! relative path `/login/` resolves to `/users/login/` at runtime.

use reinhardt::ClientRouter;
use reinhardt::url_patterns;

use crate::client::pages::{login_page, logout_page};
use crate::config::apps::InstalledApp;

#[url_patterns(InstalledApp::users, mode = client)]
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
		.route("/login/", login_page)
		.route("/logout/", logout_page)
}
