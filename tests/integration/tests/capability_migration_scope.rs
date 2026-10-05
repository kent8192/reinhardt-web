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
	invoke_with_config(consumer, project, apps, args, &serde_json::json!({}))
}

fn invoke_with_config(
	consumer: &Consumer,
	project: &Path,
	apps: &str,
	args: &[&str],
	config: &serde_json::Value,
) -> Output {
	Command::new(&consumer.binary)
		.current_dir(project)
		.args(args)
		.env("REINHARDT_TEST_INSTALLED_APPS", apps)
		.env("REINHARDT_TEST_MIGRATION_CONFIG", config.to_string())
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

fn new_project(consumer: &Consumer, name: &str) -> PathBuf {
	let project = consumer.root.path().join(name);
	fs::create_dir_all(project.join("src/bin")).unwrap();
	fs::write(project.join("src/bin/manage.rs"), "fn main() {}\n").unwrap();
	fs::create_dir_all(project.join("migrations")).unwrap();
	project
}

fn write_migration(project: &Path, app: &str, name: &str, fields: &str) {
	let directory = project.join("migrations").join(app);
	fs::create_dir_all(&directory).unwrap();
	fs::write(
		directory.join(format!("{name}.rs")),
		format!(
			r#"fn migration() -> Migration {{
			Migration {{ {fields}, ..Migration::new("{name}", "{app}") }}
		}}"#,
		),
	)
	.unwrap();
}

#[rstest]
fn capability_check_rejects_empty_and_merge_proposals_without_writes(consumer: Consumer) {
	// Arrange
	let project = new_project(&consumer, "check_modes");
	let apps = r#"["identity"]"#;
	let before = migration_files(&project);

	// Act
	let empty = invoke(
		&consumer,
		&project,
		apps,
		&["makemigrations", "identity", "--empty", "--check"],
	);

	// Assert
	assert_status(&empty, 1);
	assert!(String::from_utf8_lossy(&empty.stderr).contains("1 migration(s) would be created"));
	assert!(String::from_utf8_lossy(&empty.stdout).contains("Would create empty migration"));
	assert_eq!(migration_files(&project), before);
	let preview = invoke(
		&consumer,
		&project,
		apps,
		&["makemigrations", "identity", "--empty", "--dry-run"],
	);
	assert_status(&preview, 0);
	assert_eq!(migration_files(&project), before);

	// Arrange
	write_migration(
		&project,
		"identity",
		"0001_root",
		"operations: Vec::new(), dependencies: Vec::new()",
	);
	let before = migration_files(&project);

	// Act
	let clean = invoke(
		&consumer,
		&project,
		apps,
		&["makemigrations", "--merge", "--check"],
	);

	// Assert
	assert_status(&clean, 0);
	assert!(String::from_utf8_lossy(&clean.stdout).contains("No conflicts detected"));
	assert_eq!(migration_files(&project), before);

	// Arrange
	for name in ["0002_left", "0002_right"] {
		write_migration(
			&project,
			"identity",
			name,
			r#"operations: Vec::new(), dependencies: vec![("identity", "0001_root")]"#,
		);
	}
	let before = migration_files(&project);

	// Act
	let merge = invoke(
		&consumer,
		&project,
		apps,
		&["makemigrations", "--merge", "--check"],
	);

	// Assert
	assert_status(&merge, 1);
	assert!(String::from_utf8_lossy(&merge.stderr).contains("1 migration(s) would be created"));
	assert!(String::from_utf8_lossy(&merge.stdout).contains("Would create merge migration"));
	assert_eq!(migration_files(&project), before);
	let preview = invoke(
		&consumer,
		&project,
		apps,
		&["makemigrations", "--merge", "--dry-run"],
	);
	assert_status(&preview, 0);
	assert_eq!(migration_files(&project), before);
}

#[rstest]
fn capability_database_state_does_not_log_configured_credentials(consumer: Consumer) {
	// Arrange: synthetic credentials; this consumer deliberately enables only SQLite.
	let project = new_project(&consumer, "credentials");
	let password = "capability_secret_must_not_be_logged_6493";
	let config = serde_json::json!({"core": {"databases": {"selected": {
		"engine": "postgresql", "name": "migration_test", "user": "migration_user",
		"host": "127.0.0.1", "port": 1, "password": password,
	}}}});

	// Act
	let output = invoke_with_config(
		&consumer,
		&project,
		r#"["identity"]"#,
		&[
			"makemigrations",
			"--dry-run",
			"--state-source",
			"database",
			"--database",
			"selected",
		],
		&config,
	);

	// Assert
	assert_status(&output, 1);
	let stdout = String::from_utf8_lossy(&output.stdout);
	let stderr = String::from_utf8_lossy(&output.stderr);
	assert!(stderr.contains("Database connection failed"), "{stderr}");
	for log in [stdout, stderr] {
		assert!(
			!log.contains(password),
			"credential appeared in command output"
		);
		assert!(
			!log.contains("Database URL:"),
			"raw URL logging must be absent"
		);
	}
	assert_eq!(migration_files(&project), []);
}

