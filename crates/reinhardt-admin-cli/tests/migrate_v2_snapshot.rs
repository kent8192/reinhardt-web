//! Golden-file snapshot tests for the migrate-manouche-v2 rules.

use rstest::rstest;
use std::path::PathBuf;

use reinhardt_admin_cli::migrate_v2::rewriter::FileRewriter;
use reinhardt_admin_cli::migrate_v2::rules;

fn apply(rule: &dyn FileRewriter, src: &str) -> String {
	let ast: syn::File = syn::parse_str(src).unwrap();
	let out_ast = rule.rewrite(ast);
	prettyplease::unparse(&out_ast)
}

fn fixture_path(rule: &str, name: &str) -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("tests/fixtures/migrate_v2")
		.join(rule)
		.join(name)
}

#[rstest]
#[case::bare_ident("bare_ident")]
#[case::watch_unwrap("watch_unwrap")]
#[case::use_effect_deps("use_effect_deps")]
fn rule_matches_expected_fixture(#[case] rule_name: &str) {
	// Arrange
	let input = std::fs::read_to_string(fixture_path(rule_name, "input.rs")).unwrap();
	let expected = std::fs::read_to_string(fixture_path(rule_name, "expected.rs")).unwrap();
	let all_rules = rules::all();
	let rule = all_rules
		.iter()
		.find(|r| r.name() == rule_name)
		.unwrap_or_else(|| panic!("rule `{rule_name}` not registered"));

	// Act
	let actual = apply(&**rule, &input);

	// Assert
	assert_eq!(
		actual.trim(),
		expected.trim(),
		"{rule_name} did not produce expected output"
	);
}
