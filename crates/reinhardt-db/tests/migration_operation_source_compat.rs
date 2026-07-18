#[cfg(feature = "migrations")]
#[test]
fn legacy_drop_constraint_source_shape_compiles() {
	let tests = trybuild::TestCases::new();
	tests.pass("tests/ui/drop_constraint_legacy.rs");
}
