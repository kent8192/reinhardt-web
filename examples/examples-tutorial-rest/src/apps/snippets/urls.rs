//! URL configuration for the snippets app.
//!
//! This file is the canonical aggregator for the `snippets` app — it
//! exposes a single `url_patterns()` entry point that `src/config/urls.rs`
//! mounts under `/api/`.
//!
//! `url_patterns()` registers the function-based endpoints (Tutorial 1-5)
//! and the ViewSet endpoints (Tutorial 6) on the same router — there is
//! no `USE_VIEWSET`-style toggle. Bruno (and `curl`, `httpie`, the
//! integration tests, ...) can therefore drive either path against the
//! same running server.
//!
//! This app is REST-only — it has no client (WASM SPA) or WebSocket
//! surface.

use reinhardt::ServerRouter;

use super::views;

/// Register every snippets-app URL on a single `ServerRouter`.
///
/// Function-based endpoints (Tutorial 1-5) and the `ModelViewSet`
/// (Tutorial 6) are mounted side by side, so a single running server
/// exposes both `GET /api/snippets/` and `GET /api/snippets-viewset/`
/// (and the rest of each CRUD set). Bruno's `Snippets CRUD` and
/// `Snippets ViewSet` folders drive these in turn against the same
/// process.
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
