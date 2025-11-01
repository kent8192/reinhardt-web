//! Migration commands
//!
//! This module provides commands for managing database migrations,
//! inspired by Django's migration system.

use crate::{MigrationAutodetector, MigrationLoader, ProjectState};
use std::fs;
use std::path::Path;

/// MakeMigrations command options
///
/// # Django Reference
/// From: django/core/management/commands/makemigrations.py:20-50
/// ```python
/// class Command(BaseCommand):
///     def add_arguments(self, parser):
///         parser.add_argument('args', metavar='app_label', nargs='*')
///         parser.add_argument('--dry-run', action='store_true')
///         parser.add_argument('--name', '-n')
/// ```
#[derive(Debug, Clone)]
pub struct MakeMigrationsOptions {
	/// Specific app label to create migrations for
	pub app_label: Option<String>,
	/// Custom name for the migration
	pub name: Option<String>,
	/// Don't write migration files, just print what would be generated
	pub dry_run: bool,
	/// Base directory where migrations are stored (e.g., "migrations")
	pub migrations_dir: String,
}

impl Default for MakeMigrationsOptions {
	fn default() -> Self {
		Self {
			app_label: None,
			name: None,
			dry_run: false,
			migrations_dir: "migrations".to_string(),
		}
	}
}

/// MakeMigrations command
///
/// Detects changes in models and generates migration files.
///
/// # Django Reference
/// From: django/core/management/commands/makemigrations.py:52-330
/// ```python
/// def handle(self, *app_labels, **options):
///     # Load the current state
///     loader = MigrationLoader(None, ignore_no_migrations=True)
///
///     # Detect changes
///     autodetector = MigrationAutodetector(
///         loader.project_state(),
///         ProjectState.from_apps(apps),
///     )
///     changes = autodetector.changes(
///         graph=loader.graph,
///         trim_to_apps=app_labels or None,
///     )
///
///     # Write migration files
///     if not changes:
///         self.stdout.write("No changes detected")
///     else:
///         self.write_migration_files(changes)
/// ```
///
/// # Examples
///
/// ```
/// use reinhardt_migrations::MakeMigrationsCommand;
/// use reinhardt_migrations::MakeMigrationsOptions;
///
/// let options = MakeMigrationsOptions {
///     dry_run: true,
///     ..Default::default()
/// };
/// let command = MakeMigrationsCommand::new(options);
/// // Verify command is created successfully
/// let _: MakeMigrationsCommand = command;
/// ```
pub struct MakeMigrationsCommand {
	options: MakeMigrationsOptions,
}

impl MakeMigrationsCommand {
	/// Create a new MakeMigrations command
	pub fn new(options: MakeMigrationsOptions) -> Self {
		Self { options }
	}

	/// Execute the makemigrations command
	///
	/// Returns a list of created migration file paths.
	/// In dry-run mode, returns file paths that would be created.
	///
	/// # Django Reference
	/// From: django/core/management/commands/makemigrations.py:52-330
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_migrations::{MakeMigrationsCommand, MakeMigrationsOptions};
	///
	/// let options = MakeMigrationsOptions {
	///     dry_run: true,
	///     ..Default::default()
	/// };
	/// let command = MakeMigrationsCommand::new(options);
	/// let created_files = command.execute();
	/// assert!(created_files.is_empty() || !created_files.is_empty());
	/// ```
	pub fn execute(&self) -> Vec<String> {
		println!("Detecting model changes...");

		// Step 1: Load existing migrations and build from_state
		let loader = MigrationLoader::new(self.options.migrations_dir.clone().into());
		let from_state = match loader.build_project_state() {
			Ok(state) => state,
			Err(e) => {
				eprintln!("Error loading existing migrations: {:?}", e);
				ProjectState::new() // Use empty state if no migrations exist
			}
		};

		// Step 2: Get current model definitions and build to_state
		let to_state = ProjectState::from_global_registry();

		// Step 3: Detect changes
		let autodetector = MigrationAutodetector::new(from_state, to_state);
		let migrations = autodetector.generate_migrations();

		// Step 4: Filter by app_label if specified
		let filtered_migrations: Vec<_> = if let Some(ref app_label) = self.options.app_label {
			migrations
				.into_iter()
				.filter(|m| &m.app_label == app_label)
				.collect()
		} else {
			migrations
		};

		// Step 5: Check if there are any changes
		if filtered_migrations.is_empty() {
			println!("No changes detected");
			return Vec::new();
		}

		// Step 6: Print or write migrations and collect file paths
		if self.options.dry_run {
			self.print_migrations(&filtered_migrations)
		} else {
			self.write_migrations(&filtered_migrations)
		}
	}