#[rstest]
fn capability_dependencies_order_plans_execution_and_file_state(consumer: Consumer) {
	// Arrange
	let project = new_project(&consumer, "dependency_order");
	let apps =
		r#"["consumer.accounts", "z_accounts", "z_core", "z_feature", "z_setting", "z_installed"]"#;
	let config = serde_json::json!({
		"core": {"migration_features": ["core_enabled"], "migration_swappable_settings": {"AUTH_USER_MODEL": "absent_core.User"}},
		"migrations": {
			"migration_features": ["local_enabled"],
			"migration_settings": {"ENABLE_AUDIT": "true", "AUTH_USER_MODEL": "absent_general.User"},
			"migration_swappable_settings": {"AUTH_USER_MODEL": "z_accounts.User"},
		}
	});
	write_migration(
		&project,
		"z_accounts",
		"0001_initial",
		r#"
		operations: vec![Operation::CreateTable {
			name: "ordered_dependency".to_string(),
			columns: vec![ColumnDefinition {
				name: "id".to_string(), type_definition: FieldType::BigInteger,
				not_null: true, unique: false, primary_key: true, auto_increment: false, default: None,
			}], constraints: vec![],
		}], dependencies: Vec::new()
	"#,
	);
	for app in ["z_core", "z_feature", "z_setting", "z_installed"] {
		write_migration(
			&project,
			app,
			"0001_initial",
			"operations: Vec::new(), dependencies: Vec::new()",
		);
	}
	write_migration(
		&project,
		"identity",
		"0001_initial",
		r#"
		operations: vec![Operation::AddColumn {
			table: "ordered_dependency".to_string(),
			column: ColumnDefinition {
				name: "label".to_string(), type_definition: FieldType::Text,
				not_null: false, unique: false, primary_key: false, auto_increment: false, default: None,
			},
		}], dependencies: Vec::new(),
		swappable_dependencies: vec![SwappableDependency::new("AUTH_USER_MODEL", "absent_default", "User", "0001_initial")],
		optional_dependencies: vec![
			OptionalDependency::new("z_core", "0001_initial", DependencyCondition::FeatureEnabled("core_enabled".to_string())),
			OptionalDependency::new("z_feature", "0001_initial", DependencyCondition::FeatureEnabled("local_enabled".to_string())),
			OptionalDependency::new("z_setting", "0001_initial", DependencyCondition::SettingEnabled("ENABLE_AUDIT".to_string())),
			OptionalDependency::new("z_installed", "0001_initial", DependencyCondition::AppInstalled("identity".to_string())),
			OptionalDependency::new("absent_disabled", "0001_initial", DependencyCondition::FeatureEnabled("disabled".to_string())),
		]
	"#,
	);
	let before = migration_files(&project);

	// Act
	let plan = invoke_with_config(
		&consumer,
		&project,
		apps,
		&["migrate", "--database", "sqlite:ordered.sqlite3", "--plan"],
		&config,
	);

	// Assert
	assert_status(&plan, 0);
	let stdout = String::from_utf8_lossy(&plan.stdout);
	let dependent = stdout.find("identity:0001_initial (apply)").unwrap();
	for app in [
		"z_accounts",
		"z_core",
		"z_feature",
		"z_setting",
		"z_installed",
	] {
		assert!(
			stdout.find(&format!("{app}:0001_initial (apply)")).unwrap() < dependent,
			"{stdout}"
		);
	}
	assert_eq!(migration_files(&project), before);

	// Act: adding the column requires the swapped provider's table to exist first.
	let applied = invoke_with_config(
		&consumer,
		&project,
		apps,
		&["migrate", "--database", "sqlite:ordered.sqlite3"],
		&config,
	);

	// Assert
	assert_status(&applied, 0);
	assert!(
		String::from_utf8_lossy(&applied.stdout).contains("Applied 6 migration(s) successfully")
	);
	assert_eq!(migration_files(&project), before);

	// Act
	let replayed = invoke_with_config(
		&consumer,
		&project,
		apps,
		&["makemigrations", "--dry-run", "--state-source", "files"],
		&config,
	);

	// Assert
	assert_status(&replayed, 0);
	assert_eq!(migration_files(&project), before);
	let database_replayed = invoke_with_config(
		&consumer,
		&project,
		apps,
		&["makemigrations", "--dry-run", "--state-source", "database"],
		&serde_json::json!({
			"core": {"migration_features": ["core_enabled"], "databases": {"default": {"engine": "sqlite", "name": "ordered.sqlite3"}}},
			"migrations": config["migrations"],
		}),
	);
	assert_status(&database_replayed, 0);
	assert_eq!(migration_files(&project), before);
}

