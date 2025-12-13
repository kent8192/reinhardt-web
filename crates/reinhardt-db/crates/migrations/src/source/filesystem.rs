//! Filesystem-based migration source
//!
//! Loads migrations from `.rs` files on disk and extracts metadata using AST parsing.

use crate::ast_parser;
use crate::{Migration, MigrationError, MigrationSource, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use syn::File;

/// Migration source that loads from filesystem
///
/// This source scans directories for `.rs` migration files and parses them
/// using `syn` to extract metadata like dependencies, atomic flag, and replaces.
pub struct FilesystemSource {
	/// Root directory containing migration files
	root_dir: PathBuf,
}

impl FilesystemSource {
	/// Create a new FilesystemSource
	///
	/// # Arguments
	///
	/// * `root_dir` - Root directory to scan for migration files
	///
	/// # Example
	///
	/// ```rust,no_run
	/// use reinhardt_migrations::FilesystemSource;
	/// let source = FilesystemSource::new("./migrations");
	/// ```
	pub fn new<P: AsRef<Path>>(root_dir: P) -> Self {
		Self {
			root_dir: root_dir.as_ref().to_path_buf(),
		}
	}

	/// Parse a migration file and extract metadata
	///
	/// This function reads the file, parses it with `syn`, and extracts:
	/// - dependencies from `dependencies()` function
	/// - atomic flag from `atomic()` function
	/// - replaces from `replaces()` function
	fn parse_migration_file(&self, path: &Path) -> Result<Migration> {
		// Read file contents
		let content = std::fs::read_to_string(path).map_err(|e| {
			MigrationError::IoError(std::io::Error::other(format!(
				"Failed to read {}: {}",
				path.display(),
				e
			)))
		})?;

		// Parse with syn
		let ast: File = syn::parse_file(&content).map_err(|e| {
			MigrationError::InvalidMigration(format!("Failed to parse {}: {}", path.display(), e))
		})?;

		// Extract app_label and name from path
		// Expected format: <root_dir>/<app_label>/migrations/<name>.rs
		let (app_label, name) = self.extract_app_and_name(path)?;

		// Extract metadata from AST using ast_parser utility
		ast_parser::extract_migration_metadata(&ast, &app_label, &name)
	}

	/// Extract app_label and migration name from file path
	///
	/// Expected format: .../<app_label>/migrations/<name>.rs
	///
	/// This implementation is flexible and works with any directory structure,
	/// as long as the path contains a 'migrations' directory with the app_label
	/// as the parent directory.
	fn extract_app_and_name(&self, path: &Path) -> Result<(String, String)> {
		// Collect path components
		let components: Vec<_> = path
			.components()
			.filter_map(|c| match c {
				std::path::Component::Normal(s) => s.to_str(),
				_ => None,
			})
			.collect();

		// Find 'migrations' directory in the path
		let migrations_idx = components
			.iter()
			.rposition(|&c| c == "migrations")
			.ok_or_else(|| {
				MigrationError::InvalidMigration(
					"Path does not contain 'migrations' directory".to_string(),
				)
			})?;

		// The app_label is the directory immediately before 'migrations'
		let app_label = if migrations_idx > 0 {
			components[migrations_idx - 1].to_string()
		} else {
			return Err(MigrationError::InvalidMigration(
				"Cannot determine app_label: 'migrations' is at the root".to_string(),
			));
		};

		// Extract migration name from file name (without extension)
		let file_name = path
			.file_stem()
			.and_then(|s| s.to_str())
			.ok_or_else(|| MigrationError::InvalidMigration("Invalid file name".to_string()))?;

		Ok((app_label, file_name.to_string()))
	}
}

#[async_trait]
impl MigrationSource for FilesystemSource {
	async fn all_migrations(&self) -> Result<Vec<Migration>> {
		let mut migrations = Vec::new();

		// Walk directory tree to find all .rs files
		for entry in walkdir::WalkDir::new(&self.root_dir)
			.follow_links(true)
			.into_iter()
			.filter_map(|e| e.ok())
		{
			let path = entry.path();

			// Skip if not a .rs file
			if path.extension().and_then(|s| s.to_str()) != Some("rs") {
				continue;
			}

			// Skip if not in a migrations/ directory
			if !path
				.parent()
				.and_then(|p| p.file_name())
				.and_then(|n| n.to_str())
				.map(|n| n == "migrations")
				.unwrap_or(false)
			{
				continue;
			}

			// Parse migration file
			match self.parse_migration_file(path) {
				Ok(migration) => migrations.push(migration),
				Err(e) => {
					// Log error but continue scanning
					eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
				}
			}
		}

		Ok(migrations)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serial_test::serial;
	use std::fs;
	use tempfile::TempDir;

	/// Helper to create a test migration file
	fn create_migration_file(dir: &Path, app: &str, name: &str, content: &str) {
		let migrations_dir = dir.join(app).join("migrations");
		fs::create_dir_all(&migrations_dir).unwrap();
		let file_path = migrations_dir.join(format!("{}.rs", name));
		fs::write(file_path, content).unwrap();
	}

	#[tokio::test]
	#[serial(filesystem_source)]
	async fn test_filesystem_source_new() {
		let temp_dir = TempDir::new().unwrap();
		let source = FilesystemSource::new(temp_dir.path());
		assert_eq!(source.root_dir, temp_dir.path());
	}

	#[tokio::test]
	#[serial(filesystem_source)]
	async fn test_filesystem_source_all_migrations() {
		let temp_dir = TempDir::new().unwrap();

		// Create test migration files
		create_migration_file(
			temp_dir.path(),
			"polls",
			"0001_initial",
			r#"
use reinhardt_migrations::prelude::*;

pub fn migration() -> Migration {
	Migration {
		app_label: "polls",
		name: "0001_initial",
		operations: vec![],
		dependencies: vec![],
		atomic: true,
		replaces: vec![],
	}
}
"#,
		);

		create_migration_file(
			temp_dir.path(),
			"users",
			"0001_initial",
			r#"
use reinhardt_migrations::prelude::*;

pub fn migration() -> Migration {
	Migration {
		app_label: "users",
		name: "0001_initial",
		operations: vec![],
		dependencies: vec![],
		atomic: true,
		replaces: vec![],
	}
}
"#,
		);

		let source = FilesystemSource::new(temp_dir.path());
		let migrations = source.all_migrations().await.unwrap();

		assert_eq!(migrations.len(), 2);
		assert!(migrations.iter().any(|m| m.app_label == "polls"));
		assert!(migrations.iter().any(|m| m.app_label == "users"));
	}

	#[tokio::test]
	#[serial(filesystem_source)]
	async fn test_filesystem_source_migrations_for_app() {
		let temp_dir = TempDir::new().unwrap();

		// Create test migration files
		create_migration_file(
			temp_dir.path(),
			"polls",
			"0001_initial",
			r#"
use reinhardt_migrations::prelude::*;

pub fn migration() -> Migration {
	Migration {
		app_label: "polls",
		name: "0001_initial",
		operations: vec![],
		dependencies: vec![],
		atomic: true,
		replaces: vec![],
	}
}
"#,
		);

		create_migration_file(
			temp_dir.path(),
			"polls",
			"0002_add_field",
			r#"
use reinhardt_migrations::prelude::*;

pub fn migration() -> Migration {
	Migration {
		app_label: "polls",
		name: "0002_add_field",
		operations: vec![],
		dependencies: vec![("polls", "0001_initial")],
		atomic: true,
		replaces: vec![],
	}
}
"#,
		);

		let source = FilesystemSource::new(temp_dir.path());
		let polls_migrations = source.migrations_for_app("polls").await.unwrap();

		assert_eq!(polls_migrations.len(), 2);
		assert!(polls_migrations.iter().all(|m| m.app_label == "polls"));
	}

	#[tokio::test]
	#[serial(filesystem_source)]
	async fn test_filesystem_source_get_migration() {
		let temp_dir = TempDir::new().unwrap();

		create_migration_file(
			temp_dir.path(),
			"polls",
			"0001_initial",
			r#"
use reinhardt_migrations::prelude::*;

pub fn migration() -> Migration {
	Migration {
		app_label: "polls",
		name: "0001_initial",
		operations: vec![],
		dependencies: vec![],
		atomic: true,
		replaces: vec![],
	}
}
"#,
		);

		let source = FilesystemSource::new(temp_dir.path());
		let migration = source.get_migration("polls", "0001_initial").await.unwrap();

		assert_eq!(migration.app_label, "polls");
		assert_eq!(migration.name, "0001_initial");
	}

	#[tokio::test]
	#[serial(filesystem_source)]
	async fn test_filesystem_source_get_migration_not_found() {
		let temp_dir = TempDir::new().unwrap();

		let source = FilesystemSource::new(temp_dir.path());
		let result = source.get_migration("polls", "0001_initial").await;

		assert!(result.is_err());
		assert!(matches!(result.unwrap_err(), MigrationError::NotFound(_)));
	}
}
