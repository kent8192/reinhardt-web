//! Migration operations
//!
//! This module provides various migration operations inspired by Django's migration system.
//! Operations are organized into three categories:
//!
//! - **Model operations** (`models`): Create, delete, and rename models (tables)
//! - **Field operations** (`fields`): Add, remove, alter, and rename fields (columns)
//! - **Special operations** (`special`): Run raw SQL or custom code
//!
//! # Example
//!
//! ```rust
//! use reinhardt_migrations::operations::{
//!     models::{CreateModel, DeleteModel},
//!     fields::{AddField, RemoveField},
//!     special::RunSQL,
//!     FieldDefinition,
//! };
//! use reinhardt_migrations::ProjectState;
//!
//! let mut state = ProjectState::new();
//!
//! // Create a model
//! let create = CreateModel::new(
//!     "User",
//!     vec![
//!         FieldDefinition::new("id", "INTEGER", true, false, None),
//!         FieldDefinition::new("name", "VARCHAR(100)", false, false, None),
//!     ],
//! );
//! create.state_forwards("myapp", &mut state);
//!
//! // Add a field
//! let add = AddField::new("User", FieldDefinition::new("email", "VARCHAR(255)", false, false, None));
//! add.state_forwards("myapp", &mut state);
//!
//! // Run custom SQL
//! let sql = RunSQL::new("CREATE INDEX idx_email ON myapp_user(email)");
//! ```

pub mod fields;
pub mod models;
pub mod postgres;
pub mod special;

// Re-export commonly used types for convenience
pub use fields::{AddField, AlterField, RemoveField, RenameField};
pub use models::{CreateModel, DeleteModel, FieldDefinition, RenameModel};
pub use postgres::{CreateCollation, CreateExtension, DropExtension};
pub use special::{RunCode, RunSQL, StateOperation};

// Legacy types for backward compatibility
// These are maintained from the original operations.rs
use crate::{FieldState, ModelState, ProjectState};
use serde::{Deserialize, Serialize};

/// A migration operation (legacy enum for backward compatibility)
///
/// This enum is maintained for backward compatibility with existing code.
/// New code should use the specific operation types from the `models`, `fields`,
/// and `special` modules instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Operation {
    CreateTable {
        name: String,
        columns: Vec<ColumnDefinition>,
        #[serde(default)]
        constraints: Vec<String>,
    },
    DropTable {
        name: String,
    },
    AddColumn {
        table: String,
        column: ColumnDefinition,
    },
    DropColumn {
        table: String,
        column: String,
    },
    AlterColumn {
        table: String,
        column: String,
        new_definition: ColumnDefinition,
    },
    RenameTable {
        old_name: String,
        new_name: String,
    },
    RenameColumn {
        table: String,
        old_name: String,
        new_name: String,
    },
    AddConstraint {
        table: String,
        constraint_sql: String,
    },
    DropConstraint {
        table: String,
        constraint_name: String,
    },
    CreateIndex {
        table: String,
        columns: Vec<String>,
        unique: bool,
    },
    DropIndex {
        table: String,
        columns: Vec<String>,
    },
    RunSQL {
        sql: String,
        reverse_sql: Option<String>,
    },
    AlterTableComment {
        table: String,
        comment: Option<String>,
    },
    AlterUniqueTogether {
        table: String,
        unique_together: Vec<Vec<String>>,
    },
    AlterModelOptions {
        table: String,
        options: std::collections::HashMap<String, String>,
    },
    CreateInheritedTable {
        name: String,
        columns: Vec<ColumnDefinition>,
        base_table: String,
        join_column: String,
    },
    AddDiscriminatorColumn {
        table: String,
        column_name: String,
        default_value: String,
    },
}

