//! Trybuild tests for the `#[injectable]` macro.
//!
//! - `compile_fail/*.rs` — invalid usages must produce clear compiler errors.
//! - `compile_pass/*.rs` — valid usages must compile without the consumer
//!   pulling in extra crates (notably `async-trait`, regression test for
//!   issue #4445).

#![cfg(not(all(target_family = "wasm", target_os = "unknown")))]

#[test]
fn injectable_compile_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/compile_fail/*.rs");
}

#[test]
fn injectable_compile_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/compile_pass/*.rs");
}
