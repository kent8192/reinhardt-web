//! Cross-app rollback plans must agree with real and fake execution.

use reinhardt_commands::{BaseCommand, CommandContext, MigrateCommand};
use reinhardt_db::migrations::{DatabaseMigrationRecorder, FilesystemSource, MigrationSource};
use reinhardt_query::{
	Expr, ExprTrait, IntoIden, PostgresQueryBuilder, Query, QueryStatementBuilder, SimpleExpr,
};
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
	write_migration_definition(root, app, name, dependencies, &[], (sql, reverse_sql));
}

fn write_migration_definition(
	root: &Path,
	app: &str,
	name: &str,
	dependencies: &[(&str, &str)],
	replaces: &[(&str, &str)],
	statements: (&str, &str),
) {
	let directory = root.join("migrations").join(app);
	std::fs::create_dir_all(&directory).expect("create migration directory");
	let format_keys = |keys: &[(&str, &str)]| {
		keys.iter()
			.map(|(app, name)| format!("({app:?}.to_string(), {name:?}.to_string())"))
			.collect::<Vec<_>>()
			.join(", ")
	};
	let dependencies = format_keys(dependencies);
	let replaces = format_keys(replaces);
	let (sql, reverse_sql) = statements;
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
        replaces: vec![{replaces}],
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

#[fixture]
fn squashed_migration_project(temp_dir: TempDir, #[default(false)] nested: bool) -> TempDir {
	let project = migration_project(temp_dir);
	let root = project.path();
	std::fs::remove_file(root.join("migrations/foundation/0002_tables.rs"))
		.expect("replace the original fixture migration");
	let statements = (
		"CREATE TABLE rollback_probe.parent (id integer PRIMARY KEY)",
		"DROP TABLE rollback_probe.parent",
	);
	write_migration_definition(
		root,
		"foundation",
		"0002_intermediate",
		&[("foundation", "0001_retained")],
		&[("foundation", "0002_tables")],
		statements,
	);
	write_migration_definition(
		root,
		"foundation",
		"0002_squashed",
		&[("foundation", "0001_retained")],
		&[(
			"foundation",
			if nested {
				"0002_intermediate"
			} else {
				"0002_tables"
			},
		)],
		statements,
	);
	write_migration_definition(
		root,
		"foundation",
		"0003_future_squash",
		&[("foundation", "0001_retained")],
		&[("foundation", "0002_tables")],
		statements,
	);
	project
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

async fn table_exists(pool: &sqlx::PgPool, table: &str) -> bool {
	let query = Query::select()
		.expr(
			SimpleExpr::FunctionCall(
				"to_regclass".into_iden(),
				vec![Expr::val(table).into_simple_expr()],
			)
			.is_not_null(),
		)
		.to_string(PostgresQueryBuilder);
	sqlx::query_scalar(&query)
		.fetch_one(pool)
		.await
		.expect("inspect migration table")
}

#[rstest]
#[tokio::test]
async fn unrelated_missing_squash_preserves_pending_dependency_boundaries(
	#[future] migration_executor: MigrationExecutorFixture,
	migration_project: TempDir,
	#[values("", "fake")] mode: &str,
) {
	// Arrange: an unrelated squash has missing ancestry, and reporting references a pending key.
	let root = migration_project.path();
	write_migration_definition(
		root,
		"archive",
		"0001_squashed",
		&[],
		&[("archive", "0002_pending")],
		(
			"CREATE TABLE rollback_archive (id integer PRIMARY KEY)",
			"DROP TABLE rollback_archive",
		),
	);
	write_migration(
		root,
		"reporting",
		"0001_leaf",
		&[("consumer", "0002_pending")],
		"CREATE TABLE rollback_report (id integer PRIMARY KEY)",
		"DROP TABLE rollback_report",
	);
	let (mut executor, _container, pool, _port, url) = migration_executor.await;
	let migrations = FilesystemSource::new(root.join("migrations"))
		.all_migrations()
		.await
		.expect("load migrations with unrelated missing ancestry");
	for (app, name) in [
		("operations", "0000_environment"),
		("foundation", "0001_retained"),
		("foundation", "0002_tables"),
		("consumer", "0001_references"),
		("reporting", "0001_leaf"),
		("unrelated", "0002_tables"),
		("archive", "0001_squashed"),
	] {
		let migration = migrations
			.iter()
			.find(|migration| migration.app_label == app && migration.name == name)
			.expect("baseline migration exists");
		let result = executor
			.apply_migrations(std::slice::from_ref(migration))
			.await
			.expect("apply baseline without the pending dependency");
		assert_eq!(result.applied.len(), 1);
	}
	let recorder = DatabaseMigrationRecorder::new(executor.connection().clone());
	let before = applied_keys(&recorder).await;
	assert_eq!(before.len(), 7);
	let expected = vec![
		"consumer:0001_references",
		"foundation:0002_tables",
		"foundation:0001_retained",
	];

	// Act
	let plan = run_migrate(root, &url, "foundation", "zero", "plan");
	let planned: Vec<_> = plan
		.lines()
		.filter_map(|line| line.strip_prefix("[INFO]   - ")?.strip_suffix(" (unapply)"))
		.collect();
	assert_eq!(planned, expected);
	assert_eq!(applied_keys(&recorder).await, before);
	let output = run_migrate(root, &url, "foundation", "zero", mode);

	// Assert: missing ancestry neither rejects rollback nor traverses the pending reporting edge.
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
	let removed: BTreeSet<_> = expected.into_iter().map(str::to_owned).collect();
	assert_eq!(
		applied_keys(&recorder).await,
		before.difference(&removed).cloned().collect()
	);
	for (table, expected) in [
		("rollback_archive", true),
		("rollback_report", true),
		("rollback_unrelated", true),
		("rollback_probe.retained", mode == "fake"),
		("rollback_probe.parent", mode == "fake"),
		("rollback_probe.consumer", mode == "fake"),
		("rollback_probe.pending", false),
	] {
		assert_eq!(
			table_exists(pool.as_ref(), table).await,
			expected,
			"{table}"
		);
	}
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

#[rstest]
#[case::missing_dependent("consumer", "0001_references")]
#[case::missing_root("foundation", "0002_tables")]
#[tokio::test]
async fn incomplete_cross_app_metadata_preserves_history_and_schema(
	#[future] migration_executor: MigrationExecutorFixture,
	migration_project: TempDir,
	#[case] missing_app: &str,
	#[case] missing_name: &str,
	#[values("missing", "invalid")] metadata: &str,
	#[values("", "fake", "plan")] mode: &str,
) {
	// Arrange: apply a complete baseline before losing an applied definition.
	let (mut executor, _container, pool, _port, url) = migration_executor.await;
	let migrations = FilesystemSource::new(migration_project.path().join("migrations"))
		.all_migrations()
		.await
		.expect("load complete migration files");
	assert_eq!(migrations.len(), 7);
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
	let before = recorder
		.get_applied_migrations()
		.await
		.expect("read baseline history");
	let definition = migration_project
		.path()
		.join("migrations")
		.join(missing_app)
		.join(format!("{missing_name}.rs"));
	if metadata == "invalid" {
		std::fs::write(definition, "pub fn migration( {")
			.expect("make the applied definition unparseable");
	} else {
		std::fs::remove_file(definition).expect("remove the applied definition");
	}
	let mut ctx = CommandContext::default();
	ctx.set_option("database".into(), url);
	ctx.set_option(
		"migrations-dir".into(),
		migration_project
			.path()
			.join("migrations")
			.to_string_lossy()
			.into_owned(),
	);
	ctx.add_arg("foundation".into());
	ctx.add_arg("zero".into());
	if !mode.is_empty() {
		ctx.set_option(mode.into(), "true".into());
	}

	// Act
	let error = MigrateCommand
		.execute(&ctx)
		.await
		.expect_err("incomplete metadata must fail before any rollback effects");

	// Assert: every mode preserves the complete ledger, timestamps, and schema.
	assert_eq!(
		error.to_string(),
		format!(
			"Execution error: Cannot determine cross-app rollback dependencies: applied migration {missing_app}:{missing_name} has no available definition"
		)
	);
	let after = recorder
		.get_applied_migrations()
		.await
		.expect("read unchanged history");
	assert_eq!(
		after
			.iter()
			.map(|record| (&record.app, &record.name, &record.applied))
			.collect::<Vec<_>>(),
		before
			.iter()
			.map(|record| (&record.app, &record.name, &record.applied))
			.collect::<Vec<_>>()
	);
	for (table, expected) in [
		("rollback_probe.retained", true),
		("rollback_probe.parent", true),
		("rollback_probe.consumer", true),
		("rollback_probe.leaf", true),
		("rollback_unrelated", true),
		("rollback_probe.pending", false),
	] {
		let exists: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
			.bind(table)
			.fetch_one(pool.as_ref())
			.await
			.expect("inspect unchanged schema");
		assert_eq!(exists, expected, "table {table}");
	}
}

#[rstest]
#[case::squashed_zero("zero", false)]
#[case::squashed_target("0001_retained", false)]
#[case::nested_squash_zero("zero", true)]
#[case::nested_squash_target("0001_retained", true)]
#[tokio::test]
async fn squashed_cross_app_rollback_matches_plan_and_replays_schema(
	#[future] migration_executor: MigrationExecutorFixture,
	temp_dir: TempDir,
	#[case] target: &str,
	#[case] nested: bool,
	#[values("", "fake")] mode: &str,
) {
	// Arrange: replace the parent migration while consumers retain its historic key.
	let project = squashed_migration_project(temp_dir, nested);
	let root = project.path();
	let (mut executor, _container, pool, _port, url) = migration_executor.await;
	let migrations = FilesystemSource::new(root.join("migrations"))
		.all_migrations()
		.await
		.expect("load squashed migration files");
	assert_eq!(migrations.len(), 9);
	let baseline_keys = [
		("operations", "0000_environment"),
		("foundation", "0001_retained"),
		("foundation", "0002_squashed"),
		("consumer", "0001_references"),
		("reporting", "0001_leaf"),
		("unrelated", "0002_tables"),
	];
	let baseline: Vec<_> = baseline_keys
		.iter()
		.map(|(app, name)| {
			migrations
				.iter()
				.find(|migration| migration.app_label == *app && migration.name == *name)
				.expect("baseline migration exists")
		})
		.collect();
	// Apply the recorded path in order without rewriting historic on-disk dependencies.
	for migration in &baseline {
		let result = executor
			.apply_migrations(std::slice::from_ref(*migration))
			.await
			.expect("apply squashed foreign-key baseline");
		assert_eq!(result.applied.len(), 1);
	}
	let recorder = DatabaseMigrationRecorder::new(executor.connection().clone());
	let before = applied_keys(&recorder).await;
	assert_eq!(before.len(), 6);
	let mut expected = vec![
		"reporting:0001_leaf",
		"consumer:0001_references",
		"foundation:0002_squashed",
	];
	if target == "zero" {
		expected.push("foundation:0001_retained");
	}

	// Act: compare preview with real or fake rollback of the applied squash.
	let plan = run_migrate(root, &url, "foundation", target, "plan");
	let planned: Vec<_> = plan
		.lines()
		.filter_map(|line| line.strip_prefix("[INFO]   - ")?.strip_suffix(" (unapply)"))
		.collect();
	assert_eq!(planned, expected);
	assert_eq!(applied_keys(&recorder).await, before);
	let output = run_migrate(root, &url, "foundation", target, mode);

	// Assert: aliases connect dependents without recording or executing pending squashes.
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
	assert_eq!(
		applied_keys(&recorder).await,
		before.difference(&removed).cloned().collect()
	);
	for (table, key) in [
		("rollback_probe.retained", "foundation:0001_retained"),
		("rollback_probe.parent", "foundation:0002_squashed"),
		("rollback_probe.consumer", "consumer:0001_references"),
		("rollback_probe.leaf", "reporting:0001_leaf"),
		("rollback_unrelated", "unrelated:0002_tables"),
		("rollback_probe.pending", "consumer:0002_pending"),
	] {
		let exists: bool = sqlx::query_scalar("SELECT to_regclass($1) IS NOT NULL")
			.bind(table)
			.fetch_one(pool.as_ref())
			.await
			.expect("inspect squashed rollback schema");
		assert_eq!(
			exists,
			before.contains(key) && (mode == "fake" || !removed.contains(key))
		);
	}
	if mode.is_empty() {
		let mut applied_count = 0;
		for migration in baseline {
			applied_count += executor
				.apply_migrations(std::slice::from_ref(migration))
				.await
				.expect("replay squashed baseline")
				.applied
				.len();
		}
		assert_eq!(applied_count, expected.len());
		assert_eq!(applied_keys(&recorder).await, before);
		let parent_exists: bool =
			sqlx::query_scalar("SELECT to_regclass('rollback_probe.parent') IS NOT NULL")
				.fetch_one(pool.as_ref())
				.await
				.expect("inspect replayed parent");
		assert!(parent_exists);
	}
}

#[rstest]
#[tokio::test]
async fn incomplete_nested_squash_preserves_history_and_schema(
	#[future] migration_executor: MigrationExecutorFixture,
	temp_dir: TempDir,
	#[values("missing", "invalid")] metadata: &str,
	#[values("zero", "0001_retained")] target: &str,
	#[values("", "fake", "plan")] mode: &str,
) {
	// Arrange: apply the final squash and consumers, then lose only intermediate metadata.
	let (mut executor, _container, pool, _port, url) = migration_executor.await;
	let project = squashed_migration_project(temp_dir, true);
	let root = project.path();
	let migrations = FilesystemSource::new(root.join("migrations"))
		.all_migrations()
		.await
		.expect("load complete nested squash metadata");
	assert_eq!(migrations.len(), 9);
	for (app, name) in [
		("operations", "0000_environment"),
		("foundation", "0001_retained"),
		("foundation", "0002_squashed"),
		("consumer", "0001_references"),
		("reporting", "0001_leaf"),
		("unrelated", "0002_tables"),
	] {
		let migration = migrations
			.iter()
			.find(|migration| migration.app_label == app && migration.name == name)
			.expect("recorded baseline definition exists");
		let result = executor
			.apply_migrations(std::slice::from_ref(migration))
			.await
			.expect("apply squashed foreign-key baseline");
		assert_eq!(result.applied.len(), 1);
	}
	let recorder = DatabaseMigrationRecorder::new(executor.connection().clone());
	let before = recorder
		.get_applied_migrations()
		.await
		.expect("read complete applied history");
	assert_eq!(before.len(), 6);
	let definition = root.join("migrations/foundation/0002_intermediate.rs");
	if metadata == "invalid" {
		std::fs::write(definition, "pub fn migration( {")
			.expect("make intermediate metadata unparseable");
	} else {
		std::fs::remove_file(definition).expect("remove intermediate metadata");
	}
	let mut ctx = CommandContext::default();
	ctx.set_option("database".into(), url);
	ctx.set_option(
		"migrations-dir".into(),
		root.join("migrations").to_string_lossy().into_owned(),
	);
	ctx.add_arg("foundation".into());
	ctx.add_arg(target.into());
	if !mode.is_empty() {
		ctx.set_option(mode.into(), "true".into());
	}

	// Act
	let error = MigrateCommand
		.execute(&ctx)
		.await
		.expect_err("unresolved nested aliases must fail before any rollback effects");

	// Assert: preview, fake, and real preserve all ledger rows, timestamps, and tables.
	assert_eq!(
		error.to_string(),
		"Execution error: Cannot determine cross-app rollback dependencies: applied migration consumer:0001_references references unresolved dependency foundation:0002_tables while replacement definition foundation:0002_intermediate is unavailable"
	);
	let after = recorder
		.get_applied_migrations()
		.await
		.expect("read unchanged applied history");
	assert_eq!(
		after
			.iter()
			.map(|record| (&record.app, &record.name, &record.applied))
			.collect::<Vec<_>>(),
		before
			.iter()
			.map(|record| (&record.app, &record.name, &record.applied))
			.collect::<Vec<_>>()
	);
	for (table, expected) in [
		("rollback_probe.retained", true),
		("rollback_probe.parent", true),
		("rollback_probe.consumer", true),
		("rollback_probe.leaf", true),
		("rollback_unrelated", true),
		("rollback_probe.pending", false),
	] {
		let query = Query::select()
			.expr(
				SimpleExpr::FunctionCall(
					"to_regclass".into_iden(),
					vec![Expr::val(table).into_simple_expr()],
				)
				.is_not_null(),
			)
			.to_string(PostgresQueryBuilder);
		let exists: bool = sqlx::query_scalar(&query)
			.fetch_one(pool.as_ref())
			.await
			.expect("inspect unchanged schema");
		assert_eq!(exists, expected, "table {table}");
	}
}
