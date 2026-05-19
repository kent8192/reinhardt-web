//! E2E tests for Django-style migrate-with-target semantics.
//!
//! Covers the five scenarios listed in issue #4558:
//!
//! 1. `rollback_default_one_step_succeeds` — `migrate myapp <prev>` rolls back the
//!    most recent migration when the target is exactly one step earlier than head.
//! 2. `rollback_multi_step` — `migrate myapp <2-back>` rolls back two migrations
//!    in a single invocation.
//! 3. `rollback_dry_run_does_not_modify_db` — `migrate myapp <prev> --plan`
//!    previews the action and leaves the database untouched.
//! 4. `rollback_no_applied_migrations_exits_cleanly` — `migrate myapp zero`
//!    against an empty applied set succeeds with an informational message.
//! 5. `rollback_with_missing_reverse_sql_fails_loudly` — when a migration file
//!    referenced in the applied set is missing from disk, `migrate myapp <prev>`
//!    surfaces an error that identifies the offending `<app>.<name>`.
//!
//! These tests use the `migration_executor` fixture from `reinhardt-testkit`
//! (re-exported as `reinhardt_test::fixtures::migrations`) which spins up a
//! TestContainers PostgreSQL instance. The tests pre-populate the recorder via
//! `executor.record_migration(...)` — because the migration `.rs` files written
//! to the tempdir declare `operations: vec![]`, no real schema operations are
//! exercised; the focus is on the direction-detection + rollback orchestration
//! introduced by issue #4558.

#![cfg(feature = "testcontainers")]

use reinhardt_commands::{BaseCommand, CommandContext, MigrateCommand};
use reinhardt_db::backends::DatabaseConnection;
use reinhardt_db::migrations::DatabaseMigrationRecorder;
use reinhardt_test::fixtures::migrations::{MigrationExecutorFixture, migration_executor};
use rstest::rstest;
use serial_test::serial;
use std::path::Path;
use tempfile::TempDir;

/// Write a minimally valid migration `.rs` file under
/// `<root>/<app>/<name>.rs`. The Migration struct literal has `operations:
/// vec![]` so the file parses cleanly via `syn` but does not require any real
/// `Operation` types to be resolvable in the file's import graph (the
/// `FilesystemSource` only parses the AST — it does not compile the file).
fn write_test_migration(
	root: &Path,
	app: &str,
	name: &str,
	deps: &[(&str, &str)],
) -> std::io::Result<()> {
	let app_dir = root.join(app);
	std::fs::create_dir_all(&app_dir)?;
	let deps_literal = deps
		.iter()
		.map(|(a, n)| format!("(\"{}\".to_string(), \"{}\".to_string())", a, n))
		.collect::<Vec<_>>()
		.join(", ");
	let content = format!(
		"use reinhardt::db::migrations::prelude::*;\n\
		 pub(super) fn migration() -> Migration {{\n\
		 \tMigration {{\n\
		 \t\tapp_label: \"{app}\".to_string(),\n\
		 \t\tname: \"{name}\".to_string(),\n\
		 \t\toperations: vec![],\n\
		 \t\tdependencies: vec![{deps_literal}],\n\
		 \t\tatomic: true,\n\
		 \t\treplaces: vec![],\n\
		 \t\tinitial: None,\n\
		 \t\tstate_only: false,\n\
		 \t\tdatabase_only: false,\n\
		 \t\tswappable_dependencies: vec![],\n\
		 \t\toptional_dependencies: vec![],\n\
		 \t}}\n\
		 }}\n",
	);
	std::fs::write(app_dir.join(format!("{}.rs", name)), content)
}

/// Build a CommandContext targeting the given migrations directory and
/// database URL, with optional positional args and flags.
fn build_ctx(
	migrations_dir: &Path,
	database_url: &str,
	app_label: Option<&str>,
	target: Option<&str>,
	plan: bool,
) -> CommandContext {
	let mut ctx = CommandContext::default();
	if let Some(app) = app_label {
		ctx.add_arg(app.to_string());
		if let Some(t) = target {
			ctx.add_arg(t.to_string());
		}
	}
	ctx.set_option("database".to_string(), database_url.to_string());
	ctx.set_option(
		"migrations-dir".to_string(),
		migrations_dir.to_string_lossy().to_string(),
	);
	if plan {
		ctx.set_option("plan".to_string(), "true".to_string());
	}
	ctx
}

