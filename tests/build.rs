//! Build script for the shared test-support package.
//!
//! This package's `[lib]` points at `integration/src/lib.rs`, the same sources
//! `reinhardt-integration-tests` compiles. The model macro gates the `Validate`
//! derive and the validator attributes it generates behind `cfg(native)`, so the
//! alias has to be defined here too — otherwise the shared library would compile
//! with the generated validators silently dropped in this package only.

use cfg_aliases::cfg_aliases;

fn main() {
	println!("cargo::rustc-check-cfg=cfg(wasm)");
	println!("cargo::rustc-check-cfg=cfg(native)");

	cfg_aliases! {
		wasm: { all(target_family = "wasm", target_os = "unknown") },
		native: { not(all(target_family = "wasm", target_os = "unknown")) },
	}
}
