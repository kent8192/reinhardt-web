//! WASM entry point for {{ project_name }}.
//!
//! Bootstraps the SPA via `ClientLauncher::router_client`. Each app exposes
//! its own client router (`apps/<app>/urls/client_router.rs`); combine
//! them here when the project has more than one Pages app.
//!
//! `ClientRouter::merge` is currently `pub(crate)` (reinhardt-web#4442),
//! so today you either:
//!  - use a single app's `client_url_patterns()` as the root router, or
//!  - append additional routes inline to that root router (see
//!    examples-tutorial-basis for an example).
//!
//! Ideal implementation (after #4442 ships):
//!   .router_client(|| app_a::client_url_patterns().merge(app_b::client_url_patterns()))

use reinhardt::pages::ClientLauncher;
#[allow(unused_imports)] // Used once a per-app router is wired below.
use reinhardt::register_client_reverser;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	ClientLauncher::new("#root")
		// Uncomment and adjust once you have a Pages app:
		// .router_client(|| {
		//     let router = crate::apps::<your_app>::urls::client_router::client_url_patterns();
		//     register_client_reverser(router.to_reverser());
		//     router
		// })
		.launch()
}
