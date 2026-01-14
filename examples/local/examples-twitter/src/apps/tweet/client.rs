//! Tweet client module (WASM)
//!
//! Contains tweet-related UI components.
//! This module is only compiled for WebAssembly target.

#[cfg(target_arch = "wasm32")]
pub mod components;

#[cfg(target_arch = "wasm32")]
pub use components::*;
