use reinhardt_admin_cli::migration_source::{UpgradeSourceArgs, run};
use reinhardt_db::migrations::{FilesystemSource, Migration, MigrationSource, Operation};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

#[derive(Deserialize)]
struct SourceFile {
	relative_path: String,
	fixture_path: String,
	sha256: String,
}

#[derive(Deserialize)]
struct Manifest {
	files: Vec<SourceFile>,
}

fn workspace_root() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.ancestors()
		.nth(2)
		.unwrap()
		.to_path_buf()
}

fn fixture_manifest() -> Vec<SourceFile> {
	serde_json::from_str::<Manifest>(include_str!(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/tests/fixtures/migration_source/manifest.json"
	)))
	.unwrap()
	.files
}

fn read_original(file: &SourceFile) -> Vec<u8> {
	let bytes = fs::read(workspace_root().join(&file.fixture_path)).unwrap();
	assert_eq!(format!("{:x}", Sha256::digest(&bytes)), file.sha256);
	bytes
}

fn copy_originals(files: &[SourceFile]) -> TempDir {
	let directory = TempDir::new().unwrap();
	for file in files {
		let path = directory.path().join(&file.relative_path);
		fs::create_dir_all(path.parent().unwrap()).unwrap();
		fs::write(path, read_original(file)).unwrap();
	}
	directory
}

fn expected_migrations() -> BTreeMap<String, Migration> {
	// Deserialization supplies the current optional-field defaults only.
	// All existing values come from direct execution of the original sources.
	serde_json::from_str(include_str!(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/tests/fixtures/migration_source/expected.json"
	)))
	.unwrap()
}

fn compile_migrations(root: &Path, files: &[SourceFile]) -> BTreeMap<String, Migration> {
	let relative_paths: Vec<_> = files
		.iter()
		.map(|file| file.relative_path.as_str())
		.collect();
	compile_sources(root, &relative_paths)
}

fn compile_sources(root: &Path, relative_paths: &[&str]) -> BTreeMap<String, Migration> {
	let project = TempDir::new().unwrap();
	fs::create_dir(project.path().join("src")).unwrap();
	let mut modules = String::new();
	let mut entries = Vec::new();
	for (index, relative_path) in relative_paths.iter().enumerate() {
		let path = root.join(relative_path);
		modules.push_str(&format!("#[path = {path:?}]\nmod migration_{index};\n"));
		entries.push(format!(
			"({:?}, migration_{index}::migration())",
			relative_path
		));
	}
	let main = format!(
		"{modules}\nfn main() {{\nlet migrations = std::collections::BTreeMap::from([{}]);\
		 \nprintln!(\"{{}}\", serde_json::to_string(&migrations).unwrap());\n}}\n",
		entries.join(",\n")
	);
	fs::write(project.path().join("src/main.rs"), main).unwrap();
	fs::write(
		project.path().join("Cargo.toml"),
		format!(
			"[package]\nname=\"upgraded-migration-check\"\nversion=\"0.0.0\"\nedition=\"2024\"\n\
			 [dependencies]\nreinhardt={{package=\"reinhardt-web\",path={:?},\
			 default-features=false,features=[\"database\"]}}\nserde_json=\"1\"\n",
			workspace_root()
		),
	)
	.unwrap();
	let output = Command::new(env!("CARGO"))
		.args(["run", "--quiet"])
		.current_dir(project.path())
		.env("CARGO_TARGET_DIR", project.path().join("target"))
		.output()
		.unwrap();
	assert_eq!(
		output.status.success(),
		true,
		"public-facade compilation/execution failed:\n{}",
		String::from_utf8_lossy(&output.stderr)
	);
	serde_json::from_slice(&output.stdout).unwrap()
}

