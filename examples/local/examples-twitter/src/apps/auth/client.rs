//! Auth client module (WASM)
//!
//! Contains authentication-related UI components and state management.
//! This module is only compiled for WebAssembly target.

#[cfg(target_arch = "wasm32")]
pub mod components;
#[cfg(target_arch = "wasm32")]
pub mod state;

#[cfg(target_arch = "wasm32")]
pub use components::*;
#[cfg(target_arch = "wasm32")]
pub use state::*;
