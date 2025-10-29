//! Schema diff detection
//!
//! Detects differences between current database schema and model definitions:
//! - Table additions/removals
//! - Column modifications
//! - Index changes
//! - Constraint changes

use crate::ColumnDefinition;
use crate::operations::Operation;
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
    /// Auto increment
    pub auto_increment: bool,
    /// Maximum length (for VARCHAR, CHAR, etc.)
    pub max_length: Option<u32>,
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
                // Map columns to ColumnDefinition
                let columns: Vec<_> = table_schema
                    .columns
                    .iter()
                    .map(|(name, col)| {
                        let unique = self.extract_column_constraints(&table_name, name);
                        let auto_increment = Self::is_auto_increment(col);
                        let max_length = Self::extract_max_length(&col.data_type);

                        ColumnDefinition {
                            name: name.clone(),
                            type_definition: col.data_type.clone(),
                            not_null: !col.nullable,
                            default: col.default.clone(),
                            unique,
                            primary_key: col.primary_key,
                            auto_increment,
                            max_length,
                        }
                    })
                    .collect();

                // Extract table-level constraints
                let constraints = self.extract_constraints(&table_name);

                operations.push(Operation::CreateTable {
                    name: table_name.clone(),
                    columns,
                    constraints,
                });
            }
        }

        // Remove tables
        for table_name in diff.tables_to_remove {
            operations.push(Operation::DropTable {
                name: table_name.clone(),
            });
        }

        // Add columns
        for (table_name, col_name) in diff.columns_to_add {
            if let Some(table_schema) = self.target_schema.tables.get(&table_name) {
                if let Some(col_schema) = table_schema.columns.get(&col_name) {
                    let unique = self.extract_column_constraints(&table_name, &col_name);
                    let auto_increment = Self::is_auto_increment(col_schema);
                    let max_length = Self::extract_max_length(&col_schema.data_type);

                    operations.push(Operation::AddColumn {
                        table: table_name.clone(),
                        column: ColumnDefinition {
                            name: col_name.clone(),
                            type_definition: col_schema.data_type.clone(),
                            not_null: !col_schema.nullable,
                            default: col_schema.default.clone(),
                            unique,
                            primary_key: col_schema.primary_key,
                            auto_increment,
                            max_length,
                        },
                    });
                }
            }
        }

        // Remove columns
        for (table_name, col_name) in diff.columns_to_remove {
            operations.push(Operation::DropColumn {
                table: table_name.clone(),
                column: col_name.clone(),
            });
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

    /// Extract column-level constraints from table constraints and indexes
    fn extract_column_constraints(&self, table_name: &str, column_name: &str) -> bool {
        let table_schema = match self.target_schema.tables.get(table_name) {
            Some(t) => t,
            None => return false,
        };

        // Check if column is part of a unique constraint
        let has_unique_constraint = table_schema.constraints.iter().any(|constraint| {
            constraint.constraint_type.to_uppercase() == "UNIQUE"
                && constraint.definition.contains(column_name)
        });

        // Check if column has unique index (single-column unique index)
        let has_unique_index = table_schema.indexes.iter().any(|index| {
            index.unique && index.columns.len() == 1 && index.columns[0] == column_name
        });

        has_unique_constraint || has_unique_index
    }

    /// Extract max_length from column data type
    fn extract_max_length(data_type: &str) -> Option<u32> {
        let upper_type = data_type.to_uppercase();

        // Match VARCHAR(N), CHAR(N) patterns
        if upper_type.contains("VARCHAR") || upper_type.contains("CHAR") {
            if let Some(start) = data_type.find('(') {
                if let Some(end) = data_type.find(')') {
                    if let Ok(length) = data_type[start + 1..end].parse::<u32>() {
                        return Some(length);
                    }
                }
            }
        }

        // Match DECIMAL(P, S), NUMERIC(P, S) patterns - extract precision (P)
        if upper_type.contains("DECIMAL") || upper_type.contains("NUMERIC") {
            if let Some(start) = data_type.find('(') {
                if let Some(comma_pos) = data_type.find(',') {
                    if comma_pos > start + 1 {
                        if let Ok(precision) = data_type[start + 1..comma_pos].trim().parse::<u32>()
                        {
                            return Some(precision);
                        }
                    }
                }
            }
        }

        None
    }

    /// Detect if column is auto-increment based on column properties and data type
    fn is_auto_increment(column: &ColumnSchema) -> bool {
        // If already marked as auto_increment, trust it
        if column.auto_increment {
            return true;
        }

        let upper_type = column.data_type.to_uppercase();

        // PostgreSQL SERIAL types (SMALLSERIAL, SERIAL, BIGSERIAL)
        if upper_type.contains("SERIAL") {
            return true;
        }

        // MySQL AUTO_INCREMENT (typically in extra metadata, but check data type comments)
        if upper_type.contains("AUTO_INCREMENT") {
            return true;
        }

        // SQLite: INTEGER PRIMARY KEY is auto-increment by default
        if column.primary_key && (upper_type == "INTEGER" || upper_type == "INT") {
            return true;
        }

        false
    }

    /// Extract table-level constraint definitions
    fn extract_constraints(&self, table_name: &str) -> Vec<String> {
        let table_schema = match self.target_schema.tables.get(table_name) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut constraints = Vec::new();

        // Extract PRIMARY KEY constraint
        let pk_columns: Vec<String> = table_schema
            .columns
            .iter()
            .filter_map(|(name, col)| {
                if col.primary_key {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        if pk_columns.len() > 1 {
            // Composite primary key
            let pk_constraint = format!(
                "CONSTRAINT {}_pkey PRIMARY KEY ({})",
                table_name,
                pk_columns.join(", ")
            );
            constraints.push(pk_constraint);
        }

        // Extract UNIQUE constraints from indexes (multi-column unique indexes)
        for index in &table_schema.indexes {
            if index.unique && index.columns.len() > 1 {
                let unique_constraint = format!(
                    "CONSTRAINT {} UNIQUE ({})",
                    index.name,
                    index.columns.join(", ")
                );
                constraints.push(unique_constraint);
            }
        }

        // Extract constraints from constraint definitions
        for constraint in &table_schema.constraints {
            constraints.push(constraint.definition.clone());
        }

        constraints
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
                auto_increment: false,
                max_length: Some(255),
            },
        );
        target.tables.insert("users".to_string(), target_table);

        let diff = SchemaDiff::new(current, target);
        let result = diff.detect();

        assert_eq!(result.columns_to_add.len(), 1);
        assert_eq!(
            result.columns_to_add[0],
            ("users".to_string(), "email".to_string())
        );
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
