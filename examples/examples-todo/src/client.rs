//! WASM entry point for the Todo example.

use wasm_bindgen::prelude::*;

/// Mounts the Todo SPA into `#root`.
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	console_error_panic_hook::set_once();
	reinhardt::pages::ClientLauncher::new("#root")
		.router_client(crate::ui::client_router)
		.launch()
}
