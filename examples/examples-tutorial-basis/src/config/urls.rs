//! URL configuration for examples-tutorial-basis project
//!
//! The `routes` function defines the top-level project router.
//!
//! Middleware stack:
//! 1. `SessionMiddleware` — cookie-based session management used by the
//!    `users` app's login/logout server functions

use reinhardt::UnifiedRouter;
use reinhardt::routes;

/// Build the top-level project router.
///
/// The `#[routes]` macro registers this function for automatic discovery
/// via `inventory::submit!(UrlPatternsRegistration)` and emits a linker
/// marker to enforce single-usage.
///
/// This function aggregates the app-level server routers, the app-level
/// client routers, the admin panel, and the session middleware.
#[routes]
pub fn routes() -> UnifiedRouter {
	let router = UnifiedRouter::new();

	// Each app owns its server-function marker registration in its own
	// `urls` module. The project router only aggregates app routers.
	let router = crate::native_runtime::mount_server_url_patterns(router);

	// Aggregate every app's client routes so both native route-table
	// construction and the WASM SPA see the same URL patterns.
	//
	// Each `client_url_patterns()` already namespaces its routes
	// (`polls:` / `users:`). We compose them by wrapping each in a single-purpose
	// `UnifiedRouter` and stitching with `mount_unified`, which uses
	// `ClientRouter::merge` internally.
	//
	let router = router
		.mount_unified(
			"/",
			UnifiedRouter::new().client(|_| crate::apps::polls::urls::client_url_patterns()),
		)
		.mount_unified(
			"/",
			UnifiedRouter::new().client(|_| crate::apps::users::urls::client_url_patterns()),
		);

	// Mount the auto-generated admin panel at /admin/ (server-only).
	// `admin_routes_with_di` returns both the router and a DI registration
	// list that lazily provides `AdminDatabase` to admin handlers from the
	// project's `DatabaseConnection`.
	let router = crate::native_runtime::mount_admin_routes(router);

	// `SessionMiddleware` auto-registers its `SessionStore` as a DI singleton
	// via `Middleware::di_registrations` (keyed by `TypeId::of::<SessionStore>()`
	// post-#4437), so server functions that
	// `#[inject] session: SessionData` (or `#[inject] store: Depends<SessionStore>`)
	// can resolve the same store the middleware writes to without a parallel
	// `with_di_registrations(...)` call. See #4426 (and the original #4423
	// regression that motivated the auto-registration hook).
	crate::native_runtime::with_session_middleware(router)
}
