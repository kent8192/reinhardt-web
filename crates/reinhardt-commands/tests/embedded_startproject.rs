//! Integration test proving `startproject` produces a usable project tree
//! from embedded templates alone — no CARGO_MANIFEST_DIR dependency.

use reinhardt_commands::start_commands::StartProjectCommand;
use reinhardt_commands::{BaseCommand, CommandContext};
use rstest::*;
use serial_test::serial;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

// Assert that Cargo can fully parse the generated manifest.
//
// Uses `cargo metadata --no-deps` so no registry access is required; the
// command still exercises the same manifest-parsing step that rejects
// misconfigurations (e.g. a `default-run` pointing at a nonexistent bin)
// which would break the scaffold for a real user on `cargo run`.
fn assert_manifest_parses(manifest: &Path) {
	let output = Command::new(env!("CARGO"))
		.args(["metadata", "--no-deps", "--format-version", "1"])
		.arg("--manifest-path")
		.arg(manifest)
		.output()
		.expect("cargo metadata command spawns");
	assert!(
		output.status.success(),
		"generated manifest failed to parse: {}\nstderr:\n{}",
		manifest.display(),
		String::from_utf8_lossy(&output.stderr),
	);
}

#[rstest]
#[tokio::test]
#[serial(cwd)]
async fn startproject_restful_from_embedded_only() {
	// Arrange
	let tmp = TempDir::new().unwrap();
	let prev = std::env::current_dir().unwrap();
	std::env::set_current_dir(tmp.path()).unwrap();

	let mut ctx = CommandContext::new(vec!["sample_proj".to_string()]);
	let mut opts = HashMap::new();
	opts.insert("restful".to_string(), vec!["true".to_string()]);
	ctx = ctx.with_options(opts);

	// Act
	let res = StartProjectCommand.execute(&ctx).await;

	// Assert
	std::env::set_current_dir(prev).unwrap();
	res.expect("startproject succeeds from embedded templates");
	let generated = tmp.path().join("sample_proj");
	assert!(
		generated.join("Cargo.toml").exists(),
		"Cargo.toml must be generated"
	);
	assert!(
		generated.join("src").is_dir(),
		"src/ directory must be generated"
	);
	assert_manifest_parses(&generated.join("Cargo.toml"));
}

#[rstest]
#[tokio::test]
#[serial(cwd)]
async fn startproject_restful_honors_dependency_selection_flags() {
	let tmp = TempDir::new().unwrap();
	let prev = std::env::current_dir().unwrap();
	std::env::set_current_dir(tmp.path()).unwrap();

	let mut ctx = CommandContext::new(vec!["feature_proj".to_string()]);
	let mut opts = HashMap::new();
	opts.insert("restful".to_string(), vec!["true".to_string()]);
	opts.insert(
		"reinhardt-version".to_string(),
		vec!["0.2.0-rc.4".to_string()],
	);
	opts.insert(
		"features".to_string(),
		vec!["minimal,db-sqlite".to_string()],
	);
	opts.insert("no-interactive".to_string(), vec!["true".to_string()]);
	ctx = ctx.with_options(opts);

	let res = StartProjectCommand.execute(&ctx).await;

	std::env::set_current_dir(prev).unwrap();
	res.expect("startproject succeeds with dependency selection flags");
	let cargo_toml = std::fs::read_to_string(tmp.path().join("feature_proj/Cargo.toml")).unwrap();
	assert!(cargo_toml.contains("version = \"0.2.0-rc.4\""));
	assert!(cargo_toml.contains("default-features = false"));
	assert!(cargo_toml.contains(
		"features = [\"minimal\", \"db-sqlite\", \"conf\", \"commands\", \"db-postgres\", \"api\"]"
	));
}

