//! Native migrate output must describe the work performed by that invocation.
//!
//! Child processes capture the real command output without changing its public
//! context API or redirecting stdout shared by parallel tests.

use reinhardt_commands::{BaseCommand, CommandContext, MigrateCommand};
use reinhardt_db::migrations::{
	DatabaseMigrationRecorder, FilesystemSource, MigrationSource, SchemaEditor,
};
use reinhardt_test::fixtures::migrations::{MigrationExecutorFixture, migration_executor};
use reinhardt_test::fixtures::temp_dir;
use rstest::{fixture, rstest};
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const MIGRATION_NAMES: [&str; 3] = ["0001_first", "0002_second", "0003_third"];

#[fixture]
fn migration_project(temp_dir: TempDir) -> TempDir {
	let app_dir = temp_dir.path().join("migrations/probe");
	std::fs::create_dir_all(&app_dir).expect("create migration directory");
	for (index, name) in MIGRATION_NAMES.iter().enumerate() {
		let dependencies = if index == 0 {
			String::new()
		} else {
			format!(
				"(\"probe\".to_string(), \"{}\".to_string())",
				MIGRATION_NAMES[index - 1]
			)
		};
		std::fs::write(
			app_dir.join(format!("{name}.rs")),
			format!(
				r#"use reinhardt::db::migrations::prelude::*;
pub fn migration() -> Migration {{
    Migration {{
        app_label: "probe".to_string(),
        name: "{name}".to_string(),
        operations: vec![Operation::RunSQL {{
            sql: "CREATE TABLE migrate_reporting_{} (id integer PRIMARY KEY)".to_string(),
            reverse_sql: None,
        }}],
        dependencies: vec![{dependencies}],
        atomic: true,
        replaces: vec![],
        initial: None,
        state_only: false,
        database_only: true,
        swappable_dependencies: vec![],
        optional_dependencies: vec![],
    }}
}}
"#,
				index + 1
			),
		)
		.expect("write migration source");
	}
	temp_dir
}

