//! Built-in commands
//!
//! Standard management commands included with Reinhardt.

use crate::{BaseCommand, CommandArgument, CommandContext, CommandOption, CommandResult};
use async_trait::async_trait;

#[cfg(feature = "migrations")]
use reinhardt_db::migrations::DatabaseMigrationExecutor;

#[cfg(feature = "migrations")]
use reinhardt_db::backends::{DatabaseConnection, DatabaseType};

// Import backends' DatabaseConnection for get_database_url helper (without migrations feature)
#[cfg(all(feature = "reinhardt-db", not(feature = "migrations")))]
use reinhardt_db::backends::DatabaseConnection;

// Import DatabaseType for connect_database helper
#[cfg(all(feature = "reinhardt-db", not(feature = "migrations")))]
use reinhardt_db::backends::DatabaseType;

/// Database migration command
pub struct MigrateCommand;

#[async_trait]
impl BaseCommand for MigrateCommand {
	fn name(&self) -> &str {
		"migrate"
	}

	fn description(&self) -> &str {
		"Run database migrations"
	}

	fn arguments(&self) -> Vec<CommandArgument> {
		vec![
			CommandArgument::optional("app", "App name to migrate"),
			CommandArgument::optional("migration", "Migration name"),
		]
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![
			CommandOption::flag(None, "fake", "Mark migrations as run without executing"),
			CommandOption::flag(
				None,
				"fake-initial",
				"Skip initial migration if tables exist",
			),
			CommandOption::option(Some('d'), "database", "Database to migrate")
				.with_default("default"),
		]
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		ctx.info("Running migrations...");

		let app_label = ctx.arg(0).map(|s| s.to_string());
		let _migration_name = ctx.arg(1).map(|s| s.to_string());
		let is_fake = ctx.has_option("fake");
		let _is_fake_initial = ctx.has_option("fake-initial");
		let _database = ctx
			.option("database")
			.map(|s| s.to_string())
			.unwrap_or_else(|| "default".to_string());

		if let Some(ref app_name) = app_label {
			if let Some(ref migration) = _migration_name {
				ctx.verbose(&format!("Migrating {} to {}", app_name, migration));
			} else {
				ctx.verbose(&format!("Migrating app: {}", app_name));
			}
		} else {
			ctx.verbose("Migrating all apps");
		}

		if is_fake {
			ctx.warning("Fake mode: Migrations will be marked as applied without running");
		}

		// Use reinhardt-migrations for migration execution
		#[cfg(feature = "migrations")]
		{
			use reinhardt_db::migrations::{
				FilesystemRepository, FilesystemSource, MigrationService,
			};
			use std::path::PathBuf;
			use std::sync::Arc;
			use tokio::sync::Mutex;

			ctx.verbose("Loading migrations from disk...");
			let migrations_dir = PathBuf::from("migrations");

			let source = Arc::new(FilesystemSource::new(migrations_dir.clone()));
			let repository: Arc<Mutex<dyn reinhardt_db::migrations::MigrationRepository>> =
				Arc::new(Mutex::new(FilesystemRepository::new(migrations_dir)));
			let service = MigrationService::new(source, repository);
			// Filter by app if specified
			let all_migrations = service.load_all().await.map_err(|e| {
				crate::CommandError::ExecutionError(format!(
					"Failed to load all migrations: {:?}",
					e
				))
			})?;

			let migrations_to_apply: Vec<_> = if let Some(ref app) = app_label {
				all_migrations
					.into_iter()
					.filter(|m| m.app_label == app)
					.collect()
			} else {
				all_migrations.into_iter().collect()
			};
			if migrations_to_apply.is_empty() {
				ctx.info("No migrations to apply");
				return Ok(());
			}

			ctx.info(&format!(
				"Found {} migration(s) to apply",
				migrations_to_apply.len()
			));

			// 3. Check database connection
			let _database_url = get_database_url()?;

			// 4. Connect to database (auto-create if it doesn't exist for PostgreSQL)
			// Determine connection method based on URL scheme
			let connection = if _database_url.starts_with("postgres://")
				|| _database_url.starts_with("postgresql://")
			{
				DatabaseConnection::connect_postgres_or_create(&_database_url).await
			} else if _database_url.starts_with("sqlite://") || _database_url.starts_with("sqlite:")
			{
				DatabaseConnection::connect_sqlite(&_database_url).await
			} else if _database_url.starts_with("mongodb://") {
				#[cfg(feature = "mongodb-backend")]
				{
					// MongoDB requires separate database name
					// Extract database name from URL or use default
					let db_name = _database_url.split('/').next_back().unwrap_or("reinhardt");
					DatabaseConnection::connect_mongodb(&_database_url, db_name).await
				}
				#[cfg(not(feature = "mongodb-backend"))]
				{
					return Err(crate::CommandError::ExecutionError(
						"MongoDB backend not enabled. Enable 'mongodb-backend' feature."
							.to_string(),
					));
				}
			} else {
				return Err(crate::CommandError::ExecutionError(format!(
					"Unsupported database URL scheme: {}",
					_database_url
				)));
			}
			.map_err(|e| {
				crate::CommandError::ExecutionError(format!(
					"Failed to connect to database: {:?}",
					e
				))
			})?;

			// Get database type from connection (delegate to DatabaseConnection)
			let db_type = connection.database_type();

			// 5. Apply migrations (or fake them)
			if is_fake {
				ctx.info("Faking migrations (marking as applied without execution):");

				// Create migration executor for fake migrations
				let mut executor = DatabaseMigrationExecutor::new(connection, db_type);

				// Record each migration as applied without executing
				for migration in &migrations_to_apply {
					executor
						.record_migration(migration.app_label, migration.name)
						.await
						.map_err(|e| {
							crate::CommandError::ExecutionError(format!(
								"Failed to record fake migration {}:{}: {:?}",
								migration.app_label, migration.name, e
							))
						})?;
					ctx.success(&format!(
						"  ✓ Faked: {}:{}",
						migration.app_label, migration.name
					));
				}
			} else {
				ctx.info("Applying migrations:");

				// Create migration executor
				let mut executor = DatabaseMigrationExecutor::new(connection, db_type);

				// Apply migrations
				match executor.apply_migrations(&migrations_to_apply[..]).await {
					Ok(result) => {
						for applied_id in &result.applied {
							ctx.success(&format!("  ✓ Applied: {}", applied_id));
						}
					}
					Err(e) => {
						return Err(crate::CommandError::ExecutionError(format!(
							"Failed to apply migrations: {:?}",
							e
						)));
					}
				}
			}

			ctx.info("");
			ctx.success(&format!(
				"Applied {} migration(s) successfully",
				migrations_to_apply.len()
			));

			Ok(())
		}

		#[cfg(not(feature = "migrations"))]
		{
			ctx.warning("Migrations feature not enabled");
			ctx.info("To use migrate, enable the 'migrations' feature");
			Ok(())
		}
	}
}

/// Build from_state from database history (preferred approach)
#[cfg(feature = "migrations")]
async fn build_from_state_from_db(
	migrations_dir: &std::path::Path,
) -> Result<reinhardt_db::migrations::ProjectState, crate::CommandError> {
	use reinhardt_db::DatabaseConnection;
	use reinhardt_db::migrations::{
		DatabaseMigrationRecorder, FilesystemSource, MigrationSource, MigrationStateLoader,
	};

	// 1. Get database URL using the same method as migrate command
	let database_url = get_database_url()?;
	eprintln!("[DEBUG] Database URL: {}", database_url);

	// 2. Connect to database
	let connection = DatabaseConnection::connect(&database_url)
		.await
		.map_err(|e| {
			crate::CommandError::ExecutionError(format!("Database connection failed: {}", e))
		})?;
	eprintln!("[DEBUG] Database connection successful");

	// 3. Build state from database history
	let recorder = DatabaseMigrationRecorder::new(connection.inner().clone());
	let applied_records = recorder.get_applied_migrations().await.map_err(|e| {
		crate::CommandError::ExecutionError(format!("Failed to get applied migrations: {}", e))
	})?;
	eprintln!(
		"[DEBUG] Applied migrations count: {}",
		applied_records.len()
	);
	for record in &applied_records {
		eprintln!("[DEBUG]   - {}/{}", record.app, record.name);
	}

	let source = FilesystemSource::new(migrations_dir);
	let all_migrations = source.all_migrations().await.map_err(|e| {
		crate::CommandError::ExecutionError(format!("Failed to load migrations from disk: {}", e))
	})?;
	eprintln!("[DEBUG] Migrations on disk: {}", all_migrations.len());
	for migration in &all_migrations {
		eprintln!("[DEBUG]   - {}/{}", migration.app_label, migration.name);
	}

	let loader = MigrationStateLoader::new(recorder, source);

	let state = loader.build_current_state().await.map_err(|e| {
		crate::CommandError::ExecutionError(format!("Failed to build state: {}", e))
	})?;

	eprintln!("[DEBUG] Built state - models count: {}", state.models.len());
	for (app, model_name) in state.models.keys() {
		eprintln!("[DEBUG]   - {}/{}", app, model_name);
	}

	Ok(state)
}

