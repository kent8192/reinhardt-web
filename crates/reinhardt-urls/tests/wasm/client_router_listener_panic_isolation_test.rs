//! WASM-bindgen test asserting Inv-4: re-entrant `on_navigate` calls
//! from inside a listener must not panic on `RefCell` re-entry. The
//! snapshot-then-iterate pattern in
//! `ClientRouter::notify_observers` releases the borrow before
//! invoking listeners, so a listener may register or drop other
//! listeners mid-dispatch without panicking. (Refs #4234)
//!
//! Run with (from the workspace root):
//!   wasm-pack test --chrome --headless --features wasm-diag-test \
//!     crates/reinhardt-urls -- --test client_router_listener_panic_isolation_test

#![cfg(all(target_arch = "wasm32", feature = "wasm-diag-test"))]

use reinhardt_core::page::Page;
use reinhardt_urls::routers::ClientRouter;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn page_a() -> Page {
	Page::empty()
}

#[wasm_bindgen_test]
fn reentrant_on_navigate_does_not_panic_inv4() {
	// Arrange
	let router = Rc::new(ClientRouter::new().named_route("a", "/a", page_a));
	let later_fires = Rc::new(Cell::new(0u64));
	let router_for_listener = router.clone();
	let later_fires_clone = later_fires.clone();

	// Listener that registers a second listener mid-dispatch. The inner
	// subscription is dropped at the end of the closure, removing the
	// listener it registered; dispatch must not panic on RefCell
	// re-entry while this happens.
	let _sub = router.on_navigate(move |_, _| {
		let inner = later_fires_clone.clone();
		let _sub2 = router_for_listener.on_navigate(move |_, _| {
			inner.set(inner.get() + 1);
		});
	});

	// Act
	router.push("/a").expect("first push must succeed");
	router
		.push("/a")
		.expect("second push must not panic on RefCell");

	// Assert: the second listener (registered mid-dispatch) was dropped
	// before the second push, so it never fired.
	assert_eq!(later_fires.get(), 0);
}
