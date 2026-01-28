//! Compile tests for document and field macros

#[test]
fn test_document_macro_compile() {
	let t = trybuild::TestCases::new();
	t.pass("tests/ui/pass/*.rs");
	t.compile_fail("tests/ui/fail/*.rs");
}
