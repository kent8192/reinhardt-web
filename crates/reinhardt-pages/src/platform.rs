//! Platform abstraction for WASM and native targets.
//!
//! This module provides the raw cross-target event transport and task-spawning
//! utilities that work across both WASM and native platforms. Standard
//! intrinsic `page!` handlers should normally use the catalog-generated types
//! in [`crate::event`]; use [`Event`] directly for `@custom("name")`, explicit
//! raw adapters, or low-level integration code. The browser-WASM implementation
//! lives in the `wasm` submodule and the native stub implementation in the
//! `native` submodule; this file only performs `#[cfg]` dispatch between
//! them and re-exports the active platform's API.
//!
//! # Example
//!
//! ```rust,ignore
//! use reinhardt_pages::platform::Event;
//!
//! // Explicit raw custom event handler on both targets.
//! fn handle_custom(event: Event) {
//!     let _ = event;
//! }
//! ```

#[derive(Clone)]
pub(crate) struct InputSnapshot {
	pub(crate) data: Option<String>,
	pub(crate) input_type: Option<String>,
	pub(crate) is_composing: bool,
}

#[derive(Clone)]
pub(crate) struct KeyboardSnapshot {
	pub(crate) key: String,
	pub(crate) code: String,
	pub(crate) location: u32,
	pub(crate) repeat: bool,
	pub(crate) is_composing: bool,
	pub(crate) alt: bool,
	pub(crate) control: bool,
	pub(crate) meta: bool,
	pub(crate) shift: bool,
}

#[derive(Clone)]
pub(crate) struct MouseSnapshot {
	pub(crate) client_x: f64,
	pub(crate) client_y: f64,
	pub(crate) screen_x: f64,
	pub(crate) screen_y: f64,
	pub(crate) page_x: f64,
	pub(crate) page_y: f64,
	pub(crate) offset_x: f64,
	pub(crate) offset_y: f64,
	pub(crate) button: i16,
	pub(crate) buttons: u16,
	pub(crate) detail: i32,
	pub(crate) alt: bool,
	pub(crate) control: bool,
	pub(crate) meta: bool,
	pub(crate) shift: bool,
}

#[derive(Clone)]
pub(crate) struct PointerSnapshot {
	pub(crate) pointer_id: i32,
	pub(crate) pointer_kind: String,
	pub(crate) pressure: f32,
	pub(crate) width: f64,
	pub(crate) height: f64,
	pub(crate) tangential_pressure: f32,
	pub(crate) tilt_x: i32,
	pub(crate) tilt_y: i32,
	pub(crate) twist: i32,
	pub(crate) is_primary: bool,
}

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
