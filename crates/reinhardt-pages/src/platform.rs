//! Platform abstraction for WASM and native targets.
//!
//! This module provides unified type aliases and task-spawning utilities
//! that work across both WASM and native platforms, reducing the need for
//! conditional compilation in user code. The browser-WASM implementation
//! lives in the `wasm` submodule and the native stub implementation in the
//! `native` submodule; this file only performs `#[cfg]` dispatch between
//! them and re-exports the active platform's API.
//!
//! # Example
//!
//! ```rust,ignore
//! use reinhardt_pages::platform::Event;
//!
//! // Works on both WASM and native targets
//! fn handle_click(event: Event) {
//!     // Event handling logic
//! }
//! ```

#[cfg(wasm)]
mod wasm;
#[cfg(wasm)]
pub use wasm::*;

#[cfg(native)]
mod native;
#[cfg(native)]
pub use native::*;

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_spawn_task_compiles() {
		spawn_task(async {
			// Task body
		});
	}
}
