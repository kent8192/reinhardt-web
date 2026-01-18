//! Compile-time tests for path! macro
//!
//! These tests verify that the macro correctly rejects invalid paths at compile time.

#[test]
fn test_compile_failures() {
	let t = trybuild::TestCases::new();

	// Test invalid path patterns
	t.compile_fail("tests/ui/fail/*.rs");
}

#[test]
fn test_compile_success() {
	let t = trybuild::TestCases::new();

	// Test valid path patterns
	t.pass("tests/ui/pass/*.rs");
}
