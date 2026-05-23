//! UI tests for `ClientRouter::page` and `FromRequest`.
//!
//! - `tests/ui/page/pass/*.rs` — valid usages must compile cleanly.
//! - `tests/ui/page/fail/*.rs` — invalid usages must produce predictable
//!   compiler diagnostics (single error type per file per
//!   `instructions/DOCUMENTATION_STANDARDS.md`).
//!
//! Refs #4668 / P7 part 2.

#[test]
fn page_ui_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/page/pass/*.rs");
}

#[test]
fn page_ui_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/page/fail/*.rs");
}
