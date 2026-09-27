//! Command-boundary tests for `makemigrations`.
//!
//! These tests exercise CLI dispatch and `MakeMigrationsCommand::execute()`
//! instead of mirroring migration generation internals.

use clap::Parser;
use reinhardt_commands::{BaseCommand, Cli, CommandContext, MakeMigrationsCommand, run_command};
use reinhardt_db::migrations::model_registry::{FieldMetadata, ModelMetadata, global_registry};
use reinhardt_db::migrations::{FieldType, FilesystemSource, MigrationSource};
use rstest::{fixture, rstest};
use serial_test::serial;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

struct ProjectDirGuard {
	original_dir: PathBuf,
}

impl ProjectDirGuard {
	fn enter(project_dir: &Path) -> Self {
		let original_dir = std::env::current_dir().expect("current dir should be readable");
		std::env::set_current_dir(project_dir).expect("temporary project dir should be enterable");
		Self { original_dir }
	}
}

impl Drop for ProjectDirGuard {
	fn drop(&mut self) {
		std::env::set_current_dir(&self.original_dir)
			.expect("original current dir should be restored");
	}
}

struct ModelRegistryGuard;

impl ModelRegistryGuard {
	fn clear() -> Self {
		global_registry().clear();
		Self
	}
}

impl Drop for ModelRegistryGuard {
	fn drop(&mut self) {
		global_registry().clear();
	}
}

#[fixture]
fn create_project_root() -> TempDir {
	let project_dir = TempDir::new().expect("temporary project dir should be created");
	std::fs::create_dir_all(project_dir.path().join("src/bin")).expect("src/bin should be created");
	std::fs::write(
		project_dir.path().join("src/bin/manage.rs"),
		"fn main() {}\n",
	)
	.expect("manage.rs should be written");
	project_dir
}

fn register_test_model(app_label: &str, model_name: &str, table_name: &str) {
	let mut metadata = ModelMetadata::new(app_label, model_name, table_name);
	metadata.add_field(
		"id".to_string(),
		FieldMetadata::new(FieldType::Integer)
			.with_param("primary_key", "true")
			.with_param("auto_increment", "true"),
	);
	metadata.add_field(
		"name".to_string(),
		FieldMetadata::new(FieldType::VarChar(100)).with_param("max_length", "100"),
	);
	global_registry().register_model(metadata);
}

fn makemigrations_context(app_label: Option<&str>, migrations_dir: &Path) -> CommandContext {
	let mut ctx = CommandContext::default();
	if let Some(app_label) = app_label {
		ctx.add_arg(app_label.to_string());
	}
	ctx.set_option(
		"migrations-dir".to_string(),
		migrations_dir.to_string_lossy().to_string(),
	);
	ctx
}

fn write_migration_file(
	migrations_dir: &Path,
	app_label: &str,
	name: &str,
	dependencies: &[(&str, &str)],
) {
	let app_dir = migrations_dir.join(app_label);
	std::fs::create_dir_all(&app_dir).expect("app migration dir should be created");
	let dependencies = dependencies
		.iter()
		.map(|(app, migration)| format!("(\"{}\".to_string(), \"{}\".to_string())", app, migration))
		.collect::<Vec<_>>()
		.join(", ");
	let content = format!(
		r#"use reinhardt_db::migrations::{{Migration, Operation}};

pub(super) fn migration() -> Migration {{
	Migration {{
		app_label: "{app_label}".to_string(),
		name: "{name}".to_string(),
		operations: vec![],
		dependencies: vec![{dependencies}],
		..Default::default()
	}}
}}
"#
	);
	std::fs::write(app_dir.join(format!("{name}.rs")), content)
		.expect("migration fixture should be written");
}

fn migration_file_names(migrations_dir: &Path, app_label: &str) -> Vec<String> {
	let app_dir = migrations_dir.join(app_label);
	if !app_dir.exists() {
		return Vec::new();
	}
	let mut names = std::fs::read_dir(app_dir)
		.expect("app migration dir should be readable")
		.map(|entry| {
			entry
				.expect("directory entry should be readable")
				.file_name()
				.to_string_lossy()
				.into_owned()
		})
		.collect::<Vec<_>>();
	names.sort();
	names
}

