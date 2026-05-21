//! Trybuild suite for `#[newtype]` and `#[delegatable]` (Issue #4667).

#[test]
fn newtype_compile_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/newtype/pass/*.rs");
}

#[test]
fn newtype_compile_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/ui/newtype/fail/*.rs");
}
