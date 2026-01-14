//! Tweet server module
//!
//! Contains server-side tweet functions.
//! This module is only compiled for non-WebAssembly targets.

#[cfg(not(target_arch = "wasm32"))]
pub mod server_fn;

#[cfg(not(target_arch = "wasm32"))]
pub use server_fn::*;
