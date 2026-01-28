//! Trybuild UI tests for reinhardt-db-macros
//!
//! This test file compiles all test cases in tests/ui/pass/ and tests/ui/fail/
//! and verifies that they produce the expected compilation results.

#[test]
fn ui_tests() {
	// The trybuild! macro will automatically:
	// 1. Compile all .rs files in tests/ui/pass/ and expect them to succeed
	// 2. Compile all .rs files in tests/ui/fail/ and compare errors to .stderr files
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/basic_document.rs");
	t.pass("tests/ui/pass/field_attributes.rs");
	t.compile_fail("tests/ui/fail/missing_collection.rs");
	t.compile_fail("tests/ui/fail/unsupported_backend.rs");
	t.compile_fail("tests/ui/fail/missing_primary_key.rs");
}
