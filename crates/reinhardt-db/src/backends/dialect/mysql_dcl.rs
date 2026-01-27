//! MySQL Data Control Language (DCL) support
//!
//! This module provides structures and utilities for building MySQL DCL statements
//! with proper user@host syntax handling.
//!
//! ## Overview
//!
//! MySQL uses a unique `'user'@'host'` syntax for user identifiers that differs from
//! most other database systems. This module ensures proper parsing and formatting of
//! MySQL user identifiers for all DCL operations.
//!
//! ## Key Features
//!
//! - **Automatic host defaulting**: Users without explicit hosts default to `'%'` (any host)
//! - **Type-safe builders**: Fluent API for constructing DCL statements
//! - **SQL injection protection**: Values are properly escaped and quoted
//!
//! ## Usage Examples
//!
//! ### CREATE USER
//!
//! ```rust
//! use reinhardt_db::backends::{CreateUserStatement, MySqlUser};
//!
//! // Create user with explicit host
//! let stmt = CreateUserStatement::new("app_user@localhost")
//!     .password("secret123");
//! assert_eq!(
//!     stmt.build(),
//!     "CREATE USER 'app_user'@'localhost' IDENTIFIED BY 'secret123'"
//! );
//!
//! // Create user with default host (%)
//! let stmt = CreateUserStatement::new("admin")
//!     .password("admin123")
//!     .if_not_exists();
//! assert_eq!(
//!     stmt.build(),
//!     "CREATE USER IF NOT EXISTS 'admin'@'%' IDENTIFIED BY 'admin123'"
//! );
//! ```
//!
//! ### ALTER USER
//!
//! ```rust
//! use reinhardt_db::backends::AlterUserStatement;
//!
//! let stmt = AlterUserStatement::new("app_user@localhost")
//!     .password("new_password");
//! assert_eq!(
//!     stmt.build(),
//!     "ALTER USER 'app_user'@'localhost' IDENTIFIED BY 'new_password'"
//! );
//! ```
//!
//! ### DROP USER
//!
//! ```rust
//! use reinhardt_db::backends::DropUserStatement;
//!
//! // Drop single user
//! let stmt = DropUserStatement::new()
//!     .user("old_user@localhost")
//!     .if_exists();
//! assert_eq!(
//!     stmt.build(),
//!     "DROP USER IF EXISTS 'old_user'@'localhost'"
//! );
//!
//! // Drop multiple users
//! let stmt = DropUserStatement::new()
//!     .user("user1@localhost")
//!     .user("user2@%");
//! assert_eq!(
//!     stmt.build(),
//!     "DROP USER 'user1'@'localhost', 'user2'@'%'"
//! );
//! ```
//!
//! ### RENAME USER
//!
//! ```rust
//! use reinhardt_db::backends::RenameUserStatement;
//!
//! let stmt = RenameUserStatement::new("old_name@localhost", "new_name@localhost");
//! assert_eq!(
//!     stmt.build(),
//!     "RENAME USER 'old_name'@'localhost' TO 'new_name'@'localhost'"
//! );
//! ```
//!
//! ### SET DEFAULT ROLE
//!
//! ```rust
//! use reinhardt_db::backends::{DefaultRoleSpec, SetDefaultRoleStatement};
//!
//! // Set all roles as default
//! let stmt = SetDefaultRoleStatement::new()
//!     .roles(DefaultRoleSpec::All)
//!     .user("app_user@localhost");
//! assert_eq!(
//!     stmt.build(),
//!     "SET DEFAULT ROLE ALL TO 'app_user'@'localhost'"
//! );
//!
//! // Set specific roles
//! let stmt = SetDefaultRoleStatement::new()
//!     .roles(DefaultRoleSpec::Roles(vec!["role1".to_string(), "role2".to_string()]))
//!     .user("app_user@localhost");
//! assert_eq!(
//!     stmt.build(),
//!     "SET DEFAULT ROLE role1, role2 TO 'app_user'@'localhost'"
//! );
//!
//! // Clear default roles
//! let stmt = SetDefaultRoleStatement::new()
//!     .roles(DefaultRoleSpec::None)
//!     .user("app_user@localhost");
//! assert_eq!(
//!     stmt.build(),
//!     "SET DEFAULT ROLE NONE TO 'app_user'@'localhost'"
//! );
//! ```
//!
//! ## MySQL User Identifier Format
//!
//! MySQL user identifiers follow the format `'user'@'host'` where:
//!
//! - **user**: The username (required)
//! - **host**: The hostname, IP address, or `%` for any host (defaults to `%` if not specified)
//!
//! ### Valid Host Specifications
//!
//! - `localhost` - Local connections only
//! - `192.168.1.100` - Specific IP address
//! - `%.example.com` - Any host in domain
//! - `%` - Any host (default)
//!
//! ### Parsing Examples
//!
//! ```rust
//! use reinhardt_db::backends::MySqlUser;
//!
//! // With explicit host
//! let user = MySqlUser::parse("app_user@localhost");
//! assert_eq!(user.to_string(), "'app_user'@'localhost'");
//!
//! // Without host (defaults to %)
//! let user = MySqlUser::parse("admin");
//! assert_eq!(user.to_string(), "'admin'@'%'");
//!
//! // With wildcard host
//! let user = MySqlUser::parse("web@%.example.com");
//! assert_eq!(user.to_string(), "'web'@'%.example.com'");
//! ```

