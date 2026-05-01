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
	let router = Router::new().route("/a", page_a).route("/b", page_b);
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

/// Variant 2 (repro): the Effect closure reads the path Signal *through*
/// a thread-local `RefCell::borrow()` of a Router — the exact pattern
/// `ClientLauncher::launch` uses via `with_router`. If Task 1 passes
/// and this fails, H1 is confirmed: `Signal::get`'s `track_dependency`
/// fails to register the parent Effect when invoked through the
/// thread-local borrow.
#[test]
#[serial]
fn test_effect_refires_through_thread_local_borrow() {
	thread_local! {
		static TEST_ROUTER: RefCell<Option<Router>> = const { RefCell::new(None) };
	}

	fn with_test_router<F, R>(f: F) -> R
	where
		F: FnOnce(&Router) -> R,
	{
		TEST_ROUTER.with(|r| f(r.borrow().as_ref().expect("Test router not initialized")))
	}

	// Arrange: build the router with two routes, seed the current path
	// to "/a" via the public push API (mirroring the Task 1 fallback).
	let router = Router::new().route("/a", page_a).route("/b", page_b);
	router.push("/a").expect("seed /a");
	with_runtime(|rt| rt.flush_updates());

	TEST_ROUTER.with(|r| *r.borrow_mut() = Some(router));

	let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
	let log_clone = log.clone();

	let _effect = Effect::new(move || {
		// Mirror the launcher closure: read the path Signal *through*
		// the thread-local borrow.
		let path = with_test_router(|r| r.current_path().get());
		log_clone.borrow_mut().push(path);
	});

	assert_eq!(*log.borrow(), vec!["/a".to_string()], "initial run");

	// Act.
	with_test_router(|r| r.push("/b").expect("push /b"));
	with_runtime(|rt| rt.flush_updates());

	// Assert.
	assert_eq!(
		*log.borrow(),
		vec!["/a".to_string(), "/b".to_string()],
		"Variant 2 (thread-local borrow) — if this fails, H1 is confirmed (track_dependency lost during RefCell::borrow)"
	);

	// Cleanup the test thread-local so other tests don't see stale state.
	TEST_ROUTER.with(|r| *r.borrow_mut() = None);
}
