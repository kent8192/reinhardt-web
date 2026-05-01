#![cfg(not(target_arch = "wasm32"))]
//! Native repro tests for issue #4075 — verifies that an Effect created
//! against `Router::current_path()` re-fires when `Router::push` updates
//! the path Signal.
//!
//! - `test_effect_refires_on_direct_signal_access` (Task 1) — control:
//!   direct Signal access in the Effect closure, no thread-local borrow.
//! - `test_effect_refires_through_thread_local_borrow` (Task 2) — repro:
//!   the launcher's actual access pattern (Router accessed via a
//!   thread-local `RefCell::borrow()`).
//!
//! If Task 1 fails, the bug is in `reinhardt-core/reactive` and this fix
//! must be aborted (see spec §"Approach" Stage 3).

use reinhardt_pages::component::Page;
use reinhardt_pages::reactive::{Effect, with_runtime};
use reinhardt_pages::router::Router;
use serial_test::serial;
use std::cell::RefCell;
use std::rc::Rc;

fn page_a() -> Page {
	Page::text("A")
}

fn page_b() -> Page {
	Page::text("B")
}

/// Variant 1 (control): the Effect closure reads `router.current_path().get()`
/// directly. No thread-local borrow involved. This MUST pass on `main`.
#[test]
#[serial]
fn test_effect_refires_on_direct_signal_access() {
	// Arrange: build a Router with two routes, then move the current path to
	// "/a" via the public `push` API (Router has no test-only setter and the
	// native fallback for `current_path()` is "/", so we push + flush before
	// creating the Effect to establish the initial state).
	let router = Router::new()
		.route("/a", page_a)
		.route("/b", page_b);
	router.push("/a").expect("push /a");
	with_runtime(|rt| rt.flush_updates());

	let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

	let log_clone = log.clone();
	let path_signal = router.current_path().clone();
	let _effect = Effect::new(move || {
		log_clone.borrow_mut().push(path_signal.get());
	});

	// Initial run records "/a".
	assert_eq!(*log.borrow(), vec!["/a".to_string()]);

	// Act: push("/b") — updates the Signal.
	router.push("/b").expect("push /b");
	with_runtime(|rt| rt.flush_updates());

	// Assert: Effect re-fired and logged "/b".
	assert_eq!(
		*log.borrow(),
		vec!["/a".to_string(), "/b".to_string()],
		"Variant 1 (direct Signal access) — if this fails, the runtime is broken; abort fix and file core issue"
	);
}
