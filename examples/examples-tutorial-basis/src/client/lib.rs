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
//! Each app's `#[url_patterns(InstalledApp::<app>, mode = client)]`
//! macro produces a per-app `ClientRouter` already namespaced
//! (`polls:` / `users:`). They are stitched into the single router the
//! launcher expects with `ClientRouter::merge`.
//!
//! Once the broader ergonomics issue (#4453) lands, this entire file
//! collapses further to `ClientLauncher::new("#root").launch()` because
//! `#[url_patterns(..., mode = client)]` will register the routers via
//! inventory and the launcher will discover + merge + register the
//! reverser automatically.

use reinhardt::pages::ClientLauncher;
use reinhardt::register_client_reverser;
use wasm_bindgen::prelude::*;

use crate::apps::polls::urls::client_router::client_url_patterns as polls_client_url_patterns;
use crate::apps::users::urls::client_router::client_url_patterns as users_client_url_patterns;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	ClientLauncher::new("#root")
		.router_client(|| {
			let router = polls_client_url_patterns().merge(users_client_url_patterns());
			register_client_reverser(router.to_reverser());
			router
		})
		.launch()
}
