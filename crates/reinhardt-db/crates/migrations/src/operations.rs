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
//! use reinhardt_migrations::{ProjectState, FieldType};
//!
//! let mut state = ProjectState::new();
//!
//! // Create a model
//! let create = CreateModel::new(
//!     "User",
//!     vec![
//!         FieldDefinition::new("id", FieldType::Integer, true, false, Option::<&str>::None),
//!         FieldDefinition::new("name", FieldType::VarChar(100), false, false, Option::<&str>::None),
//!     ],
//! );
//! create.state_forwards("myapp", &mut state);
//!
//! // Add a field
//! let add = AddField::new("User", FieldDefinition::new("email", FieldType::VarChar(255), false, false, Option::<&str>::None));
//! add.state_forwards("myapp", &mut state);
//!
//! // Run custom SQL
//! let sql = RunSQL::new("CREATE INDEX idx_email ON myapp_user(email)");
//! ```

pub mod fields;
pub mod models;
pub mod postgres;
pub mod special;
mod to_tokens;

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

/// Index type for database indexes
///
/// Specifies the type of index to create. Different index types have different
/// performance characteristics and support different operators.
///
/// # Examples
///
/// ```rust
/// use reinhardt_migrations::operations::IndexType;
///
/// // B-Tree is the default, best for equality and range queries
/// let btree = IndexType::BTree;
///
/// // Hash is best for simple equality comparisons
/// let hash = IndexType::Hash;
///
/// // GIN is best for containment operators (arrays, JSONB)
/// let gin = IndexType::Gin;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IndexType {
	/// B-tree index (default)
	///
	/// Best for: equality and range queries (=, <, >, <=, >=, BETWEEN)
	/// Supported by: All databases
	#[default]
	BTree,

	/// Hash index
	///
	/// Best for: simple equality comparisons (=)
	/// Supported by: PostgreSQL, MySQL
	Hash,

	/// GIN (Generalized Inverted Index)
	///
	/// Best for: composite values (arrays, JSONB, full-text search)
	/// Supported by: PostgreSQL
	Gin,

	/// GiST (Generalized Search Tree)
	///
	/// Best for: geometric data, full-text search, range types
	/// Supported by: PostgreSQL
	Gist,

	/// BRIN (Block Range Index)
	///
	/// Best for: very large tables with naturally ordered data
	/// Supported by: PostgreSQL
	Brin,

	/// Full-text index
	///
	/// Best for: full-text search on text columns
	/// Supported by: MySQL
	Fulltext,

	/// Spatial index
	///
	/// Best for: geometric/geographic data
	/// Supported by: MySQL
	Spatial,
}

impl std::fmt::Display for IndexType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			IndexType::BTree => write!(f, "btree"),
			IndexType::Hash => write!(f, "hash"),
			IndexType::Gin => write!(f, "gin"),
			IndexType::Gist => write!(f, "gist"),
			IndexType::Brin => write!(f, "brin"),
			IndexType::Fulltext => write!(f, "fulltext"),
			IndexType::Spatial => write!(f, "spatial"),
		}
	}
}

/// Constraint definition for tables
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "type")]
pub enum Constraint {
	/// ForeignKey constraint
	ForeignKey {
		name: String,
		columns: Vec<String>,
		referenced_table: String,
		referenced_columns: Vec<String>,
		on_delete: crate::ForeignKeyAction,
		on_update: crate::ForeignKeyAction,
	},
	/// Unique constraint
	Unique { name: String, columns: Vec<String> },
	/// Check constraint
	Check { name: String, expression: String },
	/// OneToOne constraint (ForeignKey + Unique combination)
	OneToOne {
		name: String,
		column: String,
		referenced_table: String,
		referenced_column: String,
		on_delete: crate::ForeignKeyAction,
		on_update: crate::ForeignKeyAction,
	},
	/// ManyToMany relationship metadata (intermediate table reference)
	ManyToMany {
		name: String,
		through_table: String,
		source_column: String,
		target_column: String,
		target_table: String,
	},
}

impl std::fmt::Display for Constraint {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Constraint::ForeignKey {
				name,
				columns,
				referenced_table,
				referenced_columns,
				on_delete,
				on_update,
			} => {
				write!(
					f,
					"CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}({}) ON DELETE {} ON UPDATE {}",
					name,
					columns.join(", "),
					referenced_table,
					referenced_columns.join(", "),
					on_delete.to_sql_keyword(),
					on_update.to_sql_keyword()
				)
			}
			Constraint::Unique { name, columns } => {
				write!(f, "CONSTRAINT {} UNIQUE ({})", name, columns.join(", "))
			}
			Constraint::Check { name, expression } => {
				write!(f, "CONSTRAINT {} CHECK ({})", name, expression)
			}
			Constraint::OneToOne {
				name,
				column,
				referenced_table,
				referenced_column,
				on_delete,
				on_update,
			} => {
				write!(
					f,
					"CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}({}) ON DELETE {} ON UPDATE {}, CONSTRAINT {}_unique UNIQUE ({})",
					name,
					column,
					referenced_table,
					referenced_column,
					on_delete.to_sql_keyword(),
					on_update.to_sql_keyword(),
					name,
					column
				)
			}
			Constraint::ManyToMany { through_table, .. } => {
				write!(f, "-- ManyToMany via {}", through_table)
			}
		}
	}
}

/// A migration operation (legacy enum for backward compatibility)
///
/// This enum is maintained for backward compatibility with existing code.
/// New code should use the specific operation types from the `models`, `fields`,
/// and `special` modules instead.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Operation {
	CreateTable {
		name: &'static str,
		columns: Vec<ColumnDefinition>,
		#[serde(default)]
		constraints: Vec<Constraint>,
	},
	DropTable {
		name: &'static str,
	},
	AddColumn {
		table: &'static str,
		column: ColumnDefinition,
	},
	DropColumn {
		table: &'static str,
		column: &'static str,
	},
	AlterColumn {
		table: &'static str,
		column: &'static str,
		new_definition: ColumnDefinition,
	},
	RenameTable {
		old_name: &'static str,
		new_name: &'static str,
	},
	RenameColumn {
		table: &'static str,
		old_name: &'static str,
		new_name: &'static str,
	},
	AddConstraint {
		table: &'static str,
		constraint_sql: &'static str,
	},
	DropConstraint {
		table: &'static str,
		constraint_name: &'static str,
	},
	CreateIndex {
		table: &'static str,
		columns: Vec<&'static str>,
		unique: bool,
		/// Index type (B-Tree, Hash, GIN, GiST, etc.)
		///
		/// If not specified, the database will use its default index type (typically B-Tree).
		#[serde(default, skip_serializing_if = "Option::is_none")]
		index_type: Option<IndexType>,
		/// Partial index condition (WHERE clause)
		///
		/// Creates a partial index that only indexes rows matching this condition.
		/// Example: "status = 'active'" creates an index only for active rows.
		#[serde(default, skip_serializing_if = "Option::is_none")]
		where_clause: Option<&'static str>,
		/// Create index concurrently (PostgreSQL-specific)
		///
		/// When true, creates the index without locking the table for writes.
		/// This is slower but allows concurrent operations during index creation.
		#[serde(default)]
		concurrently: bool,
	},
	DropIndex {
		table: &'static str,
		columns: Vec<&'static str>,
	},
	RunSQL {
		sql: &'static str,
		reverse_sql: Option<&'static str>,
	},
	RunRust {
		code: &'static str,
		reverse_code: Option<&'static str>,
	},
	AlterTableComment {
		table: &'static str,
		comment: Option<&'static str>,
	},
	AlterUniqueTogether {
		table: &'static str,
		unique_together: Vec<Vec<&'static str>>,
	},
	AlterModelOptions {
		table: &'static str,
		options: std::collections::HashMap<&'static str, &'static str>,
	},
	CreateInheritedTable {
		name: &'static str,
		columns: Vec<ColumnDefinition>,
		base_table: &'static str,
		join_column: &'static str,
	},
	AddDiscriminatorColumn {
		table: &'static str,
		column_name: &'static str,
		default_value: &'static str,
	},
	/// Move a model from one app to another
	///
	/// This operation handles cross-app model moves by:
	/// 1. Optionally renaming the table (if naming convention changes between apps)
	/// 2. Updating FK references to use the new table name
	///
	/// Note: This generates a RenameTable SQL if table name changes.
	/// The state tracking (from_app -> to_app) is handled at the ProjectState level.
	MoveModel {
		/// Name of the model being moved
		model_name: &'static str,
		/// Source app label
		from_app: &'static str,
		/// Target app label
		to_app: &'static str,
		/// Whether to rename the underlying table
		rename_table: bool,
		/// Old table name (if rename_table is true)
		old_table_name: Option<&'static str>,
		/// New table name (if rename_table is true)
		new_table_name: Option<&'static str>,
	},
	/// Create a database schema (PostgreSQL, MySQL 5.0.2+)
	///
	/// Creates a new database schema namespace. In MySQL, this is equivalent to creating a database.
	CreateSchema {
		/// Name of the schema to create
		name: &'static str,
		/// Whether to add IF NOT EXISTS clause
		#[serde(default)]
		if_not_exists: bool,
	},
	/// Drop a database schema
	///
	/// Drops an existing database schema. Use with caution as this will drop all objects in the schema.
	DropSchema {
		/// Name of the schema to drop
		name: &'static str,
		/// Whether to add CASCADE clause (drops all contained objects)
		#[serde(default)]
		cascade: bool,
		/// Whether to add IF EXISTS clause
		#[serde(default = "default_true")]
		if_exists: bool,
	},
	/// Create a PostgreSQL extension (PostgreSQL-specific)
	///
	/// Creates a PostgreSQL extension like PostGIS, uuid-ossp, etc.
	/// This operation is only executed on PostgreSQL databases.
	CreateExtension {
		/// Name of the extension to create
		name: &'static str,
		/// Whether to add IF NOT EXISTS clause
		#[serde(default = "default_true")]
		if_not_exists: bool,
		/// Optional schema to install the extension in
		#[serde(default)]
		schema: Option<&'static str>,
	},
}

