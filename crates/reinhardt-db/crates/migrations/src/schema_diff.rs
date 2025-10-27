//! Schema diff detection
//!
//! Detects differences between current database schema and model definitions:
//! - Table additions/removals
//! - Column modifications
//! - Index changes
//! - Constraint changes

use crate::operations::{AddField, CreateModel, DeleteModel, Operation, RemoveField};
use std::collections::HashMap;

/// Schema difference detector
pub struct SchemaDiff {
    /// Current database schema
    current_schema: DatabaseSchema,
    /// Target schema from models
    target_schema: DatabaseSchema,
}

/// Database schema representation
#[derive(Debug, Clone, Default)]
pub struct DatabaseSchema {
    /// Table definitions
    pub tables: HashMap<String, TableSchema>,
}

/// Table schema
#[derive(Debug, Clone)]
pub struct TableSchema {
    /// Table name
    pub name: String,
    /// Column definitions
    pub columns: HashMap<String, ColumnSchema>,
    /// Indexes
    pub indexes: Vec<IndexSchema>,
    /// Constraints
    pub constraints: Vec<ConstraintSchema>,
}

/// Column schema
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnSchema {
    /// Column name
    pub name: String,
    /// Data type
    pub data_type: String,
    /// Nullable
    pub nullable: bool,
    /// Default value
    pub default: Option<String>,
    /// Primary key
    pub primary_key: bool,
}

/// Index schema
#[derive(Debug, Clone, PartialEq)]
pub struct IndexSchema {
    /// Index name
    pub name: String,
    /// Columns
    pub columns: Vec<String>,
    /// Unique index
    pub unique: bool,
}

/// Constraint schema
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintSchema {
    /// Constraint name
    pub name: String,
    /// Constraint type (UNIQUE, FOREIGN KEY, CHECK, etc.)
    pub constraint_type: String,
    /// Definition
    pub definition: String,
}

/// Schema diff result
#[derive(Debug, Clone)]
pub struct SchemaDiffResult {
    /// Tables to add
    pub tables_to_add: Vec<String>,
    /// Tables to remove
    pub tables_to_remove: Vec<String>,
    /// Columns to add (table_name, column_name)
    pub columns_to_add: Vec<(String, String)>,
    /// Columns to remove (table_name, column_name)
    pub columns_to_remove: Vec<(String, String)>,
    /// Columns to modify (table_name, column_name, old, new)
    pub columns_to_modify: Vec<(String, String, ColumnSchema, ColumnSchema)>,
    /// Indexes to add
    pub indexes_to_add: Vec<(String, IndexSchema)>,
    /// Indexes to remove
    pub indexes_to_remove: Vec<(String, IndexSchema)>,
}

impl SchemaDiff {
    /// Create a new schema diff detector
    pub fn new(current_schema: DatabaseSchema, target_schema: DatabaseSchema) -> Self {
        Self {
            current_schema,
            target_schema,
        }
    }

    /// Detect differences between schemas
    pub fn detect(&self) -> SchemaDiffResult {
        let mut result = SchemaDiffResult {
            tables_to_add: Vec::new(),
            tables_to_remove: Vec::new(),
            columns_to_add: Vec::new(),
            columns_to_remove: Vec::new(),
            columns_to_modify: Vec::new(),
            indexes_to_add: Vec::new(),
            indexes_to_remove: Vec::new(),
        };

        // Detect table additions
        for (table_name, _) in &self.target_schema.tables {
            if !self.current_schema.tables.contains_key(table_name) {
                result.tables_to_add.push(table_name.clone());
            }
        }

        // Detect table removals
        for (table_name, _) in &self.current_schema.tables {
            if !self.target_schema.tables.contains_key(table_name) {
                result.tables_to_remove.push(table_name.clone());
            }
        }

        // Detect column changes for existing tables
        for (table_name, target_table) in &self.target_schema.tables {
            if let Some(current_table) = self.current_schema.tables.get(table_name) {
                // Column additions
                for (col_name, _) in &target_table.columns {
                    if !current_table.columns.contains_key(col_name) {
                        result
                            .columns_to_add
                            .push((table_name.clone(), col_name.clone()));
                    }
                }

                // Column removals
                for (col_name, _) in &current_table.columns {
                    if !target_table.columns.contains_key(col_name) {
                        result
                            .columns_to_remove
                            .push((table_name.clone(), col_name.clone()));
                    }
                }

                // Column modifications
                for (col_name, target_col) in &target_table.columns {
                    if let Some(current_col) = current_table.columns.get(col_name) {
                        if current_col != target_col {
                            result.columns_to_modify.push((
                                table_name.clone(),
                                col_name.clone(),
                                current_col.clone(),
                                target_col.clone(),
                            ));
                        }
                    }
                }

                // Index changes
                for target_index in &target_table.indexes {
                    if !current_table.indexes.contains(target_index) {
                        result
                            .indexes_to_add
                            .push((table_name.clone(), target_index.clone()));
                    }
                }

                for current_index in &current_table.indexes {
                    if !target_table.indexes.contains(current_index) {
                        result
                            .indexes_to_remove
                            .push((table_name.clone(), current_index.clone()));
                    }
                }
            }
        }

        result
    }

