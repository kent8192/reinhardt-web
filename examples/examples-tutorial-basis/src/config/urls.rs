//! URL configuration for examples-tutorial-basis project
//!
//! The `routes` function defines the top-level project router.
//!
//! Middleware stack (server-only):
//! 1. `SessionMiddleware` — cookie-based session management used by the
//!    `users` app's login/logout server functions

use reinhardt::UnifiedRouter;
#[cfg(native)]
use reinhardt::admin::{admin_routes_with_di, admin_static_routes};
#[cfg(native)]
use reinhardt::pages::server_fn::ServerFnRouterExt;
use reinhardt::routes;

#[cfg(native)]
use crate::config::admin::configure_admin;

// Import server_fn marker modules (snake_case + ::marker)
#[cfg(native)]
use crate::apps::polls::server_fn::{
	create_choice, create_question, delete_choice, delete_question, get_question_detail,
	get_question_results, get_questions, submit_vote, update_choice, update_question, vote,
};
#[cfg(native)]
use crate::apps::users::server_fn::{current_user, login, logout, register};

#[cfg(native)]
use reinhardt::middleware::session::{SessionConfig, SessionMiddleware};
#[cfg(native)]
use std::time::Duration;

/// Build the session middleware with a two-week TTL and Lax SameSite.
///
/// Uses the production-oriented defaults shared by the tutorial examples.
#[cfg(native)]
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
	let router = UnifiedRouter::new().server(|s| {
		// On wasm the `s` parameter is a no-op `ServerRouter` and every
		// builder call inside this closure is absorbed by it
		// (see `reinhardt_urls::routers::ServerRouter`), so the `server_fn`
		// markers do not need to compile on wasm. We still gate the marker
		// references on `#[cfg(native)]` because the `server_fn` marker
		// modules themselves are native-only.
		#[cfg(native)]
		{
			s.server_fn(get_questions::marker)
				.server_fn(get_question_detail::marker)
				.server_fn(get_question_results::marker)
				.server_fn(vote::marker)
				.server_fn(submit_vote::marker)
				.server_fn(create_question::marker)
				.server_fn(update_question::marker)
				.server_fn(delete_question::marker)
				.server_fn(create_choice::marker)
				.server_fn(update_choice::marker)
				.server_fn(delete_choice::marker)
				.server_fn(login::marker)
				.server_fn(logout::marker)
				.server_fn(register::marker)
				.server_fn(current_user::marker)
		}
		#[cfg(not(native))]
		{
			s
		}
	});

	// Aggregate every app's client routes on wasm so the SPA route table
	// carries every app's client-side URL patterns.
	//
	// Each `client_url_patterns()` already namespaces its routes
	// (`polls:` / `users:`). We compose them by wrapping each in a single-purpose
	// `UnifiedRouter` and stitching with `mount_unified`, which uses
	// `ClientRouter::merge` internally.
	//
	// The aggregation is `#[cfg(wasm)]` because the per-app `client_router`
	// submodules are themselves wasm-only (they import `crate::client::pages::*`,
	// which is wasm-only).
	#[cfg(wasm)]
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
	#[cfg(native)]
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
	#[cfg(native)]
	let router = router.with_middleware(create_session_middleware());

	router
}