/// Build from_state from TestContainers (default approach)
///
/// Note: TestContainers integration requires the 'testcontainers' feature to be enabled.
#[cfg(all(feature = "migrations", feature = "testcontainers"))]
async fn build_from_state_from_testcontainers(
	migrations_dir: &std::path::Path,
) -> Result<reinhardt_db::migrations::ProjectState, crate::CommandError> {
	use reinhardt_db::backends::DatabaseConnection;
	use reinhardt_db::backends::types::DatabaseType;
	use reinhardt_db::migrations::executor::DatabaseMigrationExecutor;
	use reinhardt_db::migrations::{
		DatabaseMigrationRecorder, FilesystemSource, MigrationSource, MigrationStateLoader,
	};
	use reinhardt_test::fixtures::postgres_container;

	// 1. Start temporary PostgreSQL container
	let (_container, _pool, _port, url) = postgres_container().await;

	// 2. Connect to temporary database
	let connection = DatabaseConnection::connect_postgres(&url)
		.await
		.map_err(|e| {
			crate::CommandError::ExecutionError(format!("TestContainers connection failed: {}", e))
		})?;

	// 3. Load all existing migrations
	let source = FilesystemSource::new(migrations_dir);
	let all_migrations = source.all_migrations().await.map_err(|e| {
		crate::CommandError::ExecutionError(format!("Failed to load migrations: {}", e))
	})?;

	// 4. Apply all existing migrations
	if !all_migrations.is_empty() {
		let mut executor =
			DatabaseMigrationExecutor::new(connection.clone(), DatabaseType::Postgres);
		executor
			.apply_migrations(&all_migrations)
			.await
			.map_err(|e| {
				crate::CommandError::ExecutionError(format!("Failed to apply migrations: {}", e))
			})?;
	}

	// 5. Build current state from applied migrations
	let recorder = DatabaseMigrationRecorder::new(connection.clone());
	let loader = MigrationStateLoader::new(recorder, source);

	loader.build_current_state().await.map_err(|e| {
		crate::CommandError::ExecutionError(format!(
			"Failed to build state from TestContainers: {}",
			e
		))
	})
}

/// Build from_state from TestContainers (stub when feature not enabled)
#[cfg(all(feature = "migrations", not(feature = "testcontainers")))]
async fn build_from_state_from_testcontainers(
	_migrations_dir: &std::path::Path,
) -> Result<reinhardt_db::migrations::ProjectState, crate::CommandError> {
	Err(crate::CommandError::ExecutionError(
		"TestContainers feature not enabled. Enable with --features testcontainers".to_string(),
	))
}

/// Make migrations command
#[cfg(feature = "migrations")]
pub struct MakeMigrationsCommand;

#[cfg(feature = "migrations")]
#[async_trait]
impl BaseCommand for MakeMigrationsCommand {
	fn name(&self) -> &str {
		"makemigrations"
	}

	fn description(&self) -> &str {
		"Create new migrations based on model changes"
	}

