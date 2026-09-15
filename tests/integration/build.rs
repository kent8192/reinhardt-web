//! Build script for the cross-crate integration test package.
//!
//! The model macro gates the `Validate` derive and the validator attributes it generates behind
//! `cfg(native)`, and several test modules are themselves gated on `native`. `rustc-check-cfg`
//! only silences the `unexpected_cfgs` lint; the aliases have to be emitted for those blocks to
//! take effect.

use cfg_aliases::cfg_aliases;

fn main() {
	// Declare custom cfg features to avoid "unexpected cfg" warnings
	println!(
		"cargo:rustc-check-cfg=cfg(feature, values(\"hot-reload\", \"caching\", \"source-maps\", \"image-optimization\", \"graphql\"))"
	);
	println!("cargo::rustc-check-cfg=cfg(wasm)");
	println!("cargo::rustc-check-cfg=cfg(native)");

	cfg_aliases! {
		wasm: { all(target_family = "wasm", target_os = "unknown") },
		native: { not(all(target_family = "wasm", target_os = "unknown")) },
	}
}
