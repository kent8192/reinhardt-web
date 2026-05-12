//! Cross-target platform abstraction (Issue #4365).
//!
//! Unified type aliases and task-spawning utilities that work across both
//! browser-WASM and native (SSR / proc-macro host) targets. Concentrates the
//! `#[cfg(wasm)] / #[cfg(native)]` split into [`wasm`] and [`native`]
//! submodules so consumer code can write:
//!
//! ```rust,ignore
//! use reinhardt_pages::platform::{Event, spawn_task};
//!
//! fn handle_click(event: Event) {
//!     spawn_task(async move {
//!         // …cross-target async work…
//!     });
//! }
//! ```

#[cfg(wasm)]
mod wasm;

#[cfg(native)]
mod native;

#[cfg(wasm)]
pub use wasm::*;

#[cfg(native)]
pub use native::*;
