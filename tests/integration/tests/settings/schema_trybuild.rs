#[test]
fn settings_schema_compile_fail() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/settings/ui/fail/non_secret_ref.rs");
}

#[test]
fn settings_schema_compile_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/settings/ui/pass/cfg_gated_node_field.rs");
}
