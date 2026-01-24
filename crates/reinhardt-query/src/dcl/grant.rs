//! GRANT statement builder

use super::{Grantee, ObjectType, Privilege};
use crate::types::{DynIden, IntoIden};

/// GRANT statement builder
///
/// This struct provides a fluent API for building GRANT statements.
///
/// # Examples
///
/// ```
/// use reinhardt_query::dcl::{GrantStatement, Privilege, Grantee};
///
/// let stmt = GrantStatement::new()
///     .privilege(Privilege::Select)
///     .privilege(Privilege::Insert)
///     .on_table("users")
///     .to("app_user")
///     .with_grant_option(true);
/// ```
#[derive(Debug, Clone)]
pub struct GrantStatement {
	/// List of privileges to grant
	pub privileges: Vec<Privilege>,
	/// Type of object (TABLE, DATABASE, etc.)
	pub object_type: ObjectType,
	/// List of object names
	pub objects: Vec<DynIden>,
	/// List of grantees (users/roles)
	pub grantees: Vec<Grantee>,
	/// WITH GRANT OPTION flag
	pub with_grant_option: bool,
	/// GRANTED BY clause (PostgreSQL-specific)
	pub granted_by: Option<Grantee>,
}

impl GrantStatement {
	/// Create a new empty GRANT statement
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::GrantStatement;
	///
	/// let stmt = GrantStatement::new();
	/// ```
	pub fn new() -> Self {
		Self {
			privileges: Vec::new(),
			object_type: ObjectType::Table, // Default to TABLE
			objects: Vec::new(),
			grantees: Vec::new(),
			with_grant_option: false,
			granted_by: None,
		}
	}

	/// Add a single privilege to the statement
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{GrantStatement, Privilege};
	///
	/// let stmt = GrantStatement::new()
	///     .privilege(Privilege::Select)
	///     .privilege(Privilege::Insert);
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
	/// use reinhardt_query::dcl::{GrantStatement, Privilege};
	///
	/// let stmt = GrantStatement::new()
	///     .privileges(vec![Privilege::Select, Privilege::Insert, Privilege::Update]);
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

	/// Convenience method: Grant on TABLE objects
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{GrantStatement, Privilege};
	///
	/// let stmt = GrantStatement::new()
	///     .privilege(Privilege::Select)
	///     .on_table("users");
	/// ```
	pub fn on_table<T: IntoIden>(mut self, table: T) -> Self {
		self.object_type = ObjectType::Table;
		self.objects.push(table.into_iden());
		self
	}

	/// Convenience method: Grant on DATABASE objects
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{GrantStatement, Privilege};
	///
	/// let stmt = GrantStatement::new()
	///     .privilege(Privilege::Create)
	///     .on_database("mydb");
	/// ```
	pub fn on_database<T: IntoIden>(mut self, database: T) -> Self {
		self.object_type = ObjectType::Database;
		self.objects.push(database.into_iden());
		self
	}

	/// Convenience method: Grant on SCHEMA objects (PostgreSQL)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{GrantStatement, Privilege};
	///
	/// let stmt = GrantStatement::new()
	///     .privilege(Privilege::Usage)
	///     .on_schema("public");
	/// ```
	pub fn on_schema<T: IntoIden>(mut self, schema: T) -> Self {
		self.object_type = ObjectType::Schema;
		self.objects.push(schema.into_iden());
		self
	}

	/// Convenience method: Grant on SEQUENCE objects (PostgreSQL)
	pub fn on_sequence<T: IntoIden>(mut self, sequence: T) -> Self {
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

	/// Convenience method: Add a role grantee
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{GrantStatement, Privilege};
	///
	/// let stmt = GrantStatement::new()
	///     .privilege(Privilege::Select)
	///     .on_table("users")
	///     .to("app_user");
	/// ```
	pub fn to<S: Into<String>>(mut self, role: S) -> Self {
		self.grantees.push(Grantee::role(role));
		self
	}

	/// Set WITH GRANT OPTION flag
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{GrantStatement, Privilege};
	///
	/// let stmt = GrantStatement::new()
	///     .privilege(Privilege::Select)
	///     .on_table("users")
	///     .to("app_user")
	///     .with_grant_option(true);
	/// ```
	pub fn with_grant_option(mut self, flag: bool) -> Self {
		self.with_grant_option = flag;
		self
	}

	/// Set GRANTED BY clause (PostgreSQL-specific)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_query::dcl::{GrantStatement, Privilege, Grantee};
	///
	/// let stmt = GrantStatement::new()
	///     .privilege(Privilege::Select)
	///     .on_table("users")
	///     .to("app_user")
	///     .granted_by(Grantee::role("admin"));
	/// ```
	pub fn granted_by(mut self, grantee: Grantee) -> Self {
		self.granted_by = Some(grantee);
		self
	}

	/// Validate the GRANT statement
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
	/// use reinhardt_query::dcl::{GrantStatement, Privilege};
	///
	/// let stmt = GrantStatement::new()
	///     .privilege(Privilege::Select)
	///     .on_table("users")
	///     .to("app_user");
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

impl Default for GrantStatement {
	fn default() -> Self {
		Self::new()
	}
}
