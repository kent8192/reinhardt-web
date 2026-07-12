//! Compile-fail coverage for divergent public API attributes.

use reinhardt_pages::wasm_server_api;

#[wasm_server_api]
mod platform_api {
	#[must_use]
	#[wasm]
	pub fn target_name() -> &'static str {
		"wasm"
	}

	#[server]
	pub fn target_name() -> &'static str {
		"server"
	}
}

fn main() {}
