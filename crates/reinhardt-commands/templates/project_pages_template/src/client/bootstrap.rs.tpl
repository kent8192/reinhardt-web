//! WASM entry point for {{ project_name }}

use reinhardt::pages::ClientLauncher;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	ClientLauncher::new("#root")
		.router(super::router::init_router)
		.launch()
}
