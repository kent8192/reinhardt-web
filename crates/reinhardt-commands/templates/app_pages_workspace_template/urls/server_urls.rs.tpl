//! Server-side URL configuration for the {{ app_name }} application crate.
//!
//! The `#[url_patterns]` attribute auto-registers this router via inventory
//! and derives the path prefix from `InstalledApp::{{ app_name }}`, so the
//! aggregating `config/urls.rs` does not need an explicit
//! `.mount("/{{ app_name }}/", ...)` call.

use reinhardt::ServerRouter;
use reinhardt::url_patterns;

#[allow(unused_imports)] // `views` will be used once endpoints are added.
use crate::views;
use {{ project_crate_name }}::config::apps::InstalledApp;

#[url_patterns(InstalledApp::{{ app_name }}, mode = server)]
pub fn server_url_patterns() -> ServerRouter {
	ServerRouter::new()
	// Register endpoints here, e.g.:
	//     .endpoint(views::index)
}
