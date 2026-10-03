//! Installed application ownership at the makemigrations command boundary.

use reinhardt::{installed_apps, model, settings};
use reinhardt_auth::sessions::backends::database::Session;
use reinhardt_commands::{BaseCommand, CommandContext, MakeMigrationsCommand};
use reinhardt_db::migrations::autodetector::ForeignKeyInfo;
use reinhardt_db::migrations::model_registry::{FieldMetadata, ModelMetadata, global_registry};
use reinhardt_db::migrations::{
	ColumnDefinition, FieldType, FilesystemRepository, FilesystemSource, ForeignKeyAction,
	Migration, MigrationRepository, MigrationSource, Operation,
};
use reinhardt_db::orm::Model;
use rstest::{fixture, rstest};
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

installed_apps! {
	identity: "myproject.accounts",
	auth: "myproject.auth",
	provider: "myproject.providers",
}

mod first_registration {
	reinhardt::installed_apps! { first: "ambiguous.app" }
}

mod second_registration {
	reinhardt::installed_apps! { second: "ambiguous.app" }
}

mod label_collision_registration {
	reinhardt::installed_apps! { first: "identity" }
}

#[model(app_label = "identity", table_name = "own_identities")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Identity {
	#[field(primary_key = true)]
	id: i64,
}

#[settings(core: CoreSettings | contacts: ContactSettings)]
#[derive(Default)]
struct MigrationSettings;

struct ModelRegistryGuard {
	previous: Vec<ModelMetadata>,
}

impl ModelRegistryGuard {
	fn clear() -> Self {
		let previous = global_registry().get_models();
		global_registry().clear();
		Self { previous }
	}
}

impl Drop for ModelRegistryGuard {
	fn drop(&mut self) {
		global_registry().clear();
		for metadata in &self.previous {
			global_registry().register_model(metadata.clone());
		}
	}
}

struct ProjectDirGuard {
	previous: PathBuf,
}

impl ProjectDirGuard {
	fn enter(path: &Path) -> Self {
		let previous = std::env::current_dir().expect("current directory should be readable");
		std::env::set_current_dir(path).expect("temporary project should be enterable");
		Self { previous }
	}
}

impl Drop for ProjectDirGuard {
	fn drop(&mut self) {
		std::env::set_current_dir(&self.previous).expect("working directory should be restored");
	}
}

#[fixture]
fn project() -> TempDir {
	let project = TempDir::new_in("/tmp").expect("temporary project should be created");
	std::fs::create_dir_all(project.path().join("src/bin")).expect("src/bin should be created");
	std::fs::write(project.path().join("src/bin/manage.rs"), "fn main() {}\n")
		.expect("manage.rs should be written");
	project
}

fn context(project: &Path, installed_apps: Option<&[&str]>) -> CommandContext {
	let mut context = CommandContext::default();
	if let Some(apps) = installed_apps {
		let mut settings = MigrationSettings::default();
		settings.core.base_dir = project.to_path_buf();
		settings.core.installed_apps = apps.iter().map(|app| (*app).to_owned()).collect();
		context.settings = Some(Arc::new(settings));
	}
	context.set_option("force-empty-state".to_owned(), "true".to_owned());
	context.set_option(
		"migrations-dir".to_owned(),
		project.join("migrations").to_str().unwrap().to_owned(),
	);
	context
}

fn register_model(app: &str, table: &str, provider: Option<&str>) {
	let mut model = ModelMetadata::new(app, table, table);
	model.add_field(
		"id".to_owned(),
		FieldMetadata::new(FieldType::Integer).with_param("primary_key", "true"),
	);
	if let Some(provider) = provider {
		model.add_field(
			"provider_id".to_owned(),
			FieldMetadata::new(FieldType::Integer).with_foreign_key(ForeignKeyInfo {
				referenced_table: provider.to_owned(),
				referenced_column: "id".to_owned(),
				on_delete: ForeignKeyAction::Cascade,
				on_update: ForeignKeyAction::NoAction,
			}),
		);
	}
	global_registry().register_model(model);
}

async fn migration_apps(project: &Path) -> Vec<String> {
	let mut apps = FilesystemSource::new(project.join("migrations"))
		.all_migrations()
		.await
		.expect("generated migration files should be readable")
		.into_iter()
		.map(|migration| migration.app_label)
		.collect::<Vec<_>>();
	apps.sort();
	apps
}

