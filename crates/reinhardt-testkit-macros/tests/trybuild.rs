//! Run with `cargo test -p reinhardt-testkit-macros --test trybuild`.

#[test]
fn macro_ui_tests() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass_singleton.rs");
	t.pass("tests/ui/pass_factory.rs");
	t.compile_fail("tests/ui/fail_unknown_kind.rs");
}
