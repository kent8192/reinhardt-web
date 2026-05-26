//! Compile-time tests for `#[model(info = ...)]` (Issue #4194).
//!
//! Verifies that `info = false` prevents `{Model}Info` type generation.

#[test]
fn info_opt_out_compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/macros/ui/fail/info_opt_out_no_type.rs");
}
