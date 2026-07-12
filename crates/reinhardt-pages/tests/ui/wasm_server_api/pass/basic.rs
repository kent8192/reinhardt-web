//! Compile-pass coverage for the public `wasm_server_api` re-export.

use reinhardt_pages::wasm_server_api;

#[wasm_server_api]
pub mod platform_api {
	pub const SHARED_VALUE: &str = "shared";

	#[doc = "Returns the active target family name."]
	#[wasm]
	pub fn target_name(input: &'static str) -> &'static str {
		let _ = input;
		"wasm"
	}

	#[doc = "Returns the active target family name."]
	#[server]
	pub fn target_name(input: &'static str) -> &'static str {
		let _ = input;
		"server"
	}

	#[wasm]
	pub async fn load_count(id: u32) -> Result<u32, &'static str> {
		Ok(id + 1)
	}

	#[server]
	pub async fn load_count(id: u32) -> Result<u32, &'static str> {
		Ok(id + 2)
	}
}

fn main() {
	let _ = platform_api::SHARED_VALUE;
	let _ = platform_api::target_name("fixture");
}
