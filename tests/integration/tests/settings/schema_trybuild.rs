#[test]
fn settings_schema_compile_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/settings/ui/fail/non_secret_ref.rs");
}
