//! Facade feature contracts checked in consumers outside the framework workspace.
//!
//! Each consumer owns its temporary source and Cargo build directories.

use rstest::{fixture, rstest};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

struct Consumer {
	root: TempDir,
	repository: PathBuf,
}

#[fixture]
fn consumer() -> Consumer {
	Consumer {
		root: tempfile::Builder::new()
			.prefix("reinhardt-facade-uuid-")
			.tempdir_in("/tmp")
			.expect("create isolated consumer directory"),
		repository: Path::new(env!("CARGO_MANIFEST_DIR"))
			.ancestors()
			.nth(2)
			.expect("facade tests are inside tests/")
			.to_path_buf(),
	}
}

impl Consumer {
	fn write(&self, features: &[&str], source: &str) {
		let repository = toml::Value::String(self.repository.to_string_lossy().into_owned());
		let features = toml::Value::Array(
			features
				.iter()
				.map(|feature| toml::Value::String((*feature).into()))
				.collect(),
		);
		fs::create_dir_all(self.root.path().join("src")).expect("create consumer source directory");
		fs::write(
			self.root.path().join("Cargo.toml"),
			format!(
				r#"[package]
name = "facade-uuid-path-consumer"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
reinhardt = {{ package = "reinhardt-web", path = {repository}, default-features = false, features = {features} }}
uuid = "1"
"#
			),
		)
		.expect("write facade-only consumer manifest");
		fs::write(self.root.path().join("src/lib.rs"), source).expect("write consumer source");
		// Match the versions fetched by the parent build. Resolving afresh offline
		// can select newer registry versions that are absent from the Cargo cache.
		fs::copy(
			self.repository.join("Cargo.lock"),
			self.root.path().join("Cargo.lock"),
		)
		.expect("seed consumer dependencies from the workspace lockfile");
		// Dependency patches only apply at the workspace root. Fetch the
		// consumer's unpatched graph before the offline compilation assertions.
		let output = self.cargo(&["fetch"]);
		assert!(
			output.status.success(),
			"fetch isolated consumer dependencies:\n{}",
			String::from_utf8_lossy(&output.stderr),
		);
	}

	fn cargo(&self, arguments: &[&str]) -> Output {
		let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
		command
			.args(arguments)
			.arg("--manifest-path")
			.arg(self.root.path().join("Cargo.toml"))
			.current_dir(self.root.path())
			.env("CARGO_TARGET_DIR", self.root.path().join("target"))
			.env("CARGO_BUILD_BUILD_DIR", self.root.path().join("build"));
		if arguments.first() != Some(&"fetch") {
			command.arg("--offline");
		}
		command
			.output()
			.expect("run isolated consumer Cargo command")
	}
}

#[rstest]
#[case::di("di")]
#[case::minimal("minimal")]
#[case::api_only("api-only")]
fn uuid_paths_are_injectable_with_facade_only_dependencies(
	consumer: Consumer,
	#[case] preset: &str,
) {
	// Arrange
	consumer.write(
		&[preset, "uuid"],
		r#"fn assert_injectable<T: reinhardt::Injectable>() {}

pub fn uuid_extractors() {
    assert_injectable::<reinhardt::Path<uuid::Uuid>>();
    assert_injectable::<reinhardt::Path<(uuid::Uuid, uuid::Uuid)>>();
}
"#,
	);

	// Act
	let output = consumer.cargo(&["check"]);

	// Assert
	assert_eq!(
		output.status.code(),
		Some(0),
		"facade {preset} + uuid must enable single and tuple UUID path injection:\n{}",
		String::from_utf8_lossy(&output.stderr),
	);
}

#[rstest]
#[case::native(&["uuid"], None)]
#[case::wasm(&["di", "uuid"], Some("wasm32-unknown-unknown"))]
fn uuid_preserves_optional_and_native_only_di(
	consumer: Consumer,
	#[case] features: &[&str],
	#[case] target: Option<&str>,
) {
	// Arrange
	consumer.write(features, "");
	// Pages already depends on DI transitively on native targets. Inspect only
	// the facade's direct edges to verify its optional DI dependency stays off.
	let mut arguments = vec![
		"tree",
		"--package",
		"reinhardt-web",
		"--depth",
		"1",
		"--edges",
		"normal",
		"--prefix",
		"none",
	];
	if let Some(target) = target {
		arguments.extend(["--target", target]);
	}

	// Act
	let output = consumer.cargo(&arguments);

	// Assert
	assert_eq!(
		output.status.code(),
		Some(0),
		"resolve facade dependency graph:\n{}",
		String::from_utf8_lossy(&output.stderr),
	);
	let dependencies = String::from_utf8(output.stdout).expect("Cargo tree output is UTF-8");
	let di_packages: Vec<_> = dependencies
		.lines()
		.filter(|line| line.split_whitespace().next() == Some("reinhardt-di"))
		.collect();
	assert_eq!(di_packages, Vec::<&str>::new(), "{dependencies}");
}
