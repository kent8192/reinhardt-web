//! DCL (Data Control Language) statement builders
//!
//! This module provides builders for database user and role management statements:
//! - CREATE USER
//! - ALTER USER
//! - CREATE ROLE
//! - SET ROLE
//! - etc.
//!
//! All builders include consistent validation for names and parameters.

use std::sync::Arc;

use super::backend::DatabaseBackend;

/// Validate and trim a name field (user, role, etc.)
///
/// # Arguments
///
/// * `name` - The name to validate
/// * `field_name` - Description of the field for error messages (e.g., "User name", "Role name")
///
/// # Returns
///
/// * `Ok(String)` - The trimmed, validated name
/// * `Err(String)` - An error message if validation fails
///
/// # Example
///
/// ```rust,ignore
/// let name = validate_name("  admin  ", "User name")?;
/// assert_eq!(name, "admin");
///
/// let result = validate_name("", "User name");
/// assert!(result.is_err());
/// ```
pub(crate) fn validate_name(name: &str, field_name: &str) -> std::result::Result<String, String> {
	let trimmed = name.trim();
	if trimmed.is_empty() {
		return Err(format!("{} cannot be empty or whitespace", field_name));
	}
	Ok(trimmed.to_string())
}

/// CREATE USER statement builder
///
/// Builds a CREATE USER statement with consistent validation.
///
/// # Example
///
/// ```rust,ignore
/// let stmt = CreateUserStatement::new(backend)
///     .user("admin")?
///     .build();
/// ```
pub struct CreateUserStatement {
	backend: Arc<dyn DatabaseBackend>,
	user_name: Option<String>,
}

impl std::fmt::Debug for CreateUserStatement {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CreateUserStatement")
			.field("user_name", &self.user_name)
			.finish()
	}
}

impl CreateUserStatement {
	pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
		Self {
			backend,
			user_name: None,
		}
	}

	/// Set the user name (validates and trims whitespace)
	pub fn user(mut self, name: &str) -> std::result::Result<Self, String> {
		let validated = validate_name(name, "User name")?;
		self.user_name = Some(validated);
		Ok(self)
	}

	/// Build the SQL statement
	///
	/// # Returns
	///
	/// The SQL string for CREATE USER
	///
	/// # Panics
	///
	/// Panics if user name was not set
	pub fn build(&self) -> String {
		let user = self
			.user_name
			.as_ref()
			.expect("User name must be set before building");

		use super::types::DatabaseType;

		match self.backend.database_type() {
			DatabaseType::Postgres | DatabaseType::Sqlite => {
				format!("CREATE USER \"{}\"", user)
			}
			DatabaseType::Mysql => {
				format!("CREATE USER '{}'@'%'", user)
			}
		}
	}
}

/// SET ROLE statement builder
///
/// Builds a SET ROLE statement with consistent validation.
///
/// # Example
///
/// ```rust,ignore
/// let stmt = SetRoleStatement::new(backend)
///     .role("admin")?
///     .build();
/// ```
pub struct SetRoleStatement {
	backend: Arc<dyn DatabaseBackend>,
	role_name: Option<String>,
}

impl std::fmt::Debug for SetRoleStatement {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SetRoleStatement")
			.field("role_name", &self.role_name)
			.finish()
	}
}

impl SetRoleStatement {
	pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
		Self {
			backend,
			role_name: None,
		}
	}

	/// Set the role name (validates and trims whitespace)
	pub fn role(mut self, name: &str) -> std::result::Result<Self, String> {
		let validated = validate_name(name, "Role name")?;
		self.role_name = Some(validated);
		Ok(self)
	}

	/// Build the SQL statement
	///
	/// # Returns
	///
	/// The SQL string for SET ROLE
	///
	/// # Panics
	///
	/// Panics if role name was not set
	pub fn build(&self) -> String {
		let role = self
			.role_name
			.as_ref()
			.expect("Role name must be set before building");

		use super::types::DatabaseType;

		match self.backend.database_type() {
			DatabaseType::Postgres => {
				format!("SET ROLE \"{}\"", role)
			}
			DatabaseType::Mysql => {
				format!("SET ROLE '{}'", role)
			}
			DatabaseType::Sqlite => {
				// SQLite doesn't support SET ROLE
				panic!("SQLite does not support SET ROLE")
			}
		}
	}
}

