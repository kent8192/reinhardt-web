//! Build script for database-integration example (local development mode)
//!
//! This example always uses local reinhardt workspace via [patch.crates-io].
//! Tests are always enabled in local development mode.

fn main() {
	// Local development mode: always enable tests
	println!("cargo:rustc-cfg=with_reinhardt");
	println!("cargo:warning=Using local reinhardt workspace (examples/local)");
	println!("cargo:rerun-if-changed=build.rs");
}
