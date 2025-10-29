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
pub use models::{CreateModel, DeleteModel, FieldDefinition, MoveModel, RenameModel};
pub use postgres::{CreateCollation, CreateExtension, DropExtension};
pub use special::{RunCode, RunSQL, StateOperation};

// Legacy types for backward compatibility
// These are maintained from the original operations.rs
use crate::{FieldState, ModelState, ProjectState};
use pg_escape::quote_identifier;
use sea_query::{
    Alias, ColumnDef, ForeignKey, Index, IndexCreateStatement, IndexDropStatement,
    PostgresQueryBuilder, Table, TableAlterStatement, TableCreateStatement, TableDropStatement,
    TableRenameStatement,
};
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
    RunRust {
        code: String,
        reverse_code: Option<String>,
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
            | Operation::RunRust { .. }
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
            Operation::RunRust { code, .. } => {
                // For SQL generation, RunRust is a no-op comment
                format!("-- RunRust: {}", code.lines().next().unwrap_or(""))
            }
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
            Operation::RunRust { reverse_code, .. } => reverse_code.as_ref().map(|code| {
                format!(
                    "-- RunRust (reverse): {}",
                    code.lines().next().unwrap_or("")
                )
            }),
            _ => None,
        }
    }
}

/// Column definition for legacy operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub type_definition: String,
    #[serde(default)]
    pub not_null: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default)]
    pub primary_key: bool,
    #[serde(default)]
    pub auto_increment: bool,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub max_length: Option<u32>,
}

impl ColumnDefinition {
    /// Create a new column definition
    pub fn new(name: impl Into<String>, type_def: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_definition: type_def.into(),
            not_null: false,
            unique: false,
            primary_key: false,
            auto_increment: false,
            default: None,
            max_length: None,
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

/// Operation statement types (SeaQuery or sanitized raw SQL)
pub enum OperationStatement {
    TableCreate(TableCreateStatement),
    TableDrop(TableDropStatement),
    TableAlter(TableAlterStatement),
    TableRename(TableRenameStatement),
    IndexCreate(IndexCreateStatement),
    IndexDrop(IndexDropStatement),
    /// Sanitized raw SQL (identifiers escaped with pg_escape::quote_identifier)
    RawSql(String),
}

impl OperationStatement {
    /// Execute the operation statement
    pub async fn execute<'c, E>(&self, executor: E) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        match self {
            OperationStatement::TableCreate(stmt) => {
                let sql = stmt.to_string(PostgresQueryBuilder);
                sqlx::query(&sql).execute(executor).await?;
            }
            OperationStatement::TableDrop(stmt) => {
                let sql = stmt.to_string(PostgresQueryBuilder);
                sqlx::query(&sql).execute(executor).await?;
            }
            OperationStatement::TableAlter(stmt) => {
                let sql = stmt.to_string(PostgresQueryBuilder);
                sqlx::query(&sql).execute(executor).await?;
            }
            OperationStatement::TableRename(stmt) => {
                let sql = stmt.to_string(PostgresQueryBuilder);
                sqlx::query(&sql).execute(executor).await?;
            }
            OperationStatement::IndexCreate(stmt) => {
                let sql = stmt.to_string(PostgresQueryBuilder);
                sqlx::query(&sql).execute(executor).await?;
            }
            OperationStatement::IndexDrop(stmt) => {
                let sql = stmt.to_string(PostgresQueryBuilder);
                sqlx::query(&sql).execute(executor).await?;
            }
            OperationStatement::RawSql(sql) => {
                // Already sanitized with pg_escape::quote_identifier
                sqlx::query(sql).execute(executor).await?;
            }
        }
        Ok(())
    }

    /// Convert to SQL string for logging/debugging
    pub fn to_sql_string(&self) -> String {
        match self {
            OperationStatement::TableCreate(stmt) => stmt.to_string(PostgresQueryBuilder),
            OperationStatement::TableDrop(stmt) => stmt.to_string(PostgresQueryBuilder),
            OperationStatement::TableAlter(stmt) => stmt.to_string(PostgresQueryBuilder),
            OperationStatement::TableRename(stmt) => stmt.to_string(PostgresQueryBuilder),
            OperationStatement::IndexCreate(stmt) => stmt.to_string(PostgresQueryBuilder),
            OperationStatement::IndexDrop(stmt) => stmt.to_string(PostgresQueryBuilder),
            OperationStatement::RawSql(sql) => sql.clone(),
        }
    }
}

