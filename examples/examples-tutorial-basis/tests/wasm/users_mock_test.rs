//! Users Component WASM Tests with Mocking
//!
//! Layer 2 tests for users (authentication) components that interact with
//! server functions. Mirrors `polls_mock_test.rs` for the polls app and
//! exercises the four users server functions — `login`, `register`,
//! `logout`, `current_user` — under `MockServiceWorker` so each component
//! is rendered against typed mock responses without standing up a real
//! database.
//!
//! **Test Categories:**
//! - Pure rendering tests (no server_fn interaction)
//! - Real `#[server_fn]` round-trip tests via MSW (success + error paths)
//! - Component-with-MSW smoke tests
//! - Shared-type serialization sanity checks
//!
//! **Run with**: `cargo make wasm-test`
//!
//! Gated identically to `polls_mock_test.rs`: behind the project-local
//! `msw` feature so the test target is skipped on toolchains without the
//! framework MSW facade (tracked in upstream #4287 / PR #4288).

#![cfg(all(target_family = "wasm", target_os = "unknown"))]

use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

// Import the actual user-app components and server_fns we want to drive.
use examples_tutorial_basis::apps::users::client::components::{
	login_form, logout_form, signup_form,
};
use examples_tutorial_basis::apps::users::server_fn::{current_user, login, logout, register};
use examples_tutorial_basis::shared::types::UserInfo;
use reinhardt::pages::component::Page;
use reinhardt::pages::server_fn::ServerFnError;
use reinhardt::test::msw::MockServiceWorker;

// ============================================================================
// Test Fixtures
// ============================================================================

/// Create a mock UserInfo for testing. Matches the schema produced by the
/// real `From<User> for UserInfo` impl in `src/shared/types.rs`.
fn mock_user() -> UserInfo {
	UserInfo {
		id: 1,
		username: "alice".to_string(),
		is_active: true,
	}
}

// ============================================================================
// Login Form Rendering Tests
// ============================================================================

/// `login_form()` renders as a top-level `Page::Element`.
#[wasm_bindgen_test]
fn test_login_form_renders() {
	let view = login_form();
	assert!(matches!(view, Page::Element(_)));
}

/// Login form contains the expected field labels and a submit affordance.
#[wasm_bindgen_test]
fn test_login_form_has_fields() {
	let view = login_form();
	let html = view.render_to_string();
	assert!(html.contains("Username"), "expected Username label");
	assert!(html.contains("Password"), "expected Password label");
	assert!(html.contains("Sign in"), "expected submit affordance");
}

// ============================================================================
// Signup Form Rendering Tests
// ============================================================================

/// `signup_form()` renders as a top-level `Page::Element`.
#[wasm_bindgen_test]
fn test_signup_form_renders() {
	let view = signup_form();
	assert!(matches!(view, Page::Element(_)));
}

/// Signup form contains username, password, and confirmation fields.
#[wasm_bindgen_test]
fn test_signup_form_has_required_fields() {
	let view = signup_form();
	let html = view.render_to_string();
	assert!(html.contains("Username"), "expected Username label");
	assert!(html.contains("Password"), "expected Password label");
	assert!(
		html.contains("Confirm password"),
		"expected password confirmation label"
	);
	assert!(
		html.contains("Create account"),
		"expected create-account submit affordance"
	);
}

// ============================================================================
// Logout Form Rendering Tests
// ============================================================================

/// `logout_form()` renders as a top-level `Page::Element`.
#[wasm_bindgen_test]
fn test_logout_form_renders() {
	let view = logout_form();
	assert!(matches!(view, Page::Element(_)));
}

/// Logout view contains a "Sign out" affordance.
#[wasm_bindgen_test]
fn test_logout_form_has_submit() {
	let view = logout_form();
	let html = view.render_to_string();
	assert!(
		html.contains("Sign out"),
		"expected sign-out submit affordance"
	);
}

// ============================================================================
// Real server_fn round-trip tests via MSW
//
// Same pattern as `polls_mock_test.rs`: install a typed handler, invoke
// the application's `#[server_fn]` function, and assert on the payload
// the client code would see.
// ============================================================================

/// `login(...)` returns the mocked `UserInfo` on the happy path.
#[wasm_bindgen_test]
async fn test_login_returns_mocked_user() {
	let worker = MockServiceWorker::new();
	let mocked = mock_user();
	worker.handle_server_fn::<login::marker>({
		let u = mocked.clone();
		move |_args| Ok(u.clone())
	});
	worker.start().await;

	let user = login("alice".to_string(), "hunter2".to_string())
		.await
		.expect("login should succeed");

	assert_eq!(user.id, mocked.id);
	assert_eq!(user.username, "alice");
	worker.calls_to_server_fn::<login::marker>().assert_called();
}

