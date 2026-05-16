//! Server-side URL configuration for the {{ app_name }} application.
//!
//! `#[url_patterns]` auto-registers this router via inventory and derives
//! the path prefix from `InstalledApp::{{ app_name }}`, so the aggregating
//! `config/urls.rs` does not need an explicit `.mount("/{{ app_name }}/", ...)`
//! call.
//!
//! # Placeholder note
//!
//! The freshly generated function returns an empty `ServerRouter`. Register
//! views/endpoints here:
//!
//! ```rust,ignore
//! use crate::apps::{{ app_name }}::views;
//!
//! ServerRouter::new()
//!     .endpoint(views::index)
//! ```

use reinhardt::ServerRouter;
use reinhardt::url_patterns;

use crate::config::apps::InstalledApp;

#[url_patterns(InstalledApp::{{ app_name }}, mode = server)]
pub fn server_url_patterns() -> ServerRouter {
	ServerRouter::new()
}
