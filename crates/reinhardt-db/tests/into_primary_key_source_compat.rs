#![cfg(feature = "orm")]

use rstest::rstest;

#[rstest]
fn downstream_text_key_implementations_conflict_with_framework_conversions() {
	// Arrange
	let tests = trybuild::TestCases::new();

	// Act / Assert
	tests.compile_fail("tests/ui/into_primary_key_text_impl_conflicts.rs");
}
