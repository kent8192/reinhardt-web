//! Installed-app ownership through the actual capability-aware management CLI.

use reinhardt_test::fixtures::temp_dir;
use rstest::{fixture, rstest};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

struct Consumer {
	root: TempDir,
	binary: PathBuf,
}

#[fixture]
fn consumer(temp_dir: TempDir) -> Consumer {
	let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
		.ancestors()
		.nth(2)
		.expect("integration crate is under tests/");
	for (relative, template) in [
		(
			"Cargo.toml",
			include_str!("fixtures/capability_migration_scope/Cargo.toml.tpl"),
		),
		(
			"src/bin/manage.rs",
			include_str!("fixtures/capability_migration_scope/manage.rs.tpl"),
		),
	] {
		let destination = temp_dir.path().join(relative);
		fs::create_dir_all(destination.parent().unwrap()).unwrap();
		fs::write(
			destination,
			template.replace("__REINHARDT_ROOT__", repository.to_str().unwrap()),
		)
		.unwrap();
	}
	let target = std::env::var_os("CARGO_TARGET_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| repository.join("target"));
	let build = Command::new(env!("CARGO"))
		.current_dir(temp_dir.path())
		.env("CARGO_TARGET_DIR", &target)
		.env("CARGO_BUILD_JOBS", "2")
		.args(["build", "--quiet", "--bin", "capability-migration-manage"])
		.output()
		.expect("build the consumer management binary");
	assert_status(&build, 0);
	Consumer {
		root: temp_dir,
		binary: target.join("debug/capability-migration-manage"),
	}
}

fn invoke(consumer: &Consumer, project: &Path, apps: &str, args: &[&str]) -> Output {
	Command::new(&consumer.binary)
		.current_dir(project)
		.args(args)
		.env("REINHARDT_TEST_INSTALLED_APPS", apps)
		.env("NO_COLOR", "1")
		.env_remove("DATABASE_URL")
		.env_remove("REINHARDT_MIGRATION_UNRELATED_SECRET")
		.output()
		.expect("run the actual capability CLI")
}

fn assert_status(output: &Output, expected: i32) {
	assert_eq!(
		output.status.code(),
		Some(expected),
		"stdout:\n{}\nstderr:\n{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr),
	);
}

fn proposed_apps(output: &Output) -> Vec<String> {
	let mut apps = String::from_utf8(output.stdout.clone())
		.expect("management output is UTF-8")
		.lines()
		.filter_map(|line| {
			line.strip_prefix("[INFO] Migrations for '")
				.and_then(|app| app.strip_suffix("':"))
				.map(str::to_owned)
		})
		.collect::<Vec<_>>();
	apps.sort();
	apps
}

fn migration_files(project: &Path) -> Vec<(PathBuf, Vec<u8>)> {
	let root = project.join("migrations");
	let mut files = Vec::new();
	for app in fs::read_dir(&root).expect("migration root exists") {
		for file in fs::read_dir(app.unwrap().path()).expect("app migration directory exists") {
			let path = file.unwrap().path();
			files.push((
				path.strip_prefix(&root).unwrap().to_path_buf(),
				fs::read(&path).expect("migration source is readable"),
			));
		}
	}
	files.sort();
	files
}

#[rstest]
fn capability_cli_scopes_generation_and_baseline_checks(consumer: Consumer) {
	for (scenario, apps) in [
		("label", r#"["identity"]"#),
		("module_path", r#"["consumer.accounts"]"#),
	] {
		// Arrange: linked auth/session models, an installed identity app, and no secrets.
		let project = consumer.root.path().join(scenario);
		fs::create_dir_all(project.join("src/bin")).unwrap();
		fs::write(project.join("src/bin/manage.rs"), "fn main() {}\n").unwrap();
		fs::create_dir_all(project.join("migrations")).unwrap();

		// Act / Assert: dry-run and check detect exactly the installed app without writing.
		let preview = invoke(
			&consumer,
			&project,
			apps,
			&["makemigrations", "--dry-run", "--state-source", "files"],
		);
		assert_status(&preview, 0);
		assert_eq!(proposed_apps(&preview), ["identity"], "{scenario}");
		assert_eq!(migration_files(&project), []);
		let missing = invoke(
			&consumer,
			&project,
			apps,
			&[
				"makemigrations",
				"--check",
				"--dry-run",
				"--state-source",
				"files",
			],
		);
		assert_status(&missing, 1);
		assert_eq!(proposed_apps(&missing), ["identity"]);
		assert_eq!(
			String::from_utf8(missing.stderr).unwrap().lines().last(),
			Some("Execution error: 1 migration(s) would be created"),
		);
		assert_eq!(migration_files(&project), []);
		for mode in [None, Some("--empty"), Some("--merge")] {
			let mut args = vec!["makemigrations", "auth", "--dry-run"];
			if let Some(mode) = mode {
				args.push(mode);
			}
			let rejected = invoke(&consumer, &project, apps, &args);
			assert_status(&rejected, 1);
			assert_eq!(
				String::from_utf8(rejected.stderr).unwrap().trim(),
				"Execution error: App 'auth' is not in CoreSettings::installed_apps.",
			);
			assert_eq!(migration_files(&project), []);
		}

		// Generate the baseline through the same public CLI, rather than hand-authoring it.
		let generated = invoke(
			&consumer,
			&project,
			apps,
			&["makemigrations", "--state-source", "files"],
		);
		assert_status(&generated, 0);
		assert_eq!(proposed_apps(&generated), ["identity"]);
		let baseline = migration_files(&project);
		assert_eq!(
			baseline
				.iter()
				.map(|(path, _)| path.as_path())
				.collect::<Vec<_>>(),
			[Path::new("identity/0001_initial.rs")],
		);

		// The baseline is a no-op before and after applying it to a disposable SQLite DB.
		let database_url = "sqlite://db.sqlite3";
		for applied in [false, true] {
			if applied {
				let migrated = invoke(
					&consumer,
					&project,
					apps,
					&["migrate", "--database", database_url],
				);
				assert_status(&migrated, 0);
				assert_eq!(
					String::from_utf8(migrated.stdout).unwrap().lines().last(),
					Some("[SUCCESS] Applied 1 migration(s) successfully"),
				);
				assert!(project.join("db.sqlite3").is_file());
			}
			let checked = invoke(
				&consumer,
				&project,
				apps,
				&[
					"makemigrations",
					"--check",
					"--dry-run",
					"--state-source",
					"files",
				],
			);
			assert_status(&checked, 0);
			assert_eq!(proposed_apps(&checked), Vec::<String>::new());
			assert_eq!(
				String::from_utf8(checked.stdout).unwrap().lines().last(),
				Some("[INFO] No changes detected"),
			);
			assert_eq!(migration_files(&project), baseline);
		}
	}

	// Empty defaults retain automatic discovery; an installed app without models is a no-op.
	let project = consumer.root.path().join("defaults");
	fs::create_dir_all(project.join("src/bin")).unwrap();
	fs::write(project.join("src/bin/manage.rs"), "fn main() {}\n").unwrap();
	fs::create_dir_all(project.join("migrations")).unwrap();
	let automatic = invoke(
		&consumer,
		&project,
		"[]",
		&["makemigrations", "--dry-run", "--state-source", "files"],
	);
	assert_status(&automatic, 0);
	assert_eq!(proposed_apps(&automatic), ["auth", "default", "identity"]);
	let no_models = invoke(
		&consumer,
		&project,
		r#"["emptyapp"]"#,
		&[
			"makemigrations",
			"--check",
			"--dry-run",
			"--state-source",
			"files",
		],
	);
	assert_status(&no_models, 0);
	assert_eq!(proposed_apps(&no_models), Vec::<String>::new());
	assert_eq!(migration_files(&project), []);
}
