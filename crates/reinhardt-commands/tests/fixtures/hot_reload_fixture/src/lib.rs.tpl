//! Minimal cdylib used by the hot-reload integration tests.
//!
//! The `{{MARKER}}` token is rewritten by the test harness to simulate a
//! source-file edit between rebuilds.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn marker() -> u32 {
	{{MARKER}}
}

/// Host-target shim so plain `cargo build --bin manage` succeeds even when
/// the cdylib branch is what is being exercised.
pub fn host_marker() -> u32 {
	{{MARKER}}
}