#[rstest]
#[tokio::test]
#[serial(makemigrations_installed_apps)]
async fn linked_auth_and_session_models_do_not_own_project_migrations(project: TempDir) {
	// Arrange
	std::hint::black_box(Session::objects());
	std::hint::black_box(Identity::objects());
	let registry = ModelRegistryGuard::clear();
	for metadata in &registry.previous {
		global_registry().register_model(metadata.clone());
	}
	let registered = global_registry().get_models();
	for (app, table) in [
		("auth", "auth_group"),
		("auth", "auth_permission"),
		("default", "sessions"),
		("identity", "own_identities"),
	] {
		assert_eq!(
			registered
				.iter()
				.filter(|model| model.app_label == app && model.table_name == table)
				.count(),
			1,
			"linked model {app}/{table} should be registered",
		);
	}
	let _cwd = ProjectDirGuard::enter(project.path());
	let context = context(project.path(), Some(&["identity"]));

	// Act
	MakeMigrationsCommand
		.execute(&context)
		.await
		.expect("installed app migration should be generated");

	// Assert
	assert_eq!(migration_apps(project.path()).await, ["identity"]);
	let migration = FilesystemSource::new(project.path().join("migrations"))
		.get_migration("identity", "0001_initial")
		.await
		.expect("identity initial migration should exist");
	assert_eq!(migration.name, "0001_initial");
	let tables = migration
		.operations
		.iter()
		.filter_map(|operation| match operation {
			Operation::CreateTable { name, .. } => Some(name.as_str()),
			_ => None,
		})
		.collect::<Vec<_>>();
	assert_eq!(tables, ["own_identities"]);
	assert_eq!(global_registry().get_models().len(), registered.len());
	assert!(!project.path().join("migrations/auth").exists());
	assert!(!project.path().join("migrations/default").exists());
}

#[rstest]
#[case::configured(Some(&["identity"][..]), &["identity"])]
#[case::module_path(Some(&["myproject.accounts"][..]), &["identity"])]
#[case::auth_module_path(Some(&["myproject.auth"][..]), &["auth"])]
#[case::no_installed_models(Some(&["emptyapp"][..]), &[])]
#[case::no_settings(None, &["auth", "default", "identity"])]
#[case::default_settings(Some(&[][..]), &["auth", "default", "identity"])]
#[tokio::test]
#[serial(makemigrations_installed_apps)]
async fn default_selection_respects_configured_scope(
	project: TempDir,
	#[case] installed_apps: Option<&[&str]>,
	#[case] expected_apps: &[&str],
) {
	// Arrange
	let _registry = ModelRegistryGuard::clear();
	let _cwd = ProjectDirGuard::enter(project.path());
	register_model("identity", "own_identities", None);
	register_model("auth", "auth_group", None);
	register_model("default", "sessions", None);
	let context = context(project.path(), installed_apps);

	// Act
	MakeMigrationsCommand
		.execute(&context)
		.await
		.expect("app selection should succeed");

	// Assert
	assert_eq!(migration_apps(project.path()).await, expected_apps);
	assert_eq!(global_registry().get_models().len(), 3);
}

#[rstest]
#[tokio::test]
#[serial(makemigrations_installed_apps)]
async fn macro_app_paths_select_their_declared_labels(project: TempDir) {
	// Arrange
	let _registry = ModelRegistryGuard::clear();
	let _cwd = ProjectDirGuard::enter(project.path());
	register_model("identity", "own_identities", None);
	register_model("auth", "auth_group", None);
	register_model("provider", "provider_records", None);
	register_model("default", "sessions", None);
	let mut context = context(project.path(), Some(&[]));
	let mut settings = MigrationSettings::default();
	settings.core.installed_apps = InstalledApp::all_apps();
	context.settings = Some(Arc::new(settings));

	// Act
	MakeMigrationsCommand
		.execute(&context)
		.await
		.expect("macro paths should select the declared app labels");

	// Assert
	assert_eq!(
		migration_apps(project.path()).await,
		["auth", "identity", "provider"]
	);
	assert_eq!(global_registry().get_models().len(), 4);
	assert!(
		!project
			.path()
			.join("migrations/myproject.accounts")
			.exists()
	);
	assert!(!project.path().join("migrations/default").exists());
}