/// Set up a tempdir with three sequential migrations for `myapp` and mark them
/// all as applied in the recorder via `record_migration`. Returns the tempdir
/// (keep it alive for the duration of the test) and the migrations root path.
///
/// On a fresh TestContainers Postgres the `reinhardt_migrations` table does
/// not exist yet — `record_migration` does NOT auto-create it (it just issues
/// an INSERT against the table). We therefore construct a side recorder from
/// `executor.connection()` and call `ensure_schema_table()` once before
/// recording any applied migrations.
async fn arrange_three_applied_migrations(
	executor: &mut reinhardt_db::migrations::DatabaseMigrationExecutor,
) -> (TempDir, std::path::PathBuf) {
	let tempdir = tempfile::tempdir().expect("create tempdir");
	let migrations_root = tempdir.path().join("migrations");
	write_test_migration(&migrations_root, "myapp", "0001_first", &[]).unwrap();
	write_test_migration(
		&migrations_root,
		"myapp",
		"0002_second",
		&[("myapp", "0001_first")],
	)
	.unwrap();
	write_test_migration(
		&migrations_root,
		"myapp",
		"0003_third",
		&[("myapp", "0002_second")],
	)
	.unwrap();

	let recorder = DatabaseMigrationRecorder::new(executor.connection().clone());
	recorder
		.ensure_schema_table()
		.await
		.expect("ensure_schema_table on fresh Postgres");

	for name in ["0001_first", "0002_second", "0003_third"] {
		executor
			.record_migration("myapp", name)
			.await
			.expect("record_migration should succeed against a fresh TestContainers Postgres");
	}

	(tempdir, migrations_root)
}

/// Re-query the recorder for the current applied set. Builds a fresh
/// connection from `url` to avoid interfering with the fixture's executor.
/// Uses `connect_postgres` because the `migration_executor` fixture is
/// PostgreSQL-only (provided by TestContainers).
async fn applied_for_app(url: &str, app: &str) -> Vec<String> {
	let connection = DatabaseConnection::connect_postgres(url)
		.await
		.expect("re-connect to test Postgres");
	let recorder = DatabaseMigrationRecorder::new(connection.inner().clone());
	let applied = recorder
		.get_applied_migrations()
		.await
		.expect("query applied migrations");
	applied
		.into_iter()
		.filter(|r| r.app == app)
		.map(|r| r.name)
		.collect()
}

#[rstest]
#[tokio::test]
#[serial(migrate_target_e2e)]
async fn rollback_default_one_step_succeeds(
	#[future] migration_executor: MigrationExecutorFixture,
) {
	// Arrange — 3 migrations applied, target is the second-to-last one.
	let (mut executor, _container, _pool, _port, url) = migration_executor.await;
	let (_tempdir, migrations_root) = arrange_three_applied_migrations(&mut executor).await;

	// Act — `migrate myapp 0002_second` should unapply 0003_third.
	let ctx = build_ctx(
		&migrations_root,
		&url,
		Some("myapp"),
		Some("0002_second"),
		false,
	);
	MigrateCommand
		.execute(&ctx)
		.await
		.expect("rollback should succeed");

	// Assert — recorder should hold only 0001 and 0002.
	let remaining = applied_for_app(&url, "myapp").await;
	assert_eq!(remaining.len(), 2, "exactly two migrations should remain");
	assert!(
		remaining.iter().any(|n| n == "0001_first"),
		"0001_first must remain applied"
	);
	assert!(
		remaining.iter().any(|n| n == "0002_second"),
		"0002_second must remain applied"
	);
	assert!(
		!remaining.iter().any(|n| n == "0003_third"),
		"0003_third must have been rolled back"
	);
}

#[rstest]
#[tokio::test]
#[serial(migrate_target_e2e)]
async fn rollback_multi_step(#[future] migration_executor: MigrationExecutorFixture) {
	// Arrange — 3 migrations applied, target two steps back.
	let (mut executor, _container, _pool, _port, url) = migration_executor.await;
	let (_tempdir, migrations_root) = arrange_three_applied_migrations(&mut executor).await;

	// Act — `migrate myapp 0001_first` should unapply both 0002 and 0003.
	let ctx = build_ctx(
		&migrations_root,
		&url,
		Some("myapp"),
		Some("0001_first"),
		false,
	);
	MigrateCommand
		.execute(&ctx)
		.await
		.expect("multi-step rollback should succeed");

	// Assert — only 0001_first remains.
	let remaining = applied_for_app(&url, "myapp").await;
	assert_eq!(
		remaining,
		vec!["0001_first".to_string()],
		"only the target migration must remain"
	);
}