impl Operation {
    /// Apply this operation to the project state (forward)
    pub fn state_forwards(&self, app_label: &str, state: &mut ProjectState) {
        match self {
            Operation::CreateTable { name, columns, .. } => {
                let mut model = ModelState::new(app_label, name);
                for column in columns {
                    let field =
                        FieldState::new(column.name.clone(), column.type_definition.clone(), false);
                    model.add_field(field);
                }
                state.add_model(model);
            }
            Operation::DropTable { name } => {
                state.remove_model(app_label, name);
            }
            Operation::AddColumn { table, column } => {
                if let Some(model) = state.get_model_mut(app_label, table) {
                    let field =
                        FieldState::new(column.name.clone(), column.type_definition.clone(), false);
                    model.add_field(field);
                }
            }
            Operation::DropColumn { table, column } => {
                if let Some(model) = state.get_model_mut(app_label, table) {
                    model.remove_field(column);
                }
            }
            Operation::AlterColumn {
                table,
                column,
                new_definition,
            } => {
                if let Some(model) = state.get_model_mut(app_label, table) {
                    let field = FieldState::new(
                        column.clone(),
                        new_definition.type_definition.clone(),
                        false,
                    );
                    model.alter_field(column, field);
                }
            }
            Operation::RenameTable { old_name, new_name } => {
                state.rename_model(app_label, old_name, new_name.clone());
            }
            Operation::RenameColumn {
                table,
                old_name,
                new_name,
            } => {
                if let Some(model) = state.get_model_mut(app_label, table) {
                    model.rename_field(old_name, new_name.clone());
                }
            }
            Operation::CreateInheritedTable {
                name,
                columns,
                base_table,
                join_column,
            } => {
                let mut model = ModelState::new(app_label, name);
                model.base_model = Some(base_table.clone());
                model.inheritance_type = Some("joined_table".to_string());

                let join_field = FieldState::new(
                    join_column.clone(),
                    format!("INTEGER REFERENCES {}(id)", base_table),
                    false,
                );
                model.add_field(join_field);

                for column in columns {
                    let field =
                        FieldState::new(column.name.clone(), column.type_definition.clone(), false);
                    model.add_field(field);
                }
                state.add_model(model);
            }
            Operation::AddDiscriminatorColumn {
                table,
                column_name,
                default_value,
            } => {
                if let Some(model) = state.get_model_mut(app_label, table) {
                    model.discriminator_column = Some(column_name.clone());
                    model.inheritance_type = Some("single_table".to_string());
                    let field = FieldState::new(
                        column_name.clone(),
                        format!("VARCHAR(50) DEFAULT '{}'", default_value),
                        false,
                    );
                    model.add_field(field);
                }
            }
            Operation::AddConstraint { .. }
            | Operation::DropConstraint { .. }
            | Operation::CreateIndex { .. }
            | Operation::DropIndex { .. }
            | Operation::RunSQL { .. }
            | Operation::AlterTableComment { .. }
            | Operation::AlterUniqueTogether { .. }
            | Operation::AlterModelOptions { .. } => {}
        }
    }

