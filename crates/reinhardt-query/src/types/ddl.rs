//! DDL (Data Definition Language) type definitions
//!
//! This module provides types for DDL operations:
//!
//! - [`ColumnType`]: SQL column types (INTEGER, VARCHAR, etc.)
//! - [`ColumnDef`]: Column definition for CREATE TABLE
//! - [`TableConstraint`]: Table constraints (PRIMARY KEY, FOREIGN KEY, etc.)
//! - [`IndexDef`]: Index definition
//! - [`ForeignKeyAction`]: Actions for foreign key constraints

use crate::{
	expr::SimpleExpr,
	types::{DynIden, IntoIden, TableRef},
};

/// SQL column types
///
/// This enum represents the various column types supported across
/// PostgreSQL, MySQL, and SQLite.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColumnType {
	/// CHAR(n) - Fixed-length character string
	Char(Option<u32>),
	/// VARCHAR(n) - Variable-length character string
	String(Option<u32>),
	/// TEXT - Variable-length text
	Text,
	/// TINYINT - Very small integer (1 byte)
	TinyInteger,
	/// SMALLINT - Small integer (2 bytes)
	SmallInteger,
	/// INTEGER - Standard integer (4 bytes)
	Integer,
	/// BIGINT - Large integer (8 bytes)
	BigInteger,
	/// FLOAT - Single precision floating point
	Float,
	/// DOUBLE - Double precision floating point
	Double,
	/// DECIMAL(p, s) - Exact numeric with precision and scale
	Decimal(Option<(u32, u32)>),
	/// BOOLEAN - Boolean value
	Boolean,
	/// DATE - Date (year, month, day)
	Date,
	/// TIME - Time of day
	Time,
	/// DATETIME - Date and time (MySQL)
	DateTime,
	/// TIMESTAMP - Timestamp with timezone
	Timestamp,
	/// TIMESTAMPTZ - Timestamp with timezone (PostgreSQL)
	TimestampWithTimeZone,
	/// BINARY(n) - Fixed-length binary data
	Binary(Option<u32>),
	/// VARBINARY(n) - Variable-length binary data
	VarBinary(u32),
	/// BLOB - Binary large object
	Blob,
	/// UUID - Universally unique identifier
	Uuid,
	/// JSON - JSON data
	Json,
	/// JSONB - Binary JSON (PostgreSQL)
	JsonBinary,
	/// ARRAY - Array type (PostgreSQL)
	Array(Box<ColumnType>),
	/// Custom type - for database-specific types
	///
	/// # Security Note
	///
	/// Only use with trusted type names. Do not use with user input.
	Custom(String),
}

/// Column definition for CREATE TABLE
///
/// This struct represents a column definition, including its type,
/// constraints, and default value.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_query::types::ddl::{ColumnDef, ColumnType};
///
/// // id INTEGER PRIMARY KEY AUTO_INCREMENT
/// let id_col = ColumnDef::new("id")
///     .column_type(ColumnType::Integer)
///     .primary_key(true)
///     .auto_increment(true);
///
/// // name VARCHAR(100) NOT NULL
/// let name_col = ColumnDef::new("name")
///     .column_type(ColumnType::String(Some(100)))
///     .not_null(true);
/// ```
#[derive(Debug, Clone)]
pub struct ColumnDef {
	pub(crate) name: DynIden,
	pub(crate) column_type: Option<ColumnType>,
	pub(crate) not_null: bool,
	pub(crate) unique: bool,
	pub(crate) primary_key: bool,
	pub(crate) auto_increment: bool,
	pub(crate) default: Option<SimpleExpr>,
	pub(crate) check: Option<SimpleExpr>,
	pub(crate) comment: Option<String>,
}

impl ColumnDef {
	/// Create a new column definition
	pub fn new<T>(name: T) -> Self
	where
		T: IntoIden,
	{
		Self {
			name: name.into_iden(),
			column_type: None,
			not_null: false,
			unique: false,
			primary_key: false,
			auto_increment: false,
			default: None,
			check: None,
			comment: None,
		}
	}

	/// Set the column type
	pub fn column_type(mut self, column_type: ColumnType) -> Self {
		self.column_type = Some(column_type);
		self
	}

	/// Set NOT NULL constraint
	pub fn not_null(mut self, not_null: bool) -> Self {
		self.not_null = not_null;
		self
	}

