//! Compile-fail coverage for non-public target variants.

use reinhardt_pages::wasm_server_api;

#[wasm_server_api]
mod platform_api {
	#[wasm]
	fn target_name() -> &'static str {
		"wasm"
	}

	#[server]
	fn target_name() -> &'static str {
		"server"
	}
}

fn main() {}