/// ALTER USER statement builder
///
/// Builds an ALTER USER statement with consistent validation.
///
/// # Example
///
/// ```rust,ignore
/// let stmt = AlterUserStatement::new(backend)
///     .user("admin")?
///     .build();
/// ```
pub struct AlterUserStatement {
	backend: Arc<dyn DatabaseBackend>,
	user_name: Option<String>,
}

impl std::fmt::Debug for AlterUserStatement {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AlterUserStatement")
			.field("user_name", &self.user_name)
			.finish()
	}
}

impl AlterUserStatement {
	pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
		Self {
			backend,
			user_name: None,
		}
	}

	/// Set the user name (validates and trims whitespace)
	pub fn user(mut self, name: &str) -> std::result::Result<Self, String> {
		let validated = validate_name(name, "User name")?;
		self.user_name = Some(validated);
		Ok(self)
	}

	/// Build the SQL statement
	///
	/// # Returns
	///
	/// The SQL string for ALTER USER
	///
	/// # Panics
	///
	/// Panics if user name was not set
	pub fn build(&self) -> String {
		let user = self
			.user_name
			.as_ref()
			.expect("User name must be set before building");

		use super::types::DatabaseType;

		match self.backend.database_type() {
			DatabaseType::Postgres | DatabaseType::Sqlite => {
				format!("ALTER USER \"{}\"", user)
			}
			DatabaseType::Mysql => {
				format!("ALTER USER '{}'@'%'", user)
			}
		}
	}
}

/// CREATE ROLE statement builder
///
/// Builds a CREATE ROLE statement with consistent validation.
///
/// # Example
///
/// ```rust,ignore
/// let stmt = CreateRoleStatement::new(backend)
///     .role("admin")?
///     .build();
/// ```
pub struct CreateRoleStatement {
	backend: Arc<dyn DatabaseBackend>,
	role_name: Option<String>,
}

impl std::fmt::Debug for CreateRoleStatement {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CreateRoleStatement")
			.field("role_name", &self.role_name)
			.finish()
	}
}

impl CreateRoleStatement {
	pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
		Self {
			backend,
			role_name: None,
		}
	}

	/// Set the role name (validates and trims whitespace)
	pub fn role(mut self, name: &str) -> std::result::Result<Self, String> {
		let validated = validate_name(name, "Role name")?;
		self.role_name = Some(validated);
		Ok(self)
	}

	/// Build the SQL statement
	///
	/// # Returns
	///
	/// The SQL string for CREATE ROLE
	///
	/// # Panics
	///
	/// Panics if role name was not set
	pub fn build(&self) -> String {
		let role = self
			.role_name
			.as_ref()
			.expect("Role name must be set before building");

		use super::types::DatabaseType;

		match self.backend.database_type() {
			DatabaseType::Postgres => {
				format!("CREATE ROLE \"{}\"", role)
			}
			DatabaseType::Mysql => {
				format!("CREATE ROLE '{}'", role)
			}
			DatabaseType::Sqlite => {
				// SQLite doesn't support roles
				panic!("SQLite does not support CREATE ROLE")
			}
		}
	}
}

/// ALTER ROLE statement builder
///
/// Builds an ALTER ROLE statement with consistent validation.
///
/// # Example
///
/// ```rust,ignore
/// let stmt = AlterRoleStatement::new(backend)
///     .role("admin")?
///     .build();
/// ```
pub struct AlterRoleStatement {
	backend: Arc<dyn DatabaseBackend>,
	role_name: Option<String>,
}

impl std::fmt::Debug for AlterRoleStatement {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AlterRoleStatement")
			.field("role_name", &self.role_name)
			.finish()
	}
}

impl AlterRoleStatement {
	pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
		Self {
			backend,
			role_name: None,
		}
	}

	/// Set the role name (validates and trims whitespace)
	pub fn role(mut self, name: &str) -> std::result::Result<Self, String> {
		let validated = validate_name(name, "Role name")?;
		self.role_name = Some(validated);
		Ok(self)
	}

	/// Build the SQL statement
	///
	/// # Returns
	///
	/// The SQL string for ALTER ROLE
	///
	/// # Panics
	///
	/// Panics if role name was not set
	pub fn build(&self) -> String {
		let role = self
			.role_name
			.as_ref()
			.expect("Role name must be set before building");

		use super::types::DatabaseType;

		match self.backend.database_type() {
			DatabaseType::Postgres => {
				format!("ALTER ROLE \"{}\"", role)
			}
			DatabaseType::Mysql => {
				format!("ALTER ROLE '{}'", role)
			}
			DatabaseType::Sqlite => {
				// SQLite doesn't support roles
				panic!("SQLite does not support ALTER ROLE")
			}
		}
	}
}

