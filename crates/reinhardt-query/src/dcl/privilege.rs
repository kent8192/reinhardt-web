//! SQL privilege types for GRANT and REVOKE statements

use super::ObjectType;

/// SQL privilege types for GRANT and REVOKE statements
///
/// This enum represents the various privilege types that can be granted
/// or revoked in SQL databases. Some privileges are database-specific.
///
/// # Examples
///
/// ```
/// use reinhardt_query::dcl::Privilege;
///
/// let privilege = Privilege::Select;
/// assert_eq!(privilege.as_sql(), "SELECT");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Privilege {
	/// SELECT privilege - Read data from tables/views
	Select,
	/// INSERT privilege - Insert rows into tables
	Insert,
	/// UPDATE privilege - Modify rows in tables
	Update,
	/// DELETE privilege - Remove rows from tables
	Delete,
	/// REFERENCES privilege - Create foreign keys
	References,
	/// CREATE privilege - Create objects (tables, databases, etc.)
	Create,
	/// ALL PRIVILEGES - All available privileges
	All,
	/// TRUNCATE privilege - Truncate tables (PostgreSQL-specific)
	Truncate,
	/// TRIGGER privilege - Create triggers (PostgreSQL-specific)
	Trigger,
	/// MAINTAIN privilege - Maintenance operations (PostgreSQL-specific)
	Maintain,
	/// USAGE privilege - Use schemas/sequences (PostgreSQL-specific)
	Usage,
	/// CONNECT privilege - Connect to database (PostgreSQL-specific)
	Connect,
	/// TEMPORARY privilege - Create temporary tables (PostgreSQL-specific)
	Temporary,
	/// EXECUTE privilege - Execute functions/procedures (PostgreSQL-specific)
	Execute,
}

impl Privilege {
	/// Returns the SQL keyword for this privilege
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::Privilege;
	///
	/// assert_eq!(Privilege::Select.as_sql(), "SELECT");
	/// assert_eq!(Privilege::All.as_sql(), "ALL PRIVILEGES");
	/// ```
	pub fn as_sql(&self) -> &'static str {
		match self {
			Privilege::Select => "SELECT",
			Privilege::Insert => "INSERT",
			Privilege::Update => "UPDATE",
			Privilege::Delete => "DELETE",
			Privilege::References => "REFERENCES",
			Privilege::Create => "CREATE",
			Privilege::All => "ALL PRIVILEGES",
			Privilege::Truncate => "TRUNCATE",
			Privilege::Trigger => "TRIGGER",
			Privilege::Maintain => "MAINTAIN",
			Privilege::Usage => "USAGE",
			Privilege::Connect => "CONNECT",
			Privilege::Temporary => "TEMPORARY",
			Privilege::Execute => "EXECUTE",
		}
	}

	/// Checks if this privilege is PostgreSQL-specific
	///
	/// Returns `true` for privileges that are only available in PostgreSQL,
	/// `false` for privileges that are common to both PostgreSQL and MySQL.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::Privilege;
	///
	/// assert!(!Privilege::Select.is_postgres_only());  // Common privilege
	/// assert!(Privilege::Truncate.is_postgres_only()); // PostgreSQL-specific
	/// ```
	pub fn is_postgres_only(&self) -> bool {
		matches!(
			self,
			Privilege::Truncate
				| Privilege::Trigger
				| Privilege::Maintain
				| Privilege::Usage
				| Privilege::Connect
				| Privilege::Temporary
				| Privilege::Execute
		)
	}

	/// Checks if this privilege is valid for the given object type
	///
	/// Different object types support different privileges. For example,
	/// `SELECT` is valid for tables but not for databases.
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{Privilege, ObjectType};
	///
	/// assert!(Privilege::Select.is_valid_for_object(ObjectType::Table));
	/// assert!(!Privilege::Connect.is_valid_for_object(ObjectType::Table));
	/// ```
	pub fn is_valid_for_object(&self, object_type: ObjectType) -> bool {
		match (self, object_type) {
			// TABLE privileges
			(Privilege::Select, ObjectType::Table) => true,
			(Privilege::Insert, ObjectType::Table) => true,
			(Privilege::Update, ObjectType::Table) => true,
			(Privilege::Delete, ObjectType::Table) => true,
			(Privilege::References, ObjectType::Table) => true,
			(Privilege::Truncate, ObjectType::Table) => true,
			(Privilege::Trigger, ObjectType::Table) => true,
			(Privilege::Maintain, ObjectType::Table) => true,
			(Privilege::All, ObjectType::Table) => true,

			// DATABASE privileges
			(Privilege::Create, ObjectType::Database) => true,
			(Privilege::Connect, ObjectType::Database) => true,
			(Privilege::Temporary, ObjectType::Database) => true,
			(Privilege::All, ObjectType::Database) => true,

			// SCHEMA privileges
			(Privilege::Create, ObjectType::Schema) => true,
			(Privilege::Usage, ObjectType::Schema) => true,
			(Privilege::All, ObjectType::Schema) => true,

			// SEQUENCE privileges
			(Privilege::Usage, ObjectType::Sequence) => true,
			(Privilege::Select, ObjectType::Sequence) => true,
			(Privilege::Update, ObjectType::Sequence) => true,
			(Privilege::All, ObjectType::Sequence) => true,

			// Invalid combinations
			_ => false,
		}
	}
}