#[rstest]
#[tokio::test]
#[serial(cwd)]
async fn startproject_pages_from_embedded_only() {
	// Arrange
	let tmp = TempDir::new().unwrap();
	let prev = std::env::current_dir().unwrap();
	std::env::set_current_dir(tmp.path()).unwrap();

	let mut ctx = CommandContext::new(vec!["sample_pages_proj".to_string()]);
	let mut opts = HashMap::new();
	opts.insert("with-pages".to_string(), vec!["true".to_string()]);
	ctx = ctx.with_options(opts);

	// Act
	let res = StartProjectCommand.execute(&ctx).await;

	// Assert
	std::env::set_current_dir(prev).unwrap();
	res.expect("startproject --with-pages succeeds from embedded templates");
	let generated = tmp.path().join("sample_pages_proj");
	assert!(
		generated.join("Cargo.toml").exists(),
		"Cargo.toml must be generated"
	);
	assert!(
		generated.join("src").is_dir(),
		"src/ directory must be generated"
	);
	let cargo_toml = std::fs::read_to_string(generated.join("Cargo.toml")).unwrap();
	assert!(cargo_toml.contains(
		"package = \"reinhardt-web\", default-features = false, features = [\"pages\", \"client-router\"]"
	));
	assert!(cargo_toml.contains(
		"features = [\"standard\", \"full\", \"pages\", \"conf\", \"commands\", \"db-sqlite\", \"forms\", \"client-router\", \"auth-session\"]"
	));
	assert!(
		cargo_toml.contains("required-features = [\"with-reinhardt\"]")
			&& cargo_toml.contains("default = [\"with-reinhardt\"]"),
		"generated pages manifest must gate the native manage binary:\n{cargo_toml}"
	);
	let settings_rs = std::fs::read_to_string(generated.join("src/config/settings.rs")).unwrap();
	assert!(
		settings_rs.contains("#[settings(core: CoreSettings | contacts: ContactSettings)]"),
		"generated pages settings must satisfy HasCommonSettings:\n{settings_rs}"
	);
	let base_settings = std::fs::read_to_string(generated.join("settings/base.toml")).unwrap();
	assert!(
		base_settings.contains("secret_key = \"insecure-")
			&& base_settings.contains("[contacts]")
			&& base_settings.contains("admins = []")
			&& base_settings.contains("managers = []"),
		"generated base.toml must include a secret key and contacts fragment:\n{base_settings}"
	);
	assert!(
		base_settings.contains("engine = \"sqlite\"")
			&& base_settings.contains("name = \"db.sqlite3\""),
		"generated pages project should default to a local SQLite database:\n{base_settings}"
	);
	assert!(
		!base_settings.contains("Copy this file to base.toml"),
		"generated base.toml must not keep example-file copy instructions:\n{base_settings}"
	);
	let base_example =
		std::fs::read_to_string(generated.join("settings/base.example.toml")).unwrap();
	assert!(
		base_example.contains("secret_key = \"CHANGE_THIS_IN_PRODUCTION_MUST_BE_KEPT_SECRET\"")
			&& !base_example.contains("secret_key = \"insecure-"),
		"generated base.example.toml must keep a non-secret placeholder:\n{base_example}"
	);
	let makefile_toml = std::fs::read_to_string(generated.join("Makefile.toml")).unwrap();
	assert!(
		makefile_toml.contains("\"--no-input\""),
		"generated pages Makefile must use collectstatic's non-interactive flag"
	);
	assert!(
		!makefile_toml.contains("\"--noinput\""),
		"generated pages Makefile must not use the createsuperuser-only --noinput spelling"
	);
	assert!(
		makefile_toml.contains("command = \"wasm-pack\"")
			&& makefile_toml.contains("\"--lib\"")
			&& makefile_toml.contains("scripts/wasm-build-dev.sh")
			&& !makefile_toml.contains("wasm-bindgen --target web")
			&& !makefile_toml.contains("WASM_FILE=$(ls target/wasm32-unknown-unknown"),
		"generated pages Makefile must build the library bundle through wasm-pack:\n{makefile_toml}"
	);
	assert!(
		generated.join("scripts/wasm-build-dev.sh").exists()
			&& generated.join("scripts/wasm-build-release.sh").exists(),
		"generated pages project must include WASM finalize scripts"
	);
	let build_rs = std::fs::read_to_string(generated.join("build.rs")).unwrap();
	for cfg in ["client", "server", "wasm", "native"] {
		assert!(
			build_rs.contains(&format!("cargo::rustc-check-cfg=cfg({cfg})")),
			"generated pages build.rs must declare cfg({cfg}) for Rust 2024 check-cfg:\n{build_rs}"
		);
	}
	assert!(
		build_rs.contains("wasm: { target_arch = \"wasm32\" }")
			&& build_rs.contains("native: { not(target_arch = \"wasm32\") }"),
		"generated pages build.rs must keep wasm/native compatibility aliases:\n{build_rs}"
	);
	let shared_types = std::fs::read_to_string(generated.join("src/shared/types.rs")).unwrap();
	assert!(
		!shared_types.contains("\nuse serde::{Deserialize, Serialize};"),
		"generated shared types placeholder must not create an unused import warning:\n{shared_types}"
	);
	assert!(
		shared_types.contains("// use serde::{Deserialize, Serialize};"),
		"generated shared types placeholder should keep the serde import in the commented example:\n{shared_types}"
	);
	assert_manifest_parses(&generated.join("Cargo.toml"));
}

#[rstest]
#[tokio::test]
#[serial(cwd)]
async fn startproject_pages_adds_required_pages_features() {
	let tmp = TempDir::new().unwrap();
	let prev = std::env::current_dir().unwrap();
	std::env::set_current_dir(tmp.path()).unwrap();

	let mut ctx = CommandContext::new(vec!["pages_feature_proj".to_string()]);
	let mut opts = HashMap::new();
	opts.insert("with-pages".to_string(), vec!["true".to_string()]);
	opts.insert("features".to_string(), vec!["minimal".to_string()]);
	opts.insert("no-interactive".to_string(), vec!["true".to_string()]);
	ctx = ctx.with_options(opts);

	let res = StartProjectCommand.execute(&ctx).await;

	std::env::set_current_dir(prev).unwrap();
	res.expect("startproject --with-pages succeeds with dependency selection flags");
	let cargo_toml =
		std::fs::read_to_string(tmp.path().join("pages_feature_proj/Cargo.toml")).unwrap();
	assert!(cargo_toml.contains(
		"features = [\"minimal\", \"full\", \"pages\", \"conf\", \"commands\", \"db-sqlite\", \"forms\", \"client-router\", \"auth-session\"]"
	));
	assert_manifest_parses(&tmp.path().join("pages_feature_proj/Cargo.toml"));
}