#[rstest::rstest]
#[tokio::test]
async fn sql_assets_upgrade_compiles_with_complete_public_facade_parity() {
	// Arrange
	let directory = TempDir::new_in("/tmp").unwrap();
	let relative_path = "assets/0001_guard.rs";
	let app = directory.path().join("assets");
	fs::create_dir_all(app.join("sql")).unwrap();
	let forward = "-- exact 🦀\r\nDO $guard$ BEGIN PERFORM 1; END $guard$;\r\n";
	let reverse = "SELECT 'reverse; exact';\n";
	fs::write(app.join("sql/forward.txt"), forward).unwrap();
	fs::write(app.join("sql/reverse.sql"), reverse).unwrap();
	let text = r##"use reinhardt::db::migrations::prelude::*;
use reinhardt::db::migrations::{SwappableDependency, OptionalDependency, DependencyCondition};
pub fn migration() -> Migration {
    Migration {
        app_label: "assets".into(), name: ("0001_guard".into()),
        operations: vec![Operation::RunSQL {
            sql: (include_str!("sql/forward.txt")).to_owned().into(),
            reverse_sql: Some(include_str!(r#"sql/reverse.sql"#,).into()),
        }],
        dependencies: vec![("base".into(), "0001_initial".into::<>() )],
        replaces: vec![("assets".to_owned().into(), "0000_old".into())],
        atomic: false, initial: Some(false), state_only: false, database_only: true,
        swappable_dependencies: vec![SwappableDependency::new("AUTH_USER_MODEL", "auth", "User", "0001_initial")],
        optional_dependencies: vec![OptionalDependency::new("plugin", "0001_initial", DependencyCondition::AppInstalled("plugin".into()))],
    }
}
"##;
	fs::write(directory.path().join(relative_path), text).unwrap();
	// Act
	run(UpgradeSourceArgs {
		path: directory.path().into(),
		check: false,
	})
	.unwrap();
	run(UpgradeSourceArgs {
		path: directory.path().into(),
		check: true,
	})
	.unwrap();
	let compiled = compile_sources(directory.path(), &[relative_path]);
	let loaded = FilesystemSource::new(directory.path())
		.all_migrations()
		.await
		.unwrap();
	// Assert
	assert_eq!(
		serde_json::to_value(&loaded[0]).unwrap(),
		serde_json::to_value(compiled.get(relative_path).unwrap()).unwrap()
	);
	assert_eq!(
		loaded[0].operations,
		vec![Operation::RunSQL {
			sql: forward.into(),
			reverse_sql: Some(reverse.into())
		}]
	);
	let upgraded = fs::read_to_string(directory.path().join(relative_path)).unwrap();
	assert!(upgraded.starts_with("// reinhardt-migration-source: 1\n"));
	assert_eq!(upgraded.matches("include_str").count(), 2);
	assert!(upgraded.contains("sql/forward.txt") && upgraded.contains("sql/reverse.sql"));
	assert!(!upgraded.contains(forward) && !upgraded.contains(reverse));
	assert_eq!(
		fs::read(app.join("sql/forward.txt")).unwrap(),
		forward.as_bytes()
	);
	assert_eq!(
		fs::read(app.join("sql/reverse.sql")).unwrap(),
		reverse.as_bytes()
	);
}

#[rstest::rstest]
fn legacy_drop_column_into_upgrade_compiles_through_public_facade() {
	// Arrange
	let directory = TempDir::new().unwrap();
	let relative_path = "legacy/0002_remove_owner.rs";
	let path = directory.path().join(relative_path);
	fs::create_dir_all(path.parent().unwrap()).unwrap();
	fs::write(
		&path,
		r#"use reinhardt::db::migrations::prelude::*;

// Preserve the migration's explanation and surrounding layout.
pub fn migration() -> Migration {
    Migration::new("0002_remove_owner", "legacy").add_operation(Operation::DropColumn {
        table: "items".into(),
        column: "owner_id".into(),
    })
}
// Keep unrelated application text byte-for-byte.
"#,
	)
	.unwrap();

	// Act
	run(UpgradeSourceArgs {
		path: directory.path().into(),
		check: false,
	})
	.unwrap();

	// Assert
	let upgraded = fs::read_to_string(&path).unwrap();
	assert_eq!(
		upgraded,
		r#"// reinhardt-migration-source: 1
use reinhardt::db::migrations::prelude::*;

// Preserve the migration's explanation and surrounding layout.
pub fn migration() -> Migration {
    Migration::new("0002_remove_owner", "legacy").add_operation(Operation :: DropColumn { table : "items" . to_string () , column : "owner_id" . to_string () , old_definition : None })
}
// Keep unrelated application text byte-for-byte.
"#
	);
	let compiled = compile_sources(directory.path(), &[relative_path]);
	assert_eq!(
		compiled.get(relative_path).unwrap().operations,
		vec![Operation::DropColumn {
			table: "items".into(),
			column: "owner_id".into(),
			old_definition: None,
		}]
	);
}

#[rstest::rstest]
#[tokio::test]
async fn historical_source_upgrade_compiles_and_preserves_semantics() {
	// Arrange
	let files = fixture_manifest();
	assert_eq!(files.len(), 27);
	assert_eq!(
		files
			.iter()
			.filter(|file| file.relative_path.starts_with("cloud/"))
			.count(),
		23
	);
	let copies = copy_originals(&files);
	let before: Vec<_> = files.iter().map(read_original).collect();

	// Act
	let error = run(UpgradeSourceArgs {
		path: copies.path().into(),
		check: true,
	})
	.unwrap_err();

	// Assert
	assert_eq!(
		error.to_string(),
		reinhardt_admin_cli::migration_source::MigrationSourceError::Preflight(
			"27 migration source file(s) require upgrade".into()
		)
		.to_string()
	);
	for (file, original) in files.iter().zip(&before) {
		assert_eq!(
			fs::read(copies.path().join(&file.relative_path)).unwrap(),
			*original
		);
	}

	// Act
	run(UpgradeSourceArgs {
		path: copies.path().into(),
		check: false,
	})
	.unwrap();
	let upgraded: Vec<_> = files
		.iter()
		.map(|file| fs::read(copies.path().join(&file.relative_path)).unwrap())
		.collect();
	for check in [true, false] {
		run(UpgradeSourceArgs {
			path: copies.path().into(),
			check,
		})
		.unwrap();
		for (file, expected) in files.iter().zip(&upgraded) {
			assert_eq!(
				fs::read(copies.path().join(&file.relative_path)).unwrap(),
				*expected
			);
		}
	}

	// Assert
	let compiled = compile_migrations(copies.path(), &files);
	let expected = expected_migrations();
	assert_eq!(
		serde_json::to_value(&compiled).unwrap(),
		serde_json::to_value(&expected).unwrap()
	);
	for group in ["cloud", "twitter"] {
		let loaded = FilesystemSource::new(copies.path().join(group))
			.all_migrations()
			.await
			.unwrap();
		let loaded: BTreeMap<_, _> = loaded
			.into_iter()
			.map(|migration| {
				(
					format!("{group}/{}/{}.rs", migration.app_label, migration.name),
					migration,
				)
			})
			.collect();
		let expected_group: BTreeMap<_, _> = expected
			.iter()
			.filter(|(path, _)| path.starts_with(&format!("{group}/")))
			.collect();
		assert_eq!(
			serde_json::to_value(loaded).unwrap(),
			serde_json::to_value(expected_group).unwrap()
		);
	}
	assert_eq!(files.iter().map(read_original).collect::<Vec<_>>(), before);
}

#[cfg(feature = "postgres")]
use reinhardt_test::fixtures::postgres_container;

#[cfg(feature = "postgres")]
#[rstest::rstest]
#[tokio::test]
async fn cloud_source_upgrade_applies_complete_graph(
	#[future] postgres_container: (
		testcontainers::ContainerAsync<testcontainers::GenericImage>,
		std::sync::Arc<sqlx::PgPool>,
		u16,
		String,
	),
) {
	use reinhardt_db::backends::DatabaseConnection;
	use reinhardt_db::migrations::{
		DatabaseMigrationExecutor, DatabaseMigrationRecorder, MigrationGraph, MigrationKey,
	};

	// Arrange
	let (_container, _pool, _port, url) = postgres_container.await;
	let files = fixture_manifest();
	let copies = copy_originals(&files);
	run(UpgradeSourceArgs {
		path: copies.path().into(),
		check: false,
	})
	.unwrap();
	let migrations = FilesystemSource::new(copies.path().join("cloud"))
		.all_migrations()
		.await
		.unwrap();
	assert_eq!(migrations.len(), 23);
	let mut graph = MigrationGraph::new();
	for migration in &migrations {
		assert_eq!(migration.replaces.is_empty(), true);
		assert_eq!(migration.swappable_dependencies.is_empty(), true);
		assert_eq!(migration.optional_dependencies.is_empty(), true);
		graph.add_migration(
			MigrationKey::new(&migration.app_label, &migration.name),
			migration
				.dependencies
				.iter()
				.map(|(app, name)| MigrationKey::new(app, name))
				.collect(),
		);
	}
	let ordered: Vec<_> = graph
		.topological_sort()
		.unwrap()
		.iter()
		.map(|key| {
			migrations
				.iter()
				.find(|migration| {
					migration.app_label == key.app_label && migration.name == key.name
				})
				.unwrap()
				.clone()
		})
		.collect();
	let connection = DatabaseConnection::connect_postgres(&url).await.unwrap();
	let recorder = DatabaseMigrationRecorder::new(connection.clone());
	let mut executor = DatabaseMigrationExecutor::new(connection);

	// Act
	let result = executor.apply_migrations(&ordered).await.unwrap();

	// Assert
	assert_eq!(result.failed, None);
	assert_eq!(result.applied.len(), 23);
	let mut recorded: Vec<_> = recorder
		.get_applied_migrations()
		.await
		.unwrap()
		.into_iter()
		.map(|record| (record.app, record.name))
		.collect();
	recorded.sort();
	let mut expected: Vec<_> = expected_migrations()
		.into_iter()
		.filter(|(path, _)| path.starts_with("cloud/"))
		.map(|(_, migration)| (migration.app_label, migration.name))
		.collect();
	expected.sort();
	assert_eq!(recorded, expected);
	assert_eq!(
		executor
			.apply_migrations(&ordered)
			.await
			.unwrap()
			.applied
			.len(),
		0
	);
	for file in &files {
		read_original(file);
	}
}
