//! REVOKE statement builder

use super::{Grantee, ObjectType, Privilege};
use crate::types::{DynIden, IntoIden};

/// REVOKE statement builder
///
/// This struct provides a fluent API for building REVOKE statements.
///
/// # Examples
///
/// ```
/// use reinhardt_query::dcl::{RevokeStatement, Privilege, Grantee};
///
/// let stmt = RevokeStatement::new()
///     .privilege(Privilege::Insert)
///     .from_table("users")
///     .from("app_user")
///     .cascade(true);
/// ```
#[derive(Debug, Clone)]
pub struct RevokeStatement {
	/// List of privileges to revoke
	pub privileges: Vec<Privilege>,
	/// Type of object (TABLE, DATABASE, etc.)
	pub object_type: ObjectType,
	/// List of object names
	pub objects: Vec<DynIden>,
	/// List of grantees (users/roles)
	pub grantees: Vec<Grantee>,
	/// CASCADE flag (PostgreSQL-specific)
	pub cascade: bool,
	/// GRANT OPTION FOR flag (PostgreSQL-specific)
	pub grant_option_for: bool,
}

impl RevokeStatement {
	/// Create a new empty REVOKE statement
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::RevokeStatement;
	///
	/// let stmt = RevokeStatement::new();
	/// ```
	pub fn new() -> Self {
		Self {
			privileges: Vec::new(),
			object_type: ObjectType::Table, // Default to TABLE
			objects: Vec::new(),
			grantees: Vec::new(),
			cascade: false,
			grant_option_for: false,
		}
	}

	/// Add a single privilege to revoke
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{RevokeStatement, Privilege};
	///
	/// let stmt = RevokeStatement::new()
	///     .privilege(Privilege::Insert)
	///     .privilege(Privilege::Update);
	/// ```
	pub fn privilege(mut self, privilege: Privilege) -> Self {
		self.privileges.push(privilege);
		self
	}

	/// Set all privileges at once
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{RevokeStatement, Privilege};
	///
	/// let stmt = RevokeStatement::new()
	///     .privileges(vec![Privilege::Insert, Privilege::Update, Privilege::Delete]);
	/// ```
	pub fn privileges(mut self, privileges: Vec<Privilege>) -> Self {
		self.privileges = privileges;
		self
	}

	/// Set the object type
	pub fn object_type(mut self, object_type: ObjectType) -> Self {
		self.object_type = object_type;
		self
	}

	/// Add a single object
	pub fn object<T: IntoIden>(mut self, object: T) -> Self {
		self.objects.push(object.into_iden());
		self
	}

	/// Set all objects at once
	pub fn objects(mut self, objects: Vec<DynIden>) -> Self {
		self.objects = objects;
		self
	}

	/// Convenience method: Revoke from TABLE objects
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{RevokeStatement, Privilege};
	///
	/// let stmt = RevokeStatement::new()
	///     .privilege(Privilege::Insert)
	///     .from_table("users");
	/// ```
	pub fn from_table<T: IntoIden>(mut self, table: T) -> Self {
		self.object_type = ObjectType::Table;
		self.objects.push(table.into_iden());
		self
	}

	/// Convenience method: Revoke from DATABASE objects
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{RevokeStatement, Privilege};
	///
	/// let stmt = RevokeStatement::new()
	///     .privilege(Privilege::Create)
	///     .from_database("mydb");
	/// ```
	pub fn from_database<T: IntoIden>(mut self, database: T) -> Self {
		self.object_type = ObjectType::Database;
		self.objects.push(database.into_iden());
		self
	}

	/// Convenience method: Revoke from SCHEMA objects (PostgreSQL)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{RevokeStatement, Privilege};
	///
	/// let stmt = RevokeStatement::new()
	///     .privilege(Privilege::Usage)
	///     .from_schema("public");
	/// ```
	pub fn from_schema<T: IntoIden>(mut self, schema: T) -> Self {
		self.object_type = ObjectType::Schema;
		self.objects.push(schema.into_iden());
		self
	}

	/// Convenience method: Revoke from SEQUENCE objects (PostgreSQL)
	pub fn from_sequence<T: IntoIden>(mut self, sequence: T) -> Self {
		self.object_type = ObjectType::Sequence;
		self.objects.push(sequence.into_iden());
		self
	}

	/// Add a single grantee
	pub fn grantee(mut self, grantee: Grantee) -> Self {
		self.grantees.push(grantee);
		self
	}

	/// Set all grantees at once
	pub fn grantees(mut self, grantees: Vec<Grantee>) -> Self {
		self.grantees = grantees;
		self
	}

	/// Convenience method: Revoke from a role
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{RevokeStatement, Privilege};
	///
	/// let stmt = RevokeStatement::new()
	///     .privilege(Privilege::Insert)
	///     .from_table("users")
	///     .from("app_user");
	/// ```
	pub fn from<S: Into<String>>(mut self, role: S) -> Self {
		self.grantees.push(Grantee::role(role));
		self
	}

	/// Set CASCADE flag (PostgreSQL-specific)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{RevokeStatement, Privilege};
	///
	/// let stmt = RevokeStatement::new()
	///     .privilege(Privilege::All)
	///     .from_table("users")
	///     .from("app_user")
	///     .cascade(true);
	/// ```
	pub fn cascade(mut self, flag: bool) -> Self {
		self.cascade = flag;
		self
	}

	/// Set GRANT OPTION FOR flag (PostgreSQL-specific)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{RevokeStatement, Privilege};
	///
	/// let stmt = RevokeStatement::new()
	///     .privilege(Privilege::Select)
	///     .from_table("users")
	///     .from("app_user")
	///     .grant_option_for(true);
	/// ```
	pub fn grant_option_for(mut self, flag: bool) -> Self {
		self.grant_option_for = flag;
		self
	}

	/// Validate the REVOKE statement
	///
	/// # Validation Rules
	///
	/// 1. At least one privilege must be specified
	/// 2. At least one object must be specified
	/// 3. At least one grantee must be specified
	/// 4. Privilege must be valid for the object type
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{RevokeStatement, Privilege};
	///
	/// let stmt = RevokeStatement::new()
	///     .privilege(Privilege::Insert)
	///     .from_table("users")
	///     .from("app_user");
	///
	/// assert!(stmt.validate().is_ok());
	/// ```
	pub fn validate(&self) -> Result<(), String> {
		// Check at least one privilege
		if self.privileges.is_empty() {
			return Err("At least one privilege must be specified".to_string());
		}

		// Check at least one object
		if self.objects.is_empty() {
			return Err("At least one object must be specified".to_string());
		}

		// Check at least one grantee
		if self.grantees.is_empty() {
			return Err("At least one grantee must be specified".to_string());
		}

		// Check privilege-object combinations
		for privilege in &self.privileges {
			if !privilege.is_valid_for_object(self.object_type) {
				return Err(format!(
					"Privilege {:?} is not valid for object type {:?}",
					privilege, self.object_type
				));
			}
		}

		Ok(())
	}
}

impl Default for RevokeStatement {
	fn default() -> Self {
		Self::new()
	}
}
