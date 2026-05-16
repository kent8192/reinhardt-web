//! URL configuration for examples-tutorial-basis project
//!
//! The `routes` function defines the top-level project router. Per-app routes
//! are registered separately by `#[url_patterns(InstalledApp::<app>, mode = ...)]`
//! attributes on the app's URL functions (see
//! `apps/polls/urls/server_urls.rs::server_url_patterns`), so this file only
//! needs to register server functions and apply the middleware stack.
//!
//! Middleware stack (server-only):
//! 1. `SessionMiddleware` — cookie-based session management used by the
//!    `users` app's login/logout server functions

#[cfg(native)]
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
use crate::server_fn::polls::{
	create_choice, create_question, delete_choice, delete_question, get_question_detail,
	get_question_results, get_questions, get_vote_form_metadata, submit_vote, update_choice,
	update_question, vote,
};
#[cfg(native)]
use crate::server_fn::users::{current_user, login, logout, register};

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

// `#[routes(standalone)]` is applied unconditionally so that the
// generated `__url_resolver_support::ResolvedUrls` (and its re-export)
// is reachable from the WASM SPA. The macro internally gates the
// function body and the `inventory::submit!` registration on
// `#[cfg(not(wasm))]` (fixes #4175), so the function body below — the
// native server registration path — is compiled out on WASM by the
// macro and only runs on native. The `#[cfg(wasm)]` `let router`
// branch is preserved purely so the file reads naturally if the macro
// gating is ever lifted; it is unreachable on every target today.
#[routes(standalone)]
pub fn routes() -> UnifiedRouter {
	// Server: register server functions. App routers are auto-mounted via
	// `#[url_patterns(InstalledApp::<app>, mode = server)]`.
	#[cfg(native)]
	let router = UnifiedRouter::new().server(|s| {
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
	});

	// Client: empty top-level router. App client routers are registered via
	// `#[url_patterns(InstalledApp::<app>, mode = client)]` and bootstrapped
	// directly by `ClientLauncher::router_client(...)` in `client/lib.rs`.
	#[cfg(wasm)]
	let router = UnifiedRouter::new();

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
