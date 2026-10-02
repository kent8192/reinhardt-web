//! Full-expansion compile-time tests for model enum fields.

#[test]
fn model_enum_ui() {
	let t = trybuild::TestCases::new();
	t.pass("tests/macros/ui/pass/model_enum_fields.rs");
	t.compile_fail("tests/macros/ui/fail/model_enum_max_length_too_small.rs");
	t.compile_fail("tests/macros/ui/fail/model_enum_integer_max_length.rs");
}