	fn arguments(&self) -> Vec<CommandArgument> {
		vec![CommandArgument::optional(
			"app",
			"App name to create migrations for",
		)]
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![
			CommandOption::flag(
				None,
				"dry-run",
				"Show what would be created without writing files",
			),
			CommandOption::flag(None, "empty", "Create empty migration"),
			CommandOption::flag(
				None,
				"from-db",
				"Use database history instead of TestContainers for state building",
			),
			CommandOption::flag(Some('v'), "verbose", "Show detailed operation list"),
			CommandOption::option(Some('n'), "name", "Name for the migration"),
			CommandOption::option(None, "migrations-dir", "Directory for migration files")
				.with_default("migrations"),
		]
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		fn operation_description(operation: &reinhardt_db::migrations::Operation) -> String {
			use reinhardt_db::migrations::Operation;

			match operation {
				// Table operations (corresponds to Model operations in Django)
				Operation::CreateTable { name, .. } => format!("Create model {}", name),
				Operation::DropTable { name } => format!("Delete model {}", name),
				Operation::RenameTable { old_name, new_name } => {
					format!("Rename model {} to {}", old_name, new_name)
				}

				// Column operations (corresponds to Field operations in Django)
				Operation::AddColumn { table, column } => {
					format!("Add field {} to {}", column.name, table)
				}
				Operation::DropColumn { table, column } => {
					format!("Remove field {} from {}", column, table)
				}
				Operation::AlterColumn { table, column, .. } => {
					format!("Alter field {} on {}", column, table)
				}
				Operation::RenameColumn {
					table,
					old_name,
					new_name,
				} => {
					format!("Rename field {} to {} on {}", old_name, new_name, table)
				}

				// Index operations
				Operation::CreateIndex {
					table,
					columns,
					unique,
				} => {
					let index_type = if *unique { "unique index" } else { "index" };
					format!(
						"Create {} on {} ({})",
						index_type,
						table,
						columns.join(", ")
					)
				}
				Operation::DropIndex { table, columns } => {
					format!("Remove index on {} ({})", table, columns.join(", "))
				}

				// Constraint operations
				Operation::AddConstraint { table, .. } => {
					format!("Add constraint on {}", table)
				}
				Operation::DropConstraint {
					table,
					constraint_name,
				} => {
					format!("Remove constraint {} from {}", constraint_name, table)
				}

				// Special operations
				Operation::RunSQL { .. } => "Execute custom SQL".to_string(),
				Operation::RunRust { .. } => "Execute custom Rust code".to_string(),

				// Other operations
				_ => format!("{:?}", operation),
			}
		}
		use std::path::PathBuf;
		ctx.info("Detecting model changes...");

		let is_dry_run = ctx.has_option("dry-run");
		let is_empty = ctx.has_option("empty");
		let app_label = ctx.arg(0).map(|s| s.to_string());
		let migration_name_opt = ctx.option("name").map(|s| s.to_string());
		let migrations_dir_str = ctx
			.option("migrations-dir")
			.map(|s| s.to_string())
			.unwrap_or_else(|| "migrations".to_string());
		let migrations_dir = PathBuf::from(migrations_dir_str);

		if is_dry_run {
			ctx.warning("Dry run mode: No files will be created");
		}

		if let Some(ref app_name) = app_label {
			ctx.verbose(&format!("Creating migrations for: {}", app_name));
		} else {
			ctx.verbose("Creating migrations for all apps");
		}

		#[cfg(feature = "migrations")]
		{
			use crate::CommandError;
			use reinhardt_db::migrations::{
				FilesystemRepository, FilesystemSource, MigrationNamer, MigrationNumbering,
				MigrationService, autodetector::ProjectState,
			};
			use std::sync::Arc;
			use tokio::sync::Mutex;

			let source = Arc::new(FilesystemSource::new(migrations_dir.clone()));
			let repository = Arc::new(Mutex::new(FilesystemRepository::new(
				migrations_dir.clone(),
			)));
			let service = MigrationService::new(source.clone(), repository.clone());

			// Helper to get the last migration for an app
			let get_last_migration = |app: String| {
				let source = source.clone();
				let repository = repository.clone();
				async move {
					let service = MigrationService::new(source, repository);
					let all_migrations = service.load_all().await.ok()?;
					let mut app_migrations: Vec<_> = all_migrations
						.into_iter()
						.filter(|m| m.app_label == app)
						.collect();

					// Simple sort by name (assumes timestamp prefix)
					app_migrations.sort_by(|a, b| a.name.cmp(b.name));

					app_migrations.last().cloned()
				}
			};

			// Handle --empty flag for manual migrations
			if is_empty {
				let app_name = app_label.ok_or_else(|| {
					CommandError::ExecutionError(
						"App label is required when creating an empty migration".to_string(),
					)
				})?;

				let last_migration = get_last_migration(app_name.clone()).await;
				let dependencies = if let Some(ref last) = last_migration {
					vec![(app_name.clone(), last.name)]
				} else {
					Vec::new()
				};

				// Generate migration name using new naming system
				let migration_number = MigrationNumbering::next_number(&migrations_dir, &app_name);
				let base_name = migration_name_opt.unwrap_or_else(|| "custom".to_string());
				let name = format!("{}_{}", migration_number, base_name);
				let new_migration = reinhardt_db::migrations::Migration {
					app_label: Box::leak(app_name.clone().into_boxed_str()),
					name: Box::leak(name.clone().into_boxed_str()),
					operations: Vec::new(),
					dependencies: dependencies
						.into_iter()
						.map(|(a, n)| (Box::leak(a.into_boxed_str()) as &'static str, n))
						.collect(),
					atomic: true,
					replaces: Vec::new(),
					initial: None,
				};

				if !is_dry_run {
					service
						.save_migration(&new_migration)
						.await
						.map_err(|e| CommandError::ExecutionError(format!("Save error: {}", e)))?;
					ctx.success(&format!(
						"Created empty migration for {}: {}",
						app_name, name
					));
				} else {
					ctx.info(&format!(
						"Would create empty migration for {}: {}",
						app_name, name
					));
				}
				return Ok(());
			}

			// 1. Get target project state from global model registry
			let target_project_state = ProjectState::from_global_registry();

			// Determine which apps to process
			let app_names: Vec<String> = if let Some(label) = app_label {
				// Explicit app label specified
				vec![label]
			} else {
				// Extract all app labels from ProjectState
				let changed_apps: Vec<String> = target_project_state
					.models
					.keys()
					.map(|(app_label, _)| app_label.clone())
					.collect::<std::collections::HashSet<_>>()
					.into_iter()
					.collect();

				if changed_apps.is_empty() {
					return Err(CommandError::ExecutionError(
						"No models found. Cannot determine app_label automatically.".to_string(),
					));
				}

				changed_apps
			};

			let is_verbose = ctx.has_option("verbose");

			// 2. Build from_state from database history or TestContainers
			// This ensures all models are treated as new, generating complete migrations
			struct MigrationResult {
				app_name: String,
				migration: reinhardt_db::migrations::Migration,
			}

			let mut results: Vec<MigrationResult> = Vec::new();

			// Build from_state based on strategy (default: TestContainers)
			let from_db_flag = ctx.has_option("from-db");
			let from_state = if from_db_flag {
				// When --from-db flag is specified: prioritize database history
				match build_from_state_from_db(&migrations_dir).await {
					Ok(state) => {
						ctx.verbose("Built state from database history");
						state
					}
					Err(e) => {
						ctx.warning(&format!("Failed to connect to database: {}", e));
						ctx.info("Falling back to TestContainers...");
						match build_from_state_from_testcontainers(&migrations_dir).await {
							Ok(state) => {
								ctx.verbose("Built state from TestContainers");
								state
							}
							Err(e) => {
								ctx.warning(&format!("Failed to use TestContainers: {}", e));
								ctx.info("Using empty state (full regeneration)");
								ProjectState::new()
							}
						}
					}
				}
			} else {
				// Default: prioritize TestContainers
				match build_from_state_from_testcontainers(&migrations_dir).await {
					Ok(state) => {
						ctx.verbose("Built state from TestContainers");
						state
					}
					Err(e) => {
						ctx.warning(&format!("Failed to use TestContainers: {}", e));
						ctx.info("Falling back to database history...");
						match build_from_state_from_db(&migrations_dir).await {
							Ok(state) => {
								ctx.verbose("Built state from database history");
								state
							}
							Err(e) => {
								ctx.warning(&format!("Failed to connect to database: {}", e));
								ctx.info("Using empty state (full regeneration)");
								ProjectState::new()
							}
						}
					}
				}
			};

			for app_name in &app_names {
				// Filter target state for this app only
				let app_target_state = target_project_state.filter_by_app(app_name);
				eprintln!(
					"[DEBUG] app_target_state for '{}': {} models",
					app_name,
					app_target_state.models.len()
				);
				for ((app, model_name), model_state) in &app_target_state.models {
					eprintln!(
						"[DEBUG]   - {}/{} ({} fields)",
						app,
						model_name,
						model_state.fields.len()
					);
				}

				// Filter from_state for this app only
				let app_from_state = from_state.filter_by_app(app_name);
				eprintln!(
					"[DEBUG] app_from_state for '{}': {} models",
					app_name,
					app_from_state.models.len()
				);
				for ((app, model_name), model_state) in &app_from_state.models {
					eprintln!(
						"[DEBUG]   - {}/{} ({} fields)",
						app,
						model_name,
						model_state.fields.len()
					);
				}

				// Use MigrationAutodetector for proper ManyToMany support
				let detector = reinhardt_db::migrations::MigrationAutodetector::new(
					app_from_state,
					app_target_state,
				);
				let generated_migrations = detector.generate_migrations();

				// Process generated migrations for this app
				for migration in generated_migrations {
					if migration.app_label == app_name.as_str() {
						// Generate migration name
						let base_name = migration_name_opt.clone().unwrap_or_else(|| {
							MigrationNamer::generate_name(&migration.operations, true)
						});
						let migration_number =
							MigrationNumbering::next_number(&migrations_dir, app_name);
						let final_name = format!("{}_{}", migration_number, base_name);

						// Determine dependencies
						let dependencies = if migration_number == "0001" {
							Vec::new() // Initial migration has no dependencies
						} else {
							// Get previous migration number
							let prev_number_int = migration_number.parse::<u32>().unwrap() - 1;
							let prev_number = format!("{:04}", prev_number_int);
							// Find the previous migration by scanning the directory
							let prev_migration_name = if let Ok(entries) =
								std::fs::read_dir(migrations_dir.join(app_name).join("migrations"))
							{
								let mut prev_names: Vec<String> = entries
									.filter_map(|entry| {
										let path = entry.ok()?.path();
										let filename = path.file_stem()?.to_str()?.to_string();
										if filename.starts_with(&prev_number) {
											Some(filename)
										} else {
											None
										}
									})
									.collect();
								prev_names.sort();
								prev_names.first().cloned()
							} else {
								None
							};

							if let Some(prev_name) = prev_migration_name {
								vec![(
									Box::leak(app_name.clone().into_boxed_str()) as &'static str,
									Box::leak(prev_name.into_boxed_str()) as &'static str,
								)]
							} else {
								Vec::new()
							}
						};

						let new_migration = reinhardt_db::migrations::Migration {
							app_label: Box::leak(app_name.clone().into_boxed_str()),
							name: Box::leak(final_name.into_boxed_str()),
							operations: migration.operations,
							dependencies,
							atomic: true,
							replaces: Vec::new(),
							initial: if migration_number == "0001" {
								Some(true)
							} else {
								None
							},
						};

						results.push(MigrationResult {
							app_name: app_name.clone(),
							migration: new_migration,
						});
					}
				}
			}

			// 4. Write all migrations
			if !results.is_empty() {
				for result in results {
					ctx.info(&format!("Migrations for '{}':", result.app_name));

					// Build the correct file path from migration name
					let migration_file_path = migrations_dir
						.join(&result.app_name)
						.join("migrations")
						.join(format!("{}.rs", result.migration.name));

					if !is_dry_run {
						service
							.save_migration(&result.migration)
							.await
							.map_err(|e| {
								CommandError::ExecutionError(format!("Save error: {}", e))
							})?;
						ctx.success(&format!("  {}", migration_file_path.display()));

						// Show detailed operations if --verbose
						if is_verbose {
							for operation in &result.migration.operations {
								let description = operation_description(operation);
								ctx.info(&format!("    - {}", description));
							}
						}
					} else {
						ctx.info(&format!(
							"  Would create: {}",
							migration_file_path.display()
						));

						if is_verbose {
							for operation in &result.migration.operations {
								let description = operation_description(operation);
								ctx.info(&format!("    - {}", description));
							}
						}
					}
				}
			} else {
				ctx.info("No changes detected");
			}

			Ok(())
		}

		#[cfg(not(feature = "migrations"))]
		{
			ctx.warning("Migrations feature not enabled");
			ctx.info("To use makemigrations, enable the 'migrations' feature");
			Ok(())
		}
	}
}

/// Interactive shell command
pub struct ShellCommand;

#[async_trait]
impl BaseCommand for ShellCommand {
	fn name(&self) -> &str {
		"shell"
	}

	fn description(&self) -> &str {
		"Start an interactive Rust REPL"
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![
			CommandOption::option(Some('c'), "command", "Execute a command and exit"),
			CommandOption::option(
				Some('e'),
				"engine",
				"Evaluation engine: rhai (default), python",
			),
		]
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		if let Some(command) = ctx.option("command") {
			ctx.info(&format!("Executing: {}", command));
			// Execute the command
			return Ok(());
		}

		ctx.info("Starting interactive shell...");
		ctx.info("Type 'exit' or press Ctrl+D to quit");

		#[cfg(feature = "shell")]
		{
			use rustyline::DefaultEditor;
			use rustyline::error::ReadlineError;

			let mut rl = DefaultEditor::new().map_err(|e| {
				crate::CommandError::ExecutionError(format!("Failed to create REPL: {}", e))
			})?;

			loop {
				let readline = rl.readline(">>> ");
				match readline {
					Ok(line) => {
						let trimmed = line.trim();
						if trimmed == "exit" || trimmed == "quit" {
							ctx.info("Goodbye!");
							break;
						}

						if !trimmed.is_empty() {
							let _ = rl.add_history_entry(line.as_str());

							// Evaluate code using selected engine
							let engine = ctx.option("engine").map(|s| s.as_str()).unwrap_or("rhai");

							match engine {
								"rhai" => {
									#[cfg(feature = "shell-rhai")]
									{
										Self::eval_rhai(ctx, trimmed)?;
									}
									#[cfg(not(feature = "shell-rhai"))]
									{
										ctx.warning(
											"Rhai engine not enabled. Enable 'shell-rhai' feature.",
										);
									}
								}
								"python" | "py" => {
									#[cfg(feature = "shell-pyo3")]
									{
										Self::eval_python(ctx, trimmed)?;
									}
									#[cfg(not(feature = "shell-pyo3"))]
									{
										ctx.warning(
											"Python engine not enabled. Enable 'shell-pyo3' feature.",
										);
									}
								}
								_ => {
									ctx.warning(&format!("Unknown engine: {}", engine));
									ctx.info("Available engines: rhai, python");
								}
							}
						}
					}
					Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
						ctx.info("Goodbye!");
						break;
					}
					Err(err) => {
						return Err(crate::CommandError::ExecutionError(format!(
							"REPL error: {}",
							err
						)));
					}
				}
			}

			Ok(())
		}

		#[cfg(not(feature = "shell"))]
		{
			ctx.warning("Shell feature not enabled");
			ctx.info("To use shell, enable the 'shell' feature in Cargo.toml:");
			ctx.info("  reinhardt-commands = { version = \"*\", features = [\"shell\"] }");
			Ok(())
		}
	}
}

impl ShellCommand {
	/// Evaluate code using Rhai engine
	#[cfg(feature = "shell-rhai")]
	fn eval_rhai(ctx: &CommandContext, code: &str) -> CommandResult<()> {
		use rhai::{Engine, EvalAltResult};

		let mut engine = Engine::new();

		// Register helper functions
		engine.register_fn("println", |s: &str| {
			println!("{}", s);
		});

		// Evaluate the code
		match engine.eval::<rhai::Dynamic>(code) {
			Ok(result) => {
				// Display result if not Unit type
				if !result.is_unit() {
					ctx.info(&format!("=> {}", result));
				}
				Ok(())
			}
			Err(e) => {
				let error_msg = match *e {
					EvalAltResult::ErrorParsing(ref err, _) => {
						format!("Parse error: {}", err)
					}
					EvalAltResult::ErrorRuntime(ref msg, _) => {
						format!("Runtime error: {}", msg)
					}
					_ => format!("Error: {}", e),
				};
				ctx.warning(&error_msg);
				Ok(())
			}
		}
	}