impl Operation {
    /// Convert Operation to SeaQuery statement or sanitized raw SQL
    pub fn to_statement(&self) -> OperationStatement {
        match self {
            Operation::CreateTable {
                name,
                columns,
                constraints,
            } => {
                OperationStatement::TableCreate(self.build_create_table(name, columns, constraints))
            }
            Operation::DropTable { name } => {
                OperationStatement::TableDrop(self.build_drop_table(name))
            }
            Operation::AddColumn { table, column } => {
                OperationStatement::TableAlter(self.build_add_column(table, column))
            }
            Operation::DropColumn { table, column } => {
                OperationStatement::TableAlter(self.build_drop_column(table, column))
            }
            Operation::AlterColumn {
                table,
                column,
                new_definition,
            } => OperationStatement::TableAlter(self.build_alter_column(
                table,
                column,
                new_definition,
            )),
            Operation::RenameTable { old_name, new_name } => {
                OperationStatement::TableRename(self.build_rename_table(old_name, new_name))
            }
            // SeaQuery does not support RENAME COLUMN, use sanitized raw SQL
            Operation::RenameColumn {
                table,
                old_name,
                new_name,
            } => OperationStatement::RawSql(format!(
                "ALTER TABLE {} RENAME COLUMN {} TO {}",
                quote_identifier(table),
                quote_identifier(old_name),
                quote_identifier(new_name)
            )),
            Operation::AddConstraint {
                table,
                constraint_sql,
            } => {
                // NOTE: constraint_sql validation is the caller's responsibility
                OperationStatement::RawSql(format!(
                    "ALTER TABLE {} ADD {}",
                    quote_identifier(table),
                    constraint_sql
                ))
            }
            Operation::DropConstraint {
                table,
                constraint_name,
            } => OperationStatement::RawSql(format!(
                "ALTER TABLE {} DROP CONSTRAINT {}",
                quote_identifier(table),
                quote_identifier(constraint_name)
            )),
            Operation::CreateIndex {
                table,
                columns,
                unique,
            } => {
                let idx_name = format!("idx_{}_{}", table, columns.join("_"));
                OperationStatement::IndexCreate(
                    self.build_create_index(&idx_name, table, columns, *unique),
                )
            }
            Operation::DropIndex { table, columns } => {
                let idx_name = format!("idx_{}_{}", table, columns.join("_"));
                OperationStatement::IndexDrop(self.build_drop_index(&idx_name))
            }
            Operation::RunSQL { sql, .. } => OperationStatement::RawSql(sql.clone()),
            Operation::RunRust { code, .. } => {
                // RunRust operations don't produce SQL
                OperationStatement::RawSql(format!(
                    "-- RunRust: {}",
                    code.lines().next().unwrap_or("")
                ))
            }
            Operation::AlterTableComment { table, comment } => {
                // PostgreSQL-specific COMMENT ON TABLE
                OperationStatement::RawSql(if let Some(comment_text) = comment {
                    format!(
                        "COMMENT ON TABLE {} IS '{}'",
                        quote_identifier(table),
                        comment_text.replace('\'', "''") // Escape single quotes
                    )
                } else {
                    format!("COMMENT ON TABLE {} IS NULL", quote_identifier(table))
                })
            }
            Operation::AlterUniqueTogether {
                table,
                unique_together,
            } => {
                let mut sqls = Vec::new();
                for (idx, fields) in unique_together.iter().enumerate() {
                    let constraint_name = format!("{}_{}_uniq", table, idx);
                    let fields_str: Vec<String> = fields
                        .iter()
                        .map(|f| quote_identifier(f).to_string())
                        .collect();
                    sqls.push(format!(
                        "ALTER TABLE {} ADD CONSTRAINT {} UNIQUE ({})",
                        quote_identifier(table),
                        quote_identifier(&constraint_name),
                        fields_str.join(", ")
                    ));
                }
                OperationStatement::RawSql(sqls.join(";\n"))
            }
            Operation::AlterModelOptions { .. } => OperationStatement::RawSql(String::new()),
            Operation::CreateInheritedTable {
                name,
                columns,
                base_table,
                join_column,
            } => {
                let mut stmt = Table::create();
                stmt.table(Alias::new(name)).if_not_exists();

                // Add join column (foreign key to base table)
                let mut join_col = ColumnDef::new(Alias::new(join_column));
                join_col.integer();
                stmt.col(&mut join_col);

                // Add other columns
                for col in columns {
                    let mut column = ColumnDef::new(Alias::new(&col.name));
                    self.apply_column_type(&mut column, &col.type_definition, col.max_length);
                    stmt.col(&mut column);
                }

                // Add foreign key
                let mut fk = ForeignKey::create();
                fk.from_tbl(Alias::new(name))
                    .from_col(Alias::new(join_column))
                    .to_tbl(Alias::new(base_table))
                    .to_col(Alias::new("id"));
                stmt.foreign_key(&mut fk);

                OperationStatement::TableCreate(stmt.to_owned())
            }
            Operation::AddDiscriminatorColumn {
                table,
                column_name,
                default_value,
            } => {
                let mut stmt = Table::alter();
                stmt.table(Alias::new(table));

                let mut col = ColumnDef::new(Alias::new(column_name));
                col.string_len(50).default(default_value.clone());
                stmt.add_column(&mut col);

                OperationStatement::TableAlter(stmt.to_owned())
            }
        }
    }

    /// Build CREATE TABLE statement
    fn build_create_table(
        &self,
        name: &str,
        columns: &[ColumnDefinition],
        _constraints: &[String],
    ) -> TableCreateStatement {
        let mut stmt = Table::create();
        stmt.table(Alias::new(name)).if_not_exists();

        for col in columns {
            let mut column = ColumnDef::new(Alias::new(&col.name));
            self.apply_column_type(&mut column, &col.type_definition, col.max_length);

            if col.not_null {
                column.not_null();
            }
            if col.unique {
                column.unique_key();
            }
            if col.primary_key {
                column.primary_key();
            }
            if col.auto_increment {
                column.auto_increment();
            }
            if let Some(ref default) = col.default {
                column.default(self.convert_default_value(default));
            }

            stmt.col(&mut column);
        }

        stmt.to_owned()
    }

    /// Build DROP TABLE statement
    fn build_drop_table(&self, name: &str) -> TableDropStatement {
        Table::drop()
            .table(Alias::new(name))
            .if_exists()
            .cascade()
            .to_owned()
    }