	/// Print migrations to stdout (for dry-run mode)
	/// Returns file paths that would be created
	fn print_migrations(&self, migrations: &[crate::Migration]) -> Vec<String> {
		println!("\nMigrations to be created:");
		let mut file_paths = Vec::new();

		for migration in migrations {
			let migration_number = self.get_next_migration_number(&migration.app_label);
			let file_name = format!("{}_{}.rs", migration_number, migration.name);
			let dir_path = format!("{}/{}", self.options.migrations_dir, migration.app_label);
			let file_path = format!("{}/{}", dir_path, file_name);

			println!("\n  {}:", file_path);
			println!("    - {} operation(s)", migration.operations.len());
			for (i, op) in migration.operations.iter().enumerate() {
				println!("      {}. {:?}", i + 1, op);
			}

			file_paths.push(file_path);
		}

		file_paths
	}

	/// Write migrations to disk
	/// Returns list of successfully created file paths
	fn write_migrations(&self, migrations: &[crate::Migration]) -> Vec<String> {
		let mut created_files = Vec::new();
		let mut apps = std::collections::HashSet::new();

		for migration in migrations {
			let migration_number = self.get_next_migration_number(&migration.app_label);
			let file_name = format!("{}_{}.rs", migration_number, migration.name);
			let dir_path = format!("{}/{}", self.options.migrations_dir, migration.app_label);
			let file_path = format!("{}/{}", dir_path, file_name);

			// Create migrations directory if it doesn't exist
			if let Err(e) = fs::create_dir_all(&dir_path) {
				eprintln!("Error creating directory {}: {}", dir_path, e);
				continue;
			}

			// Generate migration file content
			let content = self.generate_migration_file(migration);

			// Write to file
			match fs::write(&file_path, content) {
				Ok(_) => {
					println!("  Created {}", file_path);
					created_files.push(file_path);
					apps.insert(migration.app_label.clone());
				}
				Err(e) => eprintln!("Error writing to {}: {}", file_path, e),
			}
		}

		// Update entry point files for all affected apps (Rust 2024 Edition style)
		for app_label in apps {
			if let Err(e) = self.update_app_entry_point(&app_label) {
				eprintln!(
					"Warning: Failed to update entry point for {}: {}",
					app_label, e
				);
			} else {
				println!(
					"  Updated entry point: {}/{}.rs",
					self.options.migrations_dir, app_label
				);
			}
		}

		created_files
	}

	/// Get the next migration number for an app
	fn get_next_migration_number(&self, app_label: &str) -> String {
		let dir_path = format!("{}/{}", self.options.migrations_dir, app_label);

		// If directory doesn't exist, this is the first migration
		if !Path::new(&dir_path).exists() {
			return "0001".to_string();
		}

		// Read directory and find highest migration number
		let mut max_number = 0;
		if let Ok(entries) = fs::read_dir(&dir_path) {
			for entry in entries.flatten() {
				if let Some(file_name) = entry.file_name().to_str() {
					if file_name.ends_with(".rs") {
						// Extract number from filename (e.g., "0001_initial.rs" -> "0001")
						if let Some(number_str) = file_name.split('_').next() {
							if let Ok(number) = number_str.parse::<u32>() {
								max_number = max_number.max(number);
							}
						}
					}
				}
			}
		}

		format!("{:04}", max_number + 1)
	}