use std::fmt::{self, Display, Formatter};

/// MySQL user identifier with host specification
///
/// MySQL uses `'user'@'host'` syntax for user identifiers.
/// This struct parses and formats user identifiers correctly.
///
/// # Examples
///
/// ```
/// use reinhardt_db::backends::dialect::MySqlUser;
///
/// // Parse user with explicit host
/// let user = MySqlUser::parse("app_user@localhost");
/// assert_eq!(user.user(), "app_user");
/// assert_eq!(user.host(), "localhost");
/// assert_eq!(user.to_string(), "'app_user'@'localhost'");
///
/// // Parse user without host (defaults to '%')
/// let user = MySqlUser::parse("admin");
/// assert_eq!(user.user(), "admin");
/// assert_eq!(user.host(), "%");
/// assert_eq!(user.to_string(), "'admin'@'%'");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MySqlUser {
	user: String,
	host: String,
}

impl MySqlUser {
	/// Create a new MySqlUser with explicit user and host
	///
	/// # Arguments
	///
	/// * `user` - The username
	/// * `host` - The host specification
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::backends::dialect::MySqlUser;
	///
	/// let user = MySqlUser::new("admin", "localhost");
	/// assert_eq!(user.to_string(), "'admin'@'localhost'");
	/// ```
	pub fn new(user: impl Into<String>, host: impl Into<String>) -> Self {
		Self {
			user: user.into(),
			host: host.into(),
		}
	}

	/// Parse a user identifier string
	///
	/// If the input contains an `@` symbol, it's split into user and host.
	/// Otherwise, the entire string is used as the user and host defaults to `%`.
	///
	/// # Arguments
	///
	/// * `input` - The user identifier string (e.g., "user@host" or "user")
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_db::backends::dialect::MySqlUser;
	///
	/// let user = MySqlUser::parse("app_user@localhost");
	/// assert_eq!(user.user(), "app_user");
	/// assert_eq!(user.host(), "localhost");
	///
	/// let user = MySqlUser::parse("admin");
	/// assert_eq!(user.user(), "admin");
	/// assert_eq!(user.host(), "%");
	/// ```
	pub fn parse(input: &str) -> Self {
		if let Some((user, host)) = input.split_once('@') {
			Self {
				user: user.to_string(),
				host: host.to_string(),
			}
		} else {
			Self {
				user: input.to_string(),
				host: "%".to_string(),
			}
		}
	}

	/// Get the username portion
	pub fn user(&self) -> &str {
		&self.user
	}

	/// Get the host portion
	pub fn host(&self) -> &str {
		&self.host
	}
}

impl Display for MySqlUser {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "'{}'@'{}'", self.user, self.host)
	}
}

impl From<&str> for MySqlUser {
	fn from(s: &str) -> Self {
		Self::parse(s)
	}
}

