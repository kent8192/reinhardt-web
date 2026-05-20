//! Typed URL helpers for the users (authentication) routes, backed by
//! `ResolvedUrls`.
//!
//! These helpers delegate to
//! `ResolvedUrls::from_global().resolve_client_url(name, params)` so a route
//! pattern change in `apps::users::urls::client_router` requires no
//! component-level edits.
//!
//! Route names are namespaced `users:<name>` (see
//! `#[url_patterns(InstalledApp::users, mode = client)]`).
//!
//! See [#4644](https://github.com/kent8192/reinhardt-web/issues/4644) for a
//! proposal to codegen these helpers directly from `#[url_patterns]`.

use reinhardt::ClientUrlResolver;

use crate::config::urls::ResolvedUrls;

fn urls() -> ResolvedUrls {
	ResolvedUrls::from_global()
}

fn resolve(name: &str, params: &[(&str, &str)]) -> String {
	urls().resolve_client_url(name, params)
}

/// `/users/login/` — sign-in form.
pub fn login() -> String {
	resolve("users:login", &[])
}

/// `/users/logout/` — sign-out form.
pub fn logout() -> String {
	resolve("users:logout", &[])
}

/// `/users/signup/` — account-creation form.
pub fn signup() -> String {
	resolve("users:signup", &[])
}
