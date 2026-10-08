//! Optional fixture features checked without workspace feature unification.

use rstest::{fixture, rstest};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

struct Consumer {
	root: TempDir,
}

#[fixture]
fn consumer() -> Consumer {
	Consumer {
		root: tempfile::Builder::new()
			.prefix("reinhardt-facade-fixtures-")
			.tempdir_in("/tmp")
			.expect("create isolated consumer directory"),
	}
}

impl Consumer {
	fn write(&self, features: &[&str], source: &str) {
		let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
			.ancestors()
			.nth(2)
			.expect("facade tests are inside tests/");
		fs::copy(
			repository.join("Cargo.lock"),
			self.root.path().join("Cargo.lock"),
		)
		.expect("seed consumer dependency versions from workspace lockfile");
		let repository = toml::Value::String(repository.to_string_lossy().into_owned());
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
name = "facade-fixture-consumer"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
reinhardt = {{ package = "reinhardt-web", path = {repository}, default-features = false, features = {features} }}
"#
			),
		)
		.expect("write facade-only consumer manifest");
		fs::write(self.root.path().join("src/lib.rs"), source).expect("write consumer source");
	}

	fn cargo(&self, arguments: &[&str]) -> Output {
		let output = self.cargo_once(arguments, true);
		let stderr = String::from_utf8_lossy(&output.stderr);
		// Retry only registry cache misses; compilation failures stay visible.
		let missing_cache = stderr
			.contains("attempting to make an HTTP request, but --offline was specified")
			|| (stderr.contains("no matching package named")
				&& stderr.contains("location searched: crates.io index"));
		if !output.status.success() && missing_cache {
			self.cargo_once(arguments, false)
		} else {
			output
		}
	}

	fn cargo_once(&self, arguments: &[&str], offline: bool) -> Output {
		let mut command = Command::new(env!("CARGO"));
		command
			.args(arguments)
			.arg("--manifest-path")
			.arg(self.root.path().join("Cargo.toml"))
			.current_dir(self.root.path())
			// Standalone consumers do not inherit the workspace's debug-free profile.
			.env("CARGO_PROFILE_DEV_DEBUG", "0")
			.env("CARGO_TARGET_DIR", self.root.path().join("target"))
			.env("CARGO_BUILD_BUILD_DIR", self.root.path().join("build"));
		if offline {
			command.arg("--offline");
		}
		command
			.output()
			.expect("run isolated consumer Cargo command")
	}
}

#[rstest]
fn protocol_features_expose_test_fixtures(consumer: Consumer) {
	// Arrange
	let cases: &[(&[&str], &str)] = &[
		(
			&["websockets", "test"],
			"pub use reinhardt::test::fixtures::{websocket_server, websocket_client};",
		),
		(
			&["graphql", "test"],
			"pub use reinhardt::test::fixtures::graphql_server;",
		),
		(
			&["websockets", "graphql", "test"],
			"pub use reinhardt::test::fixtures::{websocket_server, websocket_client, graphql_server};",
		),
	];

	// Act
	// Replace each manifest while reusing artifacts, so neither protocol can
	// inherit the other protocol's fixture feature from a workspace build.
	for (features, source) in cases {
		consumer.write(features, source);
		let output = consumer.cargo(&["check"]);

		// Assert
		assert_eq!(
			output.status.code(),
			Some(0),
			"fixtures must compile with {features:?}:\n{}",
			String::from_utf8_lossy(&output.stderr),
		);
	}
}

#[rstest]
#[case::websockets(&["websockets"], None, &["reinhardt-test", "reinhardt-testkit"])]
#[case::graphql(&["graphql"], None, &["reinhardt-test", "reinhardt-testkit"])]
#[case::both(&["websockets", "graphql"], None, &["reinhardt-test", "reinhardt-testkit"])]
#[case::wasm(
	&["websockets", "graphql", "test"],
	Some("wasm32-unknown-unknown"),
	&["reinhardt-testkit", "reinhardt-websockets", "reinhardt-graphql", "reinhardt-server"],
)]
fn fixture_forwarding_preserves_dependency_boundaries(
	consumer: Consumer,
	#[case] features: &[&str],
	#[case] target: Option<&str>,
	#[case] excluded: &[&str],
) {
	// Arrange
	consumer.write(features, "");
	let mut arguments = vec!["tree", "--edges", "normal", "--prefix", "none"];
	if let Some(target) = target {
		arguments.extend(["--target", target]);
	}

	// Act
	let output = consumer.cargo(&arguments);

	// Assert
	assert_eq!(
		output.status.code(),
		Some(0),
		"resolve dependency graph:\n{}",
		String::from_utf8_lossy(&output.stderr),
	);
	let dependencies = String::from_utf8(output.stdout).expect("Cargo tree output is UTF-8");
	let unexpected: Vec<_> = dependencies
		.lines()
		.filter_map(|line| line.split_whitespace().next())
		.filter(|package| excluded.contains(package))
		.collect();
	assert_eq!(unexpected, Vec::<&str>::new(), "{dependencies}");
}
