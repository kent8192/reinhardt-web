//! MySQL Data Control Language (DCL) support
//!
//! This module provides structures and utilities for building MySQL DCL statements
//! with proper user@host syntax handling.

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

#[cfg(test)]
mod tests {
	use super::*;

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
}
