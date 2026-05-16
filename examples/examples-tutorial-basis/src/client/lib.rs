//! WASM entry point.
//!
//! Bootstraps the SPA via `ClientLauncher::router_client`. The launcher
//! installs the panic hook, history listener, and DOM mount on `#root`,
//! then takes a single `ClientRouter` to use both for in-SPA navigation
//! and (after our explicit `register_client_reverser` call below) for
//! `ResolvedUrls::from_global()` lookups in components and the nav bar.
//!
//! ## Composing the polls + users client routers
//!
//! `ClientRouter::merge` is `pub(crate)` in `reinhardt-urls` (tracked
//! upstream in #4442), so user code cannot call it directly to combine
//! two SPA routers built by separate `#[url_patterns(..., mode = client)]`
//! registrations. We work around that by wrapping each app's
//! `ClientRouter` in a single-purpose `UnifiedRouter` and stitching
//! them together with `UnifiedRouter::mount_unified`, which uses the
//! same internal `merge` call but is `pub`. The merged `ClientRouter`
//! is then extracted with `UnifiedRouter::into_client`.
//!
//! When #4442 ships, this whole helper collapses to
//! `polls_client_url_patterns().merge(users_client_url_patterns())`
//! and the `UnifiedRouter` indirection can be removed.
//!
//! Once the broader ergonomics issue (#4453) lands, this entire file
//! collapses further to `ClientLauncher::new("#root").launch()` because
//! `#[url_patterns(..., mode = client)]` will register the routers via
//! inventory and the launcher will discover + merge + register the
//! reverser automatically.

use reinhardt::pages::ClientLauncher;
use reinhardt::{ClientRouter, UnifiedRouter, register_client_reverser};
use wasm_bindgen::prelude::*;

use crate::apps::polls::urls::client_router::client_url_patterns as polls_client_url_patterns;
use crate::apps::users::urls::client_router::client_url_patterns as users_client_url_patterns;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	ClientLauncher::new("#root")
		.router_client(|| {
			let router = build_spa_router();
			register_client_reverser(router.to_reverser());
			router
		})
		.launch()
}

/// Compose every app's `#[url_patterns(InstalledApp::<app>, mode = client)]`
/// router into the single `ClientRouter` that `ClientLauncher::router_client`
/// expects.
///
/// Each app's `client_url_patterns()` returns a `ClientRouter` with the
/// app's namespace (`polls:` / `users:`) already applied. Wrapping each
/// one in a `UnifiedRouter` and stitching with `mount_unified` reuses
/// the framework's existing client-router merge logic without depending
/// on the still-`pub(crate)` `ClientRouter::merge` (see #4442).
fn build_spa_router() -> ClientRouter {
	let polls = UnifiedRouter::new().client(|_| polls_client_url_patterns());
	let users = UnifiedRouter::new().client(|_| users_client_url_patterns());

	UnifiedRouter::new()
		.mount_unified("/", polls)
		.mount_unified("/", users)
		.into_client()
}