#[rstest]
fn capability_dependency_configuration_controls_cycles_in_every_command_path(consumer: Consumer) {
	for (scenario, condition, enabled) in [
		(
			"core_feature",
			"FeatureEnabled",
			serde_json::json!({"core": {"migration_features": ["audit"]}}),
		),
		(
			"local_feature",
			"FeatureEnabled",
			serde_json::json!({"migrations": {"migration_features": ["audit"]}}),
		),
		(
			"local_setting",
			"SettingEnabled",
			serde_json::json!({"migrations": {"migration_settings": {"audit": "true"}}}),
		),
	] {
		// Arrange
		let project = new_project(&consumer, scenario);
		write_migration(
			&project,
			"identity",
			"0001_root",
			&format!(
				r#"
			operations: Vec::new(), dependencies: Vec::new(),
			optional_dependencies: vec![OptionalDependency::new("identity", "0002_tail", DependencyCondition::{condition}("audit".to_string()))]
		"#
			),
		);
		write_migration(
			&project,
			"identity",
			"0002_tail",
			r#"operations: Vec::new(), dependencies: vec![("identity", "0001_root")]"#,
		);
		let before = migration_files(&project);
		for args in [
			vec!["makemigrations", "--dry-run", "--state-source", "files"],
			vec!["migrate", "--database", "sqlite::memory:", "--plan"],
			vec!["migrate", "--database", "sqlite::memory:"],
		] {
			// Act
			let disabled = invoke(&consumer, &project, r#"["identity"]"#, &args);
			let active =
				invoke_with_config(&consumer, &project, r#"["identity"]"#, &args, &enabled);

			// Assert
			assert_status(&disabled, 0);
			assert_status(&active, 1);
			assert!(
				String::from_utf8_lossy(&active.stderr).contains("Circular"),
				"{scenario}: {}",
				String::from_utf8_lossy(&active.stderr)
			);
			assert_eq!(migration_files(&project), before);
		}
	}

	// Arrange: the default dependency forms a cycle; selecting accounts removes it.
	let project = new_project(&consumer, "swappable_cycle");
	write_migration(
		&project,
		"identity",
		"0001_root",
		r#"
		operations: Vec::new(), dependencies: Vec::new(),
		swappable_dependencies: vec![SwappableDependency::new("AUTH_USER_MODEL", "identity", "User", "0001_root")]
	"#,
	);
	write_migration(
		&project,
		"z_accounts",
		"0001_root",
		"operations: Vec::new(), dependencies: Vec::new()",
	);
	let before = migration_files(&project);
	for (config, expected) in [
		(serde_json::json!({}), 1),
		(
			serde_json::json!({"core": {"migration_swappable_settings": {"AUTH_USER_MODEL": "z_accounts.User"}}}),
			0,
		),
		(
			serde_json::json!({"migrations": {"migration_swappable_settings": {"AUTH_USER_MODEL": "z_accounts.User"}}}),
			0,
		),
		(
			serde_json::json!({
				"core": {"migration_swappable_settings": {"AUTH_USER_MODEL": "identity.User"}},
				"migrations": {"migration_settings": {"AUTH_USER_MODEL": "z_accounts.User"}},
			}),
			0,
		),
		(
			serde_json::json!({
				"core": {"migration_swappable_settings": {"AUTH_USER_MODEL": "identity.User"}},
				"migrations": {
					"migration_settings": {"AUTH_USER_MODEL": "identity.User"},
					"migration_swappable_settings": {"AUTH_USER_MODEL": "z_accounts.User"},
				},
			}),
			0,
		),
	] {
		for args in [
			vec!["makemigrations", "--dry-run", "--state-source", "files"],
			vec!["migrate", "--database", "sqlite::memory:", "--plan"],
			vec!["migrate", "--database", "sqlite::memory:"],
		] {
			// Act
			let output = invoke_with_config(
				&consumer,
				&project,
				r#"["identity", "z_accounts"]"#,
				&args,
				&config,
			);

			// Assert
			assert_status(&output, expected);
			if expected == 1 {
				assert!(String::from_utf8_lossy(&output.stderr).contains("Circular"));
			}
			assert_eq!(migration_files(&project), before);
		}
	}
}
