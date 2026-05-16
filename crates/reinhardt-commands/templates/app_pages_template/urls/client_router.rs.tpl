//! Client-side routing for the {{ app_name }} SPA.
//!
//! `#[url_patterns(InstalledApp::{{ app_name }}, mode = client)]` wraps the
//! function below, emits a `client_url_resolvers` submodule, and namespaces
//! route names under `InstalledApp::{{ app_name }}`
//! (e.g. `{{ app_name }}:index`). The macro does NOT auto-mount this
//! router into `ClientLauncher`: pass `client_url_patterns()` explicitly
//! to `ClientLauncher::router_client(...)` in `src/client/lib.rs` (or
//! merge it with other apps' routers there) for the routes to become
//! active.
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