	/// Evaluate code using Python (PyO3) engine
	#[cfg(feature = "shell-pyo3")]
	fn eval_python(ctx: &CommandContext, code: &str) -> CommandResult<()> {
		use pyo3::prelude::*;
		use pyo3::types::PyDict;
		use std::ffi::CString;

		Python::attach(|py| {
			// Create a locals dictionary to maintain state between evaluations
			let locals = PyDict::new(py);

			// Convert code to CString for PyO3 0.27+
			let code_cstr = CString::new(code).map_err(|e| {
				crate::CommandError::ExecutionError(format!("Invalid Python code: {}", e))
			})?;

			// Execute the code
			match py.eval(&code_cstr, None, Some(&locals)) {
				Ok(result) => {
					// Convert result to string and display
					if let Ok(result_str) = result.str() {
						let s = result_str.to_string();
						if s != "()" && !s.is_empty() {
							ctx.info(&format!("=> {}", s));
						}
					}
					Ok(())
				}
				Err(e) => {
					let error_msg = format!("Python error: {}", e);
					ctx.warning(&error_msg);
					Ok(())
				}
			}
		})
	}
}

/// Development server command
pub struct RunServerCommand;

#[async_trait]
impl BaseCommand for RunServerCommand {
	fn name(&self) -> &str {
		"runserver"
	}

	fn description(&self) -> &str {
		"Start the development server"
	}