fn read_migration_file(migrations_dir: &Path, app_label: &str, name: &str) -> String {
	std::fs::read_to_string(migrations_dir.join(app_label).join(format!("{name}.rs")))
		.expect("migration file should be readable")
}

async fn execute_cli(arguments: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
	let cli = Cli::try_parse_from(arguments).expect("management CLI arguments should parse");
	run_command(cli.command, cli.verbosity).await
}

#[rstest]
#[tokio::test]
#[serial(makemigrations_command_boundary)]
async fn execute_generates_initial_migration_file_from_registered_model() {
	let _registry = ModelRegistryGuard::clear();
	let project_dir = create_project_root();
	let _cwd = ProjectDirGuard::enter(project_dir.path());
	let migrations_dir = project_dir.path().join("migrations");

	register_test_model("testapp", "TestModel", "testapp_testmodel");

	let mut ctx = makemigrations_context(Some("testapp"), &migrations_dir);
	ctx.set_option("force-empty-state".to_string(), "true".to_string());

	let result = MakeMigrationsCommand.execute(&ctx).await;

	assert!(result.is_ok(), "makemigrations failed: {:?}", result.err());
	let file_names = migration_file_names(&migrations_dir, "testapp");
	assert_eq!(file_names.len(), 1, "expected exactly one migration file");
	assert!(
		file_names[0].starts_with("0001_initial"),
		"expected initial migration, got {:?}",
		file_names
	);
	let content = read_migration_file(
		&migrations_dir,
		"testapp",
		file_names[0].trim_end_matches(".rs"),
	);
	assert!(content.contains("pub(super) fn migration() -> Migration"));
	assert!(content.contains("app_label: \"testapp\".to_string()"));
	assert!(content.contains("Operation::CreateTable"));
	assert!(content.contains("initial: Some(true)"));
}

#[rstest]
#[tokio::test]
#[serial(makemigrations_command_boundary)]
async fn execute_dry_run_does_not_write_migration_file() {
	let _registry = ModelRegistryGuard::clear();
	let project_dir = create_project_root();
	let _cwd = ProjectDirGuard::enter(project_dir.path());
	let migrations_dir = project_dir.path().join("migrations");

	register_test_model("testapp", "TestModel", "testapp_testmodel");

	let mut ctx = makemigrations_context(Some("testapp"), &migrations_dir);
	ctx.set_option("force-empty-state".to_string(), "true".to_string());
	ctx.set_option("dry-run".to_string(), "true".to_string());

	let result = MakeMigrationsCommand.execute(&ctx).await;

	assert!(result.is_ok(), "dry-run failed: {:?}", result.err());
	assert!(
		migration_file_names(&migrations_dir, "testapp").is_empty(),
		"dry-run must not write migration files"
	);
}

#[rstest]
#[tokio::test]
#[serial(makemigrations_command_boundary)]
async fn execute_empty_requires_app_label() {
	let _registry = ModelRegistryGuard::clear();
	let project_dir = create_project_root();
	let _cwd = ProjectDirGuard::enter(project_dir.path());
	let migrations_dir = project_dir.path().join("migrations");

	let mut ctx = makemigrations_context(None, &migrations_dir);
	ctx.set_option("empty".to_string(), "true".to_string());

	let err = MakeMigrationsCommand
		.execute(&ctx)
		.await
		.expect_err("--empty without app label should fail");

	assert!(
		err.to_string().contains("App label is required"),
		"unexpected error: {err}"
	);
	assert!(
		migration_file_names(&migrations_dir, "testapp").is_empty(),
		"failing --empty command must not write migrations"
	);
}