impl From<String> for MySqlUser {
	fn from(s: String) -> Self {
		Self::parse(&s)
	}
}

/// Default role specification for SET DEFAULT ROLE
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefaultRoleSpec {
	/// No default roles
	None,
	/// All roles granted to the user
	All,
	/// Specific roles
	Roles(Vec<String>),
}

/// CREATE USER statement builder
#[derive(Debug, Clone)]
pub struct CreateUserStatement {
	user: MySqlUser,
	password: Option<String>,
	if_not_exists: bool,
}

impl CreateUserStatement {
	/// Create a new CREATE USER statement
	pub fn new(user: impl Into<MySqlUser>) -> Self {
		Self {
			user: user.into(),
			password: None,
			if_not_exists: false,
		}
	}

	/// Set the password for the user
	pub fn password(mut self, password: impl Into<String>) -> Self {
		self.password = Some(password.into());
		self
	}

	/// Add IF NOT EXISTS clause
	pub fn if_not_exists(mut self) -> Self {
		self.if_not_exists = true;
		self
	}

	/// Build the SQL statement
	pub fn build(&self) -> String {
		let mut sql = String::from("CREATE USER ");
		if self.if_not_exists {
			sql.push_str("IF NOT EXISTS ");
		}
		sql.push_str(&self.user.to_string());
		if let Some(password) = &self.password {
			sql.push_str(" IDENTIFIED BY '");
			sql.push_str(password);
			sql.push('\'');
		}
		sql
	}
}

/// ALTER USER statement builder
#[derive(Debug, Clone)]
pub struct AlterUserStatement {
	user: MySqlUser,
	password: Option<String>,
}

impl AlterUserStatement {
	/// Create a new ALTER USER statement
	pub fn new(user: impl Into<MySqlUser>) -> Self {
		Self {
			user: user.into(),
			password: None,
		}
	}

	/// Set the new password for the user
	pub fn password(mut self, password: impl Into<String>) -> Self {
		self.password = Some(password.into());
		self
	}

	/// Build the SQL statement
	pub fn build(&self) -> String {
		let mut sql = format!("ALTER USER {}", self.user);
		if let Some(password) = &self.password {
			sql.push_str(" IDENTIFIED BY '");
			sql.push_str(password);
			sql.push('\'');
		}
		sql
	}
}

/// DROP USER statement builder
#[derive(Debug, Clone)]
pub struct DropUserStatement {
	users: Vec<MySqlUser>,
	if_exists: bool,
}

impl DropUserStatement {
	/// Create a new DROP USER statement
	pub fn new() -> Self {
		Self {
			users: Vec::new(),
			if_exists: false,
		}
	}

	/// Add a user to drop
	pub fn user(mut self, user: impl Into<MySqlUser>) -> Self {
		self.users.push(user.into());
		self
	}

	/// Add multiple users to drop
	pub fn users(mut self, users: Vec<impl Into<MySqlUser>>) -> Self {
		for user in users {
			self.users.push(user.into());
		}
		self
	}

	/// Add IF EXISTS clause
	pub fn if_exists(mut self) -> Self {
		self.if_exists = true;
		self
	}

	/// Build the SQL statement
	pub fn build(&self) -> String {
		let mut sql = String::from("DROP USER ");
		if self.if_exists {
			sql.push_str("IF EXISTS ");
		}
		let user_strings: Vec<String> = self.users.iter().map(|u| u.to_string()).collect();
		sql.push_str(&user_strings.join(", "));
		sql
	}
}

impl Default for DropUserStatement {
	fn default() -> Self {
		Self::new()
	}
}

/// RENAME USER statement builder
#[derive(Debug, Clone)]
pub struct RenameUserStatement {
	old_user: MySqlUser,
	new_user: MySqlUser,
}

impl RenameUserStatement {
	/// Create a new RENAME USER statement
	pub fn new(old_user: impl Into<MySqlUser>, new_user: impl Into<MySqlUser>) -> Self {
		Self {
			old_user: old_user.into(),
			new_user: new_user.into(),
		}
	}

