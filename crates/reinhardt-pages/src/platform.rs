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
	use std::cell::Cell;
	use std::rc::Rc;

	/// `spawn_task` must never poll the future inline: on native it drops the
	/// future (there is no event loop), and on WASM it only schedules the
	/// future on the browser event loop. Either way, the body must not have
	/// run by the time `spawn_task` returns.
	#[test]
	fn spawn_task_does_not_run_body_synchronously() {
		let runs = Rc::new(Cell::new(0_usize));
		let counter = Rc::clone(&runs);
		spawn_task(async move {
			counter.set(counter.get() + 1);
		});
		assert_eq!(
			runs.get(),
			0,
			"spawn_task must not execute the future body synchronously"
		);
	}
}
