use rstest::rstest;

#[rstest]
fn http_error_pass_cases() {
	let t = trybuild::TestCases::new();
	t.pass("tests/macros/ui/http_error/pass/*.rs");
}

#[rstest]
fn http_error_fail_cases() {
	let t = trybuild::TestCases::new();
	t.compile_fail("tests/macros/ui/http_error/fail/*.rs");
}
