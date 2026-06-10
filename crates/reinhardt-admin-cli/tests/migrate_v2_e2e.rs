//! End-to-end tests for the `migrate-manouche-v2` command.
//!
//! These tests spawn the compiled `reinhardt-admin` binary and inspect real
//! files on disk, covering command-line behavior that rule-level snapshot
//! tests cannot validate.

use proptest::prelude::*;
use rstest::rstest;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

const REINHARDT_ADMIN: &str = env!("CARGO_BIN_EXE_reinhardt-admin");

fn run_migrate(root: &Path, args: &[&str]) -> Output {
	Command::new(REINHARDT_ADMIN)
		.arg("migrate-manouche-v2")
		.args(args)
		.arg(root)
		.output()
		.expect("failed to spawn reinhardt-admin migrate-manouche-v2")
}

fn stdout(output: &Output) -> String {
	String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
	String::from_utf8_lossy(&output.stderr).into_owned()
}

fn compact_ws(input: &str) -> String {
	input.chars().filter(|c| !c.is_whitespace()).collect()
}

fn assert_success(output: &Output) {
	assert!(
		output.status.success(),
		"command failed with exit {:?}\nstdout:\n{}\nstderr:\n{}",
		output.status.code(),
		stdout(output),
		stderr(output)
	);
}

fn assert_failure(output: &Output) {
	assert!(
		!output.status.success(),
		"command unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
		stdout(output),
		stderr(output)
	);
}

fn write(path: &Path, content: &str) {
	if let Some(parent) = path.parent() {
		fs::create_dir_all(parent).expect("create parent dir");
	}
	fs::write(path, content).expect("write fixture");
}

fn generated_control_flow_page() -> impl Strategy<Value = String> {
	let match_body = prop_oneof![
		Just("match value { item => div { title } }"),
		Just("match value { Some(entry) => div { title }, None => span { fallback } }"),
		Just(
			"match result { Ok(entry) if ready => div { title }, Err(error_name) => span { fallback } }"
		),
		Just(
			"match value { Some(entry) => if (ready) { h1 { title } } else { p { fallback } }, None => span { fallback } }"
		),
	];

	(
		match_body,
		prop::sample::select(vec![
			"for item in items { Row { title } }",
			"while (ready) { span { title } break; }",
			"loop { span { fallback } break; }",
		]),
	)
		.prop_map(|(match_body, loop_body)| {
			format!(
				r#"
fn view(
    value: Option<String>,
    result: Result<String, String>,
    ready: bool,
    title: String,
    fallback: String,
    items: Vec<String>,
) {{
    page! {{
        section {{
            let local_value = fallback;
            {match_body}
            {loop_body}
        }}
    }};
}}
"#
			)
		})
}

#[rstest]
fn rewrites_multiple_files_and_reports_changed_count() {
	let tmp = TempDir::new().expect("tempdir");
	let page_file = tmp.path().join("src/page.rs");
	let props_file = tmp.path().join("src/components.rs");

	write(
		&page_file,
		r#"
fn view(name: String, count: usize) {
    page! {
        div {
            name
            watch {
                span { count }
            }
        }
    };
    use_effect(move || {
        let _ = count;
    });
}
"#,
	);
	write(
		&props_file,
		r#"
#[derive(Clone, Default)]
pub struct CardProps {
    pub title: String,
    pub subtitle: Option<String>,
    pub count: usize,
}
"#,
	);

	let output = run_migrate(tmp.path(), &[]);
	assert_success(&output);

	let page = fs::read_to_string(&page_file).expect("read page file");
	let props = fs::read_to_string(&props_file).expect("read props file");
	assert!(
		page.contains("{ name }") || page.contains("{name}"),
		"bare identifier was not wrapped:\n{page}"
	);
	assert!(
		!page.contains("watch"),
		"watch wrapper should be removed:\n{page}"
	);
	assert!(
		page.contains("compile_error!"),
		"use_effect placeholder deps were not inserted:\n{page}"
	);
	assert!(
		props.contains("bon::Builder"),
		"Props derive was not migrated:\n{props}"
	);
	assert!(
		props.contains("#[builder(default)]"),
		"default builder attributes were not inserted:\n{props}"
	);
	assert!(
		stdout(&output).contains("Done. 2 file(s) changed."),
		"unexpected stdout:\n{}",
		stdout(&output)
	);
}

#[rstest]
fn preserves_inner_module_docs_when_rewriting_first_item() {
	let tmp = TempDir::new().expect("tempdir");
	let file = tmp.path().join("page.rs");
	write(
		&file,
		r#"//! Module docs stay outside the rewritten item.
fn view(title: String) {
    page! { div { title } };
}
"#,
	);

	let output = run_migrate(tmp.path(), &[]);
	assert_success(&output);

	let after = fs::read_to_string(&file).expect("read rewritten file");
	assert!(
		after.starts_with("//! Module docs stay outside the rewritten item."),
		"inner module docs should not be replaced with the first item:\n{after}"
	);
	assert!(
		compact_ws(&after).contains("{title}"),
		"first item should still be migrated:\n{after}"
	);
}

