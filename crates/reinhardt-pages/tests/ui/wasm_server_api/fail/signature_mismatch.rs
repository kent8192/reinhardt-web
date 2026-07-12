//! Compile-fail coverage for divergent public API signatures.

use reinhardt_pages::wasm_server_api;

#[wasm_server_api]
mod platform_api {
	#[wasm]
	pub fn target_name(input: &'static str) -> &'static str {
		input
	}

	#[server]
	pub fn target_name() -> &'static str {
		"server"
	}
}

fn main() {}