    /// Build ALTER TABLE ADD COLUMN statement
    fn build_add_column(&self, table: &str, column: &ColumnDefinition) -> TableAlterStatement {
        let mut stmt = Table::alter();
        stmt.table(Alias::new(table));

        let mut col_def = ColumnDef::new(Alias::new(&column.name));
        self.apply_column_type(&mut col_def, &column.type_definition, column.max_length);

        if column.not_null {
            col_def.not_null();
        }
        if let Some(ref default) = column.default {
            col_def.default(self.convert_default_value(default));
        }

        stmt.add_column(&mut col_def);
        stmt.to_owned()
    }

    /// Build ALTER TABLE DROP COLUMN statement
    fn build_drop_column(&self, table: &str, column: &str) -> TableAlterStatement {
        Table::alter()
            .table(Alias::new(table))
            .drop_column(Alias::new(column))
            .to_owned()
    }

    /// Build ALTER TABLE ALTER COLUMN statement
    fn build_alter_column(
        &self,
        table: &str,
        column: &str,
        new_definition: &ColumnDefinition,
    ) -> TableAlterStatement {
        let mut stmt = Table::alter();
        stmt.table(Alias::new(table));

        let mut col_def = ColumnDef::new(Alias::new(column));
        self.apply_column_type(
            &mut col_def,
            &new_definition.type_definition,
            new_definition.max_length,
        );

        if new_definition.not_null {
            col_def.not_null();
        }

        stmt.modify_column(&mut col_def);
        stmt.to_owned()
    }

    /// Build ALTER TABLE RENAME statement
    fn build_rename_table(&self, old_name: &str, new_name: &str) -> TableRenameStatement {
        Table::rename()
            .table(Alias::new(old_name), Alias::new(new_name))
            .to_owned()
    }

    /// Build CREATE INDEX statement
    fn build_create_index(
        &self,
        name: &str,
        table: &str,
        columns: &[String],
        unique: bool,
    ) -> IndexCreateStatement {
        let mut stmt = Index::create();
        stmt.name(name).table(Alias::new(table));

        for col in columns {
            stmt.col(Alias::new(col));
        }

        if unique {
            stmt.unique();
        }

        stmt.to_owned()
    }

    /// Build DROP INDEX statement
    fn build_drop_index(&self, name: &str) -> IndexDropStatement {
        Index::drop().name(name).to_owned()
    }

    /// Apply column type to ColumnDef using SeaQuery's fluent API
    fn apply_column_type(&self, col_def: &mut ColumnDef, col_type: &str, max_length: Option<u32>) {
        match col_type.to_uppercase().as_str() {
            "INTEGER" | "INT" | "INT4" => {
                col_def.integer();
            }
            "BIGINT" | "INT8" => {
                col_def.big_integer();
            }
            "SMALLINT" | "INT2" => {
                col_def.small_integer();
            }
            "VARCHAR" => {
                if let Some(len) = max_length {
                    col_def.string_len(len);
                } else {
                    col_def.string();
                }
            }
            "TEXT" => {
                col_def.text();
            }
            "CHAR" => {
                if let Some(len) = max_length {
                    col_def.char_len(len);
                } else {
                    col_def.char();
                }
            }
            "BOOLEAN" | "BOOL" => {
                col_def.boolean();
            }
            "TIMESTAMP" | "TIMESTAMPTZ" => {
                col_def.timestamp();
            }
            "DATE" => {
                col_def.date();
            }
            "TIME" => {
                col_def.time();
            }
            "DECIMAL" | "NUMERIC" => {
                col_def.decimal();
            }
            "REAL" | "FLOAT4" => {
                col_def.float();
            }
            "DOUBLE" | "FLOAT8" | "DOUBLE PRECISION" => {
                col_def.double();
            }
            "JSON" => {
                col_def.json();
            }
            "JSONB" => {
                col_def.json_binary();
            }
            "UUID" => {
                col_def.uuid();
            }
            "BYTEA" => {
                col_def.binary();
            }
            _ => {
                // Custom type: use custom() method
                col_def.custom(Alias::new(col_type));
            }
        }
    }

    /// Convert default value string to SeaQuery Value
    fn convert_default_value(&self, default: &str) -> sea_query::Value {
        let trimmed = default.trim();

        // NULL
        if trimmed.eq_ignore_ascii_case("null") {
            return sea_query::Value::String(None);
        }

        // Boolean
        if trimmed.eq_ignore_ascii_case("true") {
            return sea_query::Value::Bool(Some(true));
        }
        if trimmed.eq_ignore_ascii_case("false") {
            return sea_query::Value::Bool(Some(false));
        }

        // Integer
        if let Ok(i) = trimmed.parse::<i64>() {
            return sea_query::Value::BigInt(Some(i));
        }

        // Float
        if let Ok(f) = trimmed.parse::<f64>() {
            return sea_query::Value::Double(Some(f));
        }

        // String (quoted)
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            let unquoted = &trimmed[1..trimmed.len() - 1];
            return sea_query::Value::String(Some(unquoted.to_string()));
        }

        // JSON array/object
        if (trimmed.starts_with('[') && trimmed.ends_with(']'))
            || (trimmed.starts_with('{') && trimmed.ends_with('}'))
        {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
                return json_to_sea_value(&json);
            }
        }

        // SQL function calls (e.g., NOW(), CURRENT_TIMESTAMP)
        if trimmed.ends_with("()") || trimmed.contains('(') {
            // Return as custom SQL expression
            return sea_query::Value::String(Some(trimmed.to_string()));
        }

        // Default: treat as string
        sea_query::Value::String(Some(trimmed.to_string()))
    }
}

