//! Compile-time tests for macros using trybuild
//!
//! This test suite validates that:
//! - Valid macro usage compiles successfully (tests/ui/pass/*.rs)
//! - Invalid macro usage fails to compile (tests/ui/fail/*.rs)
//!
//! This tests the reinhardt-macros crate's compile-time behavior

// Reference to reinhardt-macros being tested
use reinhardt_macros as _;

#[test]
fn test_compile_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/*.rs");
}

#[test]
fn test_compile_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/fail/*.rs");
}

#[test]
fn test_path_macro_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/path/pass/*.rs");
}

#[test]
fn test_path_macro_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/path/fail/*.rs");
}

#[test]
fn test_permission_macro_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/permissions/pass/*.rs");
}

#[test]
fn test_permission_macro_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/permissions/fail/*.rs");
}

#[test]
fn test_routes_macro_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/routes/pass/*.rs");
}

#[test]
fn test_routes_macro_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/routes/fail/*.rs");
}

#[test]
fn test_api_view_macro_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/api_view/pass/*.rs");
}

#[test]
fn test_api_view_macro_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/api_view/fail/*.rs");
}

#[test]
fn test_action_macro_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/action/pass/*.rs");
}

#[test]
fn test_action_macro_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/action/fail/*.rs");
}

// Note: Generic types, lifetimes, and macro hygiene tests are in separate test files
// (generic_types_complete.rs, lifetime_annotations.rs, macro_hygiene.rs)
// They run as regular cargo tests, not trybuild compile tests

#[test]
fn test_validation_edge_cases_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/validation_edge_cases/multiple_invalid_methods.rs");
	t.compile_fail("tests/ui/validation_edge_cases/action_missing_both.rs");
	t.compile_fail("tests/ui/validation_edge_cases/action_url_path_no_slash.rs");
}

#[test]
fn test_validation_edge_cases_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/validation_edge_cases/case_sensitive_method.rs");
	t.pass("tests/ui/validation_edge_cases/empty_methods.rs");
}

#[test]
fn test_model_derive_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/model/pass/*.rs");
}

#[test]
fn test_model_derive_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/model/fail/*.rs");
}
