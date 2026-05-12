//! Client-side routing for the {{ app_name }} SPA.
//!
//! Routes are declared with `#[url_patterns(InstalledApp::{{ app_name }}, mode = client)]`,
//! which auto-registers the router via inventory and namespaces named routes
//! under the `InstalledApp::{{ app_name }}` label (e.g. `{{ app_name }}:index`).
//! The WASM entry point consumes this builder through
//! `ClientLauncher::router_client(...)`.
//!
//! Path parameters use the typed `ClientPath<T>` extractor.

#[allow(unused_imports)] // `ClientPath` is used once typed-parameter routes are added.
use reinhardt::ClientPath;
use reinhardt::ClientRouter;
use reinhardt::url_patterns;

#[allow(unused_imports)] // `pages` will be used once client routes are added.
use crate::client::pages;
use crate::config::apps::InstalledApp;

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