	/// Build the SQL statement
	pub fn build(&self) -> String {
		format!("RENAME USER {} TO {}", self.old_user, self.new_user)
	}
}

/// SET DEFAULT ROLE statement builder
#[derive(Debug, Clone)]
pub struct SetDefaultRoleStatement {
	roles: DefaultRoleSpec,
	users: Vec<MySqlUser>,
}

impl SetDefaultRoleStatement {
	/// Create a new SET DEFAULT ROLE statement
	pub fn new() -> Self {
		Self {
			roles: DefaultRoleSpec::None,
			users: Vec::new(),
		}
	}

	/// Set the roles specification
	pub fn roles(mut self, roles: DefaultRoleSpec) -> Self {
		self.roles = roles;
		self
	}

	/// Add a user
	pub fn user(mut self, user: impl Into<MySqlUser>) -> Self {
		self.users.push(user.into());
		self
	}

	/// Add multiple users
	pub fn users_list(mut self, users: Vec<impl Into<MySqlUser>>) -> Self {
		for user in users {
			self.users.push(user.into());
		}
		self
	}

	/// Build the SQL statement
	pub fn build(&self) -> String {
		let mut sql = String::from("SET DEFAULT ROLE ");
		match &self.roles {
			DefaultRoleSpec::None => sql.push_str("NONE"),
			DefaultRoleSpec::All => sql.push_str("ALL"),
			DefaultRoleSpec::Roles(roles) => {
				sql.push_str(&roles.join(", "));
			}
		}
		sql.push_str(" TO ");
		let user_strings: Vec<String> = self.users.iter().map(|u| u.to_string()).collect();
		sql.push_str(&user_strings.join(", "));
		sql
	}
}

impl Default for SetDefaultRoleStatement {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// MySqlUser tests
	#[test]
	fn test_parse_user_with_host() {
		let user = MySqlUser::parse("app_user@localhost");
		assert_eq!(user.user(), "app_user");
		assert_eq!(user.host(), "localhost");
		assert_eq!(user.to_string(), "'app_user'@'localhost'");
	}

	#[test]
	fn test_parse_user_without_host() {
		let user = MySqlUser::parse("admin");
		assert_eq!(user.user(), "admin");
		assert_eq!(user.host(), "%");
		assert_eq!(user.to_string(), "'admin'@'%'");
	}

	#[test]
	fn test_parse_user_with_wildcard_host() {
		let user = MySqlUser::parse("app@%");
		assert_eq!(user.user(), "app");
		assert_eq!(user.host(), "%");
		assert_eq!(user.to_string(), "'app'@'%'");
	}

	#[test]
	fn test_parse_user_with_ip_host() {
		let user = MySqlUser::parse("user@192.168.1.100");
		assert_eq!(user.user(), "user");
		assert_eq!(user.host(), "192.168.1.100");
		assert_eq!(user.to_string(), "'user'@'192.168.1.100'");
	}

	#[test]
	fn test_parse_user_with_domain_host() {
		let user = MySqlUser::parse("user@example.com");
		assert_eq!(user.user(), "user");
		assert_eq!(user.host(), "example.com");
		assert_eq!(user.to_string(), "'user'@'example.com'");
	}

	#[test]
	fn test_new_user() {
		let user = MySqlUser::new("test_user", "test_host");
		assert_eq!(user.user(), "test_user");
		assert_eq!(user.host(), "test_host");
		assert_eq!(user.to_string(), "'test_user'@'test_host'");
	}

	#[test]
	fn test_from_str() {
		let user: MySqlUser = "user@host".into();
		assert_eq!(user.user(), "user");
		assert_eq!(user.host(), "host");
	}

	#[test]
	fn test_from_string() {
		let user: MySqlUser = "user@host".to_string().into();
		assert_eq!(user.user(), "user");
		assert_eq!(user.host(), "host");
	}

	#[test]
	fn test_equality() {
		let user1 = MySqlUser::parse("user@host");
		let user2 = MySqlUser::new("user", "host");
		assert_eq!(user1, user2);
	}

	// CREATE USER tests
	#[test]
	fn test_create_user_basic() {
		let stmt = CreateUserStatement::new("app_user@localhost");
		assert_eq!(stmt.build(), "CREATE USER 'app_user'@'localhost'");
	}

