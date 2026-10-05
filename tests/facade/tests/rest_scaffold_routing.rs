//! Compile the REST scaffold's routing instructions against the current facade.

use rstest::{fixture, rstest};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

struct Scaffold {
	root: TempDir,
	urls: String,
}

fn render(repository: &Path, relative: &str, key: &str, value: &str) -> String {
	let template = fs::read_to_string(
		repository
			.join("crates/reinhardt-commands/templates")
			.join(relative),
	)
	.expect("read scaffold template");
	let mut context = tera::Context::new();
	context.insert(key, value);
	tera::Tera::one_off(&template, &context, false).expect("render scaffold template")
}

#[fixture]
fn scaffold() -> Scaffold {
	let root = tempfile::Builder::new()
		.prefix("reinhardt-rest-scaffold-")
		.tempdir_in("/tmp")
		.expect("create isolated scaffold directory");
	let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
		.ancestors()
		.nth(2)
		.expect("facade tests are inside tests/");
	let dependency_path = toml::Value::String(repository.to_string_lossy().into_owned());
	fs::write(
		root.path().join("Cargo.toml"),
		format!(
			r#"[package]
name = "rest-scaffold-routing-consumer"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
reinhardt = {{ package = "reinhardt-web", path = {dependency_path}, default-features = false, features = ["minimal", "api", "client-router"] }}
async-trait = "0.1"
"#
		),
	)
	.expect("write scaffold consumer manifest");
	for directory in ["src/config", "src/apps/users", "src/apps/api_v2"] {
		fs::create_dir_all(root.path().join(directory)).expect("create scaffold source directory");
	}
	let files = [
		("src/lib.rs", "pub mod config;\npub mod apps;\n"),
		("src/config.rs", "pub mod urls;\n"),
		("src/apps.rs", "pub mod users;\npub mod api_v2;\n"),
		("src/apps/users.rs", "pub mod urls;\npub mod views;\n"),
		("src/apps/api_v2.rs", "pub mod urls;\n"),
		(
			"src/apps/api_v2/urls.rs",
			"use reinhardt::UnifiedRouter;\npub fn routes() -> UnifiedRouter {\n\
			 UnifiedRouter::new().mount(\"/\", crate::apps::users::urls::server_url_patterns())\n}\n",
		),
		(
			"src/apps/users/views.rs",
			include_str!("fixtures/rest_scaffold_views.rs"),
		),
	];
	for (relative, source) in files {
		fs::write(root.path().join(relative), source).expect("write scaffold support module");
	}
	fs::write(
		root.path().join("src/apps/users/urls.rs"),
		render(
			repository,
			"app_restful_template/urls.rs.tpl",
			"app_name",
			"users",
		),
	)
	.expect("write generated REST app aggregate");
	Scaffold {
		root,
		urls: render(
			repository,
			"project_restful_template/src/config/urls.rs.tpl",
			"project_name",
			"routing_example",
		),
	}
}

fn enable_examples(source: &str) -> String {
	let mut in_example = false;
	let mut blocks = 0;
	let mut enabled = String::new();
	for line in source.lines() {
		match line.trim() {
			"// ```rust" => {
				assert!(!in_example, "routing example blocks must not nest");
				in_example = true;
				blocks += 1;
			}
			"// ```" => {
				assert!(in_example, "routing example must have an opening fence");
				in_example = false;
			}
			_ if in_example => {
				enabled.push_str(
					line.trim_start()
						.strip_prefix("// ")
						.expect("commented Rust example"),
				);
				enabled.push('\n');
			}
			_ => {
				enabled.push_str(line);
				enabled.push('\n');
			}
		}
	}
	assert!(!in_example, "routing example must have a closing fence");
	assert_eq!(blocks, 4, "compile every documented routing example");
	enabled
}

#[rstest]
fn rest_scaffold_routing_examples_compile(scaffold: Scaffold) {
	// Arrange
	let urls_path = scaffold.root.path().join("src/config/urls.rs");
	let target = scaffold.root.path().join("target");
	let build = scaffold.root.path().join("build");

	// Act
	// Compile both the unchanged blank scaffold and all enabled examples. Reuse
	// this consumer's artifacts, while keeping Cargo outside the workspace lock.
	for (scenario, urls) in [
		("blank scaffold", scaffold.urls.clone()),
		("enabled routing examples", enable_examples(&scaffold.urls)),
	] {
		fs::write(&urls_path, urls).expect("write generated project URL configuration");
		let output = Command::new(env!("CARGO"))
			.args(["check", "--quiet", "--offline", "--manifest-path"])
			.arg(scaffold.root.path().join("Cargo.toml"))
			.current_dir(scaffold.root.path())
			.env("CARGO_TARGET_DIR", &target)
			.env("CARGO_BUILD_BUILD_DIR", &build)
			.output()
			.expect("compile generated routing instructions");

		// Assert
		assert_eq!(
			output.status.code(),
			Some(0),
			"{scenario} must compile:\n{}",
			String::from_utf8_lossy(&output.stderr),
		);
	}
}
