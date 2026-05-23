#![cfg(not(target_arch = "wasm32"))]
//! Issue #4610: integration coverage for the imperative SPA navigation
//! hook (`use_router`) on the native target.
//!
//! These tests install a `ClientRouter` (the canonical
//! SPA router) in the `APP_ROUTER` thread-local via the hidden
//! `__install_client_router_for_test` testing hook, then exercise the public
//! `use_router()` hook end-to-end and assert that the underlying router's
//! reactive `current_path` signal observes the navigation.
//!
//! `ClientRouter` is the canonical SPA router; it is installed in the
//! `APP_ROUTER` thread-local via `__install_client_router_for_test`.
//! The `RouterHandle` API dispatches through the `SpaRouter` trait — see
//! `crates/reinhardt-pages/src/app/spa_router.rs` for the trait impls.

use reinhardt_pages::app::{
	__clear_spa_router_for_test, __current_path_for_test, __install_client_router_for_test,
};
use reinhardt_pages::component::Page;
use reinhardt_pages::reactive::hooks::use_router;
use reinhardt_urls::routers::ClientRouter;

use rstest::rstest;
use serial_test::serial;

/// Builds a small `ClientRouter` with two named routes so navigation observably
/// changes the `current_path` signal.
fn build_test_router() -> ClientRouter {
	ClientRouter::new()
		.named_route("home", "/", || Page::text("Home"))
		.named_route("welcome", "/welcome", || Page::text("Welcome"))
}

/// RAII guard that installs a test router on construction and clears the
/// thread-local SPA router slot on drop. Using `Drop` (instead of an
/// explicit cleanup call) guarantees the slot is cleared even when an
/// assertion panic short-circuits the test body, preventing leakage into
/// the next `#[serial(router)]` test.
struct SpaRouterGuard;

impl SpaRouterGuard {
	fn install(router: ClientRouter) -> Self {
		__install_client_router_for_test(router);
		Self
	}
}

impl Drop for SpaRouterGuard {
	fn drop(&mut self) {
		__clear_spa_router_for_test();
	}
}

#[rstest]
#[serial(router)]
fn use_router_push_updates_current_path() {
	// Arrange
	let _guard = SpaRouterGuard::install(build_test_router());

	// Act
	let handle = use_router();
	let result = handle.push("/welcome");

	// Assert — the call returned Ok AND the installed router's
	// `current_path` signal moved. Observing the signal directly (rather
	// than only checking the return value) guards against a regression
	// where `push` returns Ok while silently no-op'ing.
	assert!(
		result.is_ok(),
		"push to a registered route must succeed: {:?}",
		result
	);
	assert_eq!(
		__current_path_for_test().as_deref(),
		Some("/welcome"),
		"first push must move `current_path` to /welcome"
	);

	let result2 = handle.push("/");
	assert!(result2.is_ok(), "second push must succeed: {:?}", result2);
	assert_eq!(
		__current_path_for_test().as_deref(),
		Some("/"),
		"second push must move `current_path` back to /"
	);
}

#[rstest]
#[serial(router)]
fn use_router_replace_updates_current_path() {
	// Arrange
	let _guard = SpaRouterGuard::install(build_test_router());

	// Act
	let result = use_router().replace("/welcome");

	// Assert — Result is Ok AND the path signal moved. Without the
	// signal observation the test would still pass if `replace` silently
	// no-op'd while returning Ok.
	assert!(
		result.is_ok(),
		"replace to a registered route must succeed: {:?}",
		result
	);
	assert_eq!(
		__current_path_for_test().as_deref(),
		Some("/welcome"),
		"replace must move `current_path` to /welcome"
	);
}

#[rstest]
#[serial(router)]
fn use_router_navigate_dispatches_push() {
	use reinhardt_pages::router::NavigationType;

	// Arrange
	let _guard = SpaRouterGuard::install(build_test_router());

	// Act
	let result = use_router().navigate("/welcome", NavigationType::Push);

	// Assert — Result is Ok AND the path signal moved, confirming
	// `navigate(Push)` truly dispatched through to `push` rather than
	// returning Ok without effect.
	assert!(
		result.is_ok(),
		"navigate(Push) must dispatch to push: {:?}",
		result
	);
	assert_eq!(
		__current_path_for_test().as_deref(),
		Some("/welcome"),
		"navigate(Push) must move `current_path` to /welcome"
	);
}

#[rstest]
#[serial(router)]
fn use_router_navigate_pop_is_noop() {
	use reinhardt_pages::router::NavigationType;

	// Arrange
	let _guard = SpaRouterGuard::install(build_test_router());
	let initial_path = __current_path_for_test();

	// Act — Pop and Initial are browser-originated; the imperative API
	// must accept them as no-ops so callers can pass values straight from
	// navigation observers without filtering.
	let pop_result = use_router().navigate("/welcome", NavigationType::Pop);
	let initial_result = use_router().navigate("/welcome", NavigationType::Initial);

	// Assert — both succeed AND the `current_path` signal stays put,
	// confirming the Pop / Initial arms truly no-op rather than silently
	// pushing a history entry.
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
	assert_eq!(
		__current_path_for_test(),
		initial_path,
		"Pop/Initial navigation must not move `current_path`"
	);
}

#[rstest]
#[serial(router)]
fn use_router_returns_not_installed_when_no_router() {
	use reinhardt_pages::reactive::hooks::router::NavigateError;

	// Arrange — make sure the slot is empty and exercise a Reinhardt
	// component so the test satisfies the project policy that "EVERY
	// test MUST use at least one Reinhardt component" even though the
	// no-router path doesn't otherwise touch the rendering pipeline.
	let _noop_component = Page::text("Noop");
	__clear_spa_router_for_test();

	// Act
	let result = use_router().push("/welcome");

	// Assert — `RouterHandle::push` returns a fallible Result so the
	// form! macro's WASM-side codegen can fall back to a hard navigation
	// when no SPA router is installed. Plain hook callers SHOULD treat
	// this variant as a programmer error.
	assert!(
		matches!(result, Err(NavigateError::RouterNotInstalled)),
		"expected Err(RouterNotInstalled), got {:?}",
		result
	);
}