#[rstest]
#[case::default(None, false)]
#[case::relative(Some("requested"), false)]
#[case::absolute_with_spaces_and_unicode(Some("custom migrations/日本語"), true)]
#[tokio::test]
#[serial(makemigrations_command_boundary)]
async fn cli_empty_migrations_use_selected_directory(
	create_project_root: TempDir,
	#[case] directory: Option<&str>,
	#[case] absolute: bool,
) {
	// Arrange
	let project_dir = create_project_root;
	let _cwd = ProjectDirGuard::enter(project_dir.path());
	let migrations_dir = project_dir.path().join(directory.unwrap_or("migrations"));
	let selected_path = if absolute {
		migrations_dir.clone()
	} else {
		PathBuf::from(directory.unwrap_or("migrations"))
	};
	let mut arguments = vec![
		"manage",
		"makemigrations",
		"testapp",
		"--empty",
		"--name",
		"manual",
	];
	if directory.is_some() {
		arguments.extend(["--migration-dir", selected_path.to_str().unwrap()]);
	}

	// Act: create the first migration in a previously nonexistent directory.
	execute_cli(&arguments)
		.await
		.expect("first migration should be created");

	// Assert
	assert_eq!(
		migration_file_names(&migrations_dir, "testapp"),
		["0001_manual.rs"]
	);
	let default_dir = project_dir.path().join("migrations");
	if directory.is_some() {
		assert!(!default_dir.exists());
		// A different history in the default directory must not affect the next migration.
		write_migration_file(&default_dir, "testapp", "0099_default", &[]);
	}

	// Act: reuse the selected history for numbering and dependencies.
	execute_cli(&arguments)
		.await
		.expect("second migration should be created");

	// Assert
	assert_eq!(
		migration_file_names(&migrations_dir, "testapp"),
		["0001_manual.rs", "0002_manual.rs"]
	);
	let migration = FilesystemSource::new(&migrations_dir)
		.get_migration("testapp", "0002_manual")
		.await
		.expect("generated migration should be readable");
	assert_eq!(
		migration.dependencies,
		[("testapp".to_owned(), "0001_manual".to_owned())]
	);
	assert!(migration.operations.is_empty());
	if directory.is_some() {
		assert_eq!(
			migration_file_names(&default_dir, "testapp"),
			["0099_default.rs"]
		);
	}
}

#[rstest]
#[case::dry_run("--dry-run")]
#[case::check("--check")]
#[tokio::test]
#[serial(makemigrations_command_boundary)]
async fn cli_inspection_modes_read_selected_directory(
	create_project_root: TempDir,
	#[case] mode: &str,
) {
	// Arrange
	let _registry = ModelRegistryGuard::clear();
	let project_dir = create_project_root;
	let _cwd = ProjectDirGuard::enter(project_dir.path());
	let migrations_dir = project_dir.path().join("requested");

	write_migration_file(&migrations_dir, "testapp", "0001_initial", &[]);
	write_migration_file(
		&migrations_dir,
		"testapp",
		"0002_left",
		&[("testapp", "0001_initial")],
	);
	write_migration_file(
		&migrations_dir,
		"testapp",
		"0002_right",
		&[("testapp", "0001_initial")],
	);
	register_test_model("testapp", "TestModel", "testapp_testmodel");

	// Act
	let err = execute_cli(&[
		"manage",
		"makemigrations",
		"testapp",
		mode,
		"--force-empty-state",
		"--migration-dir",
		"requested",
	])
	.await
	.expect_err("inspection must detect conflicts in the selected directory");

	// Assert
	assert_eq!(
		err.to_string(),
		"Execution error: Run 'makemigrations --merge' to resolve migration conflicts."
	);
	assert_eq!(
		migration_file_names(&migrations_dir, "testapp"),
		["0001_initial.rs", "0002_left.rs", "0002_right.rs"]
	);
	assert!(!project_dir.path().join("migrations").exists());
}

