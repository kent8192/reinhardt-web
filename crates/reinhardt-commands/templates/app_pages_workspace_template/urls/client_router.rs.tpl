//! Client-side routing for the {{ app_name }} SPA (workspace crate).
//!
//! Routes are declared with `#[url_patterns(InstalledApp::{{ app_name }}, mode = client)]`,
//! which wraps the function below, emits a `client_url_resolvers`
//! submodule, and namespaces named routes under the
//! `InstalledApp::{{ app_name }}` label (e.g. `{{ app_name }}:index`). The
//! macro does NOT auto-register the router with `ClientLauncher`: the
//! WASM entry point must pass `client_url_patterns()` explicitly to
//! `ClientLauncher::router_client(...)` (or merge it with other apps'
//! routers there) for the routes to become active.
//!
//! Path parameters use the typed `ClientPath<T>` extractor.

#[allow(unused_imports)] // `ClientPath` is used once typed-parameter routes are added.
use reinhardt::ClientPath;
use reinhardt::ClientRouter;
use reinhardt::url_patterns;

#[allow(unused_imports)] // `pages` will be used once client routes are added.
use crate::client::pages;
use {{ project_crate_name }}::config::apps::InstalledApp;

#[url_patterns(InstalledApp::{{ app_name }}, mode = client)]
pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
	// Register client-side routes here, e.g.:
	//     .named_route("index", "/", pages::index_page)
	//     .route_path(
	//         "/items/{id}/",
	//         |ClientPath(id): ClientPath<i64>| pages::item_detail_page(id),
	//     )
}
