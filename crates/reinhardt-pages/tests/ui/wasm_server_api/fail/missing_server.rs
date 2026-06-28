//! Compile-fail coverage for a target variant without its counterpart.

use reinhardt_pages::wasm_server_api;

#[wasm_server_api]
mod platform_api {
	#[wasm]
	pub fn target_name() -> &'static str {
		"wasm"
	}
}

fn main() {}