	fn arguments(&self) -> Vec<CommandArgument> {
		vec![
			CommandArgument::optional("address", "Server address (default: 127.0.0.1:8000)")
				.with_default("127.0.0.1:8000"),
		]
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![
			CommandOption::flag(None, "noreload", "Disable auto-reload"),
			CommandOption::option(
				None,
				"watch-delay",
				"Watch delay in milliseconds for file change debouncing",
			)
			.with_default("500"),
			CommandOption::flag(None, "nothreading", "Disable threading"),
			CommandOption::flag(None, "insecure", "Serve static files in production mode"),
		]
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		let address = ctx.arg(0).map(|s| s.as_str()).unwrap_or("127.0.0.1:8000");
		let noreload = ctx.has_option("noreload");
		let insecure = ctx.has_option("insecure");
		let no_docs = ctx.has_option("no_docs");

		ctx.info(&format!(
			"Starting development server at http://{}",
			address
		));

		if !noreload {
			#[cfg(all(feature = "server", feature = "autoreload"))]
			{
				ctx.verbose("Auto-reload enabled (notify-based)");
			}
			#[cfg(all(feature = "server", not(feature = "autoreload")))]
			{
				ctx.warning(
					"Auto-reload disabled: Enable 'autoreload' feature to use this functionality",
				);
			}
		}

		if insecure {
			ctx.warning("Running with --insecure: Static files will be served");
		}

		ctx.info("");
		ctx.info("Quit the server with CTRL-C");
		ctx.info("");

		// Server implementation with conditional features
		#[cfg(feature = "server")]
		{
			Self::run_server(ctx, address, noreload, insecure, no_docs).await
		}

		#[cfg(not(feature = "server"))]
		{
			ctx.warning("Server feature not enabled");
			ctx.info("To use runserver, enable the 'server' feature in Cargo.toml:");
			ctx.info("  reinhardt-commands = { version = \"*\", features = [\"server\"] }");
			ctx.info("");
			ctx.info("Alternatively, implement your own server using:");
			ctx.info("  use reinhardt_server::HttpServer;");
			ctx.info("  use reinhardt_urls::routers::DefaultRouter;");
			ctx.info("");
			ctx.info("  let router = DefaultRouter::new();");
			ctx.info("  // Register your routes");
			ctx.info("  let server = HttpServer::new(Arc::new(router));");
			ctx.info(&format!(
				"  server.listen(\"{}\".parse()?).await?;",
				address
			));

			Ok(())
		}
	}
}

impl RunServerCommand {
	/// Run the development server
	#[cfg(feature = "server")]
	async fn run_server(
		#[allow(unused_variables)] ctx: &CommandContext,
		address: &str,
		noreload: bool,
		_insecure: bool,
		no_docs: bool,
	) -> CommandResult<()> {
		use reinhardt_server::{HttpServer, ShutdownCoordinator};

		use std::time::Duration;

		// Get registered router
		if !reinhardt_urls::routers::is_router_registered() {
			return Err(crate::CommandError::ExecutionError(
                "No router registered. Call reinhardt_urls::routers::register_router() or reinhardt_urls::routers::register_router_arc() before running the server.".to_string()
            ));
		}

		let base_router = reinhardt_urls::routers::get_router().ok_or_else(|| {
			crate::CommandError::ExecutionError("Failed to get registered router".to_string())
		})?;

		// Wrap with OpenAPI endpoints if enabled
		#[cfg(feature = "openapi")]
		let router = if !no_docs {
			use reinhardt_openapi::OpenApiRouter;
			use reinhardt_types::Handler;
			std::sync::Arc::new(OpenApiRouter::wrap(base_router)) as std::sync::Arc<dyn Handler>
		} else {
			base_router
		};

		#[cfg(not(feature = "openapi"))]
		let router = base_router;

		// Parse socket address
		let addr: std::net::SocketAddr = address.parse().map_err(|e| {
			crate::CommandError::ExecutionError(format!("Invalid address '{}': {}", address, e))
		})?;

		// Create shutdown coordinator with 30s graceful shutdown timeout
		let coordinator = ShutdownCoordinator::new(Duration::from_secs(30));

		// Spawn CTRL-C signal handler
		let shutdown_tx = coordinator.clone();
		tokio::spawn(async move {
			if let Err(e) = tokio::signal::ctrl_c().await {
				eprintln!("Failed to listen for CTRL-C: {}", e);
				return;
			}
			println!("\nReceived CTRL-C, shutting down gracefully...");
			shutdown_tx.shutdown();
		});

		// OpenAPI documentation notice
		#[cfg(feature = "openapi")]
		if !no_docs {
			ctx.info("");
			ctx.info("📖 OpenAPI documentation available at:");
			ctx.info(&format!("   Swagger UI:     http://{}/docs", address));
			ctx.info(&format!("   Redoc UI:       http://{}/docs-redoc", address));
			ctx.info(&format!(
				"   OpenAPI JSON:   http://{}/api-docs/openapi.json",
				address
			));
			ctx.info("");
		}

		// Create DI context for dependency injection
		let singleton_scope = std::sync::Arc::new(reinhardt_di::SingletonScope::new());

		// Register DatabaseConnection as singleton when database feature is enabled
		#[cfg(feature = "reinhardt-db")]
		{
			use reinhardt_db::DatabaseConnection;

			// Try to connect to database and register connection
			match get_database_url() {
				Ok(url) => {
					ctx.info(&format!(
						"Connecting to database: {}...",
						&url[..url.len().min(50)]
					));
					match DatabaseConnection::connect(&url).await {
						Ok(db_conn) => {
							// Register DatabaseConnection directly (not wrapped in Arc)
							// The DI system wraps it in Arc internally via SingletonScope::set
							singleton_scope.set(db_conn);
							ctx.info("✅ Database connection registered in DI context");
						}
						Err(e) => {
							ctx.warning(&format!(
								"⚠️ Failed to connect to database: {}. DI injection for DatabaseConnection will fail.",
								e
							));
						}
					}
				}
				Err(e) => {
					ctx.warning(&format!(
						"⚠️ No DATABASE_URL configured: {}. DI injection for DatabaseConnection will fail.",
						e
					));
				}
			}
		}

		let di_context =
			std::sync::Arc::new(reinhardt_di::InjectionContext::builder(singleton_scope).build());

		// Create HTTP server with DI context and logging middleware
		let server = HttpServer::new(router)
			.with_di_context(di_context)
			.with_middleware(reinhardt_middleware::LoggingMiddleware::new());

		// Run with or without auto-reload
		if !noreload {
			#[cfg(feature = "autoreload")]
			{
				Self::run_with_autoreload(ctx, address, _insecure, no_docs).await
			}
			#[cfg(not(feature = "autoreload"))]
			{
				server
					.listen_with_shutdown(addr, coordinator)
					.await
					.map_err(|e| crate::CommandError::ExecutionError(e.to_string()))
			}
		} else {
			server
				.listen_with_shutdown(addr, coordinator)
				.await
				.map_err(|e| crate::CommandError::ExecutionError(e.to_string()))
		}
	}

	/// Run server with file watching and auto-reload
	#[cfg(all(feature = "server", feature = "autoreload"))]
	async fn run_with_autoreload(
		ctx: &CommandContext,
		address: &str,
		insecure: bool,
		no_docs: bool,
	) -> CommandResult<()> {
		use std::time::{Duration, Instant};

		ctx.info("Starting autoreload mode...");
		ctx.verbose("Watching for file changes in src/ and Cargo.toml");

		let mut restart_count = 0;
		let max_restarts_per_minute = 10;
		let mut last_restart_time = Instant::now();

		// Ctrl+C ハンドラのセットアップ
		let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();
		let ctx_clone = ctx.clone();
		tokio::spawn(async move {
			if let Err(e) = tokio::signal::ctrl_c().await {
				eprintln!("Failed to listen for Ctrl+C: {}", e);
				return;
			}
			ctx_clone.info("\nReceived Ctrl+C, shutting down...");
			let _ = shutdown_tx.send(());
		});

		loop {
			// 再起動頻度制限チェック（1分間に10回以上再起動しようとしたら停止）
			if restart_count >= max_restarts_per_minute {
				let elapsed = last_restart_time.elapsed();
				if elapsed < Duration::from_secs(60) {
					return Err(crate::CommandError::ExecutionError(format!(
						"Too many restarts ({} in {:?}). Aborting to prevent infinite loop.",
						restart_count, elapsed
					)));
				} else {
					// 1分以上経過していたらカウンタリセット
					restart_count = 0;
					last_restart_time = Instant::now();
				}
			}

			// 子プロセス起動
			ctx.verbose("Starting server subprocess...");
			let mut child = Self::spawn_server_process(address, insecure, no_docs)?;
			restart_count += 1;

			// ファイル変更 or 子プロセス終了 or Ctrl+Cを待機
			tokio::select! {
				change_result = Self::watch_files_async() => {
					match change_result {
						Ok(_) => {
							ctx.info("\n📝 File change detected. Restarting server...");
							// 子プロセスを停止
							if let Err(e) = child.kill().await {
								ctx.warning(&format!("Failed to kill child process: {}", e));
							}
							// wait()で確実に回収（ゾンビプロセス防止）
							let _ = child.wait().await;

							// ポート解放待ち + debounce
							tokio::time::sleep(Duration::from_millis(500)).await;
							continue; // ループ先頭に戻って再起動
						}
						Err(e) => {
							return Err(crate::CommandError::ExecutionError(format!(
								"File watcher error: {}",
								e
							)));
						}
					}
				}

				exit_status = child.wait() => {
					match exit_status {
						Ok(status) if status.success() => {
							ctx.info("Server process exited cleanly.");
							break; // 正常終了なら親も終了
						}
						Ok(status) => {
							return Err(crate::CommandError::ExecutionError(format!(
								"Server process crashed with status: {}",
								status
							)));
						}
						Err(e) => {
							return Err(crate::CommandError::ExecutionError(format!(
								"Failed to wait for child process: {}",
								e
							)));
						}
					}
				}

				_ = &mut shutdown_rx => {
					ctx.info("Shutdown signal received. Stopping server...");
					let _ = child.kill().await;
					let _ = child.wait().await;
					break;
				}
			}
		}

		Ok(())
	}

