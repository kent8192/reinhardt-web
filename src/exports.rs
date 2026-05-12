//! Layered facade for `reinhardt` (Issue #4362).
//!
//! Confines `#[cfg]` boilerplate to a small set of boundary files:
//! - [`common`] — cross-target re-exports
//! - [`macros`] — proc-macro re-exports (host-side, mostly cross-target)
//! - [`native`] — server-side re-exports (gated `#[cfg(native)]`)
//! - [`wasm`] — browser-WASM-side re-exports (gated `#[cfg(not(native))]`)
//!
//! The crate root performs `pub use exports::*;` so the public API surface is
//! unchanged. See Issue #4362 and the parent tracking work in Stage 1 (#4364)
//! for context.

pub mod common;
pub mod macros;

#[cfg(native)]
pub mod native;

#[cfg(not(native))]
pub mod wasm;

pub use common::*;
pub use macros::*;

#[cfg(native)]
pub use native::*;

// `wasm` is intentionally empty in Stage 1 (Issue #4364) — wasm-side surface
// is composed of `crate::compat` shims and cross-target items in `common`.
// Future browser-only additions land here without polluting the cross-target
// layer.
#[cfg(not(native))]
#[allow(unused_imports, unreachable_pub)]
pub use wasm::*;
