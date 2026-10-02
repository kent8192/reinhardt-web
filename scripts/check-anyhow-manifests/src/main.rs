use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use toml::Value;

const DEPENDENCY_TABLE_NAMES: [&str; 3] =
	["dependencies", "dev-dependencies", "build-dependencies"];

fn inspect_dependency_table(value: Option<&Value>, context: &str, findings: &mut Vec<String>) {
	let Some(dependencies) = value.and_then(Value::as_table) else {
		return;
	};

	for (dependency_name, specification) in dependencies {
		let detail = if dependency_name == "anyhow" {
			Some("anyhow".to_owned())
		} else if specification.get("package").and_then(Value::as_str) == Some("anyhow") {
			Some(format!(r#"{dependency_name} (package = "anyhow")"#))
		} else {
			None
		};

		if let Some(detail) = detail {
			findings.push(format!(
				"remove direct anyhow dependency from {context}: {detail}"
			));
		}
	}
}

fn inspect_manifest(document: &Value) -> Vec<String> {
	let mut findings = Vec::new();

	for table_name in DEPENDENCY_TABLE_NAMES {
		inspect_dependency_table(document.get(table_name), table_name, &mut findings);
	}

	inspect_dependency_table(
		document
			.get("workspace")
			.and_then(|workspace| workspace.get("dependencies")),
		"workspace.dependencies",
		&mut findings,
	);

	if let Some(targets) = document.get("target").and_then(Value::as_table) {
		for (target_name, target) in targets {
			for table_name in DEPENDENCY_TABLE_NAMES {
				inspect_dependency_table(
					target.get(table_name),
					&format!("target.{target_name}.{table_name}"),
					&mut findings,
				);
			}
		}
	}

	if let Some(features) = document.get("features").and_then(Value::as_table) {
		for (feature_name, values) in features {
			let Some(values) = values.as_array() else {
				continue;
			};
			for value in values {
				if value.as_str() == Some("dep:anyhow") {
					findings.push(format!(
						"remove dep:anyhow from feature features.{feature_name}"
					));
				}
			}
		}
	}

	findings
}

fn relative_manifest_path(root: &Path, manifest: &Path) -> String {
	manifest
		.strip_prefix(root)
		.unwrap_or(manifest)
		.to_string_lossy()
		.replace('\\', "/")
}

fn inspect_path(root: &Path, manifest: &Path) -> Result<(), String> {
	let relative_path = relative_manifest_path(root, manifest);
	let source = fs::read_to_string(manifest)
		.map_err(|error| format!("failed to read {relative_path}: {error}"))?;
	let document = toml::from_str::<Value>(&source)
		.map_err(|error| format!("failed to parse {relative_path}: {error}"))?;

	for finding in inspect_manifest(&document) {
		println!("{relative_path}:1:{finding}");
	}
	Ok(())
}

fn run(arguments: impl IntoIterator<Item = OsString>) -> Result<(), String> {
	let mut arguments = arguments.into_iter();
	let _program = arguments.next();
	let root = arguments
		.next()
		.map(PathBuf::from)
		.ok_or_else(|| "missing scan root argument".to_owned())?;

	for manifest in arguments {
		inspect_path(&root, &PathBuf::from(manifest))?;
	}
	Ok(())
}

fn main() -> ExitCode {
	match run(env::args_os()) {
		Ok(()) => ExitCode::SUCCESS,
		Err(error) => {
			eprintln!("anyhow-check: {error}");
			ExitCode::from(2)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::inspect_manifest;
	use toml::Value;

	fn findings(source: &str) -> Vec<String> {
		let document = toml::from_str::<Value>(source).expect("fixture must parse");
		inspect_manifest(&document)
	}

	#[test]
	fn detects_dependency_contexts_and_feature_tokens() {
		let actual = findings(
			r#"
[dependencies]
anyhow = "1"

[dev-dependencies.errors]
package = "anyhow"
version = "1"

[workspace.dependencies]
workspace_errors = { package = "anyhow", version = "1" }

[target.'cfg(unix)'.build-dependencies]
anyhow = "1"

[features]
dynamic = ["dep:anyhow"]
"#,
		);

		assert_eq!(
			actual,
			vec![
				"remove direct anyhow dependency from dependencies: anyhow",
				r#"remove direct anyhow dependency from dev-dependencies: errors (package = "anyhow")"#,
				r#"remove direct anyhow dependency from workspace.dependencies: workspace_errors (package = "anyhow")"#,
				"remove direct anyhow dependency from target.cfg(unix).build-dependencies: anyhow",
				"remove dep:anyhow from feature features.dynamic",
			]
		);
	}

	#[test]
	fn parses_spanning_workspace_values_semantically() {
		let actual = findings(
			r#"
[workspace]
members = []

[workspace.dependencies]
errors = {
  version = "1",
  note = """
}
""",
  package = """
anyhow""",
}
"#,
		);

		assert_eq!(
			actual,
			vec![
				r#"remove direct anyhow dependency from workspace.dependencies: errors (package = "anyhow")"#,
			]
		);
	}

	#[test]
	fn ignores_quoted_dotted_keys_and_package_text_inside_strings() {
		let actual = findings(
			r#"
[workspace]
members = []

["workspace.dependencies.anyhow"]
version = "1"

[workspace.dependencies]
serde = { version = "1", note = ',package=anyhow,' }
"#,
		);

		assert_eq!(actual, Vec::<String>::new());
	}
}
