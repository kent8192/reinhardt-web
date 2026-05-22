//! Server-side URL patterns for the users application.
//!
//! Defines no HTTP endpoints of its own — authentication is exposed via
//! server functions registered in `crate::config::urls::routes`. This empty
//! aggregator exists so the app label `users` is reachable through the
//! same `#[url_patterns]` discovery path as `polls`.
use crate::config::apps::InstalledApp;
use reinhardt::ServerRouter;
use reinhardt::url_patterns;
#[url_patterns(InstalledApp::users, mode = server)]
pub fn server_url_patterns() -> ServerRouter {
	ServerRouter::new()
}
