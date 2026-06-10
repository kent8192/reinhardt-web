//! Compile-time coverage for ORM QuerySet examples shown in website docs.

#[test]
fn queryset_docs_example_compile_pass() {
	let t = trybuild::TestCases::new();
	t.pass("tests/orm/ui/pass/queryset_docs_example.rs");
}