/// Default value provider for serde (returns true)
const fn default_true() -> bool {
	true
}

impl Operation {
	/// Apply this operation to the project state (forward)
	pub fn state_forwards(&self, app_label: &str, state: &mut ProjectState) {
		match self {
			Operation::CreateTable { name, columns, .. } => {
				let mut model = ModelState::new(app_label, *name);
				for column in columns {
					let field = FieldState::new(
						column.name.to_string(),
						column.type_definition.clone(),
						false,
					);
					model.add_field(field);
				}
				state.add_model(model);
			}
			Operation::DropTable { name } => {
				state.remove_model(app_label, name);
			}
			Operation::AddColumn { table, column } => {
				if let Some(model) = state.get_model_mut(app_label, table) {
					let field = FieldState::new(
						column.name.to_string(),
						column.type_definition.clone(),
						false,
					);
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
						column.to_string(),
						new_definition.type_definition.clone(),
						false,
					);
					model.alter_field(column, field);
				}
			}
			Operation::RenameTable { old_name, new_name } => {
				state.rename_model(app_label, old_name, new_name.to_string());
			}
			Operation::RenameColumn {
				table,
				old_name,
				new_name,
			} => {
				if let Some(model) = state.get_model_mut(app_label, table) {
					model.rename_field(old_name, new_name.to_string());
				}
			}
			Operation::CreateInheritedTable {
				name,
				columns,
				base_table,
				join_column,
			} => {
				let mut model = ModelState::new(app_label, *name);
				model.base_model = Some(base_table.to_string());
				model.inheritance_type = Some("joined_table".to_string());

				let join_field = FieldState::new(
					join_column.to_string(),
					crate::FieldType::Custom(format!("INTEGER REFERENCES {}(id)", base_table)),
					false,
				);
				model.add_field(join_field);

				for column in columns {
					let field = FieldState::new(
						column.name.to_string(),
						column.type_definition.clone(),
						false,
					);
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
					model.discriminator_column = Some(column_name.to_string());
					model.inheritance_type = Some("single_table".to_string());
					let field = FieldState::new(
						column_name.to_string(),
						crate::FieldType::Custom(format!(
							"VARCHAR(50) DEFAULT '{}'",
							default_value
						)),
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
			Operation::MoveModel {
				model_name,
				from_app,
				to_app,
				rename_table,
				old_table_name,
				new_table_name,
			} => {
				// Move the model from one app to another in the project state
				// First get the model, then remove it from the old location
				if let Some(model) = state.get_model(from_app, model_name).cloned() {
					state.remove_model(from_app, model_name);

					// Create a new model with updated app label
					let mut new_model = model;
					new_model.app_label = to_app.to_string();

					// Update table name if rename_table is true
					if *rename_table
						&& let (Some(_old_name), Some(new_name)) = (old_table_name, new_table_name)
					{
						new_model.table_name = new_name.to_string();
					}

					state.add_model(new_model);
				}
			}
			// Schema operations don't affect ProjectState (models/fields only)
			Operation::CreateSchema { .. }
			| Operation::DropSchema { .. }
			| Operation::CreateExtension { .. } => {
				// No state changes for schema/extension operations
			}
		}
	}

	/// Generate column SQL with all constraints
	fn column_to_sql(col: &ColumnDefinition, dialect: &SqlDialect) -> String {
		let mut parts = Vec::new();

		// Column name
		parts.push(col.name.to_string());

		// Column type (with auto_increment handling for PostgreSQL)
		if col.auto_increment {
			match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => {
					// PostgreSQL 10+ uses GENERATED BY DEFAULT AS IDENTITY
					match &col.type_definition {
						crate::FieldType::BigInteger => {
							parts.push("BIGINT GENERATED BY DEFAULT AS IDENTITY".to_string());
						}
						crate::FieldType::Integer => {
							parts.push("INTEGER GENERATED BY DEFAULT AS IDENTITY".to_string());
						}
						crate::FieldType::SmallInteger => {
							parts.push("SMALLINT GENERATED BY DEFAULT AS IDENTITY".to_string());
						}
						_ => {
							// Fallback for other types
							parts.push(col.type_definition.to_sql_for_dialect(dialect));
						}
					}
				}
				SqlDialect::Mysql => {
					parts.push(col.type_definition.to_sql_for_dialect(dialect));
					parts.push("AUTO_INCREMENT".to_string());
				}
				SqlDialect::Sqlite => {
					// SQLite uses INTEGER PRIMARY KEY for auto_increment
					// This is handled specially - INTEGER PRIMARY KEY is implicitly AUTOINCREMENT
					parts.push(col.type_definition.to_sql_for_dialect(dialect));
				}
			}
		} else {
			parts.push(col.type_definition.to_sql_for_dialect(dialect));
		}

		// NOT NULL constraint
		if col.not_null {
			parts.push("NOT NULL".to_string());
		}

		// PRIMARY KEY constraint
		if col.primary_key {
			parts.push("PRIMARY KEY".to_string());
		}

		// UNIQUE constraint
		if col.unique {
			parts.push("UNIQUE".to_string());
		}

		// DEFAULT value
		if let Some(default) = col.default {
			parts.push(format!("DEFAULT {}", default));
		}

		parts.join(" ")
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
					parts.push(format!("  {}", Self::column_to_sql(col, dialect)));
				}
				for constraint in constraints {
					parts.push(format!("  {}", constraint));
				}
				format!("CREATE TABLE {} (\n{}\n);", name, parts.join(",\n"))
			}
			Operation::DropTable { name } => format!("DROP TABLE {};", name),
			Operation::AddColumn { table, column } => {
				format!(
					"ALTER TABLE {} ADD COLUMN {};",
					table,
					Self::column_to_sql(column, dialect)
				)
			}
			Operation::DropColumn { table, column } => {
				format!("ALTER TABLE {} DROP COLUMN {};", table, column)
			}
			Operation::AlterColumn {
				table,
				column,
				new_definition,
			} => {
				let sql_type = new_definition.type_definition.to_sql_for_dialect(dialect);
				match dialect {
					SqlDialect::Postgres | SqlDialect::Cockroachdb => {
						format!(
							"ALTER TABLE {} ALTER COLUMN {} TYPE {};",
							table, column, sql_type
						)
					}
					SqlDialect::Mysql => {
						format!(
							"ALTER TABLE {} MODIFY COLUMN {} {};",
							table, column, sql_type
						)
					}
					SqlDialect::Sqlite => {
						format!(
							"-- SQLite does not support ALTER COLUMN, table recreation required for {}",
							table
						)
					}
				}
			}
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
				..
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
				match dialect {
					SqlDialect::Mysql => {
						format!("DROP INDEX {} ON {};", idx_name, table)
					}
					SqlDialect::Postgres | SqlDialect::Sqlite | SqlDialect::Cockroachdb => {
						format!("DROP INDEX {};", idx_name)
					}
				}
			}
			Operation::RunSQL { sql, .. } => sql.to_string(),
			Operation::RunRust { code, .. } => {
				// For SQL generation, RunRust is a no-op comment
				format!("-- RunRust: {}", code.lines().next().unwrap_or(""))
			}
			Operation::AlterTableComment { table, comment } => match dialect {
				SqlDialect::Postgres | SqlDialect::Cockroachdb => {
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
					parts.push(format!("  {}", Self::column_to_sql(col, dialect)));
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
			Operation::MoveModel {
				rename_table,
				old_table_name,
				new_table_name,
				..
			} => {
				// MoveModel generates a RenameTable SQL if table name changes
				// Otherwise it's a state-only operation (no SQL needed)
				if *rename_table {
					if let (Some(old_name), Some(new_name)) = (old_table_name, new_table_name) {
						match dialect {
							SqlDialect::Postgres | SqlDialect::Sqlite | SqlDialect::Cockroachdb => {
								format!("ALTER TABLE {} RENAME TO {};", old_name, new_name)
							}
							SqlDialect::Mysql => {
								format!("RENAME TABLE {} TO {};", old_name, new_name)
							}
						}
					} else {
						"-- MoveModel: No table rename specified".to_string()
					}
				} else {
					// State-only operation, no SQL needed
					"-- MoveModel: State-only operation (no table rename)".to_string()
				}
			}
			Operation::CreateSchema {
				name,
				if_not_exists,
			} => {
				let if_not_exists_clause = if *if_not_exists { " IF NOT EXISTS" } else { "" };
				format!("CREATE SCHEMA{} {};", if_not_exists_clause, name)
			}
			Operation::DropSchema {
				name,
				cascade,
				if_exists,
			} => {
				let if_exists_clause = if *if_exists { " IF EXISTS" } else { "" };
				let cascade_clause = if *cascade { " CASCADE" } else { "" };
				format!(
					"DROP SCHEMA{} {}{};",
					if_exists_clause, name, cascade_clause
				)
			}
			Operation::CreateExtension {
				name,
				if_not_exists,
				schema,
			} => {
				// PostgreSQL-specific
				let if_not_exists_clause = if *if_not_exists { " IF NOT EXISTS" } else { "" };
				let schema_clause = if let Some(s) = schema {
					format!(" SCHEMA {}", s)
				} else {
					String::new()
				};
				format!(
					"CREATE EXTENSION{} {}{};",
					if_not_exists_clause, name, schema_clause
				)
			}
		}
	}

	/// Generate reverse SQL (for rollback)
	///
	/// # Arguments
	///
	/// * `dialect` - SQL dialect for generating database-specific SQL
	/// * `project_state` - Project state for accessing model definitions (needed for DropTable, etc.)
	///
	/// # Returns
	///
	/// * `Ok(Some(sql))` - Reverse SQL generated successfully
	/// * `Ok(None)` - Operation is not reversible (e.g., DropTable without state)
	/// * `Err(_)` - Error generating reverse SQL
	pub fn to_reverse_sql(
		&self,
		dialect: &SqlDialect,
		_project_state: &ProjectState,
	) -> crate::Result<Option<String>> {
		match self {
			Operation::CreateTable { name, .. } => Ok(Some(format!("DROP TABLE {};", name))),
			Operation::AddColumn { table, column } => Ok(Some(format!(
				"ALTER TABLE {} DROP COLUMN {};",
				table, column.name
			))),
			Operation::RunSQL { reverse_sql, .. } => {
				Ok(reverse_sql.as_ref().map(|s| s.to_string()))
			}
			Operation::RunRust { reverse_code, .. } => Ok(reverse_code.as_ref().map(|code| {
				format!(
					"-- RunRust (reverse): {}",
					code.lines().next().unwrap_or("")
				)
			})),
			// Phase 1: Simple reverse operations
			Operation::RenameTable { old_name, new_name } => Ok(Some(format!(
				"ALTER TABLE {} RENAME TO {};",
				new_name, old_name
			))),
			Operation::RenameColumn {
				table,
				old_name,
				new_name,
			} => Ok(Some(format!(
				"ALTER TABLE {} RENAME COLUMN {} TO {};",
				table, new_name, old_name
			))),
			Operation::CreateIndex {
				table,
				columns,
				unique,
				..
			} => {
				// Generate index name following Django convention: {table}_{columns}_{idx/unique}
				let columns_joined = columns.join("_");
				let suffix = if *unique { "unique" } else { "idx" };
				let index_name = format!("{}_{}_{}", table, columns_joined, suffix);
				Ok(Some(format!("DROP INDEX {};", index_name)))
			}
			Operation::AddConstraint {
				table,
				constraint_sql,
			} => {
				// Extract constraint name from SQL
				// Expects format: "CONSTRAINT <name> ..." or "ADD CONSTRAINT <name> ..."
				let constraint_name =
					Self::extract_constraint_name(constraint_sql).ok_or_else(|| {
						crate::MigrationError::InvalidMigration(format!(
							"Cannot extract constraint name from: {}",
							constraint_sql
						))
					})?;
				Ok(Some(format!(
					"ALTER TABLE {} DROP CONSTRAINT {};",
					table, constraint_name
				)))
			}
			// Phase 2: Complex reverse operations
			Operation::DropColumn { table, column } => {
				// TODO: In full implementation, retrieve original column definition from ProjectState
				// For now, return None as we cannot reconstruct the full column definition
				// A complete implementation would need to look up the column in project_state
				// and generate: ALTER TABLE {table} ADD COLUMN {column} {type} {constraints}
				Ok(None)
			}
			Operation::AlterColumn {
				table,
				column,
				new_definition,
			} => {
				// TODO: In full implementation, retrieve original column definition from ProjectState
				// For now, return None as we cannot know the previous column state
				// A complete implementation would need to compare with project_state
				// and generate: ALTER TABLE {table} ALTER COLUMN {column} {old_definition}
				Ok(None)
			}
			Operation::DropIndex { table, columns } => {
				// Generate CREATE INDEX statement from dropped index information
				// This is a simplified implementation - full implementation would need index type, where clause, etc.
				let columns_joined = columns.join("_");
				let index_name = format!("{}_{}_idx", table, columns_joined);
				let columns_list = columns.join(", ");
				Ok(Some(format!(
					"CREATE INDEX {} ON {} ({});",
					index_name, table, columns_list
				)))
			}
			Operation::DropConstraint {
				table,
				constraint_name,
			} => {
				// TODO: In full implementation, retrieve constraint definition from ProjectState
				// Cannot reconstruct constraint SQL without knowing its type (CHECK, FOREIGN KEY, etc.)
				// A complete implementation would need to look up the constraint in project_state
				// and generate: ALTER TABLE {table} ADD CONSTRAINT {name} {definition}
				Ok(None)
			}
			Operation::DropTable { name } => {
				// TODO: In full implementation, retrieve table definition from ProjectState
				// Cannot reconstruct CREATE TABLE without knowing columns, constraints, etc.
				// A complete implementation would need to look up the model in project_state
				// and generate: CREATE TABLE {name} ({columns}) {constraints}
				// Note: This is intentionally last to override the earlier DropTable => Ok(None)
				Ok(None)
			}
			_ => Ok(None),
		}
	}

	/// Apply operation to project state (backward/reverse)
	///
	/// This method updates the ProjectState to reflect the reverse of this operation.
	/// Used during migration rollback to track state changes.
	///
	/// # Arguments
	///
	/// * `app_label` - Application label for the model being modified
	/// * `state` - Mutable reference to the ProjectState to update
	///
	/// # Note
	///
	/// This is a basic implementation. Full implementation would require:
	/// - Tracking column additions/removals in ModelState
	/// - Managing constraints and indexes
	/// - Handling complex operations like AlterColumn
	pub fn state_backwards(&self, app_label: &str, state: &mut ProjectState) {
		match self {
			Operation::CreateTable { name, .. } => {
				// Reverse: Remove the model from state
				state
					.models
					.remove(&(app_label.to_string(), name.to_string()));
			}
			Operation::DropTable { name } => {
				// TODO: Reverse: Add the model back to state
				// Would need to reconstruct ModelState from somewhere
				// For now, this is a no-op
			}
			Operation::RenameTable { old_name, new_name } => {
				// Reverse: Rename back from new_name to old_name
				if let Some(model) = state
					.models
					.remove(&(app_label.to_string(), new_name.to_string()))
				{
					state
						.models
						.insert((app_label.to_string(), old_name.to_string()), model);
				}
			}
			Operation::AddColumn { .. }
			| Operation::DropColumn { .. }
			| Operation::AlterColumn { .. }
			| Operation::RenameColumn { .. } => {
				// TODO: Update field state in the model
				// Would need to track individual field changes in ModelState
				// For now, this is a no-op
			}
			_ => {
				// Other operations don't affect ProjectState or are not yet implemented
			}
		}
	}

	/// Extract constraint name from constraint SQL
	///
	/// Supports patterns:
	/// - "CONSTRAINT name CHECK ..."
	/// - "ADD CONSTRAINT name ..."
	fn extract_constraint_name(constraint_sql: &str) -> Option<String> {
		let sql = constraint_sql.trim();

		// Pattern 1: "CONSTRAINT name ..."
		if sql.starts_with("CONSTRAINT ") || sql.contains(" CONSTRAINT ") {
			let parts: Vec<&str> = sql.split_whitespace().collect();
			if let Some(pos) = parts.iter().position(|&s| s == "CONSTRAINT")
				&& pos + 1 < parts.len()
			{
				return Some(parts[pos + 1].to_string());
			}
		}

		None
	}
}