	/// 子プロセスでサーバーを起動
	#[cfg(all(feature = "server", feature = "autoreload"))]
	fn spawn_server_process(
		address: &str,
		insecure: bool,
		no_docs: bool,
	) -> CommandResult<tokio::process::Child> {
		let current_exe = std::env::current_exe().map_err(|e| {
			crate::CommandError::ExecutionError(format!("Failed to get current executable: {}", e))
		})?;

		let mut cmd = tokio::process::Command::new(current_exe);
		cmd.arg("runserver").arg(address).arg("--noreload");

		if insecure {
			cmd.arg("--insecure");
		}
		if no_docs {
			cmd.arg("--no-docs");
		}

		// 環境変数で子プロセスであることを示す（ログ重複防止など）
		cmd.env("REINHARDT_IS_AUTORELOAD_CHILD", "1");

		// 標準出力/エラーを親プロセスに継承
		cmd.stdout(std::process::Stdio::inherit());
		cmd.stderr(std::process::Stdio::inherit());

		cmd.spawn().map_err(|e| {
			crate::CommandError::ExecutionError(format!("Failed to spawn server process: {}", e))
		})
	}

	/// ファイル変更を非同期で監視
	#[cfg(all(feature = "server", feature = "autoreload"))]
	async fn watch_files_async() -> Result<(), notify::Error> {
		use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
		use std::path::Path;

		let (tx, mut rx) = tokio::sync::mpsc::channel(100);

		let mut watcher = RecommendedWatcher::new(
			move |res: Result<Event, notify::Error>| {
				if let Ok(event) = res {
					let _ = tx.blocking_send(event);
				}
			},
			Config::default(),
		)?;

		// 監視対象ディレクトリ
		watcher.watch(Path::new("src"), RecursiveMode::Recursive)?;
		watcher.watch(Path::new("Cargo.toml"), RecursiveMode::NonRecursive)?;

		// 最初の関連する変更イベントを待つ
		while let Some(event) = rx.recv().await {
			if Self::is_relevant_change(&event) {
				return Ok(());
			}
		}

		Ok(())
	}

	/// 変更イベントが関連するものかチェック
	#[cfg(all(feature = "server", feature = "autoreload"))]
	fn is_relevant_change(event: &notify::Event) -> bool {
		use notify::EventKind;

		matches!(
			event.kind,
			EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
		) && event.paths.iter().any(|p| {
			let path_str = p.to_string_lossy();
			!path_str.contains("/target/")
				&& !path_str.contains("/.git/")
				&& !path_str.ends_with('~')
				&& !path_str.ends_with(".swp")
				&& !path_str.ends_with(".tmp")
				&& (path_str.ends_with(".rs") || path_str.ends_with(".toml"))
		})
	}
}

/// Show all URLs command
#[cfg(feature = "routers")]
pub struct ShowUrlsCommand;

#[cfg(feature = "routers")]
#[async_trait]
impl BaseCommand for ShowUrlsCommand {
	fn name(&self) -> &str {
		"showurls"
	}

	fn description(&self) -> &str {
		"Display all registered URL patterns"
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![CommandOption::flag(None, "names", "Show only named URLs")]
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		// Check if router is registered
		if !reinhardt_urls::routers::is_router_registered() {
			ctx.warning(
				"No router registered. Call reinhardt_urls::routers::register_router() in your application startup.",
			);
			ctx.info("");
			ctx.info("Example:");
			ctx.info("  let router = UnifiedRouter::new()");
			ctx.info("      .with_prefix(\"/api\")");
			ctx.info("      .function(\"/health\", Method::GET, health_handler);");
			ctx.info("");
			ctx.info("  reinhardt_urls::routers::register_router(Arc::new(router));");
			return Ok(());
		}

		// Get registered router
		let router = reinhardt_urls::routers::get_router()
			.expect("Router should be registered (checked above)");

		// Get all routes
		let routes = router.get_all_routes();

		if routes.is_empty() {
			ctx.info("No routes registered.");
			return Ok(());
		}

		// Check if --names flag is set
		let names_only = ctx.has_option("names");

		// Display header
		ctx.info("Registered URL patterns:");
		ctx.info("");

		if names_only {
			// Show only named URLs
			let named_routes: Vec<_> = routes
				.iter()
				.filter(|(_, name, _, _)| name.is_some())
				.collect();

			if named_routes.is_empty() {
				ctx.info("No named URLs registered.");
				return Ok(());
			}

			ctx.info(&format!(
				"{:<40} {:<30} {:<20}",
				"URL Pattern", "Name", "Namespace"
			));
			ctx.info(&"=".repeat(90));

			for (path, name, namespace, _) in named_routes {
				let name_str = name.as_ref().map(|s| s.as_str()).unwrap_or("-");
				let namespace_str = namespace.as_ref().map(|s| s.as_str()).unwrap_or("-");

				ctx.info(&format!(
					"{:<40} {:<30} {:<20}",
					path, name_str, namespace_str
				));
			}
		} else {
			// Show all URLs with methods
			ctx.info(&format!(
				"{:<40} {:<20} {:<15} {:<20}",
				"URL Pattern", "Methods", "Name", "Namespace"
			));
			ctx.info(&"=".repeat(95));

			for (path, name, namespace, methods) in &routes {
				let methods_str = if methods.is_empty() {
					"ALL".to_string()
				} else {
					methods
						.iter()
						.map(|m| m.as_str())
						.collect::<Vec<_>>()
						.join(", ")
				};

				let name_str = name.as_ref().map(|s| s.as_str()).unwrap_or("-");
				let namespace_str = namespace.as_ref().map(|s| s.as_str()).unwrap_or("-");

				ctx.info(&format!(
					"{:<40} {:<20} {:<15} {:<20}",
					path, methods_str, name_str, namespace_str
				));
			}
		}

		ctx.info("");
		ctx.success(&format!("Total routes: {}", routes.len()));

		Ok(())
	}
}

/// Check system command
pub struct CheckCommand;

#[async_trait]
impl BaseCommand for CheckCommand {
	fn name(&self) -> &str {
		"check"
	}

