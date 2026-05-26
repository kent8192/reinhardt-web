//! WASM-bindgen test asserting Inv-1 (push triggers `notify_observers`)
//! and Inv-5 (`__diag_dispatch_count` increments by exactly one per push)
//! on `urls::ClientRouter`. Mirrors Tier 1 of
//! `pages/tests/wasm/spa_navigation_diag_test.rs`. (Refs #4234)
//!
//! Run with (from the workspace root):
//!   wasm-pack test --chrome --headless --features wasm-diag-test \
//!     crates/reinhardt-urls -- --test client_router_observer_dispatch_test

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

fn page_b() -> Page {
	Page::empty()
}

#[wasm_bindgen_test]
fn push_dispatches_observer_once_inv1_inv5() {
	// Arrange
	let router = ClientRouter::new()
		.route("a", "/a", page_a)
		.route("b", "/b", page_b);
	let counter = Rc::new(Cell::new(0u64));
	let counter_clone = counter.clone();
	let _sub = router.on_navigate(move |_, _| {
		counter_clone.set(counter_clone.get() + 1);
	});
	let dispatch_before = router.__diag_dispatch_count();

	// Act
	router.push("/b").expect("push must succeed");

	// Assert
	assert_eq!(counter.get(), 1, "Inv-1: listener fired exactly once");
	assert_eq!(
		router.__diag_dispatch_count(),
		dispatch_before + 1,
		"Inv-5: dispatch_count incremented by one"
	);
}
