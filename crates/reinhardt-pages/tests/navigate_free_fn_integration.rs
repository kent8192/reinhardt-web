#![cfg(not(target_arch = "wasm32"))]
//! Issue #4610: integration coverage for the free `navigate()` SPA
//! navigation function on the native target.
//!
//! Companion to `use_router_integration.rs` — that file exercises the hook
//! form; this one verifies the free function delegates identically. The
//! form! macro's WASM-side codegen calls the free function (not the hook)
//! because hooks must be invoked from a reactive context, which the
//! generated `async fn submit(&self)` is not.

use reinhardt_pages::app::{__clear_spa_router_for_test, __install_client_router_for_test};
use reinhardt_pages::component::Page;
use reinhardt_pages::router::{NavigationType, navigate};
use reinhardt_urls::routers::ClientRouter;

use rstest::rstest;
use serial_test::serial;

/// Builds a small `ClientRouter` with two named routes so navigation observably
/// changes the `current_path` signal.
fn build_test_router() -> ClientRouter {
	ClientRouter::new()
		.named_route("home", "/", || Page::text("Home"))
		.named_route("profile", "/profile", || Page::text("Profile"))
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
fn navigate_push_succeeds() {
	// Arrange
	let _guard = SpaRouterGuard::install(build_test_router());

	// Act
	let result = navigate("/profile", NavigationType::Push);

	// Assert
	assert!(
		result.is_ok(),
		"navigate(Push) to a registered route must succeed: {:?}",
		result
	);
}

#[rstest]
#[serial(router)]
fn navigate_replace_succeeds() {
	// Arrange
	let _guard = SpaRouterGuard::install(build_test_router());

	// Act
	let result = navigate("/profile", NavigationType::Replace);

	// Assert
	assert!(
		result.is_ok(),
		"navigate(Replace) to a registered route must succeed: {:?}",
		result
	);
}

#[rstest]
#[serial(router)]
fn navigate_accepts_owned_string() {
	// Arrange — `impl Into<String>` must accept `&str` (above) and `String`.
	let _guard = SpaRouterGuard::install(build_test_router());

	// Act
	let path: String = "/profile".to_string();
	let result = navigate(path, NavigationType::Push);

	// Assert
	assert!(
		result.is_ok(),
		"navigate must accept owned String: {:?}",
		result
	);
}
