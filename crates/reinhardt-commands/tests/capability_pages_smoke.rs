//! Exercise the generated Pages management binary with runtime secrets absent.
#![cfg(feature = "contract")]

use reinhardt_commands::start_commands::StartProjectCommand;
use reinhardt_commands::{BaseCommand, CommandContext};
use rstest::*;
use serial_test::serial;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;
use toml_edit::{DocumentMut, Value, value};

struct CurrentDirectory(PathBuf);

impl CurrentDirectory {
	fn enter(path: &Path) -> Self {
		let previous = std::env::current_dir().expect("read current directory");
		std::env::set_current_dir(path).expect("enter generated project parent");
		Self(previous)
	}
}

impl Drop for CurrentDirectory {
	fn drop(&mut self) {
		std::env::set_current_dir(&self.0).expect("restore current directory");
	}
}

fn invoke(binary: &Path, project: &Path, args: &[&str]) -> Output {
	Command::new(binary)
		.current_dir(project)
		.env_remove("REINHARDT_PAGES_MISSING_SECRET_6336")
		.args(args)
		.output()
		.expect("run generated Pages management binary")
}

fn use_local_workspace_dependencies(manifest: &Path, workspace: &Path) {
	let content = fs::read_to_string(manifest).unwrap();
	let mut document = content.parse::<DocumentMut>().unwrap();
	for target in [
		"cfg(target_arch = \"wasm32\")",
		"cfg(not(target_arch = \"wasm32\"))",
	] {
		let dependency = document["target"][target]["dependencies"]["reinhardt"]
			.as_inline_table_mut()
			.unwrap();
		dependency.remove("version");
		dependency.insert("path", Value::from(workspace.to_str().unwrap()));
	}
	let native_commands = document["target"]["cfg(not(target_arch = \"wasm32\"))"]["dependencies"]
		["reinhardt-commands"]
		.as_inline_table_mut()
		.unwrap();
	native_commands.remove("version");
	native_commands.insert(
		"path",
		Value::from(
			workspace
				.join("crates/reinhardt-commands")
				.to_str()
				.unwrap(),
		),
	);
	let dev_dependency = document["dev-dependencies"]["reinhardt"]
		.as_table_mut()
		.unwrap();
	dev_dependency.remove("version");
	dev_dependency.insert("path", value(workspace.to_str().unwrap()));
	fs::write(manifest, document.to_string()).unwrap();
}

#[rstest]
#[tokio::test]
#[serial(cwd)]
async fn generated_pages_collects_assets_without_runtime_secret() {
	let temp = TempDir::new().unwrap();
	{
		let _cwd = CurrentDirectory::enter(temp.path());
		let mut options = HashMap::new();
		options.insert("with-pages".to_owned(), vec!["true".to_owned()]);
		options.insert("no-interactive".to_owned(), vec!["true".to_owned()]);
		let context =
			CommandContext::new(vec!["capability-pages".to_owned()]).with_options(options);
		StartProjectCommand.execute(&context).await.unwrap();
	}
	let project = temp.path().join("capability-pages");
	let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../..")
		.canonicalize()
		.unwrap();
	use_local_workspace_dependencies(&project.join("Cargo.toml"), &workspace);
	let config = project.join("settings/base.toml");
	let content = fs::read_to_string(&config).unwrap();
	let content = content
		.lines()
		.map(|line| {
			if line.starts_with("secret_key = ") {
				"secret_key = \"${REINHARDT_PAGES_MISSING_SECRET_6336}\""
			} else {
				line
			}
		})
		.collect::<Vec<_>>()
		.join("\n");
	fs::write(
		&config,
		format!("staticfiles_dirs = [\"assets\"]\n\n{content}\n"),
	)
	.unwrap();
	fs::create_dir_all(project.join("assets")).unwrap();
	fs::write(project.join("assets/logo.txt"), "pages smoke asset\n").unwrap();

	let target = project.join("target");
	let build = Command::new(env!("CARGO"))
		.current_dir(&project)
		.env("CARGO_TARGET_DIR", &target)
		.env("CARGO_BUILD_JOBS", "2")
		.args(["build", "--quiet", "--bin", "manage"])
		.output()
		.unwrap();
	assert!(
		build.status.success(),
		"generated Pages binary failed to build: {}",
		String::from_utf8_lossy(&build.stderr)
	);
	let binary = target.join("debug/manage");
	let collected = invoke(&binary, &project, &["collectstatic", "--no-input"]);
	assert!(
		collected.status.success(),
		"secret-free Pages collectstatic failed: {}",
		String::from_utf8_lossy(&collected.stderr)
	);
	let manifest: serde_json::Value =
		serde_json::from_slice(&fs::read(project.join("dist/manifest.json")).unwrap()).unwrap();
	let asset = manifest["paths"]["logo.txt"]
		.as_str()
		.expect("collected Pages asset");
	assert_eq!(
		fs::read_to_string(project.join("dist").join(asset)).unwrap(),
		"pages smoke asset\n"
	);
	let runtime = invoke(&binary, &project, &["runserver"]);
	assert!(!runtime.status.success());
	assert!(
		String::from_utf8_lossy(&runtime.stderr).contains("REINHARDT_PAGES_MISSING_SECRET_6336"),
		"unexpected Pages runtime rejection: {}",
		String::from_utf8_lossy(&runtime.stderr)
	);
}
