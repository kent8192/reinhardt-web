//! Database object types for DCL statements

/// Database object types for DCL statements
///
/// This enum represents the various types of database objects that can
/// be targets of GRANT and REVOKE statements.
///
/// # Examples
///
/// ```
/// use reinhardt_query::dcl::ObjectType;
///
/// let object_type = ObjectType::Table;
/// assert_eq!(object_type.as_sql(), "TABLE");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectType {
	/// Database tables
	Table,
	/// Entire databases
	Database,
	/// Database schemas (PostgreSQL-specific)
	Schema,
	/// Sequence objects (PostgreSQL-specific)
	Sequence,
}

impl ObjectType {
	/// Returns the SQL keyword for this object type
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::ObjectType;
	///
	/// assert_eq!(ObjectType::Table.as_sql(), "TABLE");
	/// assert_eq!(ObjectType::Database.as_sql(), "DATABASE");
	/// ```
	pub fn as_sql(&self) -> &'static str {
		match self {
			ObjectType::Table => "TABLE",
			ObjectType::Database => "DATABASE",
			ObjectType::Schema => "SCHEMA",
			ObjectType::Sequence => "SEQUENCE",
		}
	}

	/// Checks if this object type is PostgreSQL-specific
	///
	/// Returns `true` for object types that are only available in PostgreSQL,
	/// `false` for object types that are common to both PostgreSQL and MySQL.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::ObjectType;
	///
	/// assert!(!ObjectType::Table.is_postgres_only());   // Common
	/// assert!(ObjectType::Schema.is_postgres_only());    // PostgreSQL-specific
	/// ```
	pub fn is_postgres_only(&self) -> bool {
		matches!(self, ObjectType::Schema | ObjectType::Sequence)
	}
}