/// Column definition for legacy operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnDefinition {
	pub name: &'static str,
	pub type_definition: crate::FieldType,
	#[serde(default)]
	pub not_null: bool,
	#[serde(default)]
	pub unique: bool,
	#[serde(default)]
	pub primary_key: bool,
	#[serde(default)]
	pub auto_increment: bool,
	#[serde(default)]
	pub default: Option<&'static str>,
}

impl ColumnDefinition {
	/// Create a new column definition
	pub fn new(name: impl Into<String>, type_def: crate::FieldType) -> Self {
		Self {
			name: Box::leak(name.into().into_boxed_str()),
			type_definition: type_def,
			not_null: false,
			unique: false,
			primary_key: false,
			auto_increment: false,
			default: None,
		}
	}

	/// Create a ColumnDefinition from FieldState with attribute parsing
	///
	/// This method reads field attributes (primary_key, not_null, unique, etc.) from
	/// the FieldState.params HashMap and properly initializes ColumnDefinition fields.
	///
	/// # Arguments
	///
	/// * `name` - Column name
	/// * `field_state` - FieldState containing field metadata and params
	///
	/// # Notes
	///
	/// - If `primary_key` is true, `not_null` is automatically set to true
	/// - Default values are false/None for unspecified attributes
	pub fn from_field_state(name: impl Into<String>, field_state: &FieldState) -> Self {
		let name_str = name.into();
		let params = &field_state.params;

		// Parse attributes from params HashMap
		let primary_key = params
			.get("primary_key")
			.and_then(|v| v.parse::<bool>().ok())
			.unwrap_or(false);

		let not_null = params
			.get("not_null")
			.and_then(|v| v.parse::<bool>().ok())
			.or(if primary_key { Some(true) } else { None })
			.unwrap_or(false);

		let unique = params
			.get("unique")
			.and_then(|v| v.parse::<bool>().ok())
			.unwrap_or(false);

		let auto_increment = params
			.get("auto_increment")
			.and_then(|v| v.parse::<bool>().ok())
			.unwrap_or(false);

		let default = params
			.get("default")
			.map(|s| Box::leak(s.clone().into_boxed_str()) as &'static str);

		Self {
			name: Box::leak(name_str.into_boxed_str()),
			type_definition: field_state.field_type.clone(),
			not_null,
			unique,
			primary_key,
			auto_increment,
			default,
		}
	}
}

/// SQL dialect for generating database-specific SQL
#[derive(Debug, Clone, Copy)]
pub enum SqlDialect {
	Sqlite,
	Postgres,
	Mysql,
	Cockroachdb,
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
	///
	/// # Arguments
	///
	/// * `db_type` - Database type to generate SQL for (PostgreSQL, MySQL, SQLite)
	pub fn to_sql_string(&self, db_type: reinhardt_backends::types::DatabaseType) -> String {
		use sea_query::{MysqlQueryBuilder, SqliteQueryBuilder};

		match self {
			OperationStatement::TableCreate(stmt) => match db_type {
				reinhardt_backends::types::DatabaseType::Postgres => {
					stmt.to_string(PostgresQueryBuilder)
				}
				reinhardt_backends::types::DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
				reinhardt_backends::types::DatabaseType::Sqlite => {
					stmt.to_string(SqliteQueryBuilder)
				}
				#[cfg(feature = "mongodb-backend")]
				reinhardt_backends::types::DatabaseType::MongoDB => String::new(),
			},
			OperationStatement::TableDrop(stmt) => match db_type {
				reinhardt_backends::types::DatabaseType::Postgres => {
					stmt.to_string(PostgresQueryBuilder)
				}
				reinhardt_backends::types::DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
				reinhardt_backends::types::DatabaseType::Sqlite => {
					stmt.to_string(SqliteQueryBuilder)
				}
				#[cfg(feature = "mongodb-backend")]
				reinhardt_backends::types::DatabaseType::MongoDB => String::new(),
			},
			OperationStatement::TableAlter(stmt) => match db_type {
				reinhardt_backends::types::DatabaseType::Postgres => {
					stmt.to_string(PostgresQueryBuilder)
				}
				reinhardt_backends::types::DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
				reinhardt_backends::types::DatabaseType::Sqlite => {
					stmt.to_string(SqliteQueryBuilder)
				}
				#[cfg(feature = "mongodb-backend")]
				reinhardt_backends::types::DatabaseType::MongoDB => String::new(),
			},
			OperationStatement::TableRename(stmt) => match db_type {
				reinhardt_backends::types::DatabaseType::Postgres => {
					stmt.to_string(PostgresQueryBuilder)
				}
				reinhardt_backends::types::DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
				reinhardt_backends::types::DatabaseType::Sqlite => {
					stmt.to_string(SqliteQueryBuilder)
				}
				#[cfg(feature = "mongodb-backend")]
				reinhardt_backends::types::DatabaseType::MongoDB => String::new(),
			},
			OperationStatement::IndexCreate(stmt) => match db_type {
				reinhardt_backends::types::DatabaseType::Postgres => {
					stmt.to_string(PostgresQueryBuilder)
				}
				reinhardt_backends::types::DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
				reinhardt_backends::types::DatabaseType::Sqlite => {
					stmt.to_string(SqliteQueryBuilder)
				}
				#[cfg(feature = "mongodb-backend")]
				reinhardt_backends::types::DatabaseType::MongoDB => String::new(),
			},
			OperationStatement::IndexDrop(stmt) => match db_type {
				reinhardt_backends::types::DatabaseType::Postgres => {
					stmt.to_string(PostgresQueryBuilder)
				}
				reinhardt_backends::types::DatabaseType::Mysql => stmt.to_string(MysqlQueryBuilder),
				reinhardt_backends::types::DatabaseType::Sqlite => {
					stmt.to_string(SqliteQueryBuilder)
				}
				#[cfg(feature = "mongodb-backend")]
				reinhardt_backends::types::DatabaseType::MongoDB => String::new(),
			},
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
				..
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
			Operation::RunSQL { sql, .. } => OperationStatement::RawSql(sql.to_string()),
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
				stmt.table(Alias::new(*name)).if_not_exists();

				// Add join column (foreign key to base table)
				let mut join_col = ColumnDef::new(Alias::new(*join_column));
				join_col.integer();
				stmt.col(&mut join_col);

				// Add other columns
				for col in columns {
					let mut column = ColumnDef::new(Alias::new(col.name));
					self.apply_column_type(&mut column, &col.type_definition);
					stmt.col(&mut column);
				}

				// Add foreign key
				let mut fk = ForeignKey::create();
				fk.from_tbl(Alias::new(*name))
					.from_col(Alias::new(*join_column))
					.to_tbl(Alias::new(*base_table))
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
				stmt.table(Alias::new(*table));

				let mut col = ColumnDef::new(Alias::new(*column_name));
				col.string_len(50).default(default_value.to_string());
				stmt.add_column(&mut col);

				OperationStatement::TableAlter(stmt.to_owned())
			}
			Operation::MoveModel {
				rename_table,
				old_table_name,
				new_table_name,
				..
			} => {
				// MoveModel generates a table rename if table name changes
				if *rename_table {
					if let (Some(old_name), Some(new_name)) = (old_table_name, new_table_name) {
						OperationStatement::TableRename(self.build_rename_table(old_name, new_name))
					} else {
						// No table rename needed
						OperationStatement::RawSql("-- MoveModel: State-only operation".to_string())
					}
				} else {
					// State-only operation, no SQL
					OperationStatement::RawSql("-- MoveModel: State-only operation".to_string())
				}
			}
			Operation::CreateSchema {
				name,
				if_not_exists,
			} => {
				// Use schema.rs helper (Sea-Query doesn't support CREATE SCHEMA)
				let sql = if *if_not_exists {
					format!("CREATE SCHEMA IF NOT EXISTS {}", quote_identifier(name))
				} else {
					format!("CREATE SCHEMA {}", quote_identifier(name))
				};
				OperationStatement::RawSql(sql)
			}
			Operation::DropSchema {
				name,
				cascade,
				if_exists,
			} => {
				// Use schema.rs helper (Sea-Query doesn't support DROP SCHEMA)
				let if_exists_clause = if *if_exists { " IF EXISTS" } else { "" };
				let cascade_clause = if *cascade { " CASCADE" } else { "" };
				let sql = format!(
					"DROP SCHEMA{} {}{}",
					if_exists_clause,
					quote_identifier(name),
					cascade_clause
				);
				OperationStatement::RawSql(sql)
			}
			Operation::CreateExtension {
				name,
				if_not_exists,
				schema,
			} => {
				// PostgreSQL-specific: Use extensions.rs helper
				let if_not_exists_clause = if *if_not_exists { " IF NOT EXISTS" } else { "" };
				let schema_clause = if let Some(s) = schema {
					format!(" SCHEMA {}", quote_identifier(s))
				} else {
					String::new()
				};
				let sql = format!(
					"CREATE EXTENSION{} {}{}",
					if_not_exists_clause,
					quote_identifier(name),
					schema_clause
				);
				OperationStatement::RawSql(sql)
			}
		}
	}

