//! Server-side URL configuration for the polls application.
//!
//! The `#[url_patterns]` macro auto-registers this router via inventory and
//! derives the path prefix from `InstalledApp::polls` (= `"polls"`), so the
//! aggregating `config/urls.rs` does not need an explicit `.mount("/polls/", ...)`
//! call.

use reinhardt::ServerRouter;
use reinhardt::url_patterns;

use crate::apps::polls::views;
use crate::config::apps::InstalledApp;

#[url_patterns(InstalledApp::polls, mode = server)]
pub fn server_url_patterns() -> ServerRouter {
	ServerRouter::new()
		.endpoint(views::index)
		.endpoint(views::detail)
		.endpoint(views::results)
		.endpoint(views::vote)
}