fn run_migrate(project: &Path, database_url: &str, app_only: bool, mode: &str) -> Vec<String> {
	let output = Command::new(std::env::current_exe().expect("locate test executable"))
		.args([
			"--ignored",
			"--exact",
			"migrate_command_probe",
			"--nocapture",
		])
		.env("REINHARDT_MIGRATE_REPORT_DATABASE", database_url)
		.env(
			"REINHARDT_MIGRATE_REPORT_DIRECTORY",
			project.join("migrations"),
		)
		.env(
			"REINHARDT_MIGRATE_REPORT_APP",
			if app_only { "probe" } else { "" },
		)
		.env("REINHARDT_MIGRATE_REPORT_MODE", mode)
		.output()
		.expect("run native migration command");
	assert!(
		output.status.success(),
		"migration command failed:\n{}\n{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);
	String::from_utf8(output.stdout)
		.expect("command output is UTF-8")
		.lines()
		.filter(|line| line.starts_with("[INFO]") || line.starts_with("[SUCCESS]"))
		.map(str::to_owned)
		.collect()
}

#[rstest]
#[tokio::test]
#[ignore = "invoked in a child process by the migrate reporting tests"]
async fn migrate_command_probe() {
	// Arrange
	let Ok(database_url) = std::env::var("REINHARDT_MIGRATE_REPORT_DATABASE") else {
		return;
	};
	let mut ctx = CommandContext::default();
	ctx.set_option("database".into(), database_url);
	ctx.set_option(
		"migrations-dir".into(),
		std::env::var("REINHARDT_MIGRATE_REPORT_DIRECTORY").expect("migration directory"),
	);
	let app = std::env::var("REINHARDT_MIGRATE_REPORT_APP").expect("app selection");
	if !app.is_empty() {
		ctx.add_arg(app);
	}
	let mode = std::env::var("REINHARDT_MIGRATE_REPORT_MODE").expect("execution mode");
	if !mode.is_empty() {
		ctx.set_option(mode, "true".into());
	}

	// Act / Assert
	MigrateCommand
		.execute(&ctx)
		.await
		.expect("native migration command succeeds");
}

fn expected_apply_output(applied_names: &[&str]) -> Vec<String> {
	let mut expected = vec![
		"[INFO] Running migrations...".into(),
		"[INFO] Loaded 3 migration(s)".into(),
		"[INFO] Applying migrations:".into(),
	];
	expected.extend(
		applied_names
			.iter()
			.map(|name| format!("[SUCCESS]   ✓ Applied: probe.{name}")),
	);
	expected.push("[INFO] ".into());
	expected.push(format!(
		"[SUCCESS] Applied {} migration(s) successfully",
		applied_names.len()
	));
	expected
}

#[rstest]
#[case::fresh_database(0)]
#[case::partially_applied_history(1)]
#[case::fully_applied_history(3)]
#[tokio::test]
async fn summary_counts_newly_applied_migrations(
	#[future] migration_executor: MigrationExecutorFixture,
	migration_project: TempDir,
	#[case] already_applied: usize,
	#[values(false, true)] app_only: bool,
) {
	// Arrange
	let (mut executor, _container, _pool, _port, url) = migration_executor.await;
	let migrations = FilesystemSource::new(migration_project.path().join("migrations"))
		.all_migrations()
		.await
		.expect("load migration files");
	assert_eq!(migrations.len(), MIGRATION_NAMES.len());
	for migration in &migrations {
		assert_eq!(migration.operations.len(), 1);
	}
	if already_applied != 0 {
		let seeded = executor
			.apply_migrations(&migrations[..already_applied])
			.await
			.expect("apply initial migration history");
		assert_eq!(seeded.applied.len(), already_applied);
	}

	// Act
	let output = run_migrate(migration_project.path(), &url, app_only, "");
	let replay_output = run_migrate(migration_project.path(), &url, app_only, "");
	let replay = executor
		.apply_migrations(&migrations)
		.await
		.expect("executor replay succeeds");

	// Assert
	assert_eq!(
		output.last().map(String::as_str),
		Some(
			format!(
				"[SUCCESS] Applied {} migration(s) successfully",
				MIGRATION_NAMES.len() - already_applied
			)
			.as_str()
		)
	);
	assert_eq!(
		replay_output.last().map(String::as_str),
		Some("[SUCCESS] Applied 0 migration(s) successfully")
	);
	assert_eq!(
		output,
		expected_apply_output(&MIGRATION_NAMES[already_applied..])
	);
	assert_eq!(replay_output, expected_apply_output(&[]));
	assert_eq!(replay.applied, Vec::<String>::new());
	let recorder = DatabaseMigrationRecorder::new(executor.connection().clone());
	let mut recorded: Vec<_> = recorder
		.get_applied_migrations()
		.await
		.expect("query migration history")
		.into_iter()
		.map(|record| (record.app, record.name))
		.collect();
	recorded.sort();
	assert_eq!(
		recorded,
		MIGRATION_NAMES
			.iter()
			.map(|name| ("probe".into(), (*name).into()))
			.collect::<Vec<_>>()
	);
	let mut editor = SchemaEditor::new(
		executor.connection().clone(),
		false,
		executor.connection().database_type(),
	)
	.await
	.expect("inspect migrated schema");
	for index in 1..=MIGRATION_NAMES.len() {
		assert!(
			editor
				.table_exists(&format!("migrate_reporting_{index}"))
				.await
				.expect("inspect migration table")
		);
	}
}

#[rstest]
#[tokio::test]
async fn fake_summary_is_distinct_from_executed_migrations(
	#[future] migration_executor: MigrationExecutorFixture,
	migration_project: TempDir,
) {
	// Arrange
	let (executor, _container, _pool, _port, url) = migration_executor.await;
	let recorder = DatabaseMigrationRecorder::new(executor.connection().clone());
	recorder
		.ensure_schema_table()
		.await
		.expect("create recorder table");

	// Act
	let output = run_migrate(migration_project.path(), &url, false, "fake");

	// Assert
	let mut expected = vec![
		"[INFO] Running migrations...".into(),
		"[INFO] Loaded 3 migration(s)".into(),
		"[INFO] Faking migrations (marking as applied without execution):".into(),
	];
	expected.extend(
		MIGRATION_NAMES
			.iter()
			.map(|name| format!("[SUCCESS]   ✓ Faked: probe:{name}")),
	);
	expected.extend([
		"[INFO] ".into(),
		"[SUCCESS] Faked 3 migration(s) successfully".into(),
	]);
	assert_eq!(output, expected);
	assert_eq!(recorder.get_applied_migrations().await.unwrap().len(), 3);
	let mut editor = SchemaEditor::new(
		executor.connection().clone(),
		false,
		executor.connection().database_type(),
	)
	.await
	.expect("inspect fake schema");
	for index in 1..=MIGRATION_NAMES.len() {
		assert!(
			!editor
				.table_exists(&format!("migrate_reporting_{index}"))
				.await
				.expect("inspect migration table")
		);
	}
}

#[rstest]
#[case::fresh_database(false)]
#[case::fully_applied_history(true)]
#[tokio::test]
async fn plan_distinguishes_loaded_files_from_pending_work(
	#[future] migration_executor: MigrationExecutorFixture,
	migration_project: TempDir,
	#[case] already_applied: bool,
) {
	// Arrange
	let (mut executor, _container, _pool, _port, url) = migration_executor.await;
	if already_applied {
		let migrations = FilesystemSource::new(migration_project.path().join("migrations"))
			.all_migrations()
			.await
			.expect("load migration files");
		executor.apply_migrations(&migrations).await.unwrap();
	}

	// Act
	let output = run_migrate(migration_project.path(), &url, false, "plan");

	// Assert
	let mut expected = vec![
		"[INFO] Running migrations...".into(),
		"[INFO] Loaded 3 migration(s)".into(),
	];
	if already_applied {
		expected.push("[INFO] [plan] No unapplied migrations.".into());
	} else {
		expected.push("[INFO] [plan] Would apply 3 migration(s):".into());
		expected.extend(
			MIGRATION_NAMES
				.iter()
				.map(|name| format!("[INFO]   - probe:{name} (apply)")),
		);
	}
	assert_eq!(output, expected);
	let mut editor = SchemaEditor::new(
		executor.connection().clone(),
		false,
		executor.connection().database_type(),
	)
	.await
	.expect("inspect planned schema");
	assert_eq!(
		editor.table_exists("reinhardt_migrations").await.unwrap(),
		already_applied
	);
	for index in 1..=MIGRATION_NAMES.len() {
		assert_eq!(
			editor
				.table_exists(&format!("migrate_reporting_{index}"))
				.await
				.unwrap(),
			already_applied
		);
	}
}