#[rstest]
#[tokio::test]
#[serial(makemigrations_installed_apps)]
async fn ambiguous_installed_app_path_is_rejected(project: TempDir) {
	// Arrange
	let _registry = ModelRegistryGuard::clear();
	let _cwd = ProjectDirGuard::enter(project.path());
	register_model("first", "first_records", None);
	register_model("second", "second_records", None);
	let context = context(project.path(), Some(&["ambiguous.app"]));

	// Act
	let error = MakeMigrationsCommand
		.execute(&context)
		.await
		.expect_err("conflicting path registrations must not choose an arbitrary label");

	// Assert
	assert_eq!(
		error.to_string(),
		"Execution error: Installed app path 'ambiguous.app' is registered with multiple app labels."
	);
	assert!(!project.path().join("migrations").exists());
}

#[rstest]
#[tokio::test]
#[serial(makemigrations_installed_apps)]
async fn direct_label_wins_over_another_apps_path(
	project: TempDir,
	#[values(false, true)] explicit: bool,
) {
	// Arrange
	let _registry = ModelRegistryGuard::clear();
	let _cwd = ProjectDirGuard::enter(project.path());
	register_model("identity", "own_identities", None);
	register_model("first", "first_records", None);
	let mut context = context(project.path(), Some(&["identity"]));
	if explicit {
		context.add_arg("identity".to_owned());
	}

	// Act
	MakeMigrationsCommand
		.execute(&context)
		.await
		.expect("an exact app label must not be reinterpreted as another app's path");

	// Assert
	assert_eq!(migration_apps(project.path()).await, ["identity"]);
	assert!(!project.path().join("migrations/first").exists());
}

#[rstest]
#[case::generate(None)]
#[case::empty(Some("empty"))]
#[case::merge(Some("merge"))]
#[tokio::test]
#[serial(makemigrations_installed_apps)]
async fn explicit_installed_label_accepts_configured_path(
	project: TempDir,
	#[case] mode: Option<&str>,
) {
	// Arrange
	let _registry = ModelRegistryGuard::clear();
	let _cwd = ProjectDirGuard::enter(project.path());
	register_model("identity", "own_identities", None);
	let mut context = context(project.path(), Some(&["myproject.accounts"]));
	context.add_arg("identity".to_owned());
	context.set_option("name".to_owned(), "scoped".to_owned());
	if let Some(mode) = mode {
		context.set_option(mode.to_owned(), "true".to_owned());
	}
	if mode == Some("merge") {
		let mut repository = FilesystemRepository::new(project.path().join("migrations"));
		for name in ["0001_initial", "0002_left", "0002_right"] {
			let mut migration = Migration::new(name, "identity");
			if name != "0001_initial" {
				migration.dependencies = vec![("identity".to_owned(), "0001_initial".to_owned())];
			}
			repository
				.save(&migration)
				.await
				.expect("history should be saved");
		}
	}

	// Act
	MakeMigrationsCommand
		.execute(&context)
		.await
		.expect("installed label should be accepted in every mode");

	// Assert
	let name = if mode == Some("merge") {
		"0003_scoped"
	} else {
		"0001_scoped"
	};
	let migration = FilesystemSource::new(project.path().join("migrations"))
		.get_migration("identity", name)
		.await
		.expect("migration should use the app label");
	assert_eq!(migration.app_label, "identity");
	assert_eq!(migration.name, name);
	assert_eq!(migration.operations.len(), usize::from(mode.is_none()));
	let mut dependencies = migration.dependencies;
	dependencies.sort();
	let expected_dependencies = if mode == Some("merge") {
		vec![
			("identity".to_owned(), "0002_left".to_owned()),
			("identity".to_owned(), "0002_right".to_owned()),
		]
	} else {
		Vec::new()
	};
	assert_eq!(dependencies, expected_dependencies);
	assert!(
		!project
			.path()
			.join("migrations/myproject.accounts")
			.exists()
	);
}

#[rstest]
#[case::generate(None)]
#[case::empty(Some("empty"))]
#[case::merge(Some("merge"))]
#[tokio::test]
#[serial(makemigrations_installed_apps)]
async fn explicit_uninstalled_app_is_rejected(project: TempDir, #[case] mode: Option<&str>) {
	// Arrange
	let _registry = ModelRegistryGuard::clear();
	let _cwd = ProjectDirGuard::enter(project.path());
	register_model("auth", "auth_group", None);
	let mut context = context(project.path(), Some(&["identity"]));
	context.add_arg("auth".to_owned());
	if let Some(mode) = mode {
		context.set_option(mode.to_owned(), "true".to_owned());
	}

	// Act
	let error = MakeMigrationsCommand
		.execute(&context)
		.await
		.expect_err("uninstalled app should be rejected");

	// Assert
	assert_eq!(
		error.to_string(),
		"Execution error: App 'auth' is not in CoreSettings::installed_apps."
	);
	assert!(!project.path().join("migrations").exists());
}

