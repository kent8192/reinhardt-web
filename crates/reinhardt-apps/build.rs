//! Build script for reinhardt-apps.
//!
//! Sets up cfg aliases for simplified conditional compilation, mirroring
//! the convention established in `reinhardt-core/build.rs`.

use cfg_aliases::cfg_aliases;

fn main() {
	println!("cargo::rustc-check-cfg=cfg(wasm)");
	println!("cargo::rustc-check-cfg=cfg(native)");

	cfg_aliases! {
		wasm: { all(target_family = "wasm", target_os = "unknown") },
		native: { not(all(target_family = "wasm", target_os = "unknown")) },
	}
}
