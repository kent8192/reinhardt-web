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
            OperationStatement::TableCreate(stmt) => {
                stmt.to_string(PostgresQueryBuilder)
            }
            OperationStatement::TableDrop(stmt) => {
                stmt.to_string(PostgresQueryBuilder)
            }
            OperationStatement::TableAlter(stmt) => {
                stmt.to_string(PostgresQueryBuilder)
            }
            OperationStatement::TableRename(stmt) => {
                stmt.to_string(PostgresQueryBuilder)
            }
            OperationStatement::IndexCreate(stmt) => {
                stmt.to_string(PostgresQueryBuilder)
            }
            OperationStatement::IndexDrop(stmt) => {
                stmt.to_string(PostgresQueryBuilder)
            }
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
            } => OperationStatement::TableCreate(self.build_create_table(name, columns, constraints)),
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
            } => OperationStatement::TableAlter(self.build_alter_column(table, column, new_definition)),
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
                OperationStatement::IndexCreate(self.build_create_index(&idx_name, table, columns, *unique))
            }
            Operation::DropIndex { table, columns } => {
                let idx_name = format!("idx_{}_{}", table, columns.join("_"));
                OperationStatement::IndexDrop(self.build_drop_index(&idx_name))
            }
            Operation::RunSQL { sql, .. } => OperationStatement::RawSql(sql.clone()),
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
                    let fields_str: Vec<String> =
                        fields.iter().map(|f| quote_identifier(f).to_string()).collect();
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
                col.string_len(50)
                    .default(default_value.clone());
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
        self.apply_column_type(&mut col_def, &new_definition.type_definition, new_definition.max_length);

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
    fn test_convert_default_null() {
        let alter = AlterColumn {
            model: "test".to_string(),
            name: "field".to_string(),
            field: None,
        };

        let result = alter.convert_default_value("null");
        assert!(matches!(result, sea_query::Value::String(None)));

        let result = alter.convert_default_value("NULL");
        assert!(matches!(result, sea_query::Value::String(None)));
    }

    #[test]
    fn test_convert_default_boolean() {
        let alter = AlterColumn {
            model: "test".to_string(),
            name: "field".to_string(),
            field: None,
        };

        let result = alter.convert_default_value("true");
        assert!(matches!(result, sea_query::Value::Bool(Some(true))));

        let result = alter.convert_default_value("TRUE");
        assert!(matches!(result, sea_query::Value::Bool(Some(true))));

        let result = alter.convert_default_value("false");
        assert!(matches!(result, sea_query::Value::Bool(Some(false))));

        let result = alter.convert_default_value("FALSE");
        assert!(matches!(result, sea_query::Value::Bool(Some(false))));
    }

    #[test]
    fn test_convert_default_integer() {
        let alter = AlterColumn {
            model: "test".to_string(),
            name: "field".to_string(),
            field: None,
        };

        let result = alter.convert_default_value("42");
        assert!(matches!(result, sea_query::Value::BigInt(Some(42))));

        let result = alter.convert_default_value("-123");
        assert!(matches!(result, sea_query::Value::BigInt(Some(-123))));

        let result = alter.convert_default_value("0");
        assert!(matches!(result, sea_query::Value::BigInt(Some(0))));
    }

    #[test]
    fn test_convert_default_float() {
        let alter = AlterColumn {
            model: "test".to_string(),
            name: "field".to_string(),
            field: None,
        };

        let result = alter.convert_default_value("3.14");
        if let sea_query::Value::Double(Some(f)) = result {
            assert!((f - 3.14).abs() < f64::EPSILON);
        } else {
            panic!("Expected Double value");
        }

        let result = alter.convert_default_value("-2.5");
        if let sea_query::Value::Double(Some(f)) = result {
            assert!((f - (-2.5)).abs() < f64::EPSILON);
        } else {
            panic!("Expected Double value");
        }

        let result = alter.convert_default_value("0.0");
        if let sea_query::Value::Double(Some(f)) = result {
            assert!(f.abs() < f64::EPSILON);
        } else {
            panic!("Expected Double value");
        }
    }

    #[test]
    fn test_convert_default_quoted_string() {
        let alter = AlterColumn {
            model: "test".to_string(),
            name: "field".to_string(),
            field: None,
        };

        let result = alter.convert_default_value("\"hello\"");
        if let sea_query::Value::String(Some(s)) = result {
            assert_eq!(*s, "hello");
        } else {
            panic!("Expected String value");
        }

        let result = alter.convert_default_value("'world'");
        if let sea_query::Value::String(Some(s)) = result {
            assert_eq!(*s, "world");
        } else {
            panic!("Expected String value");
        }
    }

    #[test]
    fn test_convert_default_json_array() {
        let alter = AlterColumn {
            model: "test".to_string(),
            name: "field".to_string(),
            field: None,
        };

        let result = alter.convert_default_value("[1, 2, 3]");
        if let sea_query::Value::String(Some(s)) = result {
            assert_eq!(*s, "[1,2,3]"); // JSON serialization removes spaces
        } else {
            panic!("Expected String value for JSON array");
        }

        let result = alter.convert_default_value(r#"["a", "b"]"#);
        if let sea_query::Value::String(Some(s)) = result {
            assert_eq!(*s, r#"["a","b"]"#);
        } else {
            panic!("Expected String value for JSON array");
        }
    }

    #[test]
    fn test_convert_default_json_object() {
        let alter = AlterColumn {
            model: "test".to_string(),
            name: "field".to_string(),
            field: None,
        };

        let result = alter.convert_default_value(r#"{"key": "value"}"#);
        if let sea_query::Value::String(Some(s)) = result {
            assert_eq!(*s, r#"{"key":"value"}"#);
        } else {
            panic!("Expected String value for JSON object");
        }
    }

    #[test]
    fn test_convert_default_sql_function() {
        let alter = AlterColumn {
            model: "test".to_string(),
            name: "field".to_string(),
            field: None,
        };

        let result = alter.convert_default_value("NOW()");
        if let sea_query::Value::String(Some(s)) = result {
            assert_eq!(*s, "NOW()");
        } else {
            panic!("Expected String value for SQL function");
        }

        let result = alter.convert_default_value("CURRENT_TIMESTAMP");
        if let sea_query::Value::String(Some(s)) = result {
            assert_eq!(*s, "CURRENT_TIMESTAMP");
        } else {
            panic!("Expected String value for SQL function");
        }

        let result = alter.convert_default_value("UUID()");
        if let sea_query::Value::String(Some(s)) = result {
            assert_eq!(*s, "UUID()");
        } else {
            panic!("Expected String value for SQL function");
        }
    }

    #[test]
    fn test_convert_default_unquoted_string() {
        let alter = AlterColumn {
            model: "test".to_string(),
            name: "field".to_string(),
            field: None,
        };

        let result = alter.convert_default_value("sometext");
        if let sea_query::Value::String(Some(s)) = result {
            assert_eq!(*s, "sometext");
        } else {
            panic!("Expected String value for unquoted string");
        }
    }
}
