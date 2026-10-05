use reinhardt_db::migrations::{FilesystemSource, Migration, MigrationSource, Operation, Result};
use rstest::{fixture, rstest};
use std::fmt::Write;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[fixture]
fn migration_directory() -> TempDir {
	let directory = TempDir::new().unwrap();
	fs::create_dir(directory.path().join("example")).unwrap();
	directory
}

fn builder_source(operation_count: usize, prefix: &str, suffix: &str) -> String {
	let mut source = format!(
		"// reinhardt-migration-source: 1\n\
		 pub fn migration() -> Migration {{\n\
		 Migration::new(\"ignored_name\", \"ignored_app\"){prefix}\n"
	);
	for index in 0..operation_count {
		writeln!(
			source,
			".add_operation(Operation::RunSQL {{ sql: \"SELECT {index}\".to_string(), reverse_sql: None }})"
		)
		.unwrap();
	}
	writeln!(source, "{suffix}\n}}").unwrap();
	source
}

fn load_on_normal_stack(root: &Path) -> Result<Vec<Migration>> {
	std::thread::scope(|scope| {
		std::thread::Builder::new()
			.name("migration-builder-parser".to_string())
			.stack_size(2 * 1024 * 1024)
			.spawn_scoped(scope, || {
				let runtime = tokio::runtime::Builder::new_current_thread()
					.enable_all()
					.build()
					.unwrap();
				runtime.block_on(FilesystemSource::new(root).all_migrations())
			})
			.unwrap()
			.join()
			.unwrap()
	})
}

#[rstest]
#[case(256)]
#[case(512)]
fn filesystem_source_loads_long_builder_chains_on_normal_stack(
	migration_directory: TempDir,
	#[case] operation_count: usize,
) {
	// Arrange
	let source = builder_source(
		operation_count,
		r#".atomic(true).initial(true).state_only(false).database_only(true)
			.add_dependency("first", "0001_initial")
			.add_replacement("old", "0001_initial")"#,
		r#".atomic(false).with_initial(None).state_only(true).database_only(false)
			.add_dependency("second", "0002_next")
			.add_replacement("old", "0002_next")"#,
	);
	fs::write(
		migration_directory.path().join("example/0001_initial.rs"),
		source,
	)
	.unwrap();

	// Act
	let migrations = load_on_normal_stack(migration_directory.path()).unwrap();

	// Assert
	assert_eq!(migrations.len(), 1);
	let migration = &migrations[0];
	assert_eq!(migration.app_label, "example");
	assert_eq!(migration.name, "0001_initial");
	let expected_operations: Vec<_> = (0..operation_count)
		.map(|index| Operation::RunSQL {
			sql: format!("SELECT {index}"),
			reverse_sql: None,
		})
		.collect();
	assert_eq!(migration.operations, expected_operations);
	assert_eq!(
		migration.dependencies,
		[
			("first".to_string(), "0001_initial".to_string()),
			("second".to_string(), "0002_next".to_string()),
		]
	);
	assert_eq!(
		migration.replaces,
		[
			("old".to_string(), "0001_initial".to_string()),
			("old".to_string(), "0002_next".to_string()),
		]
	);
	assert!(!migration.atomic);
	assert_eq!(migration.initial, None);
	assert!(migration.state_only);
	assert!(!migration.database_only);
}

#[rstest]
#[case::inherited("", false)]
#[case::matching(".atomic(false)", false)]
#[case::conflicting(".atomic(true)", true)]
fn long_builder_chains_preserve_standalone_atomic_validation(
	migration_directory: TempDir,
	#[case] prefix: &str,
	#[case] conflicting: bool,
) {
	// Arrange
	let mut source = builder_source(256, prefix, "");
	source.push_str("pub fn atomic() -> bool { false }\n");
	let path = migration_directory.path().join("example/0001_initial.rs");
	fs::write(&path, source).unwrap();

	// Act
	let result = load_on_normal_stack(migration_directory.path());

	// Assert
	if conflicting {
		assert_eq!(
			result.unwrap_err().to_string(),
			format!(
				"Invalid migration: Failed to parse {}: \
				 Migration builder atomic flag conflicts with atomic() entrypoint",
				path.display()
			)
		);
	} else {
		let migrations = result.unwrap();
		assert_eq!(migrations.len(), 1);
		assert_eq!(migrations[0].operations.len(), 256);
		assert!(!migrations[0].atomic);
	}
}

#[rstest]
#[case(
	".customize(true)",
	"Migration builder method 'customize' is unsupported or malformed",
	false
)]
#[case(
	".add_operation()",
	"Migration builder method 'add_operation' is unsupported or malformed",
	false
)]
#[case(
	".add_operation(Operation::RunSQL { sql: 42, reverse_sql: None })",
	"operations[256].RunSQL.sql is unsupported or malformed",
	true
)]
fn long_builder_chains_reject_invalid_calls(
	migration_directory: TempDir,
	#[case] suffix: &str,
	#[case] expected_message: &str,
	#[case] includes_source_coordinate: bool,
) {
	// Arrange
	let source = builder_source(256, "", suffix);
	let path = migration_directory.path().join("example/0001_initial.rs");
	fs::write(&path, source).unwrap();

	// Act
	let error = load_on_normal_stack(migration_directory.path()).unwrap_err();

	// Assert
	let coordinate = if includes_source_coordinate {
		format!("{}: ", path.display())
	} else {
		String::new()
	};
	assert_eq!(
		error.to_string(),
		format!(
			"Invalid migration: Failed to parse {}: {coordinate}{expected_message}",
			path.display()
		)
	);
}