/// SET DEFAULT ROLE statement builder
///
/// Builds a SET DEFAULT ROLE statement with consistent validation.
///
/// # Example
///
/// ```rust,ignore
/// let stmt = SetDefaultRoleStatement::new(backend)
///     .user("admin")?
///     .build();
/// ```
pub struct SetDefaultRoleStatement {
	backend: Arc<dyn DatabaseBackend>,
	user_name: Option<String>,
	role_names: Vec<String>,
}

impl std::fmt::Debug for SetDefaultRoleStatement {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SetDefaultRoleStatement")
			.field("user_name", &self.user_name)
			.field("role_names", &self.role_names)
			.finish()
	}
}

impl SetDefaultRoleStatement {
	pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
		Self {
			backend,
			user_name: None,
			role_names: Vec::new(),
		}
	}

	/// Set the user name (validates and trims whitespace)
	pub fn user(mut self, name: &str) -> std::result::Result<Self, String> {
		let validated = validate_name(name, "User name")?;
		self.user_name = Some(validated);
		Ok(self)
	}

	/// Add multiple roles (validates and trims each)
	pub fn users(mut self, names: &[&str]) -> std::result::Result<Self, String> {
		for name in names {
			let validated = validate_name(name, "Role name")?;
			self.role_names.push(validated);
		}
		Ok(self)
	}

	/// Build the SQL statement
	///
	/// # Returns
	///
	/// The SQL string for SET DEFAULT ROLE
	///
	/// # Panics
	///
	/// Panics if user name was not set
	pub fn build(&self) -> String {
		let user = self
			.user_name
			.as_ref()
			.expect("User name must be set before building");

		use super::types::DatabaseType;

		match self.backend.database_type() {
			DatabaseType::Postgres => {
				if self.role_names.is_empty() {
					format!("ALTER ROLE \"{}\" SET ROLE NONE", user)
				} else {
					let roles = self.role_names.join("\", \"");
					format!("ALTER ROLE \"{}\" SET ROLE \"{}\"", user, roles)
				}
			}
			DatabaseType::Mysql => {
				if self.role_names.is_empty() {
					format!("SET DEFAULT ROLE NONE TO '{}'@'%'", user)
				} else {
					let roles = self
						.role_names
						.iter()
						.map(|r| format!("'{}'", r))
						.collect::<Vec<_>>()
						.join(", ");
					format!("SET DEFAULT ROLE {} TO '{}'@'%'", roles, user)
				}
			}
			DatabaseType::Sqlite => {
				// SQLite doesn't support roles
				panic!("SQLite does not support SET DEFAULT ROLE")
			}
		}
	}
}

/// RENAME USER statement builder
///
/// Builds a RENAME USER statement with consistent validation.
///
/// # Example
///
/// ```rust,ignore
/// let stmt = RenameUserStatement::new(backend)
///     .rename("old_name", "new_name")?
///     .build();
/// ```
pub struct RenameUserStatement {
	backend: Arc<dyn DatabaseBackend>,
	old_name: Option<String>,
	new_name: Option<String>,
}

impl std::fmt::Debug for RenameUserStatement {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RenameUserStatement")
			.field("old_name", &self.old_name)
			.field("new_name", &self.new_name)
			.finish()
	}
}

impl RenameUserStatement {
	pub fn new(backend: Arc<dyn DatabaseBackend>) -> Self {
		Self {
			backend,
			old_name: None,
			new_name: None,
		}
	}

	/// Set the old and new names (validates and trims whitespace)
	pub fn rename(mut self, old: &str, new: &str) -> std::result::Result<Self, String> {
		let old_validated = validate_name(old, "Old user name")?;
		let new_validated = validate_name(new, "New user name")?;
		self.old_name = Some(old_validated);
		self.new_name = Some(new_validated);
		Ok(self)
	}

