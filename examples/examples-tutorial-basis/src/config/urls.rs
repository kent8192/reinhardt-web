//! URL configuration for examples-tutorial-basis project
//!
//! The `routes` function defines the top-level project router. Per-app server
//! routes are auto-mounted via `#[url_patterns(InstalledApp::<app>, mode = server)]`,
//! and per-app client routes are aggregated through the `.client(|c| ...)`
//! closure below so that the `#[routes]` macro's WASM-side
//! `inventory::submit!(ClientRouterRegistration)` emission carries every
//! SPA route. `ClientLauncher::register_routes_from_inventory()` in
//! `client/lib.rs` then merges those entries and installs them as the SPA
//! route table.
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
	get_question_results, get_questions, get_vote_form_metadata, submit_vote, update_choice,
	update_question, vote,
};
#[cfg(native)]
use crate::apps::users::server_fn::{current_user, login, logout, register};

#[cfg(native)]
use reinhardt::middleware::session::{SessionConfig, SessionMiddleware};
#[cfg(native)]
use std::time::Duration;

/// Build the session middleware with a two-week TTL and Lax SameSite.
///
/// Mirrors the production defaults used in `examples-twitter/src/config/middleware.rs`.
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
/// `#[routes(standalone, client_inventory)]` opts into the new cross-target
/// convention introduced in #4453 without enabling per-app URL-resolver
/// generation (this project does not consume `installed_apps!`-generated
/// `client_url_resolvers` modules from a top-level `urls` directory; the
/// per-app `#[url_patterns(..., mode = client)]` declarations live in
/// `apps/<app>/urls/client_router.rs` instead). The flags compose:
///
/// - `client_inventory` (#4453): drops the macro's `native_only` cfg gate
///   from the user function body and emits
///   `inventory::submit!(ClientRouterRegistration)` on
///   `wasm32-unknown-unknown`. The body below MUST therefore compile on
///   both targets — `.server(|s| ...)` and the `#[cfg(wasm)]` aggregation
///   block ensure that.
/// - `standalone`: suppresses generation of `crate::urls::url_prelude` and
///   the `ResolvedUrls::<app>()` accessor methods. The project still
///   resolves SPA URLs via `register_client_reverser` (called inside
///   `collect_client_router_from_inventory`).
///
/// On native, the macro emits `inventory::submit!(UrlPatternsRegistration)`
/// for the `ServerRouter` carried by the returned `UnifiedRouter`. On wasm
/// it emits the parallel `ClientRouterRegistration`, and
/// `ClientLauncher::register_routes_from_inventory()` in
/// `client/lib.rs` consumes those entries to install the SPA route table.
///
/// Per-app server routers are still discovered through their own
/// `#[url_patterns(InstalledApp::<app>, mode = server)]` registrations; this
/// function only registers the project-level server functions, the admin
/// panel, and the session middleware on top of them.
#[routes(standalone, client_inventory)]
pub fn routes() -> UnifiedRouter {
	let router = UnifiedRouter::new().server(|s| {
		// On wasm the `s` parameter is a `ServerRouterStub` and every
		// builder call inside this closure is absorbed by the stub
		// (see `reinhardt_urls::routers::unified_router::ServerRouterStub`),
		// so the `server_fn` markers do not need to compile on wasm. We
		// still gate the marker references on `#[cfg(native)]` because
		// the `server_fn` marker modules themselves are native-only.
		#[cfg(native)]
		{
			s.server_fn(get_questions::marker)
				.server_fn(get_question_detail::marker)
				.server_fn(get_question_results::marker)
				.server_fn(vote::marker)
				.server_fn(get_vote_form_metadata::marker)
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

	// Aggregate every app's client routes on wasm so the macro-emitted
	// `ClientRouterRegistration` carries the full SPA route table.
	//
	// Each `client_url_patterns()` already namespaces its routes
	// (`polls:` / `users:`) via its own `#[url_patterns(..., mode = client)]`
	// registration. We compose them by wrapping each in a single-purpose
	// `UnifiedRouter` and stitching with `mount_unified`, which uses
	// `ClientRouter::merge` internally (still `pub(crate)` upstream —
	// tracked in #4442). When #4442 ships, this collapses to
	// `.client(|c| c.merge(polls).merge(users))`.
	//
	// The aggregation is `#[cfg(wasm)]` because:
	// - The per-app `client_router` submodules are themselves wasm-only
	//   (they import `crate::client::pages::*`, which is wasm-only).
	// - On native, `#[routes(standalone, client_inventory)]` consumes the
	//   server portion of the returned `UnifiedRouter` via
	//   `UrlPatternsRegistration`; the `ClientRouter` field is unused on
	//   the native side.
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

	// `SessionMiddleware` auto-registers its `Arc<SessionStore>` as a DI
	// singleton via `Middleware::di_registrations`, so server functions that
	// `#[inject] session: SessionData` (or `#[inject] store: SessionStoreRef`)
	// can resolve the same store the middleware writes to without a parallel
	// `with_di_registrations(...)` call. See #4426 (and the original #4423
	// regression that motivated the auto-registration hook).
	#[cfg(native)]
	let router = router.with_middleware(create_session_middleware());

	router
}
