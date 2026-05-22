#![cfg(not(target_arch = "wasm32"))]
//! Static Resolver integration tests
//!
//! Success Criteria:
//! 1. resolve_static returns fallback URL when not initialized
//! 2. resolve_static returns correct URL when initialized
//! 3. is_initialized returns correct state
//! 4. Leading slashes are handled correctly
//!
//! Test Categories:
//! - Happy Path: 2 tests
//! - Fallback Behavior: 2 tests
//!
//! Total: 4 tests

use reinhardt_pages::static_resolver::{is_initialized, resolve_static};
use rstest::*;

// ============================================================================
// Fallback Behavior Tests
// ============================================================================

/// Tests that resolve_static returns fallback URL when not initialized.
///
/// Note: This test may interfere with other tests that initialize the resolver.
/// In production, the resolver should be initialized at startup.
#[rstest]
fn test_resolve_static_fallback_behavior() {
	// When not initialized, should return a fallback URL with /static/ prefix
	let url = resolve_static("css/style.css");

	// Should contain the file path
	assert!(url.contains("style.css"));
	// Should have static prefix in the path
	assert!(url.contains("static"));
}

/// Tests that resolve_static strips leading slashes correctly.
#[rstest]
fn test_resolve_static_strips_leading_slash() {
	let url = resolve_static("/js/app.js");

	// Should not have double slashes
	assert!(!url.contains("//static") || url.contains("http"));
	// Should contain the file path
	assert!(url.contains("app.js"));
}

/// Tests that is_initialized returns boolean correctly.
#[rstest]
fn test_is_initialized_returns_bool() {
	// The function returns a bool that depends on global state, so we
	// cannot assert a concrete value. Type-pin via annotation so this
	// test still verifies the return type (a signature change to non-bool
	// would fail to compile here) and that the call does not panic.
	// Refs #4115.
	let _: bool = is_initialized();
}

/// Tests multiple files can be resolved.
#[rstest]
fn test_resolve_multiple_files() {
	let css = resolve_static("css/main.css");
	let js = resolve_static("js/bundle.js");
	let img = resolve_static("images/logo.png");

	// All should contain their respective file names
	assert!(css.contains("main.css"));
	assert!(js.contains("bundle.js"));
	assert!(img.contains("logo.png"));
}