#[rstest]
fn dry_run_reports_changes_without_writing_files() {
	let tmp = TempDir::new().expect("tempdir");
	let file = tmp.path().join("page.rs");
	let original = r#"
fn view(name: String) {
    page! { div { name } };
}
"#;
	write(&file, original);

	let output = run_migrate(tmp.path(), &["--dry-run"]);
	assert_success(&output);

	let after = fs::read_to_string(&file).expect("read file after dry-run");
	assert_eq!(after, original, "dry-run must not rewrite files");
	assert!(
		stdout(&output).contains("would rewrite:"),
		"dry-run should list pending rewrite:\n{}",
		stdout(&output)
	);
	assert!(
		stdout(&output).contains("Done. 1 file(s) would change."),
		"unexpected stdout:\n{}",
		stdout(&output)
	);
}

#[rstest]
fn skip_rule_leaves_that_rule_unapplied_but_runs_others() {
	let tmp = TempDir::new().expect("tempdir");
	let file = tmp.path().join("page.rs");
	write(
		&file,
		r#"
fn view(name: String, count: usize) {
    page! {
        div {
            name
            watch { span { count } }
        }
    };
}
"#,
	);

	let output = run_migrate(tmp.path(), &["--skip", "bare_ident"]);
	assert_success(&output);

	let after = fs::read_to_string(&file).expect("read rewritten file");
	assert!(
		after.contains("name"),
		"source should still contain the bare identifier:\n{after}"
	);
	assert!(
		!after.contains("{ name }") && !after.contains("{name}"),
		"bare_ident should have been skipped:\n{after}"
	);
	assert!(
		!after.contains("watch"),
		"watch_unwrap should still run when bare_ident is skipped:\n{after}"
	);
}

#[rstest]
fn rerunning_on_migrated_tree_is_idempotent() {
	let tmp = TempDir::new().expect("tempdir");
	let file = tmp.path().join("page.rs");
	write(
		&file,
		r#"
fn view(name: String) {
    page! { div { name } };
}
"#,
	);

	let first = run_migrate(tmp.path(), &[]);
	assert_success(&first);
	let after_first = fs::read_to_string(&file).expect("read after first run");

	let second = run_migrate(tmp.path(), &[]);
	assert_success(&second);
	let after_second = fs::read_to_string(&file).expect("read after second run");

	assert_eq!(after_second, after_first, "second run changed the file");
	assert!(
		stdout(&second).contains("Done. 0 file(s) changed."),
		"unexpected stdout:\n{}",
		stdout(&second)
	);
}

#[rstest]
fn complex_page_syntax_rewrites_nested_control_flow_without_rewrapping_expression_slots() {
	let tmp = TempDir::new().expect("tempdir");
	let file = tmp.path().join("complex_page.rs");
	write(
		&file,
		r#"
fn view(
    ready: bool,
    title: String,
    fallback_name: String,
    already_wrapped: String,
    items: Vec<Item>,
    count: usize,
) {
    page! {
        section {
            { already_wrapped }
            if (ready) {
                h1 { title }
            } else {
                p { fallback_name }
            }
            for item in items {
                Row {
                    item_name
                    span { count }
                }
            }
            loop {
                span { count }
                break;
            }
        }
    };
}
"#,
	);

	let first = run_migrate(tmp.path(), &[]);
	assert_success(&first);
	let after_first = fs::read_to_string(&file).expect("read after first run");
	let compact = compact_ws(&after_first);

	assert!(
		compact.contains("{already_wrapped}"),
		"existing expression slot should remain a single expression slot:\n{after_first}"
	);
	assert!(
		compact.contains("{title}"),
		"bare identifier inside parenthesized if body was not wrapped:\n{after_first}"
	);
	assert!(
		compact.contains("{fallback_name}"),
		"bare identifier inside else body was not wrapped:\n{after_first}"
	);
	assert!(
		compact.contains("{item_name}"),
		"bare identifier inside component body was not wrapped:\n{after_first}"
	);
	assert!(
		!after_first.contains("for { item } in"),
		"for-loop pattern variable should not be treated as a page child:\n{after_first}"
	);
	assert!(
		compact.contains("{count}"),
		"bare identifier inside loop or nested element body was not wrapped:\n{after_first}"
	);
	assert!(
		after_first.contains("break"),
		"reserved control-flow keyword should not be wrapped:\n{after_first}"
	);

	let second = run_migrate(tmp.path(), &[]);
	assert_success(&second);
	let after_second = fs::read_to_string(&file).expect("read after second run");

	assert_eq!(
		after_second, after_first,
		"complex migrated page changed on rerun"
	);
	assert!(
		stdout(&second).contains("Done. 0 file(s) changed."),
		"unexpected stdout:\n{}",
		stdout(&second)
	);
}

