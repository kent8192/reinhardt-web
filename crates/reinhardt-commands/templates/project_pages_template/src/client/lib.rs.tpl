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
		.launch()
}
