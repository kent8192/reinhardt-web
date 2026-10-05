//! Middleware component features checked without workspace feature unification.

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
			.prefix("reinhardt-facade-middleware-")
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
name = "facade-middleware-consumer"
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
		Command::new(env!("CARGO"))
			.args(arguments)
			.args(["--offline", "--manifest-path"])
			.arg(self.root.path().join("Cargo.toml"))
			.current_dir(self.root.path())
			.env("CARGO_TARGET_DIR", self.root.path().join("target"))
			.env("CARGO_BUILD_BUILD_DIR", self.root.path().join("build"))
			.output()
			.expect("run isolated consumer Cargo command")
	}
}

const CORS_SOURCE: &str = r#"pub use reinhardt::middleware::cors::create_cors_middleware_from_settings;

pub fn cors() -> reinhardt::CorsMiddleware {
    reinhardt::middleware::CorsMiddleware::permissive()
}
"#;

#[rstest]
fn middleware_components_expose_facade_apis(consumer: Consumer) {
	// Arrange
	let cases: &[(&[&str], &str)] = &[
		(&["middleware-cors"], CORS_SOURCE),
		(&["api-only", "middleware-cors"], CORS_SOURCE),
		(&["middleware", "middleware-cors"], CORS_SOURCE),
		(&["standard", "middleware-cors"], CORS_SOURCE),
		(
			&["middleware-compression"],
			"pub use reinhardt::middleware::{BrotliMiddleware, GZipMiddleware};",
		),
		(
			&["middleware-security"],
			"pub use reinhardt::middleware::SecurityMiddleware;\npub use reinhardt::SecurityMiddleware as RootSecurityMiddleware;",
		),
		(
			&["middleware-rate-limit"],
			"pub use reinhardt::middleware::RateLimitMiddleware;",
		),
		(
			&["middleware-auth-jwt"],
			"pub use reinhardt::middleware::JwtAuthMiddleware;\npub use reinhardt::JwtAuthMiddleware as RootJwtAuthMiddleware;",
		),
	];

	// Act
	// Reuse artifacts sequentially, but replace each consumer's manifest so no
	// umbrella feature can leak into a component-only dependency graph.
	for (features, source) in cases {
		consumer.write(features, source);
		let output = consumer.cargo(&["check"]);

		// Assert
		assert_eq!(
			output.status.code(),
			Some(0),
			"middleware APIs must compile with {features:?}:\n{}",
			String::from_utf8_lossy(&output.stderr),
		);
	}
}

#[rstest]
fn middleware_components_stay_out_of_the_wasm_dependency_graph(consumer: Consumer) {
	// Arrange
	consumer.write(
		&[
			"middleware-cors",
			"middleware-compression",
			"middleware-security",
			"middleware-rate-limit",
			"middleware-auth-jwt",
		],
		"",
	);

	// Act
	let output = consumer.cargo(&[
		"tree",
		"--target",
		"wasm32-unknown-unknown",
		"--edges",
		"normal",
		"--prefix",
		"none",
	]);

	// Assert
	assert_eq!(
		output.status.code(),
		Some(0),
		"resolve WASM dependency graph:\n{}",
		String::from_utf8_lossy(&output.stderr),
	);
	let dependencies = String::from_utf8(output.stdout).expect("Cargo tree output is UTF-8");
	let middleware_packages: Vec<_> = dependencies
		.lines()
		.filter(|line| line.split_whitespace().next() == Some("reinhardt-middleware"))
		.collect();
	assert_eq!(middleware_packages, Vec::<&str>::new(), "{dependencies}");
}
