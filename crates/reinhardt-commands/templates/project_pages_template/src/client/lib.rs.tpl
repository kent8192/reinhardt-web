//! WASM entry point for {{ project_name }}.
//!
//! Delegates startup to [`ClientLauncher`], which handles the panic hook,
//! reactive scheduler, DOM mounting on `#root`, history listener, and the
//! reactive re-render on route changes.
//!
//! Client routers for each app are registered via inventory by
//! `#[url_patterns(..., mode = client)]` and automatically discovered
//! by `ClientLauncher::router_client(...)`.

use reinhardt::pages::ClientLauncher;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	ClientLauncher::new("#root")
		.router_client()
		// Optional builder hooks (since reinhardt-web v0.1.0-rc.23):
		//   .intercept_links()                     // built-in SPA link interception
		//   .before_launch(|| { /* setup */ })     // pre-mount lifecycle hook
		//   .after_launch(|| { /* boot done */ })  // post-mount lifecycle hook
		//   .on_path("/login", || { /* run on exact path */ })
		//   .on_path_pattern("/users/{id}", |params| { /* path-driven side effect */ })
		.launch()
}
