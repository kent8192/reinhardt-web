//! Client-side routing for the {{ app_name }} SPA (workspace crate).
//!
//! Named routes are namespaced under the `{{ app_name }}` label (e.g.
//! `{{ app_name }}:index`). The WASM entry point must pass
//! `client_url_patterns()` explicitly to
//! `ClientLauncher::router_client(...)` (or merge it with other apps'
//! routers there) for the routes to become active.
//!
//! Path parameters use the typed `ClientPath<T>` extractor.

#[allow(unused_imports)] // `ClientPath` is used once typed-parameter routes are added.
use reinhardt::ClientPath;
use reinhardt::ClientRouter;

#[allow(unused_imports)] // `pages` will be used once client routes are added.
use crate::client::pages;

pub fn client_url_patterns() -> ClientRouter {
	ClientRouter::new()
	// Register client-side routes here, e.g.:
	//     .route("index", "/", pages::index_page)
	//     .route_path(
	//         "detail",
	//         "/items/{id}/",
	//         |ClientPath(id): ClientPath<i64>| pages::item_detail_page(id),
	//     )
}