	/// Set UNIQUE constraint
	pub fn unique(mut self, unique: bool) -> Self {
		self.unique = unique;
		self
	}

	/// Set PRIMARY KEY constraint
	pub fn primary_key(mut self, primary_key: bool) -> Self {
		self.primary_key = primary_key;
		self
	}

	/// Set AUTO_INCREMENT attribute
	pub fn auto_increment(mut self, auto_increment: bool) -> Self {
		self.auto_increment = auto_increment;
		self
	}

	/// Set DEFAULT value
	pub fn default(mut self, value: SimpleExpr) -> Self {
		self.default = Some(value);
		self
	}

	/// Set CHECK constraint
	pub fn check(mut self, expr: SimpleExpr) -> Self {
		self.check = Some(expr);
		self
	}

	/// Set column comment
	pub fn comment<S: Into<String>>(mut self, comment: S) -> Self {
		self.comment = Some(comment.into());
		self
	}
}

/// Table constraint
///
/// This enum represents various table-level constraints.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum TableConstraint {
	/// PRIMARY KEY constraint
	PrimaryKey {
		/// Constraint name
		name: Option<DynIden>,
		/// Columns in the primary key
		columns: Vec<DynIden>,
	},
	/// UNIQUE constraint
	Unique {
		/// Constraint name
		name: Option<DynIden>,
		/// Columns that must be unique
		columns: Vec<DynIden>,
	},
	/// FOREIGN KEY constraint
	ForeignKey {
		/// Constraint name
		name: Option<DynIden>,
		/// Columns in this table
		columns: Vec<DynIden>,
		/// Referenced table
		ref_table: TableRef,
		/// Referenced columns
		ref_columns: Vec<DynIden>,
		/// ON DELETE action
		on_delete: Option<ForeignKeyAction>,
		/// ON UPDATE action
		on_update: Option<ForeignKeyAction>,
	},
	/// CHECK constraint
	Check {
		/// Constraint name
		name: Option<DynIden>,
		/// Check expression
		expr: SimpleExpr,
	},
}

/// Foreign key action
///
/// This enum represents actions for foreign key constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ForeignKeyAction {
	/// RESTRICT - Reject the delete/update
	Restrict,
	/// CASCADE - Delete/update the referencing rows
	Cascade,
	/// SET NULL - Set the foreign key column(s) to NULL
	SetNull,
	/// SET DEFAULT - Set the foreign key column(s) to their default values
	SetDefault,
	/// NO ACTION - Similar to RESTRICT (default)
	NoAction,
}

impl ForeignKeyAction {
	/// Get the SQL keyword for this action
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Restrict => "RESTRICT",
			Self::Cascade => "CASCADE",
			Self::SetNull => "SET NULL",
			Self::SetDefault => "SET DEFAULT",
			Self::NoAction => "NO ACTION",
		}
	}
}

/// Index definition
///
/// This struct represents an index definition for CREATE INDEX.
///
/// Note: The `name` and `table` fields are defined for future use in CREATE TABLE statements
/// with inline index definitions, but are not yet used in the current backend implementations.
/// The dead_code warning is allowed because this is part of the planned API.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct IndexDef {
	pub(crate) name: DynIden,
	pub(crate) table: TableRef,
	pub(crate) columns: Vec<DynIden>,
	pub(crate) unique: bool,
	pub(crate) r#where: Option<SimpleExpr>,
}

impl IndexDef {
	/// Create a new index definition
	pub fn new<T, R>(name: T, table: R) -> Self
	where
		T: IntoIden,
		R: Into<TableRef>,
	{
		Self {
			name: name.into_iden(),
			table: table.into(),
			columns: Vec::new(),
			unique: false,
			r#where: None,
		}
	}

	/// Add a column to the index
	pub fn column<C>(mut self, col: C) -> Self
	where
		C: IntoIden,
	{
		self.columns.push(col.into_iden());
		self
	}

	/// Add multiple columns to the index
	pub fn columns<I, C>(mut self, cols: I) -> Self
	where
		I: IntoIterator<Item = C>,
		C: IntoIden,
	{
		for col in cols {
			self.columns.push(col.into_iden());
		}
		self
	}

	/// Set UNIQUE attribute
	pub fn unique(mut self, unique: bool) -> Self {
		self.unique = unique;
		self
	}

	/// Set WHERE clause for partial index
	pub fn r#where(mut self, expr: SimpleExpr) -> Self {
		self.r#where = Some(expr);
		self
	}
}