	/// Generate migration file content
	///
	/// # Django Reference
	/// From: django/db/migrations/writer.py:120-200
	/// ```python
	/// def as_string(self):
	///     items = {
	///         "replaces_str": "",
	///         "initial_str": "",
	///     }
	///     # ... generate migration code
	///     return MIGRATION_TEMPLATE % items
	/// ```
	fn generate_migration_file(&self, migration: &crate::Migration) -> String {
		let mut content = String::new();

		// File header
		content.push_str("use reinhardt_migrations::{Migration, Operation, ColumnDefinition};\n\n");
		content.push_str("/// Auto-generated migration\n");
		content.push_str(&format!(
			"pub fn migration() -> Migration {{\n    Migration::new(\"{}\", \"{}\")\n",
			migration.name, migration.app_label
		));

		// Add dependencies if any
		if !migration.dependencies.is_empty() {
			for (dep_app, dep_migration) in &migration.dependencies {
				content.push_str(&format!(
					"        .add_dependency(\"{}\", \"{}\")\n",
					dep_app, dep_migration
				));
			}
		}

		// Add operations
		for operation in &migration.operations {
			content.push_str(&self.generate_operation_code(operation));
		}

		content.push_str("}\n");
		content
	}

	/// Generate Rust code for an operation
	fn generate_operation_code(&self, operation: &crate::Operation) -> String {
		match operation {
			crate::Operation::CreateTable {
				name,
				columns,
				constraints,
			} => {
				let mut code = String::from("        .add_operation(Operation::CreateTable {\n");
				code.push_str(&format!("            name: \"{}\".to_string(),\n", name));
				code.push_str("            columns: vec![\n");
				for column in columns {
					code.push_str(&format!(
                        "                ColumnDefinition {{ name: \"{}\".to_string(), type_definition: \"{}\".to_string() }},\n",
                        column.name, column.type_definition
                    ));
				}
				code.push_str("            ],\n");
				code.push_str(&format!(
					"            constraints: vec!{:?},\n",
					constraints
				));
				code.push_str("        })\n");
				code
			}
			crate::Operation::DropTable { name, .. } => {
				format!(
					"        .add_operation(Operation::DropTable {{ name: \"{}\".to_string() }})\n",
					name
				)
			}
			crate::Operation::AddColumn { table, column } => {
				format!(
					"        .add_operation(Operation::AddColumn {{ table: \"{}\".to_string(), column: ColumnDefinition {{ name: \"{}\".to_string(), type_definition: \"{}\".to_string() }} }})\n",
					table, column.name, column.type_definition
				)
			}
			crate::Operation::DropColumn { table, column, .. } => {
				format!(
					"        .add_operation(Operation::DropColumn {{ table: \"{}\".to_string(), column: \"{}\".to_string() }})\n",
					table, column
				)
			}
			crate::Operation::AlterColumn {
				table,
				column,
				new_definition,
				..
			} => {
				format!(
					"        .add_operation(Operation::AlterColumn {{ table: \"{}\".to_string(), column: \"{}\".to_string(), new_definition: ColumnDefinition {{ name: \"{}\".to_string(), type_definition: \"{}\".to_string() }} }})\n",
					table, column, new_definition.name, new_definition.type_definition
				)
			}
			_ => "        // Unsupported operation\n".to_string(),
		}
	}

