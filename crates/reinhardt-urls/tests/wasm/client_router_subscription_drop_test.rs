//! WASM-bindgen test asserting Inv-2 (drop deregisters listener) and
//! Inv-6 (Weak pruning) on `urls::ClientRouter`. Mirrors the
//! subscription-drop invariant from
//! `pages/tests/wasm/spa_navigation_diag_test.rs`. (Refs #4234)
//!
//! Run with (from the workspace root):
//!   wasm-pack test --chrome --headless --features wasm-diag-test \
//!     crates/reinhardt-urls -- --test client_router_subscription_drop_test

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
fn drop_subscription_deregisters_listener_inv2_inv6() {
	// Arrange
	let router = ClientRouter::new().named_route("a", "/a", page_a);
	let counter = Rc::new(Cell::new(0u64));
	let counter_clone = counter.clone();

	// Act: register, fire, drop, fire again
	let sub = router.on_navigate(move |_, _| {
		counter_clone.set(counter_clone.get() + 1);
	});
	router.push("/a").expect("first push must succeed");
	assert_eq!(counter.get(), 1);
	drop(sub);
	router.push("/a").expect("second push must succeed");

	// Assert
	assert_eq!(counter.get(), 1, "Inv-2: dropped listener does not fire");
	assert_eq!(
		router.__diag_observer_count(),
		0,
		"Inv-6: pruned Weak<NavigationListener>"
	);
}
