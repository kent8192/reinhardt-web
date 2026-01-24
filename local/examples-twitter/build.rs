// build.rs
use cfg_aliases::cfg_aliases;

fn main() {
	// Declare custom cfg to avoid warnings in Rust 2024 edition
	println!("cargo::rustc-check-cfg=cfg(with_reinhardt)");
	println!("cargo::rustc-check-cfg=cfg(client)");
	println!("cargo::rustc-check-cfg=cfg(server)");

	// Always enable tests for local development
	println!("cargo:rustc-cfg=with_reinhardt");

	cfg_aliases! {
		// Platform aliases for simpler conditional compilation
		// Use `#[cfg(client)]` instead of `#[cfg(target_arch = "wasm32")]`
		client: { target_arch = "wasm32" },
		// Use `#[cfg(server)]` instead of `#[cfg(not(target_arch = "wasm32"))]`
		server: { not(target_arch = "wasm32") },
	}
}
