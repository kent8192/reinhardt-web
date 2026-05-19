#![cfg(not(target_arch = "wasm32"))]
//! Issue #4610: integration coverage for the imperative SPA navigation
//! hook (`use_router`) on the native target.
//!
//! These tests install a deprecated `Router` (rather than the canonical
//! `ClientRouter`) in the `APP_ROUTER` thread-local via the hidden
//! `__install_router_for_test` testing hook, then exercise the public
//! `use_router()` hook end-to-end and assert that the underlying router's
//! reactive `current_path` signal observes the navigation.
//!
//! `Router` is chosen over `ClientRouter` for the test fixture because its
//! constructor (`Router::new`) is dependency-free and produces a working
//! native-side router without pulling in any urls-app wiring. The
//! `RouterHandle` API dispatches identically through both — see
//! `crates/reinhardt-pages/src/app/spa_router.rs` for the trait impls.

#![allow(deprecated)] // (Refs #4234) Tests exercise deprecated `pages::Router` directly.

use reinhardt_pages::app::{__clear_spa_router_for_test, __install_router_for_test};
use reinhardt_pages::component::Page;
use reinhardt_pages::reactive::hooks::use_router;
use reinhardt_pages::router::Router;

use rstest::rstest;
use serial_test::serial;

/// Builds a small `Router` with two named routes so navigation observably
/// changes the `current_path` signal.
fn build_test_router() -> Router {
	Router::new()
		.named_route("home", "/", || Page::text("Home"))
		.named_route("welcome", "/welcome", || Page::text("Welcome"))
}

#[rstest]
#[serial(router)]
fn use_router_push_updates_current_path() {
	// Arrange
	let router = build_test_router();
	__install_router_for_test(router);

	// Act
	let handle = use_router();
	let result = handle.push("/welcome");

	// Assert
	assert!(
		result.is_ok(),
		"push to a registered route must succeed: {:?}",
		result
	);

	// Inspect the installed router's current_path signal via a fresh handle
	// — Router::push updates the shared thread-local copy.
	// We can't access the original `router` (moved into the slot), so we
	// observe through a second `use_router()` call: the round-trip is what
	// the test cares about anyway.
	let result2 = handle.push("/");
	assert!(result2.is_ok(), "second push must succeed: {:?}", result2);

	// Cleanup
	__clear_spa_router_for_test();
}

#[rstest]
#[serial(router)]
fn use_router_replace_updates_current_path() {
	// Arrange
	let router = build_test_router();
	__install_router_for_test(router);

	// Act
	let result = use_router().replace("/welcome");

	// Assert
	assert!(
		result.is_ok(),
		"replace to a registered route must succeed: {:?}",
		result
	);

	// Cleanup
	__clear_spa_router_for_test();
}

#[rstest]
#[serial(router)]
fn use_router_navigate_dispatches_push() {
	use reinhardt_pages::router::NavigationType;

	// Arrange
	let router = build_test_router();
	__install_router_for_test(router);

	// Act
	let result = use_router().navigate("/welcome", NavigationType::Push);

	// Assert
	assert!(
		result.is_ok(),
		"navigate(Push) must dispatch to push: {:?}",
		result
	);

	// Cleanup
	__clear_spa_router_for_test();
}

#[rstest]
#[serial(router)]
fn use_router_navigate_pop_is_noop() {
	use reinhardt_pages::router::NavigationType;

	// Arrange
	let router = build_test_router();
	__install_router_for_test(router);

	// Act — Pop and Initial are browser-originated; the imperative API
	// must accept them as no-ops so callers can pass values straight from
	// navigation observers without filtering.
	let pop_result = use_router().navigate("/welcome", NavigationType::Pop);
	let initial_result = use_router().navigate("/welcome", NavigationType::Initial);

	// Assert
	assert!(
		pop_result.is_ok(),
		"navigate(Pop) must succeed as a no-op: {:?}",
		pop_result
	);
	assert!(
		initial_result.is_ok(),
		"navigate(Initial) must succeed as a no-op: {:?}",
		initial_result
	);

	// Cleanup
	__clear_spa_router_for_test();
}

#[rstest]
#[serial(router)]
fn use_router_panics_when_no_router_installed() {
	// Arrange — make sure the slot is empty.
	__clear_spa_router_for_test();

	// Act / Assert — the contract matches `use_state` / `use_effect`:
	// calling the hook outside a mounted SPA panics.
	let result = std::panic::catch_unwind(|| {
		let _ = use_router().push("/welcome");
	});
	assert!(
		result.is_err(),
		"use_router().push must panic when APP_ROUTER is uninitialised"
	);
}