	/// Build CREATE TABLE statement
	fn build_create_table(
		&self,
		name: &str,
		columns: &[ColumnDefinition],
		constraints: &[Constraint],
	) -> TableCreateStatement {
		let mut stmt = Table::create();
		stmt.table(Alias::new(name)).if_not_exists();

		for col in columns {
			let mut column = ColumnDef::new(Alias::new(col.name));
			self.apply_column_type(&mut column, &col.type_definition);

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
			if let Some(default) = col.default {
				column.default(self.convert_default_value(default));
			}

			stmt.col(&mut column);
		}

		// Add table-level constraints
		for constraint in constraints {
			match constraint {
				Constraint::ForeignKey {
					name,
					columns,
					referenced_table,
					referenced_columns,
					on_delete,
					on_update,
				} => {
					let mut fk = sea_query::ForeignKey::create();
					fk.name(name)
						.from_tbl(Alias::new(name))
						.to_tbl(Alias::new(referenced_table.as_str()));

					for col in columns {
						fk.from_col(Alias::new(col.as_str()));
					}
					for col in referenced_columns {
						fk.to_col(Alias::new(col.as_str()));
					}

					fk.on_delete((*on_delete).into());
					fk.on_update((*on_update).into());

					stmt.foreign_key(&mut fk);
				}
				Constraint::Unique { name, columns } => {
					let mut index = sea_query::Index::create();
					index.name(name).table(Alias::new(name)).unique();
					for col in columns {
						index.col(Alias::new(col.as_str()));
					}
					// Note: SeaQuery doesn't support adding UNIQUE constraints directly in CREATE TABLE
					// They should be added separately with CREATE INDEX or ALTER TABLE
				}
				Constraint::Check { name, expression } => {
					// Note: SeaQuery doesn't have direct CHECK constraint support
					// This would need to be handled with raw SQL if needed
					let _ = (name, expression); // Suppress unused warnings
				}
				Constraint::OneToOne {
					name,
					column,
					referenced_table,
					referenced_column,
					on_delete,
					on_update,
				} => {
					// OneToOne is ForeignKey + Unique
					let mut fk = sea_query::ForeignKey::create();
					fk.name(name)
						.from_tbl(Alias::new(name))
						.to_tbl(Alias::new(referenced_table.as_str()))
						.from_col(Alias::new(column.as_str()))
						.to_col(Alias::new(referenced_column.as_str()))
						.on_delete((*on_delete).into())
						.on_update((*on_update).into());

					stmt.foreign_key(&mut fk);

					// Add UNIQUE constraint separately if needed
					// Note: This should ideally be handled via UNIQUE column definition
				}
				Constraint::ManyToMany { .. } => {
					// ManyToMany is metadata only, no actual constraint in this table
					// The intermediate table handles the relationship
				}
			}
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

		let mut col_def = ColumnDef::new(Alias::new(column.name));
		self.apply_column_type(&mut col_def, &column.type_definition);

		if column.not_null {
			col_def.not_null();
		}
		if let Some(default) = column.default {
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
		self.apply_column_type(&mut col_def, &new_definition.type_definition);

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
		columns: &[&'static str],
		unique: bool,
	) -> IndexCreateStatement {
		let mut stmt = Index::create();
		stmt.name(name).table(Alias::new(table));

		for col in columns {
			stmt.col(Alias::new(*col));
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
	fn apply_column_type(&self, col_def: &mut ColumnDef, field_type: &crate::FieldType) {
		use crate::FieldType;
		match field_type {
			FieldType::Integer => col_def.integer(),
			FieldType::BigInteger => col_def.big_integer(),
			FieldType::SmallInteger => col_def.small_integer(),
			FieldType::TinyInt => col_def.tiny_integer(),
			FieldType::VarChar(max_length) => col_def.string_len(*max_length),
			FieldType::Char(max_length) => col_def.char_len(*max_length),
			FieldType::Text | FieldType::TinyText | FieldType::MediumText | FieldType::LongText => {
				col_def.text()
			}
			FieldType::Boolean => col_def.boolean(),
			FieldType::DateTime | FieldType::TimestampTz => col_def.timestamp(),
			FieldType::Date => col_def.date(),
			FieldType::Time => col_def.time(),
			FieldType::Decimal { precision, scale } => col_def.decimal_len(*precision, *scale),
			FieldType::Float => col_def.float(),
			FieldType::Double | FieldType::Real => col_def.double(),
			FieldType::Json => col_def.json(),
			FieldType::JsonBinary => col_def.json_binary(),
			FieldType::Uuid => col_def.uuid(),
			FieldType::Binary | FieldType::Bytea => col_def.binary(),
			FieldType::Blob | FieldType::TinyBlob | FieldType::MediumBlob | FieldType::LongBlob => {
				col_def.binary()
			}
			FieldType::MediumInt => col_def.integer(),
			FieldType::Year => col_def.small_integer(),
			FieldType::Enum { values } => {
				col_def.custom(Alias::new(format!("ENUM({})", values.join(","))))
			}
			FieldType::Set { values } => {
				col_def.custom(Alias::new(format!("SET({})", values.join(","))))
			}
			FieldType::ForeignKey { .. } => {
				// ForeignKey is a relationship, the actual column is typically an integer
				col_def.integer()
			}
			FieldType::OneToOne { .. } => {
				// OneToOne is a relationship, not a column type
				// The actual column will be a foreign key (typically BigInteger)
				col_def.big_integer()
			}
			FieldType::ManyToMany { .. } => {
				// ManyToMany is a relationship, not a column type
				// No column is created in the model table (uses intermediate table)
				col_def.big_integer()
			}
			// PostgreSQL-specific types
			FieldType::Array(inner) => {
				// PostgreSQL array type: use custom with array notation
				let inner_sql = inner.to_sql_string();
				col_def.custom(Alias::new(format!("{}[]", inner_sql)))
			}
			FieldType::HStore => col_def.custom(Alias::new("HSTORE")),
			FieldType::CIText => col_def.custom(Alias::new("CITEXT")),
			FieldType::Int4Range => col_def.custom(Alias::new("INT4RANGE")),
			FieldType::Int8Range => col_def.custom(Alias::new("INT8RANGE")),
			FieldType::NumRange => col_def.custom(Alias::new("NUMRANGE")),
			FieldType::DateRange => col_def.custom(Alias::new("DATERANGE")),
			FieldType::TsRange => col_def.custom(Alias::new("TSRANGE")),
			FieldType::TsTzRange => col_def.custom(Alias::new("TSTZRANGE")),
			FieldType::TsVector => col_def.custom(Alias::new("TSVECTOR")),
			FieldType::TsQuery => col_def.custom(Alias::new("TSQUERY")),
			FieldType::Custom(custom_type) => col_def.custom(Alias::new(custom_type)),
		};
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
		if ((trimmed.starts_with('[') && trimmed.ends_with(']'))
			|| (trimmed.starts_with('{') && trimmed.ends_with('}')))
			&& let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed)
		{
			return json_to_sea_value(&json);
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

// MigrationOperation trait implementation for legacy Operation enum
use crate::operation_trait::MigrationOperation;

impl MigrationOperation for Operation {
	fn migration_name_fragment(&self) -> Option<String> {
		match self {
			Operation::CreateTable { name, .. } => Some(name.to_lowercase()),
			Operation::DropTable { name } => Some(format!("delete_{}", name.to_lowercase())),
			Operation::AddColumn { table, column } => Some(format!(
				"{}_{}",
				table.to_lowercase(),
				column.name.to_lowercase()
			)),
			Operation::DropColumn { table, column } => Some(format!(
				"remove_{}_{}",
				table.to_lowercase(),
				column.to_lowercase()
			)),
			Operation::AlterColumn { table, column, .. } => Some(format!(
				"alter_{}_{}",
				table.to_lowercase(),
				column.to_lowercase()
			)),
			Operation::RenameTable { old_name, new_name } => Some(format!(
				"rename_{}_to_{}",
				old_name.to_lowercase(),
				new_name.to_lowercase()
			)),
			Operation::RenameColumn {
				table, new_name, ..
			} => Some(format!(
				"rename_{}_{}",
				table.to_lowercase(),
				new_name.to_lowercase()
			)),
			Operation::AddConstraint { table, .. } => {
				Some(format!("add_constraint_{}", table.to_lowercase()))
			}
			Operation::DropConstraint {
				table: _,
				constraint_name,
			} => Some(format!(
				"drop_constraint_{}",
				constraint_name.to_lowercase()
			)),
			Operation::CreateIndex { table, unique, .. } => {
				if *unique {
					Some(format!("create_unique_index_{}", table.to_lowercase()))
				} else {
					Some(format!("create_index_{}", table.to_lowercase()))
				}
			}
			Operation::DropIndex { table, .. } => {
				Some(format!("drop_index_{}", table.to_lowercase()))
			}
			Operation::RunSQL { .. } => None,  // Triggers auto-naming
			Operation::RunRust { .. } => None, // Triggers auto-naming
			Operation::AlterTableComment { table, .. } => {
				Some(format!("alter_comment_{}", table.to_lowercase()))
			}
			Operation::AlterUniqueTogether { table, .. } => {
				Some(format!("alter_unique_{}", table.to_lowercase()))
			}
			Operation::AlterModelOptions { table, .. } => {
				Some(format!("alter_options_{}", table.to_lowercase()))
			}
			Operation::CreateInheritedTable { name, .. } => {
				Some(format!("create_inherited_{}", name.to_lowercase()))
			}
			Operation::AddDiscriminatorColumn { table, .. } => {
				Some(format!("add_discriminator_{}", table.to_lowercase()))
			}
			Operation::MoveModel {
				model_name,
				from_app,
				to_app,
				..
			} => Some(format!(
				"move_{}_{}_{}_{}",
				from_app.to_lowercase(),
				model_name.to_lowercase(),
				to_app.to_lowercase(),
				model_name.to_lowercase()
			)),
			Operation::CreateSchema { name, .. } => {
				Some(format!("create_schema_{}", name.to_lowercase()))
			}
			Operation::DropSchema { name, .. } => {
				Some(format!("drop_schema_{}", name.to_lowercase()))
			}
			Operation::CreateExtension { name, .. } => {
				Some(format!("create_extension_{}", name.to_lowercase()))
			}
		}
	}

	fn describe(&self) -> String {
		match self {
			Operation::CreateTable { name, .. } => format!("Create table {}", name),
			Operation::DropTable { name } => format!("Drop table {}", name),
			Operation::AddColumn { table, column } => {
				format!("Add column {} to {}", column.name, table)
			}
			Operation::DropColumn { table, column } => {
				format!("Drop column {} from {}", column, table)
			}
			Operation::AlterColumn { table, column, .. } => {
				format!("Alter column {} on {}", column, table)
			}
			Operation::RenameTable { old_name, new_name } => {
				format!("Rename table {} to {}", old_name, new_name)
			}
			Operation::RenameColumn {
				table,
				old_name,
				new_name,
			} => format!("Rename column {} to {} on {}", old_name, new_name, table),
			Operation::AddConstraint { table, .. } => format!("Add constraint on {}", table),
			Operation::DropConstraint {
				table,
				constraint_name,
			} => format!("Drop constraint {} from {}", constraint_name, table),
			Operation::CreateIndex { table, unique, .. } => {
				if *unique {
					format!("Create unique index on {}", table)
				} else {
					format!("Create index on {}", table)
				}
			}
			Operation::DropIndex { table, .. } => format!("Drop index on {}", table),
			Operation::RunSQL { sql, .. } => {
				let preview = if sql.len() > 50 {
					format!("{}...", &sql[..50])
				} else {
					(*sql).to_string()
				};
				format!("RunSQL: {}", preview)
			}
			Operation::RunRust { code, .. } => {
				let preview = if code.len() > 50 {
					format!("{}...", &code[..50])
				} else {
					(*code).to_string()
				};
				format!("RunRust: {}", preview)
			}
			Operation::AlterTableComment { table, comment } => match comment {
				Some(c) => format!("Set comment on {} to '{}'", table, c),
				None => format!("Remove comment from {}", table),
			},
			Operation::AlterUniqueTogether { table, .. } => {
				format!("Alter unique_together on {}", table)
			}
			Operation::AlterModelOptions { table, .. } => {
				format!("Alter model options on {}", table)
			}
			Operation::CreateInheritedTable {
				name, base_table, ..
			} => {
				format!("Create inherited table {} from {}", name, base_table)
			}
			Operation::AddDiscriminatorColumn {
				table, column_name, ..
			} => format!("Add discriminator column {} to {}", column_name, table),
			Operation::MoveModel {
				model_name,
				from_app,
				to_app,
				..
			} => format!("Move model {} from {} to {}", model_name, from_app, to_app),
			Operation::CreateSchema { name, .. } => format!("Create schema {}", name),
			Operation::DropSchema { name, .. } => format!("Drop schema {}", name),
			Operation::CreateExtension { name, .. } => format!("Create extension {}", name),
		}
	}

	/// Normalize operation for semantic comparison
	///
	/// Returns a normalized version where order-independent elements are sorted.
	/// This enables detection of semantically equivalent operations regardless of element ordering.
	fn normalize(&self) -> Self
	where
		Self: Sized + Clone,
	{
		match self {
			// CreateTable: Sort columns and constraints
			Operation::CreateTable {
				name,
				columns,
				constraints,
			} => {
				let mut sorted_columns = columns.clone();
				sorted_columns.sort_by(|a, b| a.name.cmp(b.name));

				let mut sorted_constraints = constraints.clone();
				sorted_constraints.sort();

				Operation::CreateTable {
					name,
					columns: sorted_columns,
					constraints: sorted_constraints,
				}
			}
			// CreateIndex: Sort columns
			Operation::CreateIndex {
				table,
				columns,
				unique,
				index_type,
				where_clause,
				concurrently,
			} => {
				let mut sorted_columns = columns.clone();
				sorted_columns.sort();

				Operation::CreateIndex {
					table,
					columns: sorted_columns,
					unique: *unique,
					index_type: *index_type,
					where_clause: *where_clause,
					concurrently: *concurrently,
				}
			}
			// DropIndex: Sort columns
			Operation::DropIndex { table, columns } => {
				let mut sorted_columns = columns.clone();
				sorted_columns.sort();

				Operation::DropIndex {
					table,
					columns: sorted_columns,
				}
			}
			// AlterUniqueTogether: Sort field lists and sort within each list
			Operation::AlterUniqueTogether {
				table,
				unique_together,
			} => {
				let mut sorted_unique_together: Vec<Vec<&'static str>> = unique_together
					.iter()
					.map(|field_list| {
						let mut sorted = field_list.clone();
						sorted.sort();
						sorted
					})
					.collect();
				sorted_unique_together.sort();

				Operation::AlterUniqueTogether {
					table,
					unique_together: sorted_unique_together,
				}
			}
			// AlterModelOptions: HashMap cannot be sorted, but we can normalize by converting to sorted Vec
			// However, since HashMap doesn't guarantee order and the operation uses HashMap,
			// we'll just clone it as-is. For true semantic equality, this would need to be changed
			// to a BTreeMap at the type level.
			Operation::AlterModelOptions { table, options } => Operation::AlterModelOptions {
				table,
				options: options.clone(),
			},
			// All other operations: Return clone (order doesn't matter or not applicable)
			_ => self.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_create_table_to_statement() {
		let op = Operation::CreateTable {
			name: "users",
			columns: vec![
				ColumnDefinition {
					name: "id",
					type_definition: crate::FieldType::Integer,
					not_null: false,
					unique: false,
					primary_key: true,
					auto_increment: true,
					default: None,
				},
				ColumnDefinition {
					name: "name",
					type_definition: crate::FieldType::VarChar(100),
					not_null: true,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
				},
			],
			constraints: vec![],
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
		let op = Operation::DropTable { name: "users" };

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			column: ColumnDefinition {
				name: "email",
				type_definition: crate::FieldType::VarChar(255),
				not_null: true,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: Some("''"),
			},
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			column: "email",
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			column: "age",
			new_definition: ColumnDefinition {
				name: "age",
				type_definition: crate::FieldType::BigInteger,
				not_null: true,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			},
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			old_name: "users",
			new_name: "accounts",
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			old_name: "name",
			new_name: "full_name",
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			constraint_sql: "CONSTRAINT age_check CHECK (age >= 0)",
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			constraint_name: "age_check",
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			columns: vec!["email"],
			unique: false,
			index_type: None,
			where_clause: None,
			concurrently: false,
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			columns: vec!["email"],
			unique: true,
			index_type: None,
			where_clause: None,
			concurrently: false,
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			columns: vec!["email"],
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			sql: "CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\"",
			reverse_sql: Some("DROP EXTENSION \"uuid-ossp\""),
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			comment: Some("User accounts table"),
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			comment: None,
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			unique_together: vec![vec!["email", "username"]],
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			unique_together: vec![],
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
		assert_eq!(
			sql, "",
			"SQL should be empty for empty unique_together constraint"
		);
	}

	#[test]
	fn test_alter_model_options_to_statement() {
		let mut options = std::collections::HashMap::new();
		options.insert("db_table", "custom_users");

		let op = Operation::AlterModelOptions {
			table: "users",
			options,
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
		assert_eq!(sql, "", "SQL should be empty for model options operation");
	}

	#[test]
	fn test_create_inherited_table_to_statement() {
		let op = Operation::CreateInheritedTable {
			name: "admin_users",
			columns: vec![ColumnDefinition {
				name: "admin_level",
				type_definition: crate::FieldType::Integer,
				not_null: true,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: Some("1"),
			}],
			base_table: "users",
			join_column: "user_id",
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			table: "users",
			column_name: "user_type",
			default_value: "regular",
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			name: "users",
			columns: vec![
				ColumnDefinition {
					name: "id",
					type_definition: crate::FieldType::Integer,
					not_null: false,
					unique: false,
					primary_key: true,
					auto_increment: true,
					default: None,
				},
				ColumnDefinition {
					name: "name",
					type_definition: crate::FieldType::VarChar(100),
					not_null: true,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
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
			crate::FieldType::Integer,
			false,
		));
		state.add_model(model);

		let op = Operation::DropTable { name: "users" };

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
			crate::FieldType::Integer,
			false,
		));
		state.add_model(model);

		let op = Operation::AddColumn {
			table: "users",
			column: ColumnDefinition {
				name: "email",
				type_definition: crate::FieldType::VarChar(255),
				not_null: true,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
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
			crate::FieldType::Integer,
			false,
		));
		model.add_field(FieldState::new(
			"email".to_string(),
			crate::FieldType::VarChar(255),
			false,
		));
		state.add_model(model);

		let op = Operation::DropColumn {
			table: "users",
			column: "email",
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
			crate::FieldType::Integer,
			false,
		));
		state.add_model(model);

		let op = Operation::RenameTable {
			old_name: "users",
			new_name: "accounts",
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
			crate::FieldType::VarChar(255),
			false,
		));
		state.add_model(model);

		let op = Operation::RenameColumn {
			table: "users",
			old_name: "name",
			new_name: "full_name",
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
			name: "users",
			columns: vec![],
			constraints: vec![],
		};

		let state = ProjectState::default();
		let reverse = op.to_reverse_sql(&SqlDialect::Postgres, &state);
		assert!(
			reverse.is_ok() && reverse.as_ref().ok().unwrap().is_some(),
			"CreateTable should have reverse SQL operation"
		);
		let sql = reverse.unwrap().unwrap();
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
		let op = Operation::DropTable { name: "users" };

		let state = ProjectState::default();
		let reverse = op.to_reverse_sql(&SqlDialect::Postgres, &state);
		assert!(
			reverse.is_ok() && reverse.as_ref().ok().unwrap().is_none(),
			"DropTable should not have reverse SQL (cannot recreate table structure)"
		);
	}

	#[test]
	fn test_to_reverse_sql_add_column() {
		let op = Operation::AddColumn {
			table: "users",
			column: ColumnDefinition {
				name: "email",
				type_definition: crate::FieldType::VarChar(255),
				not_null: false,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			},
		};

		let state = ProjectState::default();
		let reverse = op.to_reverse_sql(&SqlDialect::Postgres, &state);
		assert!(
			reverse.is_ok() && reverse.as_ref().ok().unwrap().is_some(),
			"AddColumn should have reverse SQL operation"
		);
		let sql = reverse.unwrap().unwrap();
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
			sql: "CREATE INDEX idx_name ON users(name)",
			reverse_sql: Some("DROP INDEX idx_name"),
		};

		let state = ProjectState::default();
		let reverse = op.to_reverse_sql(&SqlDialect::Postgres, &state);
		assert!(
			reverse.is_ok() && reverse.as_ref().ok().unwrap().is_some(),
			"RunSQL with reverse_sql should have reverse SQL"
		);
		let sql = reverse.unwrap().unwrap();
		assert!(
			sql.contains("DROP INDEX"),
			"Reverse SQL should contain provided reverse_sql, got: {}",
			sql
		);
	}

	#[test]
	fn test_to_reverse_sql_run_sql_without_reverse() {
		let op = Operation::RunSQL {
			sql: "CREATE INDEX idx_name ON users(name)",
			reverse_sql: None,
		};

		let state = ProjectState::default();
		let reverse = op.to_reverse_sql(&SqlDialect::Postgres, &state);
		assert!(
			reverse.is_ok() && reverse.as_ref().ok().unwrap().is_none(),
			"RunSQL without reverse_sql should not have reverse SQL"
		);
	}

	#[test]
	fn test_column_definition_new() {
		let col = ColumnDefinition::new("id", crate::FieldType::Integer);
		assert_eq!(col.name, "id", "Column name should be 'id'");
		assert_eq!(
			col.type_definition,
			crate::FieldType::Integer,
			"Column type should be Integer"
		);
		assert!(!col.not_null, "not_null should default to false");
		assert!(!col.unique, "unique should default to false");
		assert!(!col.primary_key, "primary_key should default to false");
		assert!(
			!col.auto_increment,
			"auto_increment should default to false"
		);
		assert!(col.default.is_none(), "default should be None");
	}

	#[test]
	fn test_convert_default_value_null() {
		let op = Operation::CreateTable {
			name: "test",
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
			name: "test",
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
			name: "test",
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
			name: "test",
			columns: vec![],
			constraints: vec![],
		};
		let value = op.convert_default_value("3.15");
		assert!(
			matches!(value, sea_query::Value::Double(_)),
			"Float '3.15' should be converted to sea_query::Value::Double"
		);
	}

	#[test]
	fn test_convert_default_value_string() {
		let op = Operation::CreateTable {
			name: "test",
			columns: vec![],
			constraints: vec![],
		};
		let value = op.convert_default_value("'hello'");
		match value {
			sea_query::Value::String(Some(s)) => assert_eq!(
				s, "hello",
				"Quoted string should be unquoted and stored as 'hello'"
			),
			_ => {
				panic!("Expected sea_query::Value::String(Some(\"hello\")), got different variant")
			}
		}
	}

	#[test]
	fn test_apply_column_type_integer() {
		let op = Operation::CreateTable {
			name: "test",
			columns: vec![],
			constraints: vec![],
		};
		let mut col = ColumnDef::new(Alias::new("id"));
		op.apply_column_type(&mut col, &crate::FieldType::Integer);
		// This test verifies that INTEGER type application doesn't panic
		// Internal state cannot be easily asserted with sea_query's ColumnDef API
	}

	#[test]
	fn test_apply_column_type_varchar_with_length() {
		let op = Operation::CreateTable {
			name: "test",
			columns: vec![],
			constraints: vec![],
		};
		let mut col = ColumnDef::new(Alias::new("name"));
		op.apply_column_type(&mut col, &crate::FieldType::VarChar(100));
		// This test verifies that VARCHAR(100) type application doesn't panic
		// Internal state cannot be easily asserted with sea_query's ColumnDef API
	}

	#[test]
	fn test_apply_column_type_custom() {
		let op = Operation::CreateTable {
			name: "test",
			columns: vec![],
			constraints: vec![],
		};
		let mut col = ColumnDef::new(Alias::new("data"));
		op.apply_column_type(
			&mut col,
			&crate::FieldType::Custom("CUSTOM_TYPE".to_string()),
		);
		// This test verifies that custom type application doesn't panic
		// Internal state cannot be easily asserted with sea_query's ColumnDef API
	}

	#[test]
	fn test_create_index_composite() {
		let op = Operation::CreateIndex {
			table: "users",
			columns: vec!["first_name", "last_name"],
			unique: false,
			index_type: None,
			where_clause: None,
			concurrently: false,
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
			table: "users",
			comment: Some("User's account table"),
		};

		let stmt = op.to_statement();
		let sql = stmt.to_sql_string(reinhardt_backends::types::DatabaseType::Postgres);
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
			crate::FieldType::Integer,
			false,
		));
		state.add_model(model);

		let op = Operation::AlterColumn {
			table: "users",
			column: "age",
			new_definition: ColumnDefinition {
				name: "age",
				type_definition: crate::FieldType::BigInteger,
				not_null: true,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			},
		};

		op.state_forwards("myapp", &mut state);
		let model = state.get_model("myapp", "users").unwrap();
		let field = model.fields.get("age").unwrap();
		assert_eq!(
			field.field_type,
			crate::FieldType::BigInteger,
			"Field type should be updated to BigInteger, got: {}",
			field.field_type
		);
	}

	#[test]
	fn test_state_forwards_create_inherited_table() {
		let mut state = ProjectState::new();
		let op = Operation::CreateInheritedTable {
			name: "admin_users",
			columns: vec![ColumnDefinition {
				name: "admin_level",
				type_definition: crate::FieldType::Integer,
				not_null: true,
				unique: false,
				primary_key: false,
				auto_increment: false,
				default: None,
			}],
			base_table: "users",
			join_column: "user_id",
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
			crate::FieldType::Integer,
			false,
		));
		state.add_model(model);

		let op = Operation::AddDiscriminatorColumn {
			table: "users",
			column_name: "user_type",
			default_value: "regular",
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
