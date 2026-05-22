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
#![allow(deprecated)]
use reinhardt_pages::app::{
	__clear_spa_router_for_test, __current_path_for_test, __install_router_for_test,
};
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
/// RAII guard that installs a test router on construction and clears the
/// thread-local SPA router slot on drop. Using `Drop` (instead of an
/// explicit cleanup call) guarantees the slot is cleared even when an
/// assertion panic short-circuits the test body, preventing leakage into
/// the next `#[serial(router)]` test.
struct SpaRouterGuard;
impl SpaRouterGuard {
	fn install(router: Router) -> Self {
		__install_router_for_test(router);
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
	let _guard = SpaRouterGuard::install(build_test_router());
	let handle = use_router();
	let result = handle.push("/welcome");
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
	let _guard = SpaRouterGuard::install(build_test_router());
	let result = use_router().replace("/welcome");
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
	let _guard = SpaRouterGuard::install(build_test_router());
	let result = use_router().navigate("/welcome", NavigationType::Push);
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
	let _guard = SpaRouterGuard::install(build_test_router());
	let initial_path = __current_path_for_test();
	let pop_result = use_router().navigate("/welcome", NavigationType::Pop);
	let initial_result = use_router().navigate("/welcome", NavigationType::Initial);
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
	let _noop_component = Page::text("Noop");
	__clear_spa_router_for_test();
	let result = use_router().push("/welcome");
	assert!(
		matches!(result, Err(NavigateError::RouterNotInstalled)),
		"expected Err(RouterNotInstalled), got {:?}",
		result
	);
}
