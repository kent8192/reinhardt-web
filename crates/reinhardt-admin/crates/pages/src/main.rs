//! WASM entry point for Reinhardt Admin Panel

use wasm_bindgen::prelude::*;

/// WASM entry point
///
/// This function is called when the WASM module is loaded.
/// It initializes the application and mounts it to the DOM.
#[allow(clippy::main_recursion)]
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
	// Set up panic hook for better error messages in console
	#[cfg(feature = "console_error_panic_hook")]
	console_error_panic_hook::set_once();

	// Initialize the application
	// TODO: Implement when reinhardt-pages is available
	todo!("Initialize reinhardt-pages application and mount to #app element")
}
