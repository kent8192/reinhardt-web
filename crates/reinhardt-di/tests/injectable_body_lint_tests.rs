//! Regression coverage for provider body diagnostics and cargo fix (Issue #6441).

#![cfg(all(
	feature = "macros",
	not(all(target_family = "wasm", target_os = "unknown"))
))]

use reinhardt_di::{InjectionContext, SingletonScope};
use rstest::*;
use std::fs;
use std::process::{Command, Output};
use std::sync::Arc;

#[path = "fixtures/injectable_body/src/lib.rs"]
pub mod providers;

enum Body {
	Plain,
	Injected,
	Awaited,
	Statements,
	Legacy,
}

#[rstest]
#[case::plain(Body::Plain, 3)]
#[case::injected(Body::Injected, 7)]
#[case::awaited(Body::Awaited, 11)]
#[case::statements_and_early_return(Body::Statements, 14)]
#[case::compatibility_alias(Body::Legacy, 17)]
#[tokio::test]
async fn provider_bodies_preserve_values(#[case] body: Body, #[case] expected: u32) {
	// Arrange
	let ctx = Arc::new(InjectionContext::builder(SingletonScope::new()).build());

	// Act
	let service = match body {
		Body::Plain => providers::plain_provider(ctx).await.unwrap().into_inner(),
		Body::Injected => providers::injected_provider(ctx)
			.await
			.unwrap()
			.into_inner(),
		Body::Awaited => providers::awaited_provider(ctx).await.unwrap().into_inner(),
		Body::Statements => providers::statements_provider(ctx)
			.await
			.unwrap()
			.into_inner(),
		Body::Legacy => providers::legacy::legacy_provider(ctx)
			.await
			.unwrap()
			.into_inner(),
	};

	// Assert
	assert_eq!(service, providers::Service { value: expected });
}

#[rstest]
fn cargo_fix_preserves_provider_function_bodies() {
	// Arrange
	let fixture = tempfile::Builder::new()
		.prefix("reinhardt-injectable-body-")
		.tempdir_in("/tmp")
		.expect("create disposable consumer fixture");
	let source_path = fixture.path().join("src/lib.rs");
	fs::create_dir(fixture.path().join("src")).unwrap();
	fs::write(
		fixture.path().join("Cargo.toml"),
		format!(
			r#"[package]
name = "reinhardt-injectable-body-fixture"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
reinhardt-di = {{ path = "{}", features = ["macros"] }}
"#,
			env!("CARGO_MANIFEST_DIR")
		),
	)
	.unwrap();
	let mut source = include_str!("fixtures/injectable_body/src/lib.rs")
		.replace("#![deny(unused_braces)]", "#![warn(unused_braces)]");
	// This independent warning proves cargo fix applied an edit without rolling back.
	source.push_str("\npub fn fix_control() -> u32 { let mut value = 23; value }\n");
	fs::write(&source_path, &source).unwrap();
	let run_cargo = |args: &[&str]| {
		Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
			.args(args)
			.args([
				"--offline",
				"--lib",
				"--message-format=json",
				"--target-dir",
			])
			.arg(fixture.path().join("target"))
			.env("CARGO_BUILD_BUILD_DIR", fixture.path().join("build"))
			.current_dir(fixture.path())
			.output()
			.expect("run cargo against disposable consumer")
	};

	// Act
	let before = run_cargo(&["check"]);
	let fixed = run_cargo(&["fix", "--allow-no-vcs", "--allow-dirty", "--allow-staged"]);
	let after = run_cargo(&["check"]);

	// Assert
	for output in [&before, &fixed, &after] {
		assert!(
			output.status.success(),
			"cargo must succeed\nstdout:\n{}\nstderr:\n{}",
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr),
		);
		assert_eq!(lint_count(output, "unused_braces"), 0);
	}
	assert_eq!(lint_count(&before, "unused_mut"), 1);
	assert_eq!(lint_count(&after, "unused_mut"), 0);
	assert_eq!(
		fs::read_to_string(source_path).unwrap(),
		source.replace("let mut value = 23", "let value = 23"),
		"cargo fix must change only the control binding and preserve every provider body",
	);
}

fn lint_count(output: &Output, lint: &str) -> usize {
	String::from_utf8_lossy(&output.stdout)
		.lines()
		.filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
		.filter(|message| {
			message["reason"] == "compiler-message" && message["message"]["code"]["code"] == lint
		})
		.count()
}
