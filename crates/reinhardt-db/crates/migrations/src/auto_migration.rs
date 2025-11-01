//! Auto-migration generation
//!
//! Automatically generates migration files from model definitions:
//! - Detects schema changes
//! - Generates migration operations
//! - Creates rollback scripts

use crate::migration::Migration;
use crate::operations::Operation;
use crate::schema_diff::{DatabaseSchema, SchemaDiff};
use crate::writer::MigrationWriter;
use std::path::PathBuf;

/// Auto-migration generator
pub struct AutoMigrationGenerator {
	/// Target schema from models
	target_schema: DatabaseSchema,
	/// Migration output directory
	output_dir: PathBuf,
}

/// Auto-migration result
#[derive(Debug)]
pub struct AutoMigrationResult {
	/// Generated migration file path
	pub migration_file: PathBuf,
	/// Rollback file path
	pub rollback_file: Option<PathBuf>,
	/// Number of operations generated
	pub operation_count: usize,
	/// Has destructive changes
	pub has_destructive_changes: bool,
}

impl AutoMigrationGenerator {
	/// Create a new auto-migration generator
	pub fn new(target_schema: DatabaseSchema, output_dir: PathBuf) -> Self {
		Self {
			target_schema,
			output_dir,
		}
	}

	/// Generate migration from current database state
	pub async fn generate(
		&self,
		current_schema: DatabaseSchema,
	) -> Result<AutoMigrationResult, AutoMigrationError> {
		// Detect schema differences
		let diff = SchemaDiff::new(current_schema.clone(), self.target_schema.clone());
		let operations = diff.generate_operations();

		if operations.is_empty() {
			return Err(AutoMigrationError::NoChangesDetected);
		}

		let has_destructive = diff.has_destructive_changes();

		// Generate migration name based on timestamp
		let migration_name = format!(
			"auto_migration_{}",
			chrono::Utc::now().format("%Y%m%d_%H%M%S")
		);

		// Create migration
		let migration = Migration {
			name: migration_name.clone(),
			app_label: "auto".to_string(),
			dependencies: Vec::new(),
			operations: operations.clone(),
			replaces: Vec::new(),
			atomic: true,
		};

		// Write migration file
		let writer = MigrationWriter::new(migration.clone());
		let migration_file = writer
			.write_to_file(&self.output_dir)
			.map_err(|e| AutoMigrationError::WriteError(e.to_string()))?;

		// Generate rollback operations
		let rollback_operations = self.generate_rollback(&operations);
		let rollback_file = if !rollback_operations.is_empty() {
			let rollback_migration = Migration {
				name: format!("{}_rollback", migration_name),
				app_label: "auto".to_string(),
				dependencies: vec![("auto".to_string(), migration_name)],
				operations: rollback_operations,
				replaces: Vec::new(),
				atomic: true,
			};
			let rollback_writer = MigrationWriter::new(rollback_migration);
			Some(
				rollback_writer
					.write_to_file(&self.output_dir)
					.map_err(|e| AutoMigrationError::WriteError(e.to_string()))?,
			)
		} else {
			None
		};

		Ok(AutoMigrationResult {
			migration_file: std::path::PathBuf::from(migration_file),
			rollback_file: rollback_file.map(std::path::PathBuf::from),
			operation_count: operations.len(),
			has_destructive_changes: has_destructive,
		})
	}

	/// Generate rollback operations
	fn generate_rollback(&self, operations: &[Operation]) -> Vec<Operation> {
		operations
			.iter()
			.rev()
			.filter_map(|op| match op {
				// Table operations
				Operation::CreateTable { name, .. } => {
					Some(Operation::DropTable { name: name.clone() })
				}
				Operation::DropTable { .. } => None, // Cannot rollback - data is lost
				Operation::RenameTable { old_name, new_name } => Some(Operation::RenameTable {
					old_name: new_name.clone(),
					new_name: old_name.clone(),
				}),

				// Column operations
				Operation::AddColumn { table, column } => Some(Operation::DropColumn {
					table: table.clone(),
					column: column.name.clone(),
				}),
				Operation::DropColumn { .. } => None, // Cannot rollback - data is lost
				Operation::RenameColumn {
					table,
					old_name,
					new_name,
				} => Some(Operation::RenameColumn {
					table: table.clone(),
					old_name: new_name.clone(),
					new_name: old_name.clone(),
				}),
				Operation::AlterColumn { .. } => None, // Cannot safely rollback without old definition

				// Constraint operations
				Operation::AddConstraint { .. } => None, // Cannot rollback without constraint name
				Operation::DropConstraint { .. } => None, // Cannot rollback without constraint SQL

				// Index operations
				Operation::CreateIndex { table, columns, .. } => Some(Operation::DropIndex {
					table: table.clone(),
					columns: columns.clone(),
				}),
				Operation::DropIndex { .. } => None, // Cannot rollback without index definition

				// Special operations
				Operation::RunSQL { reverse_sql, .. } => {
					reverse_sql.as_ref().map(|sql| Operation::RunSQL {
						sql: sql.clone(),
						reverse_sql: None,
					})
				}
				Operation::RunRust { reverse_code, .. } => {
					reverse_code.as_ref().map(|code| Operation::RunRust {
						code: code.clone(),
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
}

impl From<std::io::Error> for AutoMigrationError {
	fn from(err: std::io::Error) -> Self {
		AutoMigrationError::WriteError(err.to_string())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::collections::HashMap;

	#[test]
	fn test_rollback_generation() {
		let target_schema = DatabaseSchema::default();
		let generator = AutoMigrationGenerator::new(target_schema, PathBuf::from("/tmp"));

		let operations = vec![Operation::CreateTable {
			name: "users".to_string(),
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
			name: "users".to_string(),
			columns: HashMap::new(),
			indexes: Vec::new(),
			constraints: Vec::new(),
		};
		current_table.columns.insert(
			"email".to_string(),
			ColumnSchema {
				name: "email".to_string(),
				data_type: "VARCHAR(255)".to_string(),
				nullable: true,
				default: None,
				primary_key: false,
				auto_increment: false,
				max_length: Some(255),
			},
		);
		current.tables.insert("users".to_string(), current_table);

		let mut target = DatabaseSchema::default();
		let mut target_table = TableSchema {
			name: "users".to_string(),
			columns: HashMap::new(),
			indexes: Vec::new(),
			constraints: Vec::new(),
		};
		target_table.columns.insert(
			"email".to_string(),
			ColumnSchema {
				name: "email".to_string(),
				data_type: "VARCHAR(255)".to_string(),
				nullable: false, // Changed to non-nullable
				default: None,
				primary_key: false,
				auto_increment: false,
				max_length: Some(255),
			},
		);
		target.tables.insert("users".to_string(), target_table);

		let generator = AutoMigrationGenerator::new(target, PathBuf::from("/tmp"));
		let validation = generator.validate(current);

		assert!(!validation.is_valid);
		assert!(!validation.errors.is_empty());
	}
}
