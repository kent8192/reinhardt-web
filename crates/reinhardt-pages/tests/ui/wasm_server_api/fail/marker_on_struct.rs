//! Compile-fail coverage for target markers on non-function items.

use reinhardt_pages::wasm_server_api;

#[wasm_server_api]
mod platform_api {
	#[wasm]
	pub struct PlatformOnly;
}

fn main() {}