#[rstest]
#[case::installed(&["identity", "provider"], &["identity", "provider"], true)]
#[case::uninstalled(&["identity"], &[], false)]
#[case::installed_paths(&["myproject.accounts", "myproject.providers"], &["identity", "provider"], true)]
#[case::uninstalled_path(&["myproject.accounts"], &[], false)]
#[tokio::test]
#[serial(makemigrations_installed_apps)]
async fn foreign_key_providers_stay_within_installed_scope(
	project: TempDir,
	#[case] installed_apps: &[&str],
	#[case] expected_apps: &[&str],
	#[case] has_provider_dependency: bool,
) {
	// Arrange
	let _registry = ModelRegistryGuard::clear();
	let _cwd = ProjectDirGuard::enter(project.path());
	register_model("provider", "provider_records", None);
	register_model("identity", "own_identities", Some("provider_records"));
	let mut context = context(project.path(), Some(installed_apps));
	context.add_arg("identity".to_owned());

	// Act
	let result = MakeMigrationsCommand.execute(&context).await;

	// Assert
	if !has_provider_dependency {
		let error = result.expect_err("an uninstalled foreign key provider must be rejected");
		assert_eq!(
			error.to_string(),
			"Execution error: App 'identity' references uninstalled app 'provider'. Add 'provider' to CoreSettings::installed_apps before generating migrations."
		);
		assert_eq!(migration_apps(project.path()).await, expected_apps);
		assert!(!project.path().join("migrations").exists());
		assert_eq!(global_registry().get_models().len(), 2);
		return;
	}
	result.expect("installed foreign key migrations should be generated");
	assert_eq!(migration_apps(project.path()).await, expected_apps);
	let migration = FilesystemSource::new(project.path().join("migrations"))
		.get_migration("identity", "0001_initial")
		.await
		.expect("identity migration should be readable");
	let dependencies = if has_provider_dependency {
		vec![("provider".to_owned(), "0001_initial".to_owned())]
	} else {
		Vec::new()
	};
	assert_eq!(migration.dependencies, dependencies);
}

