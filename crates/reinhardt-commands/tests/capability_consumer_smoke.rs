//! Run a Cloud-shaped registry through the actual management binary.
#![cfg(feature = "contract")]

use rstest::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

const FIXTURE: &str = "tests/fixtures/capability_project";

fn materialize() -> TempDir {
	let temp = TempDir::new().expect("create consumer directory");
	let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../..")
		.canonicalize()
		.expect("resolve workspace");
	for relative in [
		"Cargo.toml",
		"src/lib.rs",
		"src/bin/manage.rs",
		"settings/base.toml",
		"assets/logo.txt",
	] {
		let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE).join(
			if relative == "assets/logo.txt" {
				relative.to_owned()
			} else {
				format!("{relative}.tpl")
			},
		);
		let content = fs::read_to_string(source)
			.expect("read consumer fixture")
			.replace("__REINHARDT_ROOT__", workspace.to_str().unwrap());
		let destination = temp.path().join(relative);
		fs::create_dir_all(destination.parent().unwrap()).unwrap();
		fs::write(destination, content).unwrap();
	}
	temp
}

fn invoke(binary: &Path, root: &Path, args: &[&str]) -> Output {
	invoke_with_database_url(binary, root, args, None)
}

fn invoke_with_database_url(
	binary: &Path,
	root: &Path,
	args: &[&str],
	database_url: Option<&str>,
) -> Output {
	let mut command = Command::new(binary);
	command.current_dir(root).args(args);
	if let Some(database_url) = database_url {
		command.env("DATABASE_URL", database_url);
	} else {
		command.env_remove("DATABASE_URL");
	}
	for name in [
		"REINHARDT_CAPABILITY_RUNTIME_SECRET_6336",
		"REINHARDT_CAPABILITY_DB_PASSWORD_6336",
		"REINHARDT_CAPABILITY_JWT_SECRET_6336",
		"REINHARDT_CAPABILITY_SESSION_SECRET_6336",
	] {
		command.env_remove(name);
	}
	command.output().expect("run consumer manage")
}

#[rstest]
fn static_collection_uses_only_declared_configuration() {
	let consumer = materialize();
	let root = consumer.path();
	let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../..")
		.canonicalize()
		.unwrap();
	let target = std::env::var_os("CARGO_TARGET_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| workspace.join("target"));
	let build = Command::new(env!("CARGO"))
		.current_dir(root)
		.env("CARGO_TARGET_DIR", &target)
		.env("CARGO_BUILD_JOBS", "2")
		.args(["build", "--quiet", "--bin", "capability-consumer-manage"])
		.output()
		.expect("build consumer management binary");
	assert!(
		build.status.success(),
		"consumer failed to build: {}",
		String::from_utf8_lossy(&build.stderr)
	);
	let binary = target.join("debug/capability-consumer-manage");

	let collected = invoke(&binary, root, &["collectstatic", "--no-input"]);
	assert!(
		collected.status.success(),
		"secret-free collectstatic failed: {}",
		String::from_utf8_lossy(&collected.stderr)
	);
	let manifest: serde_json::Value =
		serde_json::from_slice(&fs::read(root.join("dist/manifest.json")).unwrap()).unwrap();
	let published_name = manifest["paths"]["logo.txt"]
		.as_str()
		.expect("collected logo");
	assert_eq!(
		fs::read_to_string(root.join("dist").join(published_name)).unwrap(),
		"static smoke asset\n"
	);

	let config = root.join("settings/base.toml");
	let original = fs::read_to_string(&config).unwrap();
	fs::write(&config, original.replace("root = \"dist\"", "root = 7")).unwrap();
	let invalid = invoke(&binary, root, &["collectstatic", "--no-input"]);
	assert!(!invalid.status.success());
	assert!(String::from_utf8_lossy(&invalid.stderr).contains("static"));
	fs::write(&config, &original).unwrap();
	let visibility = invoke(
		&binary,
		root,
		&["showmigrations", "--database-url", "sqlite::memory:"],
	);
	assert!(
		visibility.status.success(),
		"migration visibility with an explicit database should skip runtime secrets: {}",
		String::from_utf8_lossy(&visibility.stderr)
	);
	let plan = invoke(
		&binary,
		root,
		&["migrate", "--database", "sqlite::memory:", "--plan"],
	);
	assert!(
		plan.status.success(),
		"migration plan with an explicit database should skip runtime secrets: {}",
		String::from_utf8_lossy(&plan.stderr)
	);
	let env_plan = invoke_with_database_url(
		&binary,
		root,
		&["migrate", "--plan"],
		Some("sqlite::memory:"),
	);
	assert!(
		env_plan.status.success(),
		"migration plan should use DATABASE_URL before the configured default: {}",
		String::from_utf8_lossy(&env_plan.stderr)
	);
	let check = invoke_with_database_url(&binary, root, &["check"], Some("sqlite::memory:"));
	assert!(
		check.status.success(),
		"scoped system checks should skip unrelated runtime secrets: {}",
		String::from_utf8_lossy(&check.stderr)
	);
	let deploy = invoke_with_database_url(
		&binary,
		root,
		&["check", "--deploy"],
		Some("sqlite::memory:"),
	);
	assert!(!deploy.status.success());
	assert!(
		String::from_utf8_lossy(&deploy.stderr)
			.contains("REINHARDT_CAPABILITY_RUNTIME_SECRET_6336")
	);

	let offline_migrations = invoke(&binary, root, &["makemigrations", "--check"]);
	assert!(
		offline_migrations.status.success(),
		"database-free migration check failed: {}",
		String::from_utf8_lossy(&offline_migrations.stderr)
	);
	let forced_empty = invoke(
		&binary,
		root,
		&["makemigrations", "--force-empty-state", "--dry-run"],
	);
	assert!(!forced_empty.status.success());
	assert!(
		String::from_utf8_lossy(&forced_empty.stderr)
			.contains("This may create duplicate migrations!"),
		"empty-state safety warning missing: {}",
		String::from_utf8_lossy(&forced_empty.stderr)
	);
	assert!(String::from_utf8_lossy(&forced_empty.stderr).contains("No models found"));
	let missing_alias = invoke(
		&binary,
		root,
		&[
			"makemigrations",
			"--state-source",
			"database",
			"--database",
			"missing",
		],
	);
	assert!(!missing_alias.status.success());
	assert!(
		String::from_utf8_lossy(&missing_alias.stderr).contains("core.databases.missing"),
		"unexpected selected-database rejection: {}",
		String::from_utf8_lossy(&missing_alias.stderr)
	);

	let runtime = invoke(&binary, root, &["runserver"]);
	assert!(!runtime.status.success());
	assert!(
		String::from_utf8_lossy(&runtime.stderr).contains("REINHARDT_CAPABILITY_JWT_SECRET_6336"),
		"unexpected runtime rejection: {}",
		String::from_utf8_lossy(&runtime.stderr),
	);

	fs::write(&config, "not = [valid TOML").unwrap();
	assert!(invoke(&binary, root, &["--help"]).status.success());
	assert!(invoke(&binary, root, &["--version"]).status.success());
	let routes = invoke(&binary, root, &["showurls"]);
	let route_error = String::from_utf8_lossy(&routes.stderr);
	assert!(route_error.contains("No URL patterns registered."));
	assert!(!route_error.contains("invalid TOML"));
	let unknown = invoke(&binary, root, &["unknown-command"]);
	assert!(!unknown.status.success());
	assert!(!String::from_utf8_lossy(&unknown.stderr).contains("invalid TOML"));
}
