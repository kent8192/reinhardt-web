//! Filesystem-based migration repository
//!
//! Persists migrations as `.rs` files on disk.

use crate::ast_parser;
use crate::{Migration, MigrationError, MigrationRepository, Result};
use async_trait::async_trait;
use quote::quote;
use std::path::{Path, PathBuf};
use syn::parse_quote;

/// Repository that persists migrations as `.rs` files
///
/// This repository writes migrations to disk in the format:
/// ```rust,no_run
/// // <app_label>/migrations/<name>.rs
/// use reinhardt_db::migrations::Migration;
/// // use reinhardt::db::migrations::prelude::*;
/// // use reinhardt::db::migrations::FieldType;
///
/// pub fn migration() -> Migration {
///     Migration::new("0001_initial", "app")
/// }
/// ```
pub struct FilesystemRepository {
	/// Root directory for migration files
	root_dir: PathBuf,
}

impl FilesystemRepository {
	/// Create a new FilesystemRepository
	///
	/// # Arguments
	///
	/// * `root_dir` - Root directory where migration files will be stored
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_db::migrations::FilesystemRepository;
	/// let repo = FilesystemRepository::new("./migrations");
	/// ```
	pub fn new<P: AsRef<Path>>(root_dir: P) -> Self {
		Self {
			root_dir: root_dir.as_ref().to_path_buf(),
		}
	}

	/// Get the path for a migration file
	///
	/// Returns: `<root_dir>/<app_label>/<name>.rs`
	fn migration_path(&self, app_label: &str, name: &str) -> PathBuf {
		self.root_dir.join(app_label).join(format!("{}.rs", name))
	}