#[rstest]
#[tokio::test]
#[serial(makemigrations_installed_apps)]
async fn previous_app_history_is_preserved_for_model_moves(
	project: TempDir,
	#[values(false, true)] rename_table: bool,
	#[values(false, true)] explicit: bool,
) {
	// Arrange
	let _registry = ModelRegistryGuard::clear();
	let _cwd = ProjectDirGuard::enter(project.path());
	let mut previous = ModelMetadata::new("legacy", "Deployment", "legacy_deployment");
	previous.add_field(
		"id".to_owned(),
		FieldMetadata::new(FieldType::Integer).with_param("primary_key", "true"),
	);
	for field in ["created_at", "updated_at"] {
		previous.add_field(field.to_owned(), FieldMetadata::new(FieldType::DateTime));
	}
	previous.add_field(
		"status".to_owned(),
		FieldMetadata::new(FieldType::VarChar(32)),
	);
	let previous_state = previous.to_model_state();
	let mut initial = Migration::new("0001_initial", "legacy");
	initial.operations.push(Operation::CreateTable {
		name: previous_state.table_name.clone(),
		columns: previous_state
			.fields
			.iter()
			.map(|(name, field)| ColumnDefinition::from_field_state(name, field))
			.collect(),
		constraints: Vec::new(),
		without_rowid: None,
		interleave_in_parent: None,
		partition: None,
	});
	let mut repository = FilesystemRepository::new(project.path().join("migrations"));
	repository
		.save(&initial)
		.await
		.expect("previous app history should be saved");
	let mut current = previous;
	current.app_label = "deployments".to_owned();
	current.model_name = "Project".to_owned();
	if rename_table {
		current.table_name = "deployments_project".to_owned();
	}
	current.add_field(
		"project_name".to_owned(),
		FieldMetadata::new(FieldType::VarChar(255)).with_nullable(true),
	);
	global_registry().register_model(current);
	let mut context = context(project.path(), Some(&["deployments"]));
	context.options.remove("force-empty-state");
	// Avoid relying on a configured database if container replay needs a fallback.
	context.set_option("database".to_owned(), "invalid://offline".to_owned());
	if explicit {
		context.add_arg("deployments".to_owned());
	}

	// Act
	MakeMigrationsCommand
		.execute(&context)
		.await
		.expect("a model move must retain the old app's history");
	let migrations = FilesystemSource::new(project.path().join("migrations"))
		.all_migrations()
		.await
		.expect("generated migrations should be readable");
	MakeMigrationsCommand
		.execute(&context)
		.await
		.expect("saved move migrations must replay without recreating an existing table");

	// Assert
	assert_eq!(
		migration_apps(project.path()).await,
		["deployments", "legacy"]
	);
	assert_eq!(migrations.len(), 2);
	assert_eq!(
		migrations
			.iter()
			.filter(|migration| migration.app_label == "legacy")
			.count(),
		1
	);
	let migration = migrations
		.iter()
		.find(|migration| migration.app_label == "deployments")
		.expect("only the new app should receive a migration");
	assert_eq!(
		migration.dependencies,
		[("legacy".to_owned(), "0001_initial".to_owned())]
	);
	assert_eq!(
		migration.operations.len(),
		2,
		"unexpected operations: {:?}",
		migration.operations
	);
	let move_index = migration
		.operations
		.iter()
		.position(|operation| matches!(operation, Operation::MoveModel { .. }))
		.expect("a cross-app move must remain in the saved migration");
	let column_index = migration
		.operations
		.iter()
		.position(|operation| matches!(operation, Operation::AddColumn { .. }))
		.expect("the moved model's added field must remain in the saved migration");
	assert!(
		matches!(
			&migration.operations[move_index],
			Operation::MoveModel { from_app, to_app, rename_table: actual_rename, old_table_name, new_table_name, .. }
				if from_app == "legacy" && to_app == "deployments"
					&& *actual_rename == rename_table
					&& old_table_name.as_deref() == rename_table.then_some("legacy_deployment")
					&& new_table_name.as_deref() == rename_table.then_some("deployments_project")
		),
		"unexpected move: {:?}",
		migration.operations[move_index]
	);
	assert!(matches!(
		&migration.operations[column_index], Operation::AddColumn { column, .. } if column.name == "project_name"
	));
	if rename_table {
		assert!(
			move_index < column_index,
			"the table must be renamed before its new column is added"
		);
	}
}

#[rstest]
#[case::generate(false)]
#[case::merge(true)]
#[tokio::test]
#[serial(makemigrations_installed_apps)]
async fn uninstalled_migration_conflicts_are_ignored(
	project: TempDir,
	#[case] merge: bool,
	#[values("identity", "myproject.accounts")] installed_app: &str,
) {
	// Arrange
	let _registry = ModelRegistryGuard::clear();
	let _cwd = ProjectDirGuard::enter(project.path());
	register_model("identity", "own_identities", None);
	let migrations_dir = project.path().join("migrations");
	let mut repository = FilesystemRepository::new(&migrations_dir);
	for name in ["0001_initial", "0002_left", "0002_right"] {
		let mut migration = Migration::new(name, "auth");
		if name != "0001_initial" {
			migration.dependencies = vec![("auth".to_owned(), "0001_initial".to_owned())];
		}
		repository
			.save(&migration)
			.await
			.expect("history fixture should be saved");
	}
	let mut context = context(project.path(), Some(&[installed_app]));
	if merge {
		context.set_option("merge".to_owned(), "true".to_owned());
	}

	// Act
	MakeMigrationsCommand
		.execute(&context)
		.await
		.expect("uninstalled conflicts should not participate");

	// Assert
	let mut names = FilesystemSource::new(&migrations_dir)
		.all_migrations()
		.await
		.expect("migration history should be readable")
		.into_iter()
		.map(|migration| (migration.app_label, migration.name))
		.collect::<Vec<_>>();
	names.sort();
	let mut expected = vec![
		("auth".to_owned(), "0001_initial".to_owned()),
		("auth".to_owned(), "0002_left".to_owned()),
		("auth".to_owned(), "0002_right".to_owned()),
	];
	if !merge {
		expected.push(("identity".to_owned(), "0001_initial".to_owned()));
	}
	assert_eq!(names, expected);
}
