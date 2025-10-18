//! Compile-time tests for template! macro
//!
//! These tests verify that the macro correctly rejects invalid template paths at compile time.

#[test]
fn test_compile_failures() {
    let t = trybuild::TestCases::new();

    // Test invalid template paths
    t.compile_fail("tests/ui/fail/*.rs");
}

#[test]
fn test_compile_success() {
    let t = trybuild::TestCases::new();

    // Test valid template paths
    t.pass("tests/ui/pass/*.rs");
}
