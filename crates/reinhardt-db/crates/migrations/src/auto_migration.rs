//! Auto-migration generation
//!
//! Automatically generates migration files from model definitions:
//! - Detects schema changes
//! - Generates migration operations
//! - Creates rollback scripts

use crate::migration::Migration;
use crate::migration_namer::MigrationNamer;
use crate::operations::Operation;
use crate::repository::MigrationRepository;
use crate::schema_diff::{DatabaseSchema, SchemaDiff};
use std::path::PathBuf;
use std::sync::Arc;

/// Auto-migration generator
pub struct AutoMigrationGenerator {
	/// Target schema from models
	target_schema: DatabaseSchema,
	/// Migration output directory
	output_dir: PathBuf,
	/// Repository for checking existing migrations and saving new ones
	repository: Arc<tokio::sync::Mutex<dyn MigrationRepository>>,
}

/// Auto-migration result
#[derive(Debug)]
pub struct AutoMigrationResult {
	/// Generated migration file path
	pub migration_file: PathBuf,
	/// Rollback file path
	pub rollback_file: Option<PathBuf>,
	/// Generated operations
	pub operations: Vec<Operation>,
	/// Number of operations generated
	pub operation_count: usize,
	/// Has destructive changes
	pub has_destructive_changes: bool,
}

impl AutoMigrationGenerator {
	/// Create a new auto-migration generator
	///
	/// # Arguments
	///
	/// * `target_schema` - The target schema from model definitions
	/// * `output_dir` - Directory where migration files will be written
	/// * `repository` - Repository for checking existing migrations and saving new ones
	pub fn new(
		target_schema: DatabaseSchema,
		output_dir: PathBuf,
		repository: Arc<tokio::sync::Mutex<dyn MigrationRepository>>,
	) -> Self {
		Self {
			target_schema,
			output_dir,
			repository,
		}
	}

	/// Generate migration from current database state
	///
	/// # Arguments
	///
	/// * `app_label` - The app label for the migration
	/// * `current_schema` - Current database schema
	///
	/// # Errors
	///
	/// Returns `AutoMigrationError::NoChangesDetected` if no changes are detected.
	/// Returns `AutoMigrationError::DuplicateMigration` if identical operations already exist.
	pub async fn generate(
		&self,
		app_label: &str,
		current_schema: DatabaseSchema,
	) -> Result<AutoMigrationResult, AutoMigrationError> {
		// Detect schema differences
		let diff = SchemaDiff::new(current_schema.clone(), self.target_schema.clone());
		let operations = diff.generate_operations();

		if operations.is_empty() {
			return Err(AutoMigrationError::NoChangesDetected);
		}

		// Get existing migrations from repository
		let repo = self.repository.lock().await;
		let existing_migrations = repo
			.list(app_label)
			.await
			.map_err(|e| AutoMigrationError::WriteError(e.to_string()))?;

		// Check for duplicate operations (compare with last migration)
		if let Some(last_migration) = existing_migrations.last()
			&& self.is_duplicate_operations(&operations, &last_migration.operations)
		{
			return Err(AutoMigrationError::DuplicateMigration);
		}

		// Generate migration name using MigrationNamer
		let migration_name = MigrationNamer::auto_name();

		// Check if migration with this name already exists
		if repo
			.exists(app_label, &migration_name)
			.await
			.map_err(|e| AutoMigrationError::WriteError(e.to_string()))?
		{
			return Err(AutoMigrationError::DuplicateMigration);
		}
		drop(repo);

		let has_destructive = diff.has_destructive_changes();

		// Create migration
		let migration = Migration {
			name: Box::leak(migration_name.clone().into_boxed_str()),
			app_label: Box::leak(app_label.to_string().into_boxed_str()),
			dependencies: Vec::new(),
			operations: operations.clone(),
			replaces: Vec::new(),
			atomic: true,
			initial: None,
		};

		// Save migration via repository
		let mut repo = self.repository.lock().await;
		repo.save(&migration)
			.await
			.map_err(|e| AutoMigrationError::WriteError(e.to_string()))?;
		drop(repo);

		let migration_file = self
			.output_dir
			.join(app_label)
			.join("migrations")
			.join(format!("{}.rs", migration_name));

		// Generate rollback operations
		let rollback_operations = self.generate_rollback(&operations);
		let rollback_file = if !rollback_operations.is_empty() {
			Some(
				self.output_dir
					.join(app_label)
					.join("migrations")
					.join(format!("{}_rollback.rs", migration_name)),
			)
		} else {
			None
		};

		Ok(AutoMigrationResult {
			migration_file,
			rollback_file,
			operations: operations.clone(),
			operation_count: operations.len(),
			has_destructive_changes: has_destructive,
		})
	}