	/// Build the SQL statement
	///
	/// # Returns
	///
	/// The SQL string for RENAME USER
	///
	/// # Panics
	///
	/// Panics if names were not set
	pub fn build(&self) -> String {
		let old = self
			.old_name
			.as_ref()
			.expect("Old user name must be set before building");
		let new = self
			.new_name
			.as_ref()
			.expect("New user name must be set before building");

		use super::types::DatabaseType;

		match self.backend.database_type() {
			DatabaseType::Postgres => {
				format!("ALTER USER \"{}\" RENAME TO \"{}\"", old, new)
			}
			DatabaseType::Mysql => {
				format!("RENAME USER '{}'@'%' TO '{}'@'%'", old, new)
			}
			DatabaseType::Sqlite => {
				// SQLite doesn't support RENAME USER
				panic!("SQLite does not support RENAME USER")
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::backends::backend::DatabaseBackend;
	use crate::backends::error::Result;
	use crate::backends::types::{DatabaseType, QueryResult, QueryValue, Row, TransactionExecutor};

	// Mock transaction executor for testing
	struct MockTransactionExecutor;

	#[async_trait::async_trait]
	impl TransactionExecutor for MockTransactionExecutor {
		async fn execute(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
			Ok(QueryResult { rows_affected: 0 })
		}

		async fn fetch_one(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
			Ok(Row::new())
		}

		async fn fetch_all(&mut self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
			Ok(Vec::new())
		}

		async fn fetch_optional(
			&mut self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> Result<Option<Row>> {
			Ok(None)
		}

		async fn commit(self: Box<Self>) -> Result<()> {
			Ok(())
		}

		async fn rollback(self: Box<Self>) -> Result<()> {
			Ok(())
		}
	}

	struct MockBackend {
		db_type: DatabaseType,
	}

	impl MockBackend {
		fn postgres() -> Arc<Self> {
			Arc::new(Self {
				db_type: DatabaseType::Postgres,
			})
		}

		fn mysql() -> Arc<Self> {
			Arc::new(Self {
				db_type: DatabaseType::Mysql,
			})
		}
	}

	#[async_trait::async_trait]
	impl DatabaseBackend for MockBackend {
		fn database_type(&self) -> DatabaseType {
			self.db_type
		}

		fn placeholder(&self, index: usize) -> String {
			format!("${}", index)
		}

		fn supports_returning(&self) -> bool {
			matches!(self.db_type, DatabaseType::Postgres)
		}

		fn supports_on_conflict(&self) -> bool {
			true
		}

		async fn execute(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<QueryResult> {
			Ok(QueryResult { rows_affected: 1 })
		}

		async fn fetch_one(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Row> {
			Ok(Row::new())
		}

		async fn fetch_all(&self, _sql: &str, _params: Vec<QueryValue>) -> Result<Vec<Row>> {
			Ok(Vec::new())
		}

		async fn fetch_optional(
			&self,
			_sql: &str,
			_params: Vec<QueryValue>,
		) -> Result<Option<Row>> {
			Ok(None)
		}

		fn as_any(&self) -> &dyn std::any::Any {
			self
		}

		async fn begin(&self) -> Result<Box<dyn TransactionExecutor>> {
			Ok(Box::new(MockTransactionExecutor))
		}
	}

	#[test]
	fn test_validate_name_success() {
		let result = validate_name("admin", "User name");
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), "admin");
	}

	#[test]
	fn test_validate_name_trims_whitespace() {
		let result = validate_name("  admin  ", "User name");
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), "admin");
	}

	#[test]
	fn test_validate_name_rejects_empty() {
		let result = validate_name("", "User name");
		assert!(result.is_err());
		assert_eq!(
			result.unwrap_err(),
			"User name cannot be empty or whitespace"
		);
	}

	#[test]
	fn test_validate_name_rejects_whitespace_only() {
		let result = validate_name("   ", "Role name");
		assert!(result.is_err());
		assert_eq!(
			result.unwrap_err(),
			"Role name cannot be empty or whitespace"
		);
	}

	#[test]
	fn test_validate_name_rejects_tabs_and_newlines() {
		let result = validate_name("\t\n", "User name");
		assert!(result.is_err());
	}

	#[test]
	fn test_create_user_postgres() {
		let backend = MockBackend::postgres();
		let stmt = CreateUserStatement::new(backend).user("admin").unwrap();
		assert_eq!(stmt.build(), "CREATE USER \"admin\"");
	}

	#[test]
	fn test_create_user_mysql() {
		let backend = MockBackend::mysql();
		let stmt = CreateUserStatement::new(backend).user("admin").unwrap();
		assert_eq!(stmt.build(), "CREATE USER 'admin'@'%'");
	}

	#[test]
	fn test_create_user_validates_empty() {
		let backend = MockBackend::postgres();
		let result = CreateUserStatement::new(backend).user("");
		assert!(result.is_err());
		assert_eq!(
			result.unwrap_err(),
			"User name cannot be empty or whitespace"
		);
	}

	#[test]
	fn test_create_user_validates_whitespace() {
		let backend = MockBackend::postgres();
		let result = CreateUserStatement::new(backend).user("   ");
		assert!(result.is_err());
	}

	#[test]
	fn test_create_user_trims_name() {
		let backend = MockBackend::postgres();
		let stmt = CreateUserStatement::new(backend).user("  user  ").unwrap();
		assert_eq!(stmt.build(), "CREATE USER \"user\"");
	}

	#[test]
	fn test_set_role_postgres() {
		let backend = MockBackend::postgres();
		let stmt = SetRoleStatement::new(backend).role("admin").unwrap();
		assert_eq!(stmt.build(), "SET ROLE \"admin\"");
	}

	#[test]
	fn test_set_role_mysql() {
		let backend = MockBackend::mysql();
		let stmt = SetRoleStatement::new(backend).role("admin").unwrap();
		assert_eq!(stmt.build(), "SET ROLE 'admin'");
	}

	#[test]
	fn test_set_role_validates_empty() {
		let backend = MockBackend::postgres();
		let result = SetRoleStatement::new(backend).role("");
		assert!(result.is_err());
		assert_eq!(
			result.unwrap_err(),
			"Role name cannot be empty or whitespace"
		);
	}

	#[test]
	fn test_alter_user_validates() {
		let backend = MockBackend::postgres();
		let result = AlterUserStatement::new(backend).user("");
		assert!(result.is_err());
	}

	#[test]
	fn test_create_role_validates() {
		let backend = MockBackend::postgres();
		let result = CreateRoleStatement::new(backend).role("");
		assert!(result.is_err());
	}

	#[test]
	fn test_alter_role_validates() {
		let backend = MockBackend::postgres();
		let result = AlterRoleStatement::new(backend).role("");
		assert!(result.is_err());
	}

	#[test]
	fn test_set_default_role_validates_user() {
		let backend = MockBackend::postgres();
		let result = SetDefaultRoleStatement::new(backend).user("");
		assert!(result.is_err());
	}

	#[test]
	fn test_set_default_role_validates_users() {
		let backend = MockBackend::postgres();
		let result = SetDefaultRoleStatement::new(backend)
			.user("admin")
			.unwrap()
			.users(&["role1", "", "role2"]);
		assert!(result.is_err());
	}

	#[test]
	fn test_rename_user_validates() {
		let backend = MockBackend::postgres();

		// Test empty old name
		let result = RenameUserStatement::new(backend.clone()).rename("", "new");
		assert!(result.is_err());

		// Test empty new name
		let result = RenameUserStatement::new(backend.clone()).rename("old", "   ");
		assert!(result.is_err());

		// Test valid rename
		let result = RenameUserStatement::new(backend).rename("old", "new");
		assert!(result.is_ok());
	}

	#[test]
	fn test_rename_user_postgres() {
		let backend = MockBackend::postgres();
		let stmt = RenameUserStatement::new(backend)
			.rename("olduser", "newuser")
			.unwrap();
		assert_eq!(stmt.build(), "ALTER USER \"olduser\" RENAME TO \"newuser\"");
	}

	#[test]
	fn test_rename_user_mysql() {
		let backend = MockBackend::mysql();
		let stmt = RenameUserStatement::new(backend)
			.rename("olduser", "newuser")
			.unwrap();
		assert_eq!(stmt.build(), "RENAME USER 'olduser'@'%' TO 'newuser'@'%'");
	}
}
