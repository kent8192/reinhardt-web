//! Cross-app rollback plans must agree with real and fake execution.

use reinhardt_commands::{BaseCommand, CommandContext, MigrateCommand};
use reinhardt_db::migrations::{DatabaseMigrationRecorder, FilesystemSource, MigrationSource};
use reinhardt_test::fixtures::migrations::{MigrationExecutorFixture, migration_executor};
use reinhardt_test::fixtures::temp_dir;
use rstest::{fixture, rstest};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn write_migration(
	root: &Path,
	app: &str,
	name: &str,
	dependencies: &[(&str, &str)],
	sql: &str,
	reverse_sql: &str,
) {
	let directory = root.join("migrations").join(app);
	std::fs::create_dir_all(&directory).expect("create migration directory");
	let dependencies = dependencies
		.iter()
		.map(|(app, name)| format!("({app:?}.to_string(), {name:?}.to_string())"))
		.collect::<Vec<_>>()
		.join(", ");
	std::fs::write(
		directory.join(format!("{name}.rs")),
		format!(
			r#"use reinhardt::db::migrations::prelude::*;
pub fn migration() -> Migration {{
    Migration {{
        app_label: {app:?}.to_string(),
        name: {name:?}.to_string(),
        operations: vec![Operation::RunSQL {{
            sql: {sql:?}.to_string(),
            reverse_sql: Some({reverse_sql:?}.to_string()),
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
"#
		),
	)
	.expect("write migration source");
}

#[fixture]
fn migration_project(temp_dir: TempDir) -> TempDir {
	write_migration(
		temp_dir.path(),
		"operations",
		"0000_environment",
		&[],
		"CREATE SCHEMA rollback_probe",
		"DROP SCHEMA rollback_probe",
	);
	write_migration(
		temp_dir.path(),
		"foundation",
		"0001_retained",
		&[("operations", "0000_environment")],
		"CREATE TABLE rollback_probe.retained (id integer PRIMARY KEY)",
		"DROP TABLE rollback_probe.retained",
	);
	write_migration(
		temp_dir.path(),
		"foundation",
		"0002_tables",
		&[("foundation", "0001_retained")],
		"CREATE TABLE rollback_probe.parent (id integer PRIMARY KEY)",
		"DROP TABLE rollback_probe.parent",
	);
	write_migration(
		temp_dir.path(),
		"consumer",
		"0001_references",
		&[("foundation", "0002_tables")],
		"CREATE TABLE rollback_probe.consumer (id integer PRIMARY KEY, parent_id integer REFERENCES rollback_probe.parent(id))",
		"DROP TABLE rollback_probe.consumer",
	);
	write_migration(
		temp_dir.path(),
		"reporting",
		"0001_leaf",
		&[("consumer", "0001_references")],
		"CREATE TABLE rollback_probe.leaf (id integer PRIMARY KEY, consumer_id integer REFERENCES rollback_probe.consumer(id))",
		"DROP TABLE rollback_probe.leaf",
	);
	write_migration(
		temp_dir.path(),
		"consumer",
		"0002_pending",
		&[("consumer", "0001_references")],
		"CREATE TABLE rollback_probe.pending (id integer PRIMARY KEY)",
		"DROP TABLE rollback_probe.pending",
	);
	write_migration(
		temp_dir.path(),
		"unrelated",
		"0002_tables",
		&[],
		"CREATE TABLE rollback_unrelated (id integer PRIMARY KEY)",
		"DROP TABLE rollback_unrelated",
	);
	temp_dir
}

fn run_migrate(project: &Path, url: &str, app: &str, target: &str, mode: &str) -> String {
	let output = Command::new(std::env::current_exe().expect("locate test executable"))
		.args([
			"--ignored",
			"--exact",
			"migrate_rollback_probe",
			"--nocapture",
		])
		.env("REINHARDT_ROLLBACK_DATABASE", url)
		.env("REINHARDT_ROLLBACK_DIRECTORY", project.join("migrations"))
		.env("REINHARDT_ROLLBACK_APP", app)
		.env("REINHARDT_ROLLBACK_TARGET", target)
		.env("REINHARDT_ROLLBACK_MODE", mode)
		.output()
		.expect("run native migration command");
	assert!(
		output.status.success(),
		"migration command failed:\n{}\n{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);
	String::from_utf8(output.stdout).expect("command output is UTF-8")
}

#[rstest]
#[tokio::test]
#[ignore = "invoked in a child process by the cross-app rollback tests"]
async fn migrate_rollback_probe() {
	// Arrange: environment values belong to this child process only.
	let mut ctx = CommandContext::default();
	ctx.set_option(
		"database".into(),
		std::env::var("REINHARDT_ROLLBACK_DATABASE").unwrap(),
	);
	ctx.set_option(
		"migrations-dir".into(),
		std::env::var("REINHARDT_ROLLBACK_DIRECTORY").unwrap(),
	);
	ctx.add_arg(std::env::var("REINHARDT_ROLLBACK_APP").unwrap());
	ctx.add_arg(std::env::var("REINHARDT_ROLLBACK_TARGET").unwrap());
	let mode = std::env::var("REINHARDT_ROLLBACK_MODE").unwrap();
	if !mode.is_empty() {
		ctx.set_option(mode, "true".into());
	}

	// Act / Assert
	MigrateCommand
		.execute(&ctx)
		.await
		.expect("native migration command succeeds");
}

async fn applied_keys(recorder: &DatabaseMigrationRecorder) -> BTreeSet<String> {
	recorder
		.get_applied_migrations()
		.await
		.expect("query applied migrations")
		.into_iter()
		.map(|record| format!("{}:{}", record.app, record.name))
		.collect()
}

#[rstest]
#[case::partial_target("foundation", "0001_retained", vec![
	"reporting:0001_leaf", "consumer:0001_references", "foundation:0002_tables",
])]
#[case::app_zero("foundation", "zero", vec![
	"reporting:0001_leaf", "consumer:0001_references", "foundation:0002_tables", "foundation:0001_retained",
])]
#[case::environment_zero("operations", "zero", vec![
	"reporting:0001_leaf", "consumer:0001_references", "foundation:0002_tables", "foundation:0001_retained", "operations:0000_environment",
])]
#[tokio::test]
async fn cross_app_rollback_matches_plan_and_replays_schema(
	#[future] migration_executor: MigrationExecutorFixture,
	migration_project: TempDir,
	#[case] app: &str,
	#[case] target: &str,
	#[case] expected: Vec<&str>,
	#[values("", "fake")] mode: &str,
) {
	// Arrange: a schema baseline with transitive foreign keys and an unapplied dependent.
	let (mut executor, _container, pool, _port, url) = migration_executor.await;
	let migrations = FilesystemSource::new(migration_project.path().join("migrations"))
		.all_migrations()
		.await
		.expect("load migration files");
	assert_eq!(migrations.len(), 7);
	assert!(
		migrations
			.iter()
			.all(|migration| migration.operations.len() == 1)
	);
	let baseline: Vec<_> = migrations
		.into_iter()
		.filter(|migration| migration.name != "0002_pending")
		.collect();
	let result = executor
		.apply_migrations(&baseline)
		.await
		.expect("apply foreign-key baseline");
	assert_eq!(result.applied.len(), 6);
	let recorder = DatabaseMigrationRecorder::new(executor.connection().clone());
	let before = applied_keys(&recorder).await;

	// Act: preview first, then execute the same target in real or fake mode.
	let plan = run_migrate(migration_project.path(), &url, app, target, "plan");
	let planned: Vec<_> = plan
		.lines()
		.filter_map(|line| line.strip_prefix("[INFO]   - ")?.strip_suffix(" (unapply)"))
		.collect();
	assert_eq!(planned, expected);
	assert_eq!(
		applied_keys(&recorder).await,
		before,
		"preview preserves the ledger"
	);
	let output = run_migrate(migration_project.path(), &url, app, target, mode);

	// Assert: all modes select the same keys in reverse dependency order.
	let prefix = if mode == "fake" {
		"[SUCCESS]   ✓ Faked rollback: "
	} else {
		"[SUCCESS]   ✓ Rolled back: "
	};
	let executed: Vec<_> = output
		.lines()
		.filter_map(|line| line.strip_prefix(prefix))
		.map(|key| key.replace('.', ":"))
		.collect();
	assert_eq!(executed, expected);
	let removed: BTreeSet<_> = expected.iter().map(|key| (*key).to_owned()).collect();
	let remaining: BTreeSet<_> = before.difference(&removed).cloned().collect();
	assert_eq!(applied_keys(&recorder).await, remaining);
	for (table, key) in [
		("rollback_probe.retained", "foundation:0001_retained"),
		("rollback_probe.parent", "foundation:0002_tables"),
		("rollback_probe.consumer", "consumer:0001_references"),
		("rollback_probe.leaf", "reporting:0001_leaf"),
		("rollback_unrelated", "unrelated:0002_tables"),
		("rollback_probe.pending", "consumer:0002_pending"),
	] {
		let exists: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
			.bind(table)
			.fetch_one(pool.as_ref())
			.await
			.expect("inspect rollback schema");
		assert_eq!(
			exists,
			before.contains(key) && (mode == "fake" || !removed.contains(key)),
			"table {table}"
		);
	}
	let schema_exists: bool =
		sqlx::query_scalar("SELECT to_regnamespace('rollback_probe') IS NOT NULL")
			.fetch_one(pool.as_ref())
			.await
			.expect("inspect environment schema");
	assert_eq!(schema_exists, mode == "fake" || app != "operations");

	if mode.is_empty() {
		let replay = executor
			.apply_migrations(&baseline)
			.await
			.expect("replay reversed baseline");
		assert_eq!(replay.applied.len(), expected.len());
		assert_eq!(applied_keys(&recorder).await, before);
		for table in [
			"rollback_probe.retained",
			"rollback_probe.parent",
			"rollback_probe.consumer",
			"rollback_probe.leaf",
			"rollback_unrelated",
		] {
			let exists: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
				.bind(table)
				.fetch_one(pool.as_ref())
				.await
				.expect("inspect replayed schema");
			assert!(exists, "replay restores {table}");
		}
	}
}