	/// Check if operations are duplicates
	///
	/// Compares operations using semantic equality, which normalizes order-independent
	/// elements before comparison. This allows detection of duplicate migrations even
	/// when the order of columns, constraints, or other elements differs.
	fn is_duplicate_operations(&self, ops1: &[Operation], ops2: &[Operation]) -> bool {
		use crate::operation_trait::MigrationOperation;

		if ops1.len() != ops2.len() {
			return false;
		}

		// Semantic equality check using normalize() and comparison
		ops1.iter()
			.zip(ops2.iter())
			.all(|(op1, op2)| op1.semantically_equal(op2))
	}

	/// Generate rollback operations
	fn generate_rollback(&self, operations: &[Operation]) -> Vec<Operation> {
		operations
			.iter()
			.rev()
			.filter_map(|op| match op {
				// Table operations
				Operation::CreateTable { name, .. } => Some(Operation::DropTable { name }),
				Operation::DropTable { .. } => None, // Cannot rollback - data is lost
				Operation::RenameTable { old_name, new_name } => Some(Operation::RenameTable {
					old_name: new_name,
					new_name: old_name,
				}),

				// Column operations
				Operation::AddColumn { table, column } => Some(Operation::DropColumn {
					table,
					column: column.name,
				}),
				Operation::DropColumn { .. } => None, // Cannot rollback - data is lost
				Operation::RenameColumn {
					table,
					old_name,
					new_name,
				} => Some(Operation::RenameColumn {
					table,
					old_name: new_name,
					new_name: old_name,
				}),
				Operation::AlterColumn { .. } => None, // Cannot safely rollback without old definition

				// Constraint operations
				Operation::AddConstraint { .. } => None, // Cannot rollback without constraint name
				Operation::DropConstraint { .. } => None, // Cannot rollback without constraint SQL

				// Index operations
				Operation::CreateIndex { table, columns, .. } => Some(Operation::DropIndex {
					table,
					columns: columns.clone(),
				}),
				Operation::DropIndex { .. } => None, // Cannot rollback without index definition

				// Special operations
				Operation::RunSQL { reverse_sql, .. } => {
					reverse_sql.as_ref().map(|sql| Operation::RunSQL {
						sql,
						reverse_sql: None,
					})
				}
				Operation::RunRust { reverse_code, .. } => {
					reverse_code.as_ref().map(|code| Operation::RunRust {
						code,
						reverse_code: None,
					})
				}

				// Other operations - no rollback
				Operation::AlterTableComment { .. }
				| Operation::AlterUniqueTogether { .. }
				| Operation::AlterModelOptions { .. }
				| Operation::CreateInheritedTable { .. }
				| Operation::AddDiscriminatorColumn { .. } => None,
			})
			.collect()
	}

	/// Validate migration before generation
	pub fn validate(&self, current_schema: DatabaseSchema) -> ValidationResult {
		let diff = SchemaDiff::new(current_schema, self.target_schema.clone());
		let diff_result = diff.detect();

		let mut warnings = Vec::new();
		let mut errors = Vec::new();

		// Check for data loss risks
		if !diff_result.tables_to_remove.is_empty() {
			warnings.push(format!(
				"Removing tables will cause data loss: {:?}",
				diff_result.tables_to_remove
			));
		}

		if !diff_result.columns_to_remove.is_empty() {
			warnings.push(format!(
				"Removing columns will cause data loss: {:?}",
				diff_result.columns_to_remove
			));
		}

		// Check for column type changes
		for (table, col, old, new) in &diff_result.columns_to_modify {
			if old.data_type != new.data_type {
				warnings.push(format!(
					"Column type change in {}.{}: {} -> {} (may cause data loss or conversion errors)",
					table, col, old.data_type, new.data_type
				));
			}

			if old.nullable && !new.nullable {
				errors.push(format!(
                    "Column {}.{} changed from nullable to non-nullable (requires default value or data migration)",
                    table, col
                ));
			}
		}

		ValidationResult {
			is_valid: errors.is_empty(),
			warnings,
			errors,
		}
	}
}