	/// Generate Rust code for a migration file
	fn generate_migration_code(&self, migration: &Migration) -> Result<String> {
		// Build dependencies vector
		let deps: Vec<_> = migration
			.dependencies
			.iter()
			.map(|(app, name)| {
				quote! { (#app, #name) }
			})
			.collect();

		// Build replaces vector
		let replaces: Vec<_> = migration
			.replaces
			.iter()
			.map(|(app, name)| {
				quote! { (#app, #name) }
			})
			.collect();

		let app_label = &migration.app_label;
		let name = &migration.name;
		let atomic = migration.atomic;

		// Generate operation code
		let ops_tokens = migration.operations.iter();
		let operations_code = quote! { vec![#(#ops_tokens),*] };

		// Generate full migration file
		let file: syn::File = parse_quote! {
			use reinhardt::db::migrations::prelude::*;
			use reinhardt::db::migrations::FieldType;

			pub fn migration() -> Migration {
				Migration {
					app_label: #app_label,
					name: #name,
					operations: #operations_code,
					dependencies: vec![#(#deps),*],
					atomic: #atomic,
					replaces: vec![#(#replaces),*],
				}
			}
		};

		// Format with prettyplease first, then apply rustfmt
		let prettyplease_output = prettyplease::unparse(&file);
		let formatted = Self::format_with_rustfmt(&prettyplease_output)?;
		Ok(formatted)
	}

	/// Format code with rustfmt, applying project's rustfmt.toml settings (hard_tabs = true)
	///
	/// Falls back to prettyplease output if rustfmt is not available or fails.
	fn format_with_rustfmt(code: &str) -> Result<String> {
		use std::io::Write;
		use std::process::{Command, Stdio};

		// Try to run rustfmt
		let child = Command::new("rustfmt")
			.arg("--edition=2024")
			.stdin(Stdio::piped())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.spawn();

		match child {
			Ok(mut child_process) => {
				// Write code to stdin
				if let Some(stdin) = child_process.stdin.as_mut() {
					stdin.write_all(code.as_bytes()).map_err(|e| {
						MigrationError::IoError(std::io::Error::other(format!(
							"Failed to write to rustfmt stdin: {}",
							e
						)))
					})?;
				}

				// Get formatted output
				let output = child_process.wait_with_output().map_err(|e| {
					MigrationError::IoError(std::io::Error::other(format!(
						"Failed to read rustfmt output: {}",
						e
					)))
				})?;

				if output.status.success() {
					String::from_utf8(output.stdout).map_err(|e| {
						MigrationError::IoError(std::io::Error::other(format!(
							"Invalid UTF-8 from rustfmt: {}",
							e
						)))
					})
				} else {
					// rustfmt failed, fallback to prettyplease output
					eprintln!("Warning: rustfmt failed, using prettyplease output");
					Ok(code.to_string())
				}
			}
			Err(_) => {
				// rustfmt not available, use prettyplease output
				eprintln!("Warning: rustfmt not found, using prettyplease output (space-indented)");
				Ok(code.to_string())
			}
		}
	}

	/// Check if two migrations have identical operations
	///
	/// Returns true if the operations vectors are equal.
	fn has_identical_operations(&self, m1: &Migration, m2: &Migration) -> bool {
		m1.operations == m2.operations
	}
}

#[async_trait]
impl MigrationRepository for FilesystemRepository {
	async fn save(&mut self, migration: &Migration) -> Result<()> {
		let path = self.migration_path(&migration.app_label, &migration.name);

		// Check if migration file already exists to prevent overwriting
		if tokio::fs::try_exists(&path).await.unwrap_or(false) {
			return Err(MigrationError::IoError(std::io::Error::other(format!(
				"Migration file already exists: {}. \
				If you want to replace it, please delete the existing file first.",
				path.display()
			))));
		}

		// Check for duplicate operations with existing migrations
		let existing_migrations = self.list(&migration.app_label).await?;
		for existing in &existing_migrations {
			if self.has_identical_operations(existing, migration) {
				return Err(MigrationError::DuplicateOperations(format!(
					"Migration '{}' has identical operations to existing migration '{}'. \
					This usually indicates a problem with from_state construction. \
					The existing migration was created at the same location and performs \
					the same database changes.",
					migration.name, existing.name
				)));
			}
		}

		// Create parent directories
		if let Some(parent) = path.parent() {
			tokio::fs::create_dir_all(parent).await.map_err(|e| {
				MigrationError::IoError(std::io::Error::other(format!(
					"Failed to create directory {}: {}",
					parent.display(),
					e
				)))
			})?;
		}

		// Generate migration code
		let code = self.generate_migration_code(migration)?;

		// Write to file
		tokio::fs::write(&path, code).await.map_err(|e| {
			MigrationError::IoError(std::io::Error::other(format!(
				"Failed to write {}: {}",
				path.display(),
				e
			)))
		})?;

		Ok(())
	}

	async fn get(&self, app_label: &str, name: &str) -> Result<Migration> {
		let path = self.migration_path(app_label, name);

		if !path.exists() {
			return Err(MigrationError::NotFound(format!("{}.{}", app_label, name)));
		}

		// Read and parse file
		let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
			MigrationError::IoError(std::io::Error::other(format!(
				"Failed to read {}: {}",
				path.display(),
				e
			)))
		})?;

		// Parse with syn
		let ast: syn::File = syn::parse_file(&content).map_err(|e| {
			MigrationError::InvalidMigration(format!("Failed to parse {}: {}", path.display(), e))
		})?;

		// Extract migration data from AST using ast_parser utility
		ast_parser::extract_migration_metadata(&ast, app_label, name)
	}

	async fn list(&self, app_label: &str) -> Result<Vec<Migration>> {
		let migrations_dir = self.root_dir.join(app_label);

		if !migrations_dir.exists() {
			return Ok(vec![]);
		}

		let mut migrations = Vec::new();

		// Read directory
		let mut entries = tokio::fs::read_dir(&migrations_dir).await.map_err(|e| {
			MigrationError::IoError(std::io::Error::other(format!(
				"Failed to read directory {}: {}",
				migrations_dir.display(),
				e
			)))
		})?;

		while let Some(entry) = entries.next_entry().await.map_err(|e| {
			MigrationError::IoError(std::io::Error::other(format!(
				"Failed to read directory entry: {}",
				e
			)))
		})? {
			let path = entry.path();

			// Skip non-.rs files
			if path.extension().and_then(|s| s.to_str()) != Some("rs") {
				continue;
			}

			// Extract name from filename
			if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
				// Get migration
				match self.get(app_label, name).await {
					Ok(migration) => migrations.push(migration),
					Err(e) => {
						eprintln!("Warning: Failed to load migration {}: {}", name, e);
					}
				}
			}
		}

		Ok(migrations)
	}

	async fn delete(&mut self, app_label: &str, name: &str) -> Result<()> {
		let path = self.migration_path(app_label, name);

		if !path.exists() {
			return Err(MigrationError::NotFound(format!("{}.{}", app_label, name)));
		}

		tokio::fs::remove_file(&path).await.map_err(|e| {
			MigrationError::IoError(std::io::Error::other(format!(
				"Failed to delete {}: {}",
				path.display(),
				e
			)))
		})?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::fields::FieldType;
	use crate::operations::{ColumnDefinition, Operation};
	use serial_test::serial;
	use tempfile::TempDir;

	/// Creates a test migration with a unique CreateTable operation based on the migration name.
	/// This ensures each migration has distinct operations to avoid duplicate detection errors.
	fn create_test_migration(app_label: &str, name: &str) -> Migration {
		let mut migration = Migration::new(name, app_label);

		// Create a unique table name derived from the migration name
		let table_name = format!("table_{}", name.replace('-', "_"));
		migration.operations.push(Operation::CreateTable {
			name: table_name,
			columns: vec![ColumnDefinition::new("id", FieldType::Integer)],
			constraints: vec![],
			without_rowid: None,
			partition: None,
			interleave_in_parent: None,
		});

		migration
	}

	#[tokio::test]
	#[serial(filesystem_repository)]
	async fn test_filesystem_repository_new() {
		let temp_dir = TempDir::new().unwrap();
		let repo = FilesystemRepository::new(temp_dir.path());
		assert_eq!(repo.root_dir, temp_dir.path());
	}

	#[tokio::test]
	#[serial(filesystem_repository)]
	async fn test_filesystem_repository_save() {
		let temp_dir = TempDir::new().unwrap();
		let mut repo = FilesystemRepository::new(temp_dir.path());

		let migration = create_test_migration("polls", "0001_initial");
		repo.save(&migration).await.unwrap();

		// Verify file exists
		let path = repo.migration_path("polls", "0001_initial");
		assert!(tokio::fs::try_exists(&path).await.unwrap());

		// Verify file content is valid Rust
		let content = tokio::fs::read_to_string(&path).await.unwrap();
		assert!(content.contains("pub fn migration() -> Migration"));
		assert!(content.contains("app_label: \"polls\""));
		assert!(content.contains("name: \"0001_initial\""));
	}

	#[tokio::test]
	#[serial(filesystem_repository)]
	async fn test_filesystem_repository_get() {
		let temp_dir = TempDir::new().unwrap();
		let mut repo = FilesystemRepository::new(temp_dir.path());

		// Save a migration
		let migration = create_test_migration("polls", "0001_initial");
		repo.save(&migration).await.unwrap();

		// Retrieve it
		let retrieved = repo.get("polls", "0001_initial").await.unwrap();
		assert_eq!(retrieved.app_label, "polls");
		assert_eq!(retrieved.name, "0001_initial");
	}

	#[tokio::test]
	#[serial(filesystem_repository)]
	async fn test_filesystem_repository_get_not_found() {
		let temp_dir = TempDir::new().unwrap();
		let repo = FilesystemRepository::new(temp_dir.path());

		let result = repo.get("polls", "0001_initial").await;
		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), MigrationError::NotFound(_)));
	}

	#[tokio::test]
	#[serial(filesystem_repository)]
	async fn test_filesystem_repository_list() {
		let temp_dir = TempDir::new().unwrap();
		let mut repo = FilesystemRepository::new(temp_dir.path());

		// Save multiple migrations
		repo.save(&create_test_migration("polls", "0001_initial"))
			.await
			.unwrap();
		repo.save(&create_test_migration("polls", "0002_add_field"))
			.await
			.unwrap();

		// List them
		let migrations = repo.list("polls").await.unwrap();
		assert_eq!(migrations.len(), 2);
	}

	#[tokio::test]
	#[serial(filesystem_repository)]
	async fn test_filesystem_repository_list_empty() {
		let temp_dir = TempDir::new().unwrap();
		let repo = FilesystemRepository::new(temp_dir.path());

		let migrations = repo.list("polls").await.unwrap();
		assert_eq!(migrations.len(), 0);
	}

	#[tokio::test]
	#[serial(filesystem_repository)]
	async fn test_filesystem_repository_delete() {
		let temp_dir = TempDir::new().unwrap();
		let mut repo = FilesystemRepository::new(temp_dir.path());

		// Save a migration
		let migration = create_test_migration("polls", "0001_initial");
		repo.save(&migration).await.unwrap();

		// Verify it exists
		let path = repo.migration_path("polls", "0001_initial");
		assert!(tokio::fs::try_exists(&path).await.unwrap());

		// Delete it
		repo.delete("polls", "0001_initial").await.unwrap();

		// Verify it's gone
		assert!(!tokio::fs::try_exists(&path).await.unwrap());
	}

	#[tokio::test]
	#[serial(filesystem_repository)]
	async fn test_filesystem_repository_delete_not_found() {
		let temp_dir = TempDir::new().unwrap();
		let mut repo = FilesystemRepository::new(temp_dir.path());

		let result = repo.delete("polls", "0001_initial").await;
		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), MigrationError::NotFound(_)));
	}