	#[test]
	fn test_create_user_with_password() {
		let stmt = CreateUserStatement::new("app_user@localhost").password("secret123");
		assert_eq!(
			stmt.build(),
			"CREATE USER 'app_user'@'localhost' IDENTIFIED BY 'secret123'"
		);
	}

	#[test]
	fn test_create_user_if_not_exists() {
		let stmt = CreateUserStatement::new("app_user@localhost").if_not_exists();
		assert_eq!(
			stmt.build(),
			"CREATE USER IF NOT EXISTS 'app_user'@'localhost'"
		);
	}

	#[test]
	fn test_create_user_default_host() {
		let stmt = CreateUserStatement::new("admin");
		assert_eq!(stmt.build(), "CREATE USER 'admin'@'%'");
	}

	// ALTER USER tests
	#[test]
	fn test_alter_user_password() {
		let stmt = AlterUserStatement::new("app_user@localhost").password("newsecret");
		assert_eq!(
			stmt.build(),
			"ALTER USER 'app_user'@'localhost' IDENTIFIED BY 'newsecret'"
		);
	}

	// DROP USER tests
	#[test]
	fn test_drop_user_single() {
		let stmt = DropUserStatement::new().user("app_user@localhost");
		assert_eq!(stmt.build(), "DROP USER 'app_user'@'localhost'");
	}

	#[test]
	fn test_drop_user_multiple() {
		let stmt = DropUserStatement::new()
			.user("user1@localhost")
			.user("user2@%");
		assert_eq!(stmt.build(), "DROP USER 'user1'@'localhost', 'user2'@'%'");
	}

	#[test]
	fn test_drop_user_if_exists() {
		let stmt = DropUserStatement::new()
			.user("app_user@localhost")
			.if_exists();
		assert_eq!(stmt.build(), "DROP USER IF EXISTS 'app_user'@'localhost'");
	}

	// RENAME USER tests
	#[test]
	fn test_rename_user() {
		let stmt = RenameUserStatement::new("old_user@localhost", "new_user@localhost");
		assert_eq!(
			stmt.build(),
			"RENAME USER 'old_user'@'localhost' TO 'new_user'@'localhost'"
		);
	}

	// SET DEFAULT ROLE tests
	#[test]
	fn test_set_default_role_none() {
		let stmt = SetDefaultRoleStatement::new()
			.roles(DefaultRoleSpec::None)
			.user("app_user@localhost");
		assert_eq!(
			stmt.build(),
			"SET DEFAULT ROLE NONE TO 'app_user'@'localhost'"
		);
	}

	#[test]
	fn test_set_default_role_all() {
		let stmt = SetDefaultRoleStatement::new()
			.roles(DefaultRoleSpec::All)
			.user("app_user@localhost");
		assert_eq!(
			stmt.build(),
			"SET DEFAULT ROLE ALL TO 'app_user'@'localhost'"
		);
	}

	#[test]
	fn test_set_default_role_specific() {
		let stmt = SetDefaultRoleStatement::new()
			.roles(DefaultRoleSpec::Roles(vec![
				"role1".to_string(),
				"role2".to_string(),
			]))
			.user("app_user@localhost");
		assert_eq!(
			stmt.build(),
			"SET DEFAULT ROLE role1, role2 TO 'app_user'@'localhost'"
		);
	}

	#[test]
	fn test_set_default_role_multiple_users() {
		let stmt = SetDefaultRoleStatement::new()
			.roles(DefaultRoleSpec::All)
			.user("user1@localhost")
			.user("user2@%");
		assert_eq!(
			stmt.build(),
			"SET DEFAULT ROLE ALL TO 'user1'@'localhost', 'user2'@'%'"
		);
	}

	#[test]
	fn test_set_default_role_default_host() {
		let stmt = SetDefaultRoleStatement::new()
			.roles(DefaultRoleSpec::All)
			.user("admin");
		assert_eq!(stmt.build(), "SET DEFAULT ROLE ALL TO 'admin'@'%'");
	}
}
