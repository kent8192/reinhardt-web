//! Client-side routing for the {{ app_name }} SPA.
//!
//! Routes are registered via inventory by `#[url_patterns(..., mode = client)]`
//! and namespaced under `InstalledApp::{{ app_name }}`
//! (e.g. `{{ app_name }}:index`). `ClientLauncher::router_client(...)`
//! discovers and mounts the result.
//!
//! # Placeholder note
//!
//! The freshly generated function returns an empty `ClientRouter`. Wire
//! the placeholder page (or your real pages) once they exist:
//!
//! ```rust,ignore
//! use reinhardt::ClientPath;
//! use crate::apps::{{ app_name }}::client::pages;
//!
//! ClientRouter::new()
//!     .named_route("placeholder", "/", pages::placeholder_page)
//!     .route_path(
//!         "/items/{id}/",
//!         |ClientPath(id): ClientPath<i64>| pages::item_detail_page(id),
//!     )
//! ```

use reinhardt::ClientRouter;
use reinhardt::url_patterns;

use crate::config::apps::InstalledApp;

#[url_patterns(InstalledApp::{{ app_name }}, mode = client)]
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
}