/// Validation result
#[derive(Debug)]
pub struct ValidationResult {
	/// Is migration valid
	pub is_valid: bool,
	/// Warnings (non-blocking)
	pub warnings: Vec<String>,
	/// Errors (blocking)
	pub errors: Vec<String>,
}

/// Auto-migration error
#[derive(Debug, thiserror::Error)]
pub enum AutoMigrationError {
	#[error("No schema changes detected")]
	NoChangesDetected,

	#[error("Failed to write migration file: {0}")]
	WriteError(String),

	#[error("Migration validation failed: {0}")]
	ValidationError(String),

	#[error(
		"Duplicate migration detected: generated operations are identical to the last migration.\nThis usually means you're trying to generate the same migration twice.\nIf you need to modify the previous migration, delete it first and run makemigrations again."
	)]
	DuplicateMigration,
}

impl From<std::io::Error> for AutoMigrationError {
	fn from(err: std::io::Error) -> Self {
		AutoMigrationError::WriteError(err.to_string())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{FieldType, repository::MigrationRepository};
	use async_trait::async_trait;
	use std::collections::{BTreeMap, HashMap};
	use tokio::sync::Mutex;

	/// Test repository implementation
	struct TestRepository {
		migrations: HashMap<(String, String), Migration>,
	}

	impl TestRepository {
		fn new() -> Self {
			Self {
				migrations: HashMap::new(),
			}
		}
	}

	#[async_trait]
	impl MigrationRepository for TestRepository {
		async fn save(&mut self, migration: &Migration) -> crate::Result<()> {
			let key = (migration.app_label.to_string(), migration.name.to_string());
			self.migrations.insert(key, migration.clone());
			Ok(())
		}

		async fn get(&self, app_label: &str, name: &str) -> crate::Result<Migration> {
			let key = (app_label.to_string(), name.to_string());
			self.migrations
				.get(&key)
				.cloned()
				.ok_or_else(|| crate::MigrationError::NotFound(format!("{}.{}", app_label, name)))
		}

		async fn list(&self, app_label: &str) -> crate::Result<Vec<Migration>> {
			Ok(self
				.migrations
				.values()
				.filter(|m| m.app_label == app_label)
				.cloned()
				.collect())
		}

		async fn exists(&self, app_label: &str, name: &str) -> crate::Result<bool> {
			Ok(self
				.get(app_label, name)
				.await
				.map(|_| true)
				.unwrap_or(false))
		}
	}

	#[test]
	fn test_rollback_generation() {
		let target_schema = DatabaseSchema::default();
		let repo = Arc::new(Mutex::new(TestRepository::new()));
		let generator = AutoMigrationGenerator::new(target_schema, PathBuf::from("/tmp"), repo);

		let operations = vec![Operation::CreateTable {
			name: "users",
			columns: Vec::new(),
			constraints: Vec::new(),
		}];

		let rollback = generator.generate_rollback(&operations);
		assert_eq!(rollback.len(), 1);
		assert!(matches!(rollback[0], Operation::DropTable { .. }));
	}

	#[test]
	fn test_validation_nullable_change() {
		use crate::schema_diff::{ColumnSchema, TableSchema};

		let mut current = DatabaseSchema::default();
		let mut current_table = TableSchema {
			name: "users",
			columns: BTreeMap::new(),
			indexes: Vec::new(),
			constraints: Vec::new(),
		};
		current_table.columns.insert(
			"email".to_string(),
			ColumnSchema {
				name: "email",
				data_type: FieldType::VarChar(255),
				nullable: true,
				default: None,
				primary_key: false,
				auto_increment: false,
			},
		);
		current.tables.insert("users".to_string(), current_table);

		let mut target = DatabaseSchema::default();
		let mut target_table = TableSchema {
			name: "users",
			columns: BTreeMap::new(),
			indexes: Vec::new(),
			constraints: Vec::new(),
		};
		target_table.columns.insert(
			"email".to_string(),
			ColumnSchema {
				name: "email",
				data_type: FieldType::VarChar(255),
				nullable: false, // Changed to non-nullable
				default: None,
				primary_key: false,
				auto_increment: false,
			},
		);
		target.tables.insert("users".to_string(), target_table);

		let repo = Arc::new(Mutex::new(TestRepository::new()));
		let generator = AutoMigrationGenerator::new(target, PathBuf::from("/tmp"), repo);
		let validation = generator.validate(current);

		assert!(!validation.is_valid);
		assert!(!validation.errors.is_empty());
	}
}