	#[tokio::test]
	#[serial(filesystem_repository)]
	async fn test_filesystem_repository_save_with_dependencies() {
		let temp_dir = TempDir::new().unwrap();
		let mut repo = FilesystemRepository::new(temp_dir.path());

		let migration =
			Migration::new("0002_add_field", "polls").add_dependency("polls", "0001_initial");

		repo.save(&migration).await.unwrap();

		// Verify file contains dependencies
		let path = repo.migration_path("polls", "0002_add_field");
		let content = tokio::fs::read_to_string(&path).await.unwrap();
		assert!(content.contains("dependencies"));
	}

	#[tokio::test]
	#[serial(filesystem_repository)]
	async fn test_filesystem_repository_save_prevents_overwrite() {
		let temp_dir = TempDir::new().unwrap();
		let mut repo = FilesystemRepository::new(temp_dir.path());

		// Save initial migration
		let migration = create_test_migration("polls", "0001_initial");
		repo.save(&migration).await.unwrap();

		// Verify file exists
		let path = repo.migration_path("polls", "0001_initial");
		assert!(tokio::fs::try_exists(&path).await.unwrap());

		// Attempt to save again with same name should fail
		let duplicate_migration = create_test_migration("polls", "0001_initial");
		let result = repo.save(&duplicate_migration).await;

		// Should return error
		assert!(result.is_err());

		// Error message should indicate file exists
		let err = result.unwrap_err();
		assert!(matches!(err, MigrationError::IoError(_)));
		assert!(err.to_string().contains("already exists"));
	}
}
