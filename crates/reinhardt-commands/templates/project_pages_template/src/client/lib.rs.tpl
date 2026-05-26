//! WASM entry point for {{ project_name }}.
//!
//! Bootstraps the SPA via `ClientLauncher::router_client`. Each app exposes
//! its own client router (`apps/<app>/urls/client_router.rs`); combine
//! them here when the project has more than one Pages app.
//!
//! The freshly scaffolded project installs an empty `ClientRouter` so the
//! WASM bundle starts cleanly even before any Pages app is added —
//! `ClientLauncher::launch()` returns an error if neither `router(...)` nor
//! `router_client(...)` has been configured. Replace the default closure
//! with your app's `client_url_patterns()` once you run `startapp`.
//!
//! `ClientRouter::merge` is currently `pub(crate)` (reinhardt-web#4442),
//! so today you either:
//!  - use a single app's `client_url_patterns()` as the root router, or
//!  - append additional routes inline to that root router (see
//!    examples-tutorial-basis for an example).
//!
//! Ideal implementation (after #4442 ships):
//!   .router_client(|| app_a::client_url_patterns().merge(app_b::client_url_patterns()))

use reinhardt::ClientRouter;
use reinhardt::pages::ClientLauncher;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	ClientLauncher::new("#root")
		// Replace this empty router with your app's client routes once a
		// Pages app exists. Example:
		//   .router_client(|| {
		//       crate::apps::<your_app>::urls::client_router::client_url_patterns()
		//   })
		.router_client(ClientRouter::new)
		.launch()
}