/// `login(...)` propagates a 401 from MSW (invalid credentials).
#[wasm_bindgen_test]
async fn test_login_surfaces_invalid_credentials() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<login::marker>(|_args| {
		Err(ServerFnError::server(401, "Invalid credentials"))
	});
	worker.start().await;

	let err = login("alice".to_string(), "wrong".to_string())
		.await
		.expect_err("expected invalid-credentials error");
	match err {
		ServerFnError::Server { status, message } => {
			assert_eq!(status, 401, "expected HTTP 401 status");
			assert_eq!(
				message, "Invalid credentials",
				"expected mocked credentials message to propagate verbatim"
			);
		}
		other => panic!("expected ServerFnError::Server, got: {other:?}"),
	}
}

/// `register(...)` returns the freshly created `UserInfo` on success.
#[wasm_bindgen_test]
async fn test_register_returns_mocked_user() {
	let worker = MockServiceWorker::new();
	let mocked = mock_user();
	worker.handle_server_fn::<register::marker>({
		let u = mocked.clone();
		move |_args| Ok(u.clone())
	});
	worker.start().await;

	let user = register(
		"alice".to_string(),
		"hunter2".to_string(),
		"hunter2".to_string(),
	)
	.await
	.expect("register should succeed");

	assert_eq!(user.username, "alice");
	worker
		.calls_to_server_fn::<register::marker>()
		.assert_called();
}

/// `register(...)` surfaces a 400 from MSW when a validation rule fires
/// server-side (e.g., duplicate username, mismatched confirmation).
#[wasm_bindgen_test]
async fn test_register_surfaces_validation_error() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<register::marker>(|_args| {
		Err(ServerFnError::server(400, "Username is already taken"))
	});
	worker.start().await;

	let err = register(
		"alice".to_string(),
		"hunter2".to_string(),
		"hunter2".to_string(),
	)
	.await
	.expect_err("expected validation error");
	match err {
		ServerFnError::Server { status, message } => {
			assert_eq!(status, 400, "expected HTTP 400 status");
			assert_eq!(
				message, "Username is already taken",
				"expected mocked validation message to propagate verbatim"
			);
		}
		other => panic!("expected ServerFnError::Server, got: {other:?}"),
	}
}

/// `logout()` round-trips through MSW.
#[wasm_bindgen_test]
async fn test_logout_succeeds() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<logout::marker>(|_args| Ok(()));
	worker.start().await;

	logout().await.expect("logout should succeed");
	worker
		.calls_to_server_fn::<logout::marker>()
		.assert_called();
}

/// `current_user()` returns `Some(user)` when a session is active.
#[wasm_bindgen_test]
async fn test_current_user_returns_authenticated_user() {
	let worker = MockServiceWorker::new();
	let mocked = mock_user();
	worker.handle_server_fn::<current_user::marker>({
		let u = mocked.clone();
		move |_args| Ok(Some(u.clone()))
	});
	worker.start().await;

	let user = current_user().await.expect("current_user should succeed");

	let user = user.expect("authenticated session should return a user");
	assert_eq!(user.id, mocked.id);
	assert_eq!(user.username, mocked.username);
	assert_eq!(user.is_active, mocked.is_active);
	worker
		.calls_to_server_fn::<current_user::marker>()
		.assert_called();
}

/// `current_user()` returns `None` for anonymous (unauthenticated) callers.
#[wasm_bindgen_test]
async fn test_current_user_returns_none_for_anonymous() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<current_user::marker>(|_args| Ok(None));
	worker.start().await;

	let user = current_user()
		.await
		.expect("current_user should succeed even when anonymous");

	assert!(user.is_none(), "anonymous session should resolve to None");
}

// ============================================================================
// Component-with-MSW smoke tests
//
// These prove the form components construct without panicking when MSW
// is active — the same regression class chased by the polls smoke tests.
// ============================================================================

/// `login_form()` constructs cleanly with MSW intercepting `login`.
#[wasm_bindgen_test]
async fn test_login_form_with_msw_active() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<login::marker>(|_args| Ok(mock_user()));
	worker.start().await;

	let view = login_form();
	assert!(matches!(view, Page::Element(_)));
}

/// `signup_form()` constructs cleanly with MSW intercepting `register`.
#[wasm_bindgen_test]
async fn test_signup_form_with_msw_active() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<register::marker>(|_args| Ok(mock_user()));
	worker.start().await;

	let view = signup_form();
	assert!(matches!(view, Page::Element(_)));
}

/// `logout_form()` constructs cleanly with MSW intercepting `logout`.
#[wasm_bindgen_test]
async fn test_logout_form_with_msw_active() {
	let worker = MockServiceWorker::new();
	worker.handle_server_fn::<logout::marker>(|_args| Ok(()));
	worker.start().await;

	let view = logout_form();
	assert!(matches!(view, Page::Element(_)));
}

// ============================================================================
// Shared Types Serialization Tests
// ============================================================================

/// `UserInfo` round-trips through serde JSON without losing fields.
#[wasm_bindgen_test]
fn test_user_info_round_trip() {
	let original = mock_user();
	let json = serde_json::to_string(&original).expect("UserInfo should serialize");
	let parsed: UserInfo = serde_json::from_str(&json).expect("UserInfo should deserialize");
	assert_eq!(parsed.id, original.id);
	assert_eq!(parsed.username, original.username);
	assert_eq!(parsed.is_active, original.is_active);
}
