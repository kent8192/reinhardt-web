//! WASM entry point for {{ project_name }}.
//!
//! Delegates startup to [`ClientLauncher`], which handles the panic hook,
//! reactive scheduler, DOM mounting on `#root`, history listener, and the
//! reactive re-render on route changes.

use reinhardt::pages::ClientLauncher;
use wasm_bindgen::prelude::*;

use super::router;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	ClientLauncher::new("#root")
		.router(router::init_router)
		// Optional builder hooks (since reinhardt-web v0.1.0-rc.23):
		//   .intercept_links()                     // built-in SPA link interception
		//   .before_launch(|| { /* setup */ })     // pre-mount lifecycle hook
		//   .after_launch(|| { /* boot done */ })  // post-mount lifecycle hook
		//   .on_path("/login", || { /* run on exact path */ })
		//   .on_path_pattern("/users/{id}", |params| { /* path-driven side effect */ })
		.launch()
}