#[rstest]
fn match_patterns_and_let_statements_are_not_wrapped_as_page_children() {
	let tmp = TempDir::new().expect("tempdir");
	let file = tmp.path().join("match_and_let.rs");
	write(
		&file,
		r#"
fn view(value: Option<String>, fallback: String, title: String) {
    page! {
        section {
            match value {
                item => div { fallback }
            }
            let local = fallback;
            let child = div { title };
            child
        }
    };
}
"#,
	);

	let first = run_migrate(tmp.path(), &[]);
	assert_success(&first);
	let after_first = fs::read_to_string(&file).expect("read after first run");
	let compact = compact_ws(&after_first);

	assert!(
		!compact.contains("{{item}}=>") && !compact.contains("{item}=>"),
		"match arm pattern should not be wrapped:\n{after_first}"
	);
	assert!(
		!compact.contains("let{local}="),
		"let binding pattern should not be wrapped:\n{after_first}"
	);
	assert!(
		!compact.contains("let{child}="),
		"let binding with element initializer should not be wrapped:\n{after_first}"
	);
	assert!(
		compact.contains("div{{fallback}}") || compact.contains("div{{{fallback}}}"),
		"match arm page body should still be migrated:\n{after_first}"
	);
	assert!(
		compact.contains("div{{title}}"),
		"let initializer element body should still be migrated:\n{after_first}"
	);
	assert!(
		compact.contains("{child}"),
		"page child following let statement should still be migrated:\n{after_first}"
	);

	let second = run_migrate(tmp.path(), &[]);
	assert_success(&second);
	let after_second = fs::read_to_string(&file).expect("read after second run");
	assert_eq!(
		after_second, after_first,
		"match and let migration changed on rerun"
	);
}

proptest! {
	#![proptest_config(ProptestConfig::with_cases(24))]

	#[test]
	fn fuzz_migrate_page_control_syntax_preserves_rust_patterns_and_is_idempotent(
		source in generated_control_flow_page(),
	) {
		let tmp = TempDir::new().expect("tempdir");
		let file = tmp.path().join("fuzz_page.rs");
		write(&file, &source);

		let first = run_migrate(tmp.path(), &[]);
		prop_assert!(
			first.status.success(),
			"command failed with exit {:?}\nstdout:\n{}\nstderr:\n{}\nsource:\n{}",
			first.status.code(),
			stdout(&first),
			stderr(&first),
			source
		);
		let after_first = fs::read_to_string(&file).expect("read after first run");
		let compact = compact_ws(&after_first);

		prop_assert!(
			!compact.contains("let{local_value}="),
			"let binding was wrapped as a page child:\n{after_first}"
		);
		for invalid in [
			"{item}=>",
			"{entry}=>",
			"{entry}ifready=>",
			"{ready}=>",
			"{error_name}=>",
			"for{item}in",
		] {
			prop_assert!(
				!compact.contains(invalid),
				"Rust pattern/control syntax was wrapped as a page child ({invalid}):\n{after_first}"
			);
		}
		prop_assert!(
			compact.contains("{title}") || compact.contains("{fallback}"),
			"generated page children were not migrated:\n{after_first}"
		);

		let second = run_migrate(tmp.path(), &[]);
		prop_assert!(
			second.status.success(),
			"second command failed with exit {:?}\nstdout:\n{}\nstderr:\n{}\nsource:\n{}",
			second.status.code(),
			stdout(&second),
			stderr(&second),
			source
		);
		let after_second = fs::read_to_string(&file).expect("read after second run");
		prop_assert_eq!(
			after_second,
			after_first,
			"fuzz-generated page changed on rerun"
		);
	}
}

#[rstest]
fn complex_props_migration_preserves_non_default_derives_and_existing_builder_defaults() {
	let tmp = TempDir::new().expect("tempdir");
	let file = tmp.path().join("props.rs");
	write(
		&file,
		r#"
#[repr(C)]
#[derive(Clone)]
#[derive(Debug, Default, PartialEq)]
pub struct PanelProps {
    pub id: String,
    #[builder(default)]
    pub existing: Option<String>,
    pub count: usize,
    pub subtitle: Option<String>,
}
"#,
	);

	let first = run_migrate(tmp.path(), &[]);
	assert_success(&first);
	let after_first = fs::read_to_string(&file).expect("read after first run");

	assert!(
		after_first.contains("#[repr(C)]"),
		"non-derive attributes should be preserved:\n{after_first}"
	);
	assert!(
		after_first.contains("Clone")
			&& after_first.contains("Debug")
			&& after_first.contains("PartialEq")
			&& after_first.contains("bon::Builder"),
		"derive list should preserve non-Default derives and add bon::Builder:\n{after_first}"
	);
	assert!(
		!after_first.contains("Default"),
		"Default derive should be removed:\n{after_first}"
	);
	let builder_default_count = after_first.matches("#[builder(default)]").count();
	assert_eq!(
		builder_default_count, 3,
		"expected exactly one builder default for existing, count, and subtitle:\n{after_first}"
	);

	let second = run_migrate(tmp.path(), &[]);
	assert_success(&second);
	let after_second = fs::read_to_string(&file).expect("read after second run");

	assert_eq!(
		after_second, after_first,
		"complex Props migration changed on rerun"
	);
	assert!(
		stdout(&second).contains("Done. 0 file(s) changed."),
		"unexpected stdout:\n{}",
		stdout(&second)
	);
}