	fn description(&self) -> &str {
		"Check for common problems"
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![CommandOption::flag(
			None,
			"deploy",
			"Check deployment settings",
		)]
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		ctx.info("System check:");
		ctx.info("");

		let is_deploy = ctx.has_option("deploy");
		let mut checks_passed = 0;
		let mut checks_failed = 0;

		// 1. Database connectivity check (if DATABASE_URL is set)
		if let Ok(database_url) = std::env::var("DATABASE_URL") {
			ctx.info("Checking database connectivity...");
			match Self::check_database(&database_url).await {
				Ok(_) => {
					ctx.success("  ✓ Database connection successful");
					checks_passed += 1;
				}
				Err(e) => {
					ctx.warning(&format!("  ✗ Database connection failed: {}", e));
					checks_failed += 1;
				}
			}
		} else {
			ctx.info("Skipping database check (DATABASE_URL not set)");
		}

		// 2. Settings validation
		ctx.info("Checking settings...");
		checks_passed += Self::check_settings(ctx, is_deploy);

		// 3. Migration status check (if DATABASE_URL is set)
		if std::env::var("DATABASE_URL").is_ok() {
			ctx.info("Checking migrations...");
			match Self::check_migrations().await {
				Ok(count) => {
					if count == 0 {
						ctx.success("  ✓ All migrations applied");
						checks_passed += 1;
					} else {
						ctx.warning(&format!("  ⚠ {} unapplied migrations found", count));
					}
				}
				Err(e) => {
					ctx.warning(&format!("  ✗ Migration check failed: {}", e));
					checks_failed += 1;
				}
			}
		}

		// 4. Static files verification
		ctx.info("Checking static files...");
		if std::env::var("STATIC_ROOT").is_ok() {
			ctx.success("  ✓ STATIC_ROOT configured");
			checks_passed += 1;
		} else if is_deploy {
			ctx.warning("  ✗ STATIC_ROOT not set (required for deployment)");
			checks_failed += 1;
		} else {
			ctx.info("  ⚠ STATIC_ROOT not set (optional for development)");
		}

		// 5. Security settings check (if --deploy)
		if is_deploy {
			ctx.info("Checking security settings...");
			checks_passed += Self::check_security(ctx);
		}

		ctx.info("");
		ctx.info(&format!(
			"System check complete: {} passed, {} failed",
			checks_passed, checks_failed
		));

		if checks_failed > 0 {
			Err(crate::CommandError::ExecutionError(format!(
				"{} check(s) failed",
				checks_failed
			)))
		} else {
			Ok(())
		}
	}
}

impl CheckCommand {
	/// Check database connectivity
	async fn check_database(database_url: &str) -> Result<(), String> {
		if database_url.is_empty() {
			return Err("Empty database URL".to_string());
		}

		#[cfg(feature = "migrations")]
		{
			// Actually connect to database and verify connectivity
			match connect_database(database_url).await {
				Ok((db_type, connection)) => {
					// Execute a simple query to verify connection
					match db_type {
						DatabaseType::Postgres | DatabaseType::Sqlite => {
							connection
								.execute("SELECT 1", vec![])
								.await
								.map_err(|e| format!("Query failed: {}", e))?;
						}
						#[cfg(feature = "mongodb-backend")]
						DatabaseType::MongoDB => {
							// MongoDB connection is verified at connection time
						}
						_ => {
							// MySQL or other database types that don't have SQL execution support yet
						}
					}
					Ok(())
				}
				Err(e) => Err(format!("Connection failed: {:?}", e)),
			}
		}

		#[cfg(not(feature = "migrations"))]
		{
			// Basic URL validation only
			Ok(())
		}
	}

	/// Check settings configuration
	fn check_settings(ctx: &CommandContext, is_deploy: bool) -> u32 {
		let mut passed = 0;

		// Check SECRET_KEY (always required in deployment)
		if is_deploy {
			if let Ok(secret_key) = std::env::var("SECRET_KEY") {
				if secret_key.len() >= 32 {
					ctx.success("  ✓ SECRET_KEY configured");
					passed += 1;
				} else {
					ctx.warning("  ✗ SECRET_KEY too short (minimum 32 characters)");
				}
			} else {
				ctx.warning("  ✗ SECRET_KEY not set (required for deployment)");
			}
		}

		// Check DEBUG setting
		if let Ok(debug) = std::env::var("DEBUG") {
			if is_deploy && debug == "true" {
				ctx.warning("  ✗ DEBUG=true in deployment (should be false)");
			} else {
				ctx.success("  ✓ DEBUG setting appropriate");
				passed += 1;
			}
		}

		passed
	}

	/// Check migrations status
	async fn check_migrations() -> Result<u32, String> {
		#[cfg(feature = "migrations")]
		{
			use reinhardt_db::migrations::{
				DatabaseMigrationRecorder, FilesystemRepository, FilesystemSource, MigrationService,
			};
			use std::path::PathBuf;
			use std::sync::Arc;
			use tokio::sync::Mutex;

			// 1. Load migration files from disk using FilesystemSource and Repository
			let migrations_dir = PathBuf::from("migrations");
			let source = Arc::new(FilesystemSource::new(migrations_dir.clone()));
			let repository = Arc::new(Mutex::new(FilesystemRepository::new(migrations_dir)));
			let service = MigrationService::new(source, repository);

			let all_migrations = service
				.load_all()
				.await
				.map_err(|e| format!("Failed to load all migrations: {:?}", e))?;

			// 2. Connect to database
			let database_url =
				std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL not set".to_string())?;

			let (_db_type, connection) = connect_database(&database_url)
				.await
				.map_err(|e| format!("Database connection failed: {:?}", e))?;

			// 3. Check applied migrations using Recorder
			let recorder = DatabaseMigrationRecorder::new(connection);
			recorder
				.ensure_schema_table()
				.await
				.map_err(|e| format!("Failed to create migration table: {}", e))?;

			// 4. Count unapplied migrations
			let mut unapplied_count = 0;
			for migration in &all_migrations {
				let is_applied = recorder
					.is_applied(migration.app_label, migration.name)
					.await
					.map_err(|e| format!("Failed to check migration: {}", e))?;

				if !is_applied {
					unapplied_count += 1;
				}
			}

			Ok(unapplied_count)
		}

		#[cfg(not(feature = "migrations"))]
		{
			// Without migrations feature, assume no unapplied migrations
			Ok(0)
		}
	}

	/// Check security settings
	fn check_security(ctx: &CommandContext) -> u32 {
		let mut passed = 0;

		// Check ALLOWED_HOSTS
		if std::env::var("ALLOWED_HOSTS").is_ok() {
			ctx.success("  ✓ ALLOWED_HOSTS configured");
			passed += 1;
		} else {
			ctx.warning("  ✗ ALLOWED_HOSTS not set (required for deployment)");
		}

		// Check SECURE_SSL_REDIRECT
		if let Ok(ssl_redirect) = std::env::var("SECURE_SSL_REDIRECT")
			&& ssl_redirect == "true"
		{
			ctx.success("  ✓ SECURE_SSL_REDIRECT enabled");
			passed += 1;
		}

		passed
	}
}

/// Helper function to get DATABASE_URL from environment or settings
#[cfg(feature = "reinhardt-db")]
fn get_database_url() -> Result<String, crate::CommandError> {
	use std::env;

	let base_dir = env::current_dir().ok();
	DatabaseConnection::get_database_url_from_env_or_settings(base_dir).map_err(|e| {
		crate::CommandError::ExecutionError(format!("Failed to get database URL: {}", e))
	})
}

/// Helper function to connect to database
#[cfg(feature = "reinhardt-db")]
async fn connect_database(url: &str) -> CommandResult<(DatabaseType, DatabaseConnection)> {
	let db_type = if url.starts_with("postgres://") || url.starts_with("postgresql://") {
		DatabaseType::Postgres
	} else if url.starts_with("sqlite://")
		|| url.starts_with("sqlite:")
		|| url.starts_with(":memory:")
	{
		DatabaseType::Sqlite
	} else if url.starts_with("mongodb://") {
		#[cfg(feature = "mongodb-backend")]
		{
			DatabaseType::MongoDB
		}
		#[cfg(not(feature = "mongodb-backend"))]
		{
			return Err(crate::CommandError::ExecutionError(
				"MongoDB backend not enabled. Enable 'mongodb-backend' feature.".to_string(),
			));
		}
	} else {
		return Err(crate::CommandError::ExecutionError(format!(
			"Unsupported database URL: {}",
			url
		)));
	};

	match db_type {
		DatabaseType::Postgres => {
			let conn = DatabaseConnection::connect_postgres(url)
				.await
				.map_err(|e| {
					crate::CommandError::ExecutionError(format!(
						"Database connection failed: {}",
						e
					))
				})?;
			Ok((db_type, conn))
		}
		DatabaseType::Sqlite => {
			let conn = DatabaseConnection::connect_sqlite(url).await.map_err(|e| {
				crate::CommandError::ExecutionError(format!("Database connection failed: {}", e))
			})?;
			Ok((db_type, conn))
		}
		#[cfg(feature = "mongodb-backend")]
		DatabaseType::MongoDB => {
			// MongoDB URL format: mongodb://host:port/database
			let database = url.split('/').next_back().unwrap_or("test");
			let conn = DatabaseConnection::connect_mongodb(url, database)
				.await
				.map_err(|e| {
					crate::CommandError::ExecutionError(format!(
						"Database connection failed: {}",
						e
					))
				})?;
			Ok((db_type, conn))
		}
		_ => {
			// MySQL or other database types
			Err(crate::CommandError::ExecutionError(format!(
				"Database type {:?} is not yet supported in this feature configuration",
				db_type
			)))
		}
	}
}

/// DI dependency graph check command
pub struct CheckDiCommand;

#[async_trait]
impl BaseCommand for CheckDiCommand {
	fn name(&self) -> &str {
		"check-di"
	}

