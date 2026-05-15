//! WASM entry point.
//!
//! Bootstraps the SPA via `ClientLauncher::router_client`. The launcher
//! installs the panic hook, history listener, and DOM mount on `#root`,
//! then takes a single `ClientRouter` to use both for in-SPA navigation
//! and (after our explicit `register_client_reverser` call below) for
//! `ResolvedUrls::from_global()` lookups in components and the nav bar.
//!
//! ## Why we merge `users` routes inline instead of calling
//! `users::client_url_patterns()` + `.merge(...)`:
//! `ClientRouter::merge` is `pub(crate)` in `reinhardt-urls`
//! (tracked upstream in #4442), so user code cannot combine two SPA
//! routers built by separate `#[url_patterns(..., mode = client)]`
//! registrations. Until that issue is resolved, appending the users
//! routes inline here keeps a single `ClientRouter` (and therefore a
//! single `ClientUrlReverser`) that covers every page reachable in
//! the SPA. The names use the fully-qualified `users:<name>` form
//! because the polls router's `with_namespace("polls")` has already
//! been applied by the `#[url_patterns(InstalledApp::polls, ...)]`
//! macro, so further routes added at this layer are stored verbatim.
//! When #4442 ships, this block collapses to
//! `polls_client_url_patterns().merge(users_client_url_patterns())`
//! and the inline route patterns / pages can be removed.

use reinhardt::pages::ClientLauncher;
use reinhardt::register_client_reverser;
use wasm_bindgen::prelude::*;

use crate::apps::polls::urls::client_router::client_url_patterns as polls_client_url_patterns;
use crate::client::pages::{login_page, logout_page};

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	ClientLauncher::new("#root")
		.router_client(|| {
			let router = polls_client_url_patterns()
				.named_route("users:login", "/users/login/", login_page)
				.named_route("users:logout", "/users/logout/", logout_page);
			register_client_reverser(router.to_reverser());
			router
		})
		.launch()
}