    /// Generate forward SQL
    pub fn to_sql(&self, dialect: &SqlDialect) -> String {
        match self {
            Operation::CreateTable {
                name,
                columns,
                constraints,
            } => {
                let mut parts = Vec::new();
                for col in columns {
                    parts.push(format!("  {} {}", col.name, col.type_definition));
                }
                for constraint in constraints {
                    parts.push(format!("  {}", constraint));
                }
                format!("CREATE TABLE {} (\n{}\n);", name, parts.join(",\n"))
            }
            Operation::DropTable { name } => format!("DROP TABLE {};", name),
            Operation::AddColumn { table, column } => {
                format!(
                    "ALTER TABLE {} ADD COLUMN {} {};",
                    table, column.name, column.type_definition
                )
            }
            Operation::DropColumn { table, column } => {
                format!("ALTER TABLE {} DROP COLUMN {};", table, column)
            }
            Operation::AlterColumn {
                table,
                column,
                new_definition,
            } => match dialect {
                SqlDialect::Postgres => {
                    format!(
                        "ALTER TABLE {} ALTER COLUMN {} TYPE {};",
                        table, column, new_definition.type_definition
                    )
                }
                SqlDialect::Mysql => {
                    format!(
                        "ALTER TABLE {} MODIFY COLUMN {} {};",
                        table, column, new_definition.type_definition
                    )
                }
                SqlDialect::Sqlite => {
                    format!(
                        "-- SQLite does not support ALTER COLUMN, table recreation required for {}",
                        table
                    )
                }
            },
            Operation::RenameColumn {
                table,
                old_name,
                new_name,
            } => {
                format!(
                    "ALTER TABLE {} RENAME COLUMN {} TO {};",
                    table, old_name, new_name
                )
            }
            Operation::RenameTable { old_name, new_name } => {
                format!("ALTER TABLE {} RENAME TO {};", old_name, new_name)
            }
            Operation::AddConstraint {
                table,
                constraint_sql,
            } => {
                format!("ALTER TABLE {} ADD {};", table, constraint_sql)
            }
            Operation::DropConstraint {
                table,
                constraint_name,
            } => {
                format!("ALTER TABLE {} DROP CONSTRAINT {};", table, constraint_name)
            }
            Operation::CreateIndex {
                table,
                columns,
                unique,
            } => {
                let unique_str = if *unique { "UNIQUE " } else { "" };
                let idx_name = format!("idx_{}_{}", table, columns.join("_"));
                format!(
                    "CREATE {}INDEX {} ON {} ({});",
                    unique_str,
                    idx_name,
                    table,
                    columns.join(", ")
                )
            }
            Operation::DropIndex { table, columns } => {
                let idx_name = format!("idx_{}_{}", table, columns.join("_"));
                format!("DROP INDEX {};", idx_name)
            }
            Operation::RunSQL { sql, .. } => sql.clone(),
            Operation::AlterTableComment { table, comment } => match dialect {
                SqlDialect::Postgres => {
                    if let Some(comment_text) = comment {
                        format!("COMMENT ON TABLE {} IS '{}';", table, comment_text)
                    } else {
                        format!("COMMENT ON TABLE {} IS NULL;", table)
                    }
                }
                SqlDialect::Mysql => {
                    if let Some(comment_text) = comment {
                        format!("ALTER TABLE {} COMMENT='{}';", table, comment_text)
                    } else {
                        format!("ALTER TABLE {} COMMENT='';", table)
                    }
                }
                SqlDialect::Sqlite => String::new(),
            },
            Operation::AlterUniqueTogether {
                table,
                unique_together,
            } => {
                let mut sql = Vec::new();
                for (idx, fields) in unique_together.iter().enumerate() {
                    let constraint_name = format!("{}_{}_uniq", table, idx);
                    let fields_str = fields.join(", ");
                    sql.push(format!(
                        "ALTER TABLE {} ADD CONSTRAINT {} UNIQUE ({});",
                        table, constraint_name, fields_str
                    ));
                }
                sql.join("\n")
            }
            Operation::AlterModelOptions { .. } => String::new(),
            Operation::CreateInheritedTable {
                name,
                columns,
                base_table,
                join_column,
            } => {
                let mut parts = Vec::new();
                parts.push(format!(
                    "  {} INTEGER REFERENCES {}(id)",
                    join_column, base_table
                ));
                for col in columns {
                    parts.push(format!("  {} {}", col.name, col.type_definition));
                }
                format!("CREATE TABLE {} (\n{}\n);", name, parts.join(",\n"))
            }
            Operation::AddDiscriminatorColumn {
                table,
                column_name,
                default_value,
            } => {
                format!(
                    "ALTER TABLE {} ADD COLUMN {} VARCHAR(50) DEFAULT '{}';",
                    table, column_name, default_value
                )
            }
        }
    }

    /// Generate reverse SQL (for rollback)
    pub fn to_reverse_sql(&self, _dialect: &SqlDialect) -> Option<String> {
        match self {
            Operation::CreateTable { name, .. } => Some(format!("DROP TABLE {};", name)),
            Operation::DropTable { .. } => None,
            Operation::AddColumn { table, column } => Some(format!(
                "ALTER TABLE {} DROP COLUMN {};",
                table, column.name
            )),
            Operation::RunSQL { reverse_sql, .. } => reverse_sql.clone(),
            _ => None,
        }
    }
}

/// Column definition for legacy operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub type_definition: String,
}

impl ColumnDefinition {
    /// Create a new column definition
    pub fn new(name: impl Into<String>, type_def: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_definition: type_def.into(),
        }
    }
}

/// SQL dialect for generating database-specific SQL
#[derive(Debug, Clone, Copy)]
pub enum SqlDialect {
    Sqlite,
    Postgres,
    Mysql,
}

// Re-export for convenience (legacy)
pub use Operation::{AddColumn, AlterColumn, CreateTable, DropColumn};
