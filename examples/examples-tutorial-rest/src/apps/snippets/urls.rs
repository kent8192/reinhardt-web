//! URL configuration for the snippets app.
//!
//! This file is the canonical aggregator for the `snippets` app — it
//! exposes a single `url_patterns()` entry point that `src/config/urls.rs`
//! mounts under `/api/`. The `#[url_patterns(InstalledApp::snippets, mode = server)]`
//! attribute lives here so the routes macro can find the generated
//! `url_resolvers` module at the canonical path
//! `crate::apps::snippets::urls::url_resolvers`.
//!
//! `url_patterns()` registers the function-based endpoints (Tutorial 1-5)
//! and the ViewSet endpoints (Tutorial 6) on the same router — there is
//! no `USE_VIEWSET`-style toggle. Bruno (and `curl`, `httpie`, the
//! integration tests, …) can therefore drive either path against the
//! same running server.
//!
//! This app is REST-only — it has no client (WASM SPA) or WebSocket
//! surface, so the routes macro is invoked in
//! `examples/examples-tutorial-rest/src/config/urls.rs` with
//! `#[routes(server_only)]` (Issue #4509) and the per-app
//! `client_url_resolvers` / `ws_url_resolvers` module lookups are
//! skipped. No stub modules are required.
//!
//! ### Why the routes are inlined here instead of being aggregated from
//! ### per-style submodules
//!
//! An earlier draft split the function-based endpoints into
//! `urls/function_urls.rs` and the ViewSet into `urls/viewset_urls.rs`,
//! and tried to combine them via
//! `ServerRouter::new().mount("/", function_urls::function_url_patterns())
//!  .mount("/", viewset_urls::viewset_url_patterns())`. That requires each
//! helper to have its own `#[url_patterns(InstalledApp::snippets, mode = server)]`
//! attribute (otherwise the macro's `build_mount_reexport` cannot find a
//! sibling `url_resolvers` module on the mount target). Adding the
//! attribute to multiple sibling functions makes both modules emit a
//! `__for_each_url_resolver` macro of the same name, and the
//! aggregator's macro then fails with `error[E0659]: __for_each_url_resolver`
//! is ambiguous`. The framework currently supports at most one
//! `#[url_patterns(..., mode = server)]` per app, so we keep the macro
//! here and inline the endpoint/viewset registrations.

use reinhardt::ServerRouter;
use reinhardt::url_patterns;

use super::views;
use crate::config::apps::InstalledApp;

/// Register every snippets-app URL on a single `ServerRouter`.
///
/// Function-based endpoints (Tutorial 1-5) and the `ModelViewSet`
/// (Tutorial 6) are mounted side by side, so a single running server
/// exposes both `GET /api/snippets/` and `GET /api/snippets-viewset/`
/// (and the rest of each CRUD set). Bruno's `Snippets CRUD` and
/// `Snippets ViewSet` folders drive these in turn against the same
/// process.
#[url_patterns(InstalledApp::snippets, mode = server)]
pub fn url_patterns() -> ServerRouter {
	ServerRouter::new()
		// Function-based endpoints (Tutorial 1-5)
		// - GET    /snippets/        — views::list
		// - POST   /snippets/        — views::create
		// - GET    /snippets/{id}/   — views::retrieve
		// - PUT    /snippets/{id}/   — views::update
		// - DELETE /snippets/{id}/   — views::delete
		.endpoint(views::list)
		.endpoint(views::create)
		.endpoint(views::retrieve)
		.endpoint(views::update)
		.endpoint(views::delete)
		// ViewSet endpoints (Tutorial 6, rc.23+ real CRUD)
		// - GET    /snippets-viewset/         — list (pagination/filter/order)
		// - POST   /snippets-viewset/         — create
		// - GET    /snippets-viewset/{id}/    — retrieve
		// - PUT    /snippets-viewset/{id}/    — update
		// - PATCH  /snippets-viewset/{id}/    — partial update
		// - DELETE /snippets-viewset/{id}/    — delete
		.viewset("/snippets-viewset", views::viewset())
}