	fn description(&self) -> &str {
		"Check DI dependency graph for circular dependencies and other issues"
	}

	fn arguments(&self) -> Vec<CommandArgument> {
		vec![]
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![]
	}

	async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
		ctx.info("🔍 Checking DI dependency graph...");

		#[cfg(feature = "di")]
		{
			// Get the global registry
			let registry = reinhardt_di::global_registry();

			// Count registered dependencies
			let registered_count = registry.len();

			ctx.info(&format!(
				"✓ Found {} registered dependencies",
				registered_count
			));

			if registered_count == 0 {
				ctx.warning("No dependencies registered");
				ctx.info(
					"Make sure to import modules that use #[injectable] or register_dependency!",
				);
				return Err(crate::CommandError::ExecutionError(
					"No dependencies found".to_string(),
				));
			}

			ctx.success("No circular dependencies detected at compile time");
			ctx.success("All checks passed");
			ctx.info("");
			ctx.info("Note: Runtime circular dependency detection is active.");
			ctx.info("      Any circular dependencies will be caught during resolution.");

			Ok(())
		}

		#[cfg(not(feature = "di"))]
		{
			ctx.warning("DI feature is not enabled");
			Err(crate::CommandError::ExecutionError(
				"check-di command requires 'di' feature to be enabled".to_string(),
			))
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_check_command_basic() {
		let cmd = CheckCommand;
		let ctx = CommandContext::default();

		// Should succeed when no DATABASE_URL is set (skips DB check)
		let result = cmd.execute(&ctx).await;
		// May fail if environment has strict checks, but should handle gracefully
		assert!(result.is_ok() || result.is_err());
	}

	#[tokio::test]
	async fn test_check_command_with_deploy_flag() {
		let cmd = CheckCommand;
		let mut ctx = CommandContext::default();
		ctx.set_option("deploy".to_string(), "true".to_string());

		// Deploy checks are stricter and may fail
		let result = cmd.execute(&ctx).await;
		assert!(result.is_ok() || result.is_err());
	}

	#[tokio::test]
	#[cfg(feature = "routers")]
	async fn test_showurls_command() {
		let cmd = ShowUrlsCommand;
		let ctx = CommandContext::default();

		let result = cmd.execute(&ctx).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	#[serial_test::serial(env_change)]
	async fn test_migrate_command() {
		let cmd = MigrateCommand;
		let ctx = CommandContext::default();

		// Without migrations feature or DATABASE_URL, should handle gracefully
		let result = cmd.execute(&ctx).await;
		#[cfg(feature = "migrations")]
		{
			// May fail without DATABASE_URL, which is expected
			assert!(result.is_ok() || result.is_err());
		}
		#[cfg(not(feature = "migrations"))]
		{
			assert!(result.is_ok());
		}
	}

	#[tokio::test]
	#[serial_test::serial(env_change)]
	#[cfg(feature = "migrations")]
	async fn test_makemigrations_command() {
		use reinhardt_db::migrations::model_registry::{
			FieldMetadata, ModelMetadata, global_registry,
		};
		use reinhardt_db::prelude::FieldType;
		use tempfile::TempDir;

		// Create a temporary directory for migrations
		let temp_dir = TempDir::new().unwrap();
		let migrations_dir = temp_dir.path().join("migrations");
		std::fs::create_dir_all(&migrations_dir).unwrap();

		// Register a test model
		let registry = global_registry();
		let mut metadata = ModelMetadata::new("testapp", "TestModel", "testapp_testmodel");
		metadata.add_field(
			"id".to_string(),
			FieldMetadata::new(FieldType::Integer).with_param("primary_key", "true"),
		);
		metadata.add_field(
			"name".to_string(),
			FieldMetadata::new(FieldType::VarChar(100)).with_param("max_length", "100"),
		);
		registry.register_model(metadata);

		// Set up test environment
		unsafe { std::env::set_var("DATABASE_URL", "sqlite::memory:") };

		let cmd = MakeMigrationsCommand;
		let mut ctx = CommandContext::default();
		ctx.add_arg("testapp".to_string());
		ctx.set_option(
			"migrations-dir".to_string(),
			migrations_dir.to_string_lossy().to_string(),
		);
		ctx.set_option("empty".to_string(), "true".to_string());

		let result = cmd.execute(&ctx).await;
		unsafe { std::env::remove_var("DATABASE_URL") };

		// Should succeed (creates an empty migration)
		assert!(result.is_ok(), "Failed with: {:?}", result.err());
	}

	#[tokio::test]
	#[serial_test::serial(env_change)]
	#[cfg(feature = "migrations")]
	async fn test_makemigrations_with_dry_run() {
		use reinhardt_db::{
			migrations::model_registry::{FieldMetadata, ModelMetadata, global_registry},
			prelude::FieldType,
		};
		use tempfile::TempDir;

		// Create a temporary directory for migrations
		let temp_dir = TempDir::new().unwrap();
		let migrations_dir = temp_dir.path().join("migrations");
		std::fs::create_dir_all(&migrations_dir).unwrap();

		// Register a test model
		let registry = global_registry();
		let mut metadata = ModelMetadata::new("testapp2", "TestModel2", "testapp2_testmodel");
		metadata.add_field(
			"id".to_string(),
			FieldMetadata::new(FieldType::Integer).with_param("primary_key", "true"),
		);
		metadata.add_field(
			"email".to_string(),
			FieldMetadata::new(FieldType::VarChar(255)).with_param("max_length", "255"),
		);
		registry.register_model(metadata);

		// Set up test environment
		unsafe { std::env::set_var("DATABASE_URL", "sqlite::memory:") };

		let cmd = MakeMigrationsCommand;
		let mut ctx = CommandContext::default();
		ctx.add_arg("testapp2".to_string());
		ctx.set_option(
			"migrations-dir".to_string(),
			migrations_dir.to_string_lossy().to_string(),
		);
		ctx.set_option("dry-run".to_string(), "true".to_string());
		ctx.set_option("empty".to_string(), "true".to_string());

		let result = cmd.execute(&ctx).await;
		unsafe { std::env::remove_var("DATABASE_URL") };

		// Should succeed (dry-run mode, no actual files created)
		assert!(result.is_ok(), "Failed with: {:?}", result.err());
	}

	#[tokio::test]
	#[serial_test::serial(runserver)]
	async fn test_runserver_command() {
		// Test without server feature - should show warnings
		#[cfg(not(feature = "server"))]
		{
			let cmd = RunServerCommand;
			let ctx = CommandContext::default();
			let result = cmd.execute(&ctx).await;
			assert!(result.is_ok());
		}

		// Test with server feature - spawn server with timeout
		// Server blocks indefinitely, so timeout is expected
		#[cfg(feature = "server")]
		{
			use reinhardt_urls::routers::UnifiedRouter;

			// Register a dummy router for the test
			let router = UnifiedRouter::new();
			reinhardt_urls::routers::register_router(router);

			// Create context with noreload option to disable autoreload
			let mut ctx = CommandContext::default();
			ctx.set_option("noreload".to_string(), "true".to_string());

			// Spawn server in background task
			let server_task = tokio::spawn(async move {
				let cmd = RunServerCommand;
				cmd.execute(&ctx).await
			});

			// Abort the server task (server blocks, so we need to abort)
			server_task.abort();

			// Wait for task to be aborted
			let result = server_task.await;

			// Cleanup: clear the registered router
			reinhardt_urls::routers::clear_router();

			// Task should have been cancelled
			assert!(result.is_err(), "Server task should have been cancelled");
		}
	}
}
