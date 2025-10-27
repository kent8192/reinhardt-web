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
    pub async fn generate(&self, current_schema: DatabaseSchema) -> Result<AutoMigrationResult, AutoMigrationError> {
        // Detect schema differences
        let diff = SchemaDiff::new(current_schema.clone(), self.target_schema.clone());
        let operations = diff.generate_operations();

        if operations.is_empty() {
            return Err(AutoMigrationError::NoChangesDetected);
        }

        let has_destructive = diff.has_destructive_changes();

        // Generate migration name based on timestamp
        let migration_name = format!("auto_migration_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));

        // Create migration
        let migration = Migration {
            name: migration_name.clone(),
            app: "auto".to_string(),
            dependencies: Vec::new(),
            operations: operations.clone(),
        };

        // Write migration file
        let writer = MigrationWriter::new(self.output_dir.clone());
        let migration_file = writer.write(&migration)?;

        // Generate rollback operations
        let rollback_operations = self.generate_rollback(&operations);
        let rollback_file = if !rollback_operations.is_empty() {
            let rollback_migration = Migration {
                name: format!("{}_rollback", migration_name),
                app: "auto".to_string(),
                dependencies: vec![migration_name],
                operations: rollback_operations,
            };
            Some(writer.write(&rollback_migration)?)
        } else {
            None
        };

        Ok(AutoMigrationResult {
            migration_file,
            rollback_file,
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
                Operation::CreateModel(create) => Some(Operation::DeleteModel(crate::operations::DeleteModel {
                    name: create.name.clone(),
                })),
                Operation::DeleteModel(_) => None, // Cannot rollback delete
                Operation::AddField(add) => Some(Operation::RemoveField(crate::operations::RemoveField {
                    model_name: add.model_name.clone(),
                    name: add.name.clone(),
                })),
                Operation::RemoveField(_) => None, // Cannot rollback remove
                _ => None,
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
    use crate::operations::CreateModel;
    use std::collections::HashMap;

    #[test]
    fn test_rollback_generation() {
        let target_schema = DatabaseSchema::default();
        let generator = AutoMigrationGenerator::new(target_schema, PathBuf::from("/tmp"));

        let operations = vec![Operation::CreateModel(CreateModel {
            name: "users".to_string(),
            fields: Vec::new(),
            options: HashMap::new(),
        })];

        let rollback = generator.generate_rollback(&operations);
        assert_eq!(rollback.len(), 1);
        assert!(matches!(rollback[0], Operation::DeleteModel(_)));
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
            },
        );
        target.tables.insert("users".to_string(), target_table);

        let generator = AutoMigrationGenerator::new(target, PathBuf::from("/tmp"));
        let validation = generator.validate(current);

        assert!(!validation.is_valid);
        assert!(!validation.errors.is_empty());
    }
}