/// Helper function to convert serde_json::Value to sea_query::Value
fn json_to_sea_value(json: &serde_json::Value) -> sea_query::Value {
    match json {
        serde_json::Value::Null => sea_query::Value::String(None),
        serde_json::Value::Bool(b) => sea_query::Value::Bool(Some(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                sea_query::Value::BigInt(Some(i))
            } else if let Some(f) = n.as_f64() {
                sea_query::Value::Double(Some(f))
            } else {
                sea_query::Value::String(Some(n.to_string()))
            }
        }
        serde_json::Value::String(s) => sea_query::Value::String(Some(s.clone())),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            // Store as JSON string
            sea_query::Value::String(Some(json.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_table_to_statement() {
        let op = Operation::CreateTable {
            name: "users".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    type_definition: "INTEGER".to_string(),
                    not_null: false,
                    unique: false,
                    primary_key: true,
                    auto_increment: true,
                    default: None,
                    max_length: None,
                },
                ColumnDefinition {
                    name: "name".to_string(),
                    type_definition: "VARCHAR".to_string(),
                    not_null: true,
                    unique: false,
                    primary_key: false,
                    auto_increment: false,
                    default: None,
                    max_length: Some(100),
                },
            ],
            constraints: vec![],
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("CREATE TABLE"),
            "SQL should contain CREATE TABLE keyword, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("id") && sql.contains("name"),
            "SQL should contain both 'id' and 'name' columns, got: {}",
            sql
        );
    }

    #[test]
    fn test_drop_table_to_statement() {
        let op = Operation::DropTable {
            name: "users".to_string(),
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("DROP TABLE"),
            "SQL should contain DROP TABLE keyword, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("CASCADE"),
            "SQL should include CASCADE option, got: {}",
            sql
        );
    }

    #[test]
    fn test_add_column_to_statement() {
        let op = Operation::AddColumn {
            table: "users".to_string(),
            column: ColumnDefinition {
                name: "email".to_string(),
                type_definition: "VARCHAR".to_string(),
                not_null: true,
                unique: false,
                primary_key: false,
                auto_increment: false,
                default: Some("''".to_string()),
                max_length: Some(255),
            },
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("ALTER TABLE"),
            "SQL should contain ALTER TABLE keyword, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("ADD COLUMN"),
            "SQL should contain ADD COLUMN clause, got: {}",
            sql
        );
        assert!(
            sql.contains("email"),
            "SQL should reference 'email' column, got: {}",
            sql
        );
    }

    #[test]
    fn test_drop_column_to_statement() {
        let op = Operation::DropColumn {
            table: "users".to_string(),
            column: "email".to_string(),
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("ALTER TABLE"),
            "SQL should contain ALTER TABLE keyword, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("DROP COLUMN"),
            "SQL should contain DROP COLUMN clause, got: {}",
            sql
        );
        assert!(
            sql.contains("email"),
            "SQL should reference 'email' column, got: {}",
            sql
        );
    }

    #[test]
    fn test_alter_column_to_statement() {
        let op = Operation::AlterColumn {
            table: "users".to_string(),
            column: "age".to_string(),
            new_definition: ColumnDefinition {
                name: "age".to_string(),
                type_definition: "BIGINT".to_string(),
                not_null: true,
                unique: false,
                primary_key: false,
                auto_increment: false,
                default: None,
                max_length: None,
            },
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("ALTER TABLE"),
            "SQL should contain ALTER TABLE keyword, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("age"),
            "SQL should reference 'age' column, got: {}",
            sql
        );
    }

    #[test]
    fn test_rename_table_to_statement() {
        let op = Operation::RenameTable {
            old_name: "users".to_string(),
            new_name: "accounts".to_string(),
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("users"),
            "SQL should reference old table name 'users', got: {}",
            sql
        );
        assert!(
            sql.contains("accounts"),
            "SQL should reference new table name 'accounts', got: {}",
            sql
        );
    }

    #[test]
    fn test_rename_column_to_statement() {
        let op = Operation::RenameColumn {
            table: "users".to_string(),
            old_name: "name".to_string(),
            new_name: "full_name".to_string(),
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("ALTER TABLE"),
            "SQL should contain ALTER TABLE keyword, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("RENAME COLUMN"),
            "SQL should contain RENAME COLUMN clause, got: {}",
            sql
        );
        assert!(
            sql.contains("name"),
            "SQL should reference old column name 'name', got: {}",
            sql
        );
        assert!(
            sql.contains("full_name"),
            "SQL should reference new column name 'full_name', got: {}",
            sql
        );
    }

    #[test]
    fn test_add_constraint_to_statement() {
        let op = Operation::AddConstraint {
            table: "users".to_string(),
            constraint_sql: "CONSTRAINT age_check CHECK (age >= 0)".to_string(),
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("ALTER TABLE"),
            "SQL should contain ALTER TABLE keyword, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("ADD"),
            "SQL should contain ADD keyword, got: {}",
            sql
        );
        assert!(
            sql.contains("age_check"),
            "SQL should contain constraint name 'age_check', got: {}",
            sql
        );
    }

    #[test]
    fn test_drop_constraint_to_statement() {
        let op = Operation::DropConstraint {
            table: "users".to_string(),
            constraint_name: "age_check".to_string(),
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("ALTER TABLE"),
            "SQL should contain ALTER TABLE keyword, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("DROP CONSTRAINT"),
            "SQL should contain DROP CONSTRAINT clause, got: {}",
            sql
        );
        assert!(
            sql.contains("age_check"),
            "SQL should reference constraint 'age_check', got: {}",
            sql
        );
    }

    #[test]
    fn test_create_index_to_statement() {
        let op = Operation::CreateIndex {
            table: "users".to_string(),
            columns: vec!["email".to_string()],
            unique: false,
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("CREATE INDEX"),
            "SQL should contain CREATE INDEX keywords, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("email"),
            "SQL should reference 'email' column, got: {}",
            sql
        );
    }

    #[test]
    fn test_create_unique_index_to_statement() {
        let op = Operation::CreateIndex {
            table: "users".to_string(),
            columns: vec!["email".to_string()],
            unique: true,
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("CREATE UNIQUE INDEX"),
            "SQL should contain CREATE UNIQUE INDEX keywords, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("email"),
            "SQL should reference 'email' column, got: {}",
            sql
        );
    }

    #[test]
    fn test_drop_index_to_statement() {
        let op = Operation::DropIndex {
            table: "users".to_string(),
            columns: vec!["email".to_string()],
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("DROP INDEX"),
            "SQL should contain DROP INDEX keywords, got: {}",
            sql
        );
        assert!(
            sql.contains("idx_users_email"),
            "SQL should contain generated index name 'idx_users_email', got: {}",
            sql
        );
    }

    #[test]
    fn test_run_sql_to_statement() {
        let op = Operation::RunSQL {
            sql: "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"".to_string(),
            reverse_sql: Some("DROP EXTENSION \"uuid-ossp\"".to_string()),
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("CREATE EXTENSION"),
            "SQL should contain CREATE EXTENSION keywords, got: {}",
            sql
        );
        assert!(
            sql.contains("uuid-ossp"),
            "SQL should reference 'uuid-ossp' extension, got: {}",
            sql
        );
    }

    #[test]
    fn test_alter_table_comment_to_statement() {
        let op = Operation::AlterTableComment {
            table: "users".to_string(),
            comment: Some("User accounts table".to_string()),
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("COMMENT ON TABLE"),
            "SQL should contain COMMENT ON TABLE keywords, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("User accounts table"),
            "SQL should include comment text 'User accounts table', got: {}",
            sql
        );
    }

    #[test]
    fn test_alter_table_comment_null_to_statement() {
        let op = Operation::AlterTableComment {
            table: "users".to_string(),
            comment: None,
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("COMMENT ON TABLE"),
            "SQL should contain COMMENT ON TABLE keywords, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("NULL"),
            "SQL should include NULL for null comment, got: {}",
            sql
        );
    }

    #[test]
    fn test_alter_unique_together_to_statement() {
        let op = Operation::AlterUniqueTogether {
            table: "users".to_string(),
            unique_together: vec![vec!["email".to_string(), "username".to_string()]],
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("ALTER TABLE"),
            "SQL should contain ALTER TABLE keyword, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("ADD CONSTRAINT"),
            "SQL should contain ADD CONSTRAINT clause, got: {}",
            sql
        );
        assert!(
            sql.contains("UNIQUE"),
            "SQL should contain UNIQUE keyword, got: {}",
            sql
        );
        assert!(
            sql.contains("email") && sql.contains("username"),
            "SQL should reference both 'email' and 'username' columns, got: {}",
            sql
        );
    }

    #[test]
    fn test_alter_unique_together_empty() {
        let op = Operation::AlterUniqueTogether {
            table: "users".to_string(),
            unique_together: vec![],
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert_eq!(
            sql, "",
            "SQL should be empty for empty unique_together constraint"
        );
    }

    #[test]
    fn test_alter_model_options_to_statement() {
        let mut options = std::collections::HashMap::new();
        options.insert("db_table".to_string(), "custom_users".to_string());

        let op = Operation::AlterModelOptions {
            table: "users".to_string(),
            options,
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert_eq!(sql, "", "SQL should be empty for model options operation");
    }

    #[test]
    fn test_create_inherited_table_to_statement() {
        let op = Operation::CreateInheritedTable {
            name: "admin_users".to_string(),
            columns: vec![ColumnDefinition {
                name: "admin_level".to_string(),
                type_definition: "INTEGER".to_string(),
                not_null: true,
                unique: false,
                primary_key: false,
                auto_increment: false,
                default: Some("1".to_string()),
                max_length: None,
            }],
            base_table: "users".to_string(),
            join_column: "user_id".to_string(),
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("CREATE TABLE"),
            "SQL should contain CREATE TABLE keywords, got: {}",
            sql
        );
        assert!(
            sql.contains("admin_users"),
            "SQL should reference 'admin_users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("user_id"),
            "SQL should include join column 'user_id', got: {}",
            sql
        );
    }

    #[test]
    fn test_add_discriminator_column_to_statement() {
        let op = Operation::AddDiscriminatorColumn {
            table: "users".to_string(),
            column_name: "user_type".to_string(),
            default_value: "regular".to_string(),
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("ALTER TABLE"),
            "SQL should contain ALTER TABLE keyword, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "SQL should reference 'users' table, got: {}",
            sql
        );
        assert!(
            sql.contains("ADD COLUMN"),
            "SQL should contain ADD COLUMN clause, got: {}",
            sql
        );
        assert!(
            sql.contains("user_type"),
            "SQL should reference 'user_type' column, got: {}",
            sql
        );
    }

    #[test]
    fn test_state_forwards_create_table() {
        let mut state = ProjectState::new();
        let op = Operation::CreateTable {
            name: "users".to_string(),
            columns: vec![
                ColumnDefinition {
                    name: "id".to_string(),
                    type_definition: "INTEGER".to_string(),
                    not_null: false,
                    unique: false,
                    primary_key: true,
                    auto_increment: true,
                    default: None,
                    max_length: None,
                },
                ColumnDefinition {
                    name: "name".to_string(),
                    type_definition: "VARCHAR".to_string(),
                    not_null: true,
                    unique: false,
                    primary_key: false,
                    auto_increment: false,
                    default: None,
                    max_length: Some(100),
                },
            ],
            constraints: vec![],
        };

        op.state_forwards("myapp", &mut state);
        let model = state.get_model("myapp", "users");
        assert!(model.is_some(), "Model 'users' should exist in state");
        let model = model.unwrap();
        assert_eq!(
            model.fields.len(),
            2,
            "Model should have exactly 2 fields, got: {}",
            model.fields.len()
        );
        assert!(
            model.fields.contains_key("id"),
            "Model should contain 'id' field"
        );
        assert!(
            model.fields.contains_key("name"),
            "Model should contain 'name' field"
        );
    }

    #[test]
    fn test_state_forwards_drop_table() {
        let mut state = ProjectState::new();
        let mut model = ModelState::new("myapp", "users");
        model.add_field(FieldState::new(
            "id".to_string(),
            "INTEGER".to_string(),
            false,
        ));
        state.add_model(model);

        let op = Operation::DropTable {
            name: "users".to_string(),
        };

        op.state_forwards("myapp", &mut state);
        assert!(
            state.get_model("myapp", "users").is_none(),
            "Model 'users' should be removed from state after drop"
        );
    }

    #[test]
    fn test_state_forwards_add_column() {
        let mut state = ProjectState::new();
        let mut model = ModelState::new("myapp", "users");
        model.add_field(FieldState::new(
            "id".to_string(),
            "INTEGER".to_string(),
            false,
        ));
        state.add_model(model);

        let op = Operation::AddColumn {
            table: "users".to_string(),
            column: ColumnDefinition {
                name: "email".to_string(),
                type_definition: "VARCHAR".to_string(),
                not_null: true,
                unique: false,
                primary_key: false,
                auto_increment: false,
                default: None,
                max_length: Some(255),
            },
        };

        op.state_forwards("myapp", &mut state);
        let model = state.get_model("myapp", "users").unwrap();
        assert_eq!(
            model.fields.len(),
            2,
            "Model should have 2 fields after adding 'email', got: {}",
            model.fields.len()
        );
        assert!(
            model.fields.contains_key("email"),
            "Model should contain newly added 'email' field"
        );
    }

    #[test]
    fn test_state_forwards_drop_column() {
        let mut state = ProjectState::new();
        let mut model = ModelState::new("myapp", "users");
        model.add_field(FieldState::new(
            "id".to_string(),
            "INTEGER".to_string(),
            false,
        ));
        model.add_field(FieldState::new(
            "email".to_string(),
            "VARCHAR".to_string(),
            false,
        ));
        state.add_model(model);

        let op = Operation::DropColumn {
            table: "users".to_string(),
            column: "email".to_string(),
        };

        op.state_forwards("myapp", &mut state);
        let model = state.get_model("myapp", "users").unwrap();
        assert_eq!(
            model.fields.len(),
            1,
            "Model should have 1 field after dropping 'email', got: {}",
            model.fields.len()
        );
        assert!(
            !model.fields.contains_key("email"),
            "Model should not contain dropped 'email' field"
        );
    }

    #[test]
    fn test_state_forwards_rename_table() {
        let mut state = ProjectState::new();
        let mut model = ModelState::new("myapp", "users");
        model.add_field(FieldState::new(
            "id".to_string(),
            "INTEGER".to_string(),
            false,
        ));
        state.add_model(model);

        let op = Operation::RenameTable {
            old_name: "users".to_string(),
            new_name: "accounts".to_string(),
        };

        op.state_forwards("myapp", &mut state);
        assert!(
            state.get_model("myapp", "users").is_none(),
            "Old model name 'users' should not exist after rename"
        );
        assert!(
            state.get_model("myapp", "accounts").is_some(),
            "New model name 'accounts' should exist after rename"
        );
    }

    #[test]
    fn test_state_forwards_rename_column() {
        let mut state = ProjectState::new();
        let mut model = ModelState::new("myapp", "users");
        model.add_field(FieldState::new(
            "name".to_string(),
            "VARCHAR".to_string(),
            false,
        ));
        state.add_model(model);

        let op = Operation::RenameColumn {
            table: "users".to_string(),
            old_name: "name".to_string(),
            new_name: "full_name".to_string(),
        };

        op.state_forwards("myapp", &mut state);
        let model = state.get_model("myapp", "users").unwrap();
        assert!(
            !model.fields.contains_key("name"),
            "Old field name 'name' should not exist after rename"
        );
        assert!(
            model.fields.contains_key("full_name"),
            "New field name 'full_name' should exist after rename"
        );
    }

    #[test]
    fn test_to_reverse_sql_create_table() {
        let op = Operation::CreateTable {
            name: "users".to_string(),
            columns: vec![],
            constraints: vec![],
        };

        let reverse = op.to_reverse_sql(&SqlDialect::Postgres);
        assert!(
            reverse.is_some(),
            "CreateTable should have reverse SQL operation"
        );
        let sql = reverse.unwrap();
        assert!(
            sql.contains("DROP TABLE"),
            "Reverse SQL should contain DROP TABLE, got: {}",
            sql
        );
        assert!(
            sql.contains("users"),
            "Reverse SQL should reference 'users' table, got: {}",
            sql
        );
    }

    #[test]
    fn test_to_reverse_sql_drop_table() {
        let op = Operation::DropTable {
            name: "users".to_string(),
        };

        let reverse = op.to_reverse_sql(&SqlDialect::Postgres);
        assert!(
            reverse.is_none(),
            "DropTable should not have reverse SQL (cannot recreate table structure)"
        );
    }

    #[test]
    fn test_to_reverse_sql_add_column() {
        let op = Operation::AddColumn {
            table: "users".to_string(),
            column: ColumnDefinition {
                name: "email".to_string(),
                type_definition: "VARCHAR".to_string(),
                not_null: false,
                unique: false,
                primary_key: false,
                auto_increment: false,
                default: None,
                max_length: None,
            },
        };

        let reverse = op.to_reverse_sql(&SqlDialect::Postgres);
        assert!(
            reverse.is_some(),
            "AddColumn should have reverse SQL operation"
        );
        let sql = reverse.unwrap();
        assert!(
            sql.contains("DROP COLUMN"),
            "Reverse SQL should contain DROP COLUMN, got: {}",
            sql
        );
        assert!(
            sql.contains("email"),
            "Reverse SQL should reference 'email' column, got: {}",
            sql
        );
    }

    #[test]
    fn test_to_reverse_sql_run_sql_with_reverse() {
        let op = Operation::RunSQL {
            sql: "CREATE INDEX idx_name ON users(name)".to_string(),
            reverse_sql: Some("DROP INDEX idx_name".to_string()),
        };

        let reverse = op.to_reverse_sql(&SqlDialect::Postgres);
        assert!(
            reverse.is_some(),
            "RunSQL with reverse_sql should have reverse SQL"
        );
        let sql = reverse.unwrap();
        assert!(
            sql.contains("DROP INDEX"),
            "Reverse SQL should contain provided reverse_sql, got: {}",
            sql
        );
    }

    #[test]
    fn test_to_reverse_sql_run_sql_without_reverse() {
        let op = Operation::RunSQL {
            sql: "CREATE INDEX idx_name ON users(name)".to_string(),
            reverse_sql: None,
        };

        let reverse = op.to_reverse_sql(&SqlDialect::Postgres);
        assert!(
            reverse.is_none(),
            "RunSQL without reverse_sql should not have reverse SQL"
        );
    }

    #[test]
    fn test_column_definition_new() {
        let col = ColumnDefinition::new("id", "INTEGER");
        assert_eq!(col.name, "id", "Column name should be 'id'");
        assert_eq!(
            col.type_definition, "INTEGER",
            "Column type should be 'INTEGER'"
        );
        assert!(!col.not_null, "not_null should default to false");
        assert!(!col.unique, "unique should default to false");
        assert!(!col.primary_key, "primary_key should default to false");
        assert!(
            !col.auto_increment,
            "auto_increment should default to false"
        );
        assert!(col.default.is_none(), "default should be None");
        assert!(col.max_length.is_none(), "max_length should be None");
    }

    #[test]
    fn test_convert_default_value_null() {
        let op = Operation::CreateTable {
            name: "test".to_string(),
            columns: vec![],
            constraints: vec![],
        };
        let value = op.convert_default_value("null");
        assert!(
            matches!(value, sea_query::Value::String(None)),
            "NULL value should be converted to sea_query::Value::String(None)"
        );
    }

    #[test]
    fn test_convert_default_value_bool() {
        let op = Operation::CreateTable {
            name: "test".to_string(),
            columns: vec![],
            constraints: vec![],
        };
        let value = op.convert_default_value("true");
        assert!(
            matches!(value, sea_query::Value::Bool(Some(true))),
            "'true' should be converted to sea_query::Value::Bool(Some(true))"
        );

        let value = op.convert_default_value("false");
        assert!(
            matches!(value, sea_query::Value::Bool(Some(false))),
            "'false' should be converted to sea_query::Value::Bool(Some(false))"
        );
    }

    #[test]
    fn test_convert_default_value_integer() {
        let op = Operation::CreateTable {
            name: "test".to_string(),
            columns: vec![],
            constraints: vec![],
        };
        let value = op.convert_default_value("42");
        assert!(
            matches!(value, sea_query::Value::BigInt(Some(42))),
            "Integer '42' should be converted to sea_query::Value::BigInt(Some(42))"
        );
    }

    #[test]
    fn test_convert_default_value_float() {
        let op = Operation::CreateTable {
            name: "test".to_string(),
            columns: vec![],
            constraints: vec![],
        };
        let value = op.convert_default_value("3.14");
        assert!(
            matches!(value, sea_query::Value::Double(_)),
            "Float '3.14' should be converted to sea_query::Value::Double"
        );
    }

    #[test]
    fn test_convert_default_value_string() {
        let op = Operation::CreateTable {
            name: "test".to_string(),
            columns: vec![],
            constraints: vec![],
        };
        let value = op.convert_default_value("'hello'");
        match value {
            sea_query::Value::String(Some(s)) => assert_eq!(
                s, "hello",
                "Quoted string should be unquoted and stored as 'hello'"
            ),
            _ => panic!("Expected sea_query::Value::String(Some(\"hello\")), got different variant"),
        }
    }

    #[test]
    fn test_apply_column_type_integer() {
        let op = Operation::CreateTable {
            name: "test".to_string(),
            columns: vec![],
            constraints: vec![],
        };
        let mut col = ColumnDef::new(Alias::new("id"));
        op.apply_column_type(&mut col, "INTEGER", None);
        // This test verifies that INTEGER type application doesn't panic
        // Internal state cannot be easily asserted with sea_query's ColumnDef API
    }

    #[test]
    fn test_apply_column_type_varchar_with_length() {
        let op = Operation::CreateTable {
            name: "test".to_string(),
            columns: vec![],
            constraints: vec![],
        };
        let mut col = ColumnDef::new(Alias::new("name"));
        op.apply_column_type(&mut col, "VARCHAR", Some(100));
        // This test verifies that VARCHAR(100) type application doesn't panic
        // Internal state cannot be easily asserted with sea_query's ColumnDef API
    }

    #[test]
    fn test_apply_column_type_custom() {
        let op = Operation::CreateTable {
            name: "test".to_string(),
            columns: vec![],
            constraints: vec![],
        };
        let mut col = ColumnDef::new(Alias::new("data"));
        op.apply_column_type(&mut col, "CUSTOM_TYPE", None);
        // This test verifies that custom type application doesn't panic
        // Internal state cannot be easily asserted with sea_query's ColumnDef API
    }

    #[test]
    fn test_create_index_composite() {
        let op = Operation::CreateIndex {
            table: "users".to_string(),
            columns: vec!["first_name".to_string(), "last_name".to_string()],
            unique: false,
        };

        let sql = op.to_sql(&SqlDialect::Postgres);
        assert!(
            sql.contains("first_name"),
            "SQL should include 'first_name' column, got: {}",
            sql
        );
        assert!(
            sql.contains("last_name"),
            "SQL should include 'last_name' column, got: {}",
            sql
        );
        assert!(
            sql.contains("idx_users_first_name_last_name"),
            "SQL should include composite index name, got: {}",
            sql
        );
    }

    #[test]
    fn test_alter_table_comment_with_quotes() {
        let op = Operation::AlterTableComment {
            table: "users".to_string(),
            comment: Some("User's account table".to_string()),
        };

        let stmt = op.to_statement();
        let sql = stmt.to_sql_string();
        assert!(
            sql.contains("COMMENT ON TABLE"),
            "SQL should contain COMMENT ON TABLE keywords, got: {}",
            sql
        );
        assert!(
            sql.contains("User''s account table"),
            "SQL should properly escape single quotes in comment, got: {}",
            sql
        );
    }

    #[test]
    fn test_state_forwards_alter_column() {
        let mut state = ProjectState::new();
        let mut model = ModelState::new("myapp", "users");
        model.add_field(FieldState::new(
            "age".to_string(),
            "INTEGER".to_string(),
            false,
        ));
        state.add_model(model);

        let op = Operation::AlterColumn {
            table: "users".to_string(),
            column: "age".to_string(),
            new_definition: ColumnDefinition {
                name: "age".to_string(),
                type_definition: "BIGINT".to_string(),
                not_null: true,
                unique: false,
                primary_key: false,
                auto_increment: false,
                default: None,
                max_length: None,
            },
        };

        op.state_forwards("myapp", &mut state);
        let model = state.get_model("myapp", "users").unwrap();
        let field = model.fields.get("age").unwrap();
        assert_eq!(
            field.field_type, "BIGINT",
            "Field type should be updated to BIGINT, got: {}",
            field.field_type
        );
    }

    #[test]
    fn test_state_forwards_create_inherited_table() {
        let mut state = ProjectState::new();
        let op = Operation::CreateInheritedTable {
            name: "admin_users".to_string(),
            columns: vec![ColumnDefinition {
                name: "admin_level".to_string(),
                type_definition: "INTEGER".to_string(),
                not_null: true,
                unique: false,
                primary_key: false,
                auto_increment: false,
                default: None,
                max_length: None,
            }],
            base_table: "users".to_string(),
            join_column: "user_id".to_string(),
        };

        op.state_forwards("myapp", &mut state);
        let model = state.get_model("myapp", "admin_users");
        assert!(
            model.is_some(),
            "Inherited table 'admin_users' should exist in state"
        );
        let model = model.unwrap();
        assert_eq!(
            model.base_model,
            Some("users".to_string()),
            "base_model should be set to 'users'"
        );
        assert_eq!(
            model.inheritance_type,
            Some("joined_table".to_string()),
            "inheritance_type should be 'joined_table'"
        );
    }

    #[test]
    fn test_state_forwards_add_discriminator_column() {
        let mut state = ProjectState::new();
        let mut model = ModelState::new("myapp", "users");
        model.add_field(FieldState::new(
            "id".to_string(),
            "INTEGER".to_string(),
            false,
        ));
        state.add_model(model);

        let op = Operation::AddDiscriminatorColumn {
            table: "users".to_string(),
            column_name: "user_type".to_string(),
            default_value: "regular".to_string(),
        };

        op.state_forwards("myapp", &mut state);
        let model = state.get_model("myapp", "users").unwrap();
        assert_eq!(
            model.discriminator_column,
            Some("user_type".to_string()),
            "discriminator_column should be set to 'user_type'"
        );
        assert_eq!(
            model.inheritance_type,
            Some("single_table".to_string()),
            "inheritance_type should be 'single_table'"
        );
    }
}