#[rstest]
#[tokio::test]
#[serial(makemigrations_command_boundary)]
async fn cli_merge_uses_selected_directory(create_project_root: TempDir) {
	// Arrange
	let project_dir = create_project_root;
	let _cwd = ProjectDirGuard::enter(project_dir.path());
	let migrations_dir = project_dir.path().join("requested");

	write_migration_file(&migrations_dir, "testapp", "0001_initial", &[]);
	write_migration_file(
		&migrations_dir,
		"testapp",
		"0002_left",
		&[("testapp", "0001_initial")],
	);
	write_migration_file(
		&migrations_dir,
		"testapp",
		"0002_right",
		&[("testapp", "0001_initial")],
	);

	// Act: dry-run must inspect the selected graph without resolving it on disk.
	execute_cli(&[
		"manage",
		"makemigrations",
		"testapp",
		"--merge",
		"--dry-run",
		"--migration-dir",
		"requested",
	])
	.await
	.expect("merge dry-run should succeed");
	assert_eq!(
		migration_file_names(&migrations_dir, "testapp"),
		["0001_initial.rs", "0002_left.rs", "0002_right.rs"]
	);
	assert!(!project_dir.path().join("migrations").exists());

	execute_cli(&[
		"manage",
		"makemigrations",
		"testapp",
		"--merge",
		"--name",
		"merge",
		"--migration-dir",
		"requested",
	])
	.await
	.expect("merge should resolve the selected graph");

	// Assert
	let mut migration = FilesystemSource::new(&migrations_dir)
		.get_migration("testapp", "0003_merge")
		.await
		.expect("merge migration should be readable");
	migration.dependencies.sort();
	assert_eq!(
		migration.dependencies,
		[
			("testapp".to_owned(), "0002_left".to_owned()),
			("testapp".to_owned(), "0002_right".to_owned()),
		]
	);
	assert!(migration.operations.is_empty());
	assert_eq!(
		migration_file_names(&migrations_dir, "testapp"),
		[
			"0001_initial.rs",
			"0002_left.rs",
			"0002_right.rs",
			"0003_merge.rs"
		]
	);
	assert!(!project_dir.path().join("migrations").exists());
}

#[rstest]
#[tokio::test]
#[serial(makemigrations_command_boundary)]
async fn execute_merge_dry_run_writes_no_merge_file() {
	let _registry = ModelRegistryGuard::clear();
	let project_dir = create_project_root();
	let _cwd = ProjectDirGuard::enter(project_dir.path());
	let migrations_dir = project_dir.path().join("migrations");

	write_migration_file(&migrations_dir, "testapp", "0001_initial", &[]);
	write_migration_file(
		&migrations_dir,
		"testapp",
		"0002_left",
		&[("testapp", "0001_initial")],
	);
	write_migration_file(
		&migrations_dir,
		"testapp",
		"0002_right",
		&[("testapp", "0001_initial")],
	);

	let mut ctx = makemigrations_context(Some("testapp"), &migrations_dir);
	ctx.set_option("merge".to_string(), "true".to_string());
	ctx.set_option("dry-run".to_string(), "true".to_string());
	ctx.set_option("name".to_string(), "merge".to_string());

	let result = MakeMigrationsCommand.execute(&ctx).await;

	assert!(result.is_ok(), "merge dry-run failed: {:?}", result.err());
	assert!(
		!migrations_dir
			.join("testapp")
			.join("0003_merge.rs")
			.exists(),
		"merge dry-run must not write migration file"
	);
}

#[rstest]
#[tokio::test]
#[serial(makemigrations_command_boundary)]
async fn execute_outside_project_root_errors_before_writing() {
	let _registry = ModelRegistryGuard::clear();
	let project_dir = TempDir::new().expect("temporary non-project dir should be created");
	let _cwd = ProjectDirGuard::enter(project_dir.path());
	let migrations_dir = project_dir.path().join("migrations");

	register_test_model("testapp", "TestModel", "testapp_testmodel");

	let mut ctx = makemigrations_context(Some("testapp"), &migrations_dir);
	ctx.set_option("force-empty-state".to_string(), "true".to_string());

	let err = MakeMigrationsCommand
		.execute(&ctx)
		.await
		.expect_err("makemigrations outside project root should fail");

	assert!(
		err.to_string().contains("Cannot find src/bin/manage.rs"),
		"unexpected error: {err}"
	);
	assert!(
		!migrations_dir.exists(),
		"project-root guard must run before creating migration files"
	);
}
