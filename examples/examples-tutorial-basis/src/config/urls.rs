//! URL configuration for examples-tutorial-basis project
//!
//! The `routes` function defines the top-level project router.
//!
//! Middleware stack (server-only):
//! 1. `SessionMiddleware` — cookie-based session management used by the
//!    `users` app's login/logout server functions

use reinhardt::UnifiedRouter;
#[cfg(server)]
use reinhardt::admin::{admin_routes_with_di, admin_static_routes};
use reinhardt::routes;

#[cfg(server)]
use crate::config::admin::configure_admin;

#[cfg(server)]
use reinhardt::middleware::session::{SessionConfig, SessionMiddleware};
#[cfg(server)]
use std::time::Duration;

/// Build the session middleware with a two-week TTL and Lax SameSite.
///
/// Uses the production-oriented defaults shared by the tutorial examples.
#[cfg(server)]
fn create_session_middleware() -> SessionMiddleware {
	let config = SessionConfig::new("sessionid".to_string(), Duration::from_secs(1_209_600))
		.with_http_only(true)
		.with_same_site("Lax".to_string())
		.with_path("/".to_string());
	SessionMiddleware::new(config)
}

/// Build the top-level project router.
///
/// The `#[routes]` macro registers this function for automatic discovery
/// via `inventory::submit!(UrlPatternsRegistration)` and emits a linker
/// marker to enforce single-usage.
///
/// This function registers the project-level server functions, the admin
/// panel, and the session middleware.
#[routes]
pub fn routes() -> UnifiedRouter {
	let router = UnifiedRouter::new();

	// Per-app server URL modules are native-only because they register
	// `#[server_fn]` marker modules emitted for the server build.
	#[cfg(server)]
	let router = router.server(|s| {
		s.mount(
			"/",
			crate::apps::polls::urls::server_urls::server_url_patterns(),
		)
		.mount(
			"/",
			crate::apps::users::urls::server_urls::server_url_patterns(),
		)
	});

	// Aggregate every app's client routes on wasm so the SPA route table
	// carries every app's client-side URL patterns.
	//
	// Each `client_url_patterns()` already namespaces its routes
	// (`polls:` / `users:`). We compose them by wrapping each in a single-purpose
	// `UnifiedRouter` and stitching with `mount_unified`, which uses
	// `ClientRouter::merge` internally.
	//
	// The aggregation is `#[cfg(client)]` because the per-app `client_router`
	// submodules are themselves wasm-only (they import `crate::client::pages::*`,
	// which is wasm-only).
	#[cfg(client)]
	let router = router
		.mount_unified(
			"/",
			UnifiedRouter::new()
				.client(|_| crate::apps::polls::urls::client_router::client_url_patterns()),
		)
		.mount_unified(
			"/",
			UnifiedRouter::new()
				.client(|_| crate::apps::users::urls::client_router::client_url_patterns()),
		);

	// Mount the auto-generated admin panel at /admin/ (server-only).
	// `admin_routes_with_di` returns both the router and a DI registration
	// list that lazily provides `AdminDatabase` to admin handlers from the
	// project's `DatabaseConnection`.
	#[cfg(server)]
	let router = {
		let admin_site = std::sync::Arc::new(configure_admin());
		let (admin_router, admin_di) = admin_routes_with_di(admin_site);
		router
			.mount("/admin/", admin_router)
			.mount("/static/admin/", admin_static_routes())
			.with_di_registrations(admin_di)
	};

	// `SessionMiddleware` auto-registers its `SessionStore` as a DI singleton
	// via `Middleware::di_registrations` (keyed by `TypeId::of::<SessionStore>()`
	// post-#4437), so server functions that
	// `#[inject] session: SessionData` (or `#[inject] store: Depends<SessionStore>`)
	// can resolve the same store the middleware writes to without a parallel
	// `with_di_registrations(...)` call. See #4426 (and the original #4423
	// regression that motivated the auto-registration hook).
	#[cfg(server)]
	let router = router.with_middleware(create_session_middleware());

	router
}