#[rstest]
fn unknown_skip_rule_fails() {
	let tmp = TempDir::new().expect("tempdir");
	write(&tmp.path().join("page.rs"), "fn view() {}\n");

	let output = run_migrate(tmp.path(), &["--skip", "not_a_rule"]);
	assert_failure(&output);
	assert!(
		stderr(&output).contains("unknown --skip rule(s): not_a_rule"),
		"unexpected stderr:\n{}",
		stderr(&output)
	);
}

#[rstest]
fn missing_path_fails() {
	let tmp = TempDir::new().expect("tempdir");
	let missing = tmp.path().join("does-not-exist");

	let output = run_migrate(&missing, &[]);
	assert_failure(&output);
	assert!(
		stderr(&output).contains("No such file")
			|| stderr(&output).contains("does-not-exist")
			|| stderr(&output).contains("not found"),
		"unexpected stderr:\n{}",
		stderr(&output)
	);
}

#[rstest]
fn invalid_rust_file_is_skipped_while_valid_files_are_rewritten() {
	let tmp = TempDir::new().expect("tempdir");
	let valid = tmp.path().join("valid.rs");
	let invalid = tmp.path().join("invalid.rs");
	let invalid_source = "fn broken(\n";
	write(
		&valid,
		r#"
fn view(name: String) {
    page! { div { name } };
}
"#,
	);
	write(&invalid, invalid_source);

	let output = run_migrate(tmp.path(), &[]);
	assert_success(&output);

	let valid_after = fs::read_to_string(&valid).expect("read valid file");
	let invalid_after = fs::read_to_string(&invalid).expect("read invalid file");
	assert!(
		valid_after.contains("{ name }") || valid_after.contains("{name}"),
		"valid file was not rewritten:\n{valid_after}"
	);
	assert_eq!(
		invalid_after, invalid_source,
		"invalid Rust file should be skipped unchanged"
	);
	assert!(
		stdout(&output).contains("Done. 1 file(s) changed."),
		"unexpected stdout:\n{}",
		stdout(&output)
	);
}

#[rstest]
fn skips_target_and_hidden_directories() {
	let tmp = TempDir::new().expect("tempdir");
	let source = tmp.path().join("src/page.rs");
	let target = tmp.path().join("target/generated.rs");
	let hidden = tmp.path().join(".git/ignored.rs");
	let body = r#"
fn view(name: String) {
    page! { div { name } };
}
"#;
	write(&source, body);
	write(&target, body);
	write(&hidden, body);

	let output = run_migrate(tmp.path(), &[]);
	assert_success(&output);

	let source_after = fs::read_to_string(&source).expect("read source file");
	let target_after = fs::read_to_string(&target).expect("read target file");
	let hidden_after = fs::read_to_string(&hidden).expect("read hidden file");
	assert!(
		source_after.contains("{ name }") || source_after.contains("{name}"),
		"source file should be rewritten:\n{source_after}"
	);
	assert_eq!(target_after, body, "target/ file should be skipped");
	assert_eq!(
		hidden_after, body,
		"hidden directory file should be skipped"
	);
	assert!(
		stdout(&output).contains("Done. 1 file(s) changed."),
		"unexpected stdout:\n{}",
		stdout(&output)
	);
}

#[rstest]
fn single_rs_file_path_is_rewritten() {
	let tmp = TempDir::new().expect("tempdir");
	let file = tmp.path().join("single.rs");
	write(
		&file,
		r#"
fn view(name: String) {
    page! { div { name } };
}
"#,
	);

	let output = run_migrate(&file, &[]);
	assert_success(&output);

	let after = fs::read_to_string(&file).expect("read rewritten file");
	assert!(
		after.contains("{ name }") || after.contains("{name}"),
		"single file path was not rewritten:\n{after}"
	);
	assert!(
		stdout(&output).contains("Done. 1 file(s) changed."),
		"unexpected stdout:\n{}",
		stdout(&output)
	);
}