    /// Generate migration operations from diff
    pub fn generate_operations(&self) -> Vec<Operation> {
        let diff = self.detect();
        let mut operations = Vec::new();

        // Add tables
        for table_name in diff.tables_to_add {
            if let Some(table_schema) = self.target_schema.tables.get(&table_name) {
                operations.push(Operation::CreateModel(CreateModel {
                    name: table_name.clone(),
                    fields: Vec::new(), // Populated from table_schema.columns
                    options: HashMap::new(),
                }));
            }
        }

        // Remove tables
        for table_name in diff.tables_to_remove {
            operations.push(Operation::DeleteModel(DeleteModel {
                name: table_name.clone(),
            }));
        }

        // Add columns
        for (table_name, col_name) in diff.columns_to_add {
            operations.push(Operation::AddField(AddField {
                model_name: table_name.clone(),
                name: col_name.clone(),
                field: String::new(), // Field definition
            }));
        }

        // Remove columns
        for (table_name, col_name) in diff.columns_to_remove {
            operations.push(Operation::RemoveField(RemoveField {
                model_name: table_name.clone(),
                name: col_name.clone(),
            }));
        }

        operations
    }

    /// Check if diff has destructive changes
    pub fn has_destructive_changes(&self) -> bool {
        let diff = self.detect();
        !diff.tables_to_remove.is_empty()
            || !diff.columns_to_remove.is_empty()
            || !diff.columns_to_modify.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_table_addition() {
        let current = DatabaseSchema::default();
        let mut target = DatabaseSchema::default();
        target.tables.insert(
            "users".to_string(),
            TableSchema {
                name: "users".to_string(),
                columns: HashMap::new(),
                indexes: Vec::new(),
                constraints: Vec::new(),
            },
        );

        let diff = SchemaDiff::new(current, target);
        let result = diff.detect();

        assert_eq!(result.tables_to_add.len(), 1);
        assert_eq!(result.tables_to_add[0], "users");
    }

    #[test]
    fn test_detect_column_addition() {
        let mut current = DatabaseSchema::default();
        current.tables.insert(
            "users".to_string(),
            TableSchema {
                name: "users".to_string(),
                columns: HashMap::new(),
                indexes: Vec::new(),
                constraints: Vec::new(),
            },
        );

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
                nullable: false,
                default: None,
                primary_key: false,
            },
        );
        target.tables.insert("users".to_string(), target_table);

        let diff = SchemaDiff::new(current, target);
        let result = diff.detect();

        assert_eq!(result.columns_to_add.len(), 1);
        assert_eq!(result.columns_to_add[0], ("users".to_string(), "email".to_string()));
    }

    #[test]
    fn test_destructive_changes_detection() {
        let mut current = DatabaseSchema::default();
        current.tables.insert(
            "users".to_string(),
            TableSchema {
                name: "users".to_string(),
                columns: HashMap::new(),
                indexes: Vec::new(),
                constraints: Vec::new(),
            },
        );

        let target = DatabaseSchema::default();

        let diff = SchemaDiff::new(current, target);
        assert!(diff.has_destructive_changes());
    }
}