#[rstest]
#[tokio::test]
#[serial(migrate_target_e2e)]
async fn rollback_dry_run_does_not_modify_db(
	#[future] migration_executor: MigrationExecutorFixture,
) {
	// Arrange — 3 migrations applied; we'll preview a 2-step rollback.
	let (mut executor, _container, _pool, _port, url) = migration_executor.await;
	let (_tempdir, migrations_root) = arrange_three_applied_migrations(&mut executor).await;

	// Act — `migrate myapp 0001_first --plan` should print the plan but not
	// touch the database.
	let ctx = build_ctx(
		&migrations_root,
		&url,
		Some("myapp"),
		Some("0001_first"),
		true,
	);
	MigrateCommand
		.execute(&ctx)
		.await
		.expect("dry-run should succeed");

	// Assert — all three migrations remain applied.
	let remaining = applied_for_app(&url, "myapp").await;
	assert_eq!(
		remaining.len(),
		3,
		"--plan must not modify the recorder; got remaining = {:?}",
		remaining
	);
}

#[rstest]
#[tokio::test]
#[serial(migrate_target_e2e)]
async fn rollback_no_applied_migrations_exits_cleanly(
	#[future] migration_executor: MigrationExecutorFixture,
) {
	// Arrange — recorder is empty; only the on-disk migration files exist.
	let (_executor, _container, _pool, _port, url) = migration_executor.await;
	let tempdir = tempfile::tempdir().expect("create tempdir");
	let migrations_root = tempdir.path().join("migrations");
	write_test_migration(&migrations_root, "myapp", "0001_first", &[]).unwrap();

	// Act — `migrate myapp zero` against an empty applied set should report
	// "nothing to do" and return Ok(()).
	let ctx = build_ctx(&migrations_root, &url, Some("myapp"), Some("zero"), false);
	MigrateCommand
		.execute(&ctx)
		.await
		.expect("zero-rollback on empty state should be a no-op success");

	// Assert — recorder still has no applied migrations for myapp.
	let remaining = applied_for_app(&url, "myapp").await;
	assert!(
		remaining.is_empty(),
		"recorder must remain empty; got {:?}",
		remaining
	);
}

#[rstest]
#[tokio::test]
#[serial(migrate_target_e2e)]
async fn rollback_with_missing_reverse_sql_fails_loudly(
	#[future] migration_executor: MigrationExecutorFixture,
) {
	// Arrange — record three migrations as applied, but the `.rs` file for
	// 0003_third is intentionally absent from disk. The rollback path must
	// surface a clear error naming the missing migration when it tries to
	// load reverse-SQL for it via the FilesystemSource.
	let (mut executor, _container, _pool, _port, url) = migration_executor.await;
	let tempdir = tempfile::tempdir().expect("create tempdir");
	let migrations_root = tempdir.path().join("migrations");
	write_test_migration(&migrations_root, "myapp", "0001_first", &[]).unwrap();
	write_test_migration(
		&migrations_root,
		"myapp",
		"0002_second",
		&[("myapp", "0001_first")],
	)
	.unwrap();
	// Note: 0003_third is intentionally NOT written to disk.

	// `record_migration` does not auto-create `reinhardt_migrations`; ensure
	// the recorder table exists on the fresh container before recording.
	let recorder = DatabaseMigrationRecorder::new(executor.connection().clone());
	recorder.ensure_schema_table().await.unwrap();

	for name in ["0001_first", "0002_second", "0003_third"] {
		executor.record_migration("myapp", name).await.unwrap();
	}

	// Act — `migrate myapp 0002_second` will try to load 0003_third's
	// reverse-SQL but the file is missing.
	let ctx = build_ctx(
		&migrations_root,
		&url,
		Some("myapp"),
		Some("0002_second"),
		false,
	);
	let result = MigrateCommand.execute(&ctx).await;

	// Assert — the error message points at the missing migration.
	let err = result.expect_err("should fail when reverse SQL cannot be loaded");
	let msg = err.to_string();
	assert!(
		msg.contains("0003_third"),
		"error must name the offending migration; got: {}",
		msg
	);
}