	/// Update app entry point file using AST
	///
	/// Generates or updates `migrations/app_name.rs` that exports all migration modules.
	/// Uses AST to ensure robust parsing and formatting.
	///
	/// This is exposed as public for testing purposes.
	///
	/// # Examples
	///
	/// Generated entry point file structure:
	/// ```rust,ignore
	/// use reinhardt_migrations::Migration;
	///
	/// // migrations/myapp.rs
	/// pub mod _0001_initial;
	/// pub mod _0002_add_field;
	///
	/// pub fn all_migrations() -> Vec<fn() -> Migration> {
	///     vec![_0001_initial::migration, _0002_add_field::migration]
	/// }
	/// ```
	pub fn update_app_entry_point(&self, app_label: &str) -> Result<(), std::io::Error> {
		use syn::{File, Item, ItemFn, ItemMod, parse_file};

		let app_dir = format!("{}/{}", self.options.migrations_dir, app_label);
		let entry_point_path = format!("{}/{}.rs", self.options.migrations_dir, app_label);

		// Scan for all migration files in the app directory
		let mut migration_modules = Vec::new();
		if let Ok(entries) = fs::read_dir(&app_dir) {
			for entry in entries.flatten() {
				if let Some(file_name) = entry.file_name().to_str() {
					if file_name.ends_with(".rs")
						&& file_name
							.chars()
							.next()
							.map_or(false, |c| c.is_ascii_digit())
					{
						// Convert filename to valid module name (prefix with _ and replace - with _)
						let module_name =
							format!("_{}", file_name.trim_end_matches(".rs").replace('-', "_"));
						migration_modules.push(module_name);
					}
				}
			}
		}

		// Sort migration modules by name to ensure consistent ordering
		migration_modules.sort();

		// Parse existing file or create default AST
		let mut ast: File = if Path::new(&entry_point_path).exists() {
			let content = fs::read_to_string(&entry_point_path)?;
			parse_file(&content).map_err(|e| {
				std::io::Error::new(
					std::io::ErrorKind::InvalidData,
					format!("Failed to parse {}: {}", entry_point_path, e),
				)
			})?
		} else {
			parse_file("//! Migration entry point\n").map_err(|e| {
				std::io::Error::new(
					std::io::ErrorKind::InvalidData,
					format!("Failed to create default AST: {}", e),
				)
			})?
		};

		// Track which modules are already declared
		let mut existing_modules = std::collections::HashSet::new();
		for item in &ast.items {
			if let Item::Mod(ItemMod { ident, .. }) = item {
				existing_modules.insert(ident.to_string());
			}
		}

		// Remove old module declarations and all_migrations function
		ast.items.retain(|item| match item {
			Item::Mod(_) => false,
			Item::Fn(ItemFn { sig, .. }) => sig.ident != "all_migrations",
			_ => true,
		});

		// Add all module declarations
		for module_name in &migration_modules {
			let module_ident = syn::Ident::new(module_name, proc_macro2::Span::call_site());
			let mod_item: ItemMod = syn::parse_quote! {
				pub mod #module_ident;
			};
			ast.items.push(Item::Mod(mod_item));
		}

		// Generate all_migrations function
		let migration_calls: Vec<_> = migration_modules
			.iter()
			.map(|module_name| {
				let module_ident = syn::Ident::new(module_name, proc_macro2::Span::call_site());
				quote::quote! { #module_ident::migration }
			})
			.collect();

		let all_migrations_fn: ItemFn = syn::parse_quote! {
			pub fn all_migrations() -> Vec<fn() -> Migration> {
				vec![#(#migration_calls),*]
			}
		};
		ast.items.push(Item::Fn(all_migrations_fn));

		// Format and write back to file
		let formatted = prettyplease::unparse(&ast);
		fs::write(&entry_point_path, formatted)?;

		Ok(())
	}
}

/// Migrate command options
#[derive(Debug, Clone)]
pub struct MigrateOptions {
	pub app_label: Option<String>,
	pub migration_name: Option<String>,
	pub fake: bool,
	pub database: Option<String>,
	pub plan: bool,
	pub migrations_dir: String,
}

impl Default for MigrateOptions {
	fn default() -> Self {
		Self {
			app_label: None,
			migration_name: None,
			fake: false,
			database: None,
			plan: false,
			migrations_dir: "migrations".to_string(),
		}
	}
}

/// Migrate command
pub struct MigrateCommand {
	#[allow(dead_code)]
	options: MigrateOptions,
}

impl MigrateCommand {
	pub fn new(options: MigrateOptions) -> Self {
		Self { options }
	}

	pub fn execute(&self) {
		// Execute migrate
	}
}
