//! Migration file writer
//!
//! Generates Rust migration files from Migration structs.

use crate::{Migration, Operation, Result};
use std::fs;
use std::path::Path;

/// Writer for generating migration files
pub struct MigrationWriter {
    migration: Migration,
}

impl MigrationWriter {
    /// Create a new migration writer
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{Migration, writer::MigrationWriter};
    ///
    /// let migration = Migration::new("0001_initial", "myapp");
    /// let writer = MigrationWriter::new(migration);
    /// ```
    pub fn new(migration: Migration) -> Self {
        Self { migration }
    }
    /// Generate the migration file content
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_migrations::{Migration, Operation, ColumnDefinition, writer::MigrationWriter};
    ///
    /// let migration = Migration::new("0001_initial", "myapp")
    ///     .add_operation(Operation::CreateTable {
    ///         name: "users".to_string(),
    ///         columns: vec![ColumnDefinition::new("id", "INTEGER PRIMARY KEY")],
    ///         constraints: vec![],
    ///     });
    ///
    /// let writer = MigrationWriter::new(migration);
    /// let content = writer.as_string();
    ///
    /// assert!(content.contains("//! Name: 0001_initial"));
    /// assert!(content.contains("//! App: myapp"));
    /// assert!(content.contains("Migration::new"));
    /// ```
    pub fn as_string(&self) -> String {
        let mut content = String::new();

        // Add file header
        content.push_str("//! Auto-generated migration\n");
        content.push_str(&format!("//! Name: {}\n", self.migration.name));
        content.push_str(&format!("//! App: {}\n\n", self.migration.app_label));

        // Add imports
        content.push_str("use reinhardt_migrations::{\n");
        content.push_str("    Migration, Operation, CreateTable, AddColumn, AlterColumn,\n");
        content.push_str("    ColumnDefinition,\n");
        content.push_str("};\n\n");

        // Generate migration function
        content.push_str(&format!(
            "pub fn migration_{}() -> Migration {{\n",
            self.migration.name.replace('-', "_")
        ));
        content.push_str(&format!(
            "    Migration::new(\"{}\", \"{}\")\n",
            self.migration.name, self.migration.app_label
        ));

        // Add dependencies
        for (dep_app, dep_name) in &self.migration.dependencies {
            content.push_str(&format!(
                "        .add_dependency(\"{}\", \"{}\")\n",
                dep_app, dep_name
            ));
        }

        // Add operations
        for operation in &self.migration.operations {
            content.push_str(&self.serialize_operation(operation, 2));
        }

        content.push_str("}\n");

        content
    }

    /// Serialize an operation to Rust code
    fn serialize_operation(&self, operation: &Operation, indent_level: usize) -> String {
        let indent = "    ".repeat(indent_level);
        let mut result = String::new();

        match operation {
            Operation::CreateTable {
                name,
                columns,
                constraints,
            } => {
                result.push_str(&format!(
                    "{}    .add_operation(Operation::CreateTable {{\n",
                    indent
                ));
                result.push_str(&format!(
                    "{}        name: \"{}\".to_string(),\n",
                    indent, name
                ));
                result.push_str(&format!("{}        columns: vec![\n", indent));

                for column in columns {
                    result.push_str(&self.serialize_column(column, indent_level + 3));
                }

                result.push_str(&format!("{}        ],\n", indent));
                result.push_str(&format!("{}        constraints: vec![\n", indent));

                for constraint in constraints {
                    result.push_str(&format!(
                        "{}            \"{}\".to_string(),\n",
                        indent, constraint
                    ));
                }

                result.push_str(&format!("{}        ],\n", indent));
                result.push_str(&format!("{}    }})\n", indent));
            }
            Operation::DropTable { name } => {
                result.push_str(&format!(
                    "{}    .add_operation(Operation::DropTable {{\n",
                    indent
                ));
                result.push_str(&format!(
                    "{}        name: \"{}\".to_string(),\n",
                    indent, name
                ));
                result.push_str(&format!("{}    }})\n", indent));
            }
            Operation::AddColumn { table, column } => {
                result.push_str(&format!(
                    "{}    .add_operation(Operation::AddColumn {{\n",
                    indent
                ));
                result.push_str(&format!(
                    "{}        table: \"{}\".to_string(),\n",
                    indent, table
                ));
                result.push_str(&format!("{}        column: ", indent));
                result.push_str(&self.serialize_column(column, indent_level + 2));
                result.push_str(&format!("{}    }})\n", indent));
            }
            Operation::AlterColumn {
                table,
                column,
                new_definition,
            } => {
                result.push_str(&format!(
                    "{}    .add_operation(Operation::AlterColumn {{\n",
                    indent
                ));
                result.push_str(&format!(
                    "{}        table: \"{}\".to_string(),\n",
                    indent, table
                ));
                result.push_str(&format!(
                    "{}        column: \"{}\".to_string(),\n",
                    indent, column
                ));
                result.push_str(&format!("{}        new_definition: ", indent));
                result.push_str(&self.serialize_column(new_definition, indent_level + 2));
                result.push_str(&format!("{}    }})\n", indent));
            }
            Operation::DropColumn { table, column } => {
                result.push_str(&format!(
                    "{}    .add_operation(Operation::DropColumn {{\n",
                    indent
                ));
                result.push_str(&format!(
                    "{}        table: \"{}\".to_string(),\n",
                    indent, table
                ));
                result.push_str(&format!(
                    "{}        column: \"{}\".to_string(),\n",
                    indent, column
                ));
                result.push_str(&format!("{}    }})\n", indent));
            }
            _ => {
                // Other operations not yet supported
                result.push_str(&format!("{}    // Unsupported operation\n", indent));
            }
        }

        result
    }

    /// Serialize a column definition to Rust code
    fn serialize_column(&self, column: &crate::ColumnDefinition, indent_level: usize) -> String {
        let indent = "    ".repeat(indent_level);
        let mut result = String::new();

        result.push_str("ColumnDefinition {\n");
        result.push_str(&format!(
            "{}    name: \"{}\".to_string(),\n",
            indent, column.name
        ));
        result.push_str(&format!(
            "{}    type_definition: \"{}\".to_string(),\n",
            indent, column.type_definition
        ));
        result.push_str(&format!("{}}},\n", indent));

        result
    }
    /// Write migration to file
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use reinhardt_migrations::{Migration, writer::MigrationWriter};
    /// use std::path::PathBuf;
    ///
    /// let migration = Migration::new("0001_initial", "myapp");
    /// let writer = MigrationWriter::new(migration);
    ///
    /// let temp_dir = PathBuf::from("/tmp/migrations");
    /// let filepath = writer.write_to_file(&temp_dir).unwrap();
    /// assert!(filepath.ends_with("0001_initial.rs"));
    /// ```
    pub fn write_to_file<P: AsRef<Path>>(&self, directory: P) -> Result<String> {
        let dir_path = directory.as_ref();
        fs::create_dir_all(dir_path)?;

        let filename = format!("{}.rs", self.migration.name);
        let filepath = dir_path.join(&filename);

        fs::write(&filepath, self.as_string())?;

        Ok(filepath.to_string_lossy().into_owned())
    }
}

// Tests are in tests/test_writer.rs
