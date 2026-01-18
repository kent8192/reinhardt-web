//! Database configuration for settings
//!
//! This module provides the `DatabaseConfig` struct and its methods for
//! configuring database connections in Reinhardt settings files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Database configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DatabaseConfig {
	/// Database engine/backend
	pub engine: String,

	/// Database name or path
	pub name: String,

	/// Database user (if applicable)
	pub user: Option<String>,

	/// Database password (if applicable)
	pub password: Option<String>,

	/// Database host (if applicable)
	pub host: Option<String>,

	/// Database port (if applicable)
	pub port: Option<u16>,

	/// Additional options
	pub options: HashMap<String, String>,
}

impl DatabaseConfig {
	/// Create a SQLite database configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_conf::settings::DatabaseConfig;
	///
	/// let db = DatabaseConfig::sqlite("myapp.db");
	///
	/// assert_eq!(db.engine, "reinhardt.db.backends.sqlite3");
	/// assert_eq!(db.name, "myapp.db");
	/// assert!(db.user.is_none());
	/// assert!(db.password.is_none());
	/// ```
	pub fn sqlite(name: impl Into<String>) -> Self {
		Self {
			engine: "reinhardt.db.backends.sqlite3".to_string(),
			name: name.into(),
			user: None,
			password: None,
			host: None,
			port: None,
			options: HashMap::new(),
		}
	}
	/// Create a PostgreSQL database configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_conf::settings::DatabaseConfig;
	///
	/// let db = DatabaseConfig::postgresql("mydb", "admin", "password123", "localhost", 5432);
	///
	/// assert_eq!(db.engine, "reinhardt.db.backends.postgresql");
	/// assert_eq!(db.name, "mydb");
	/// assert_eq!(db.user, Some("admin".to_string()));
	/// assert_eq!(db.password, Some("password123".to_string()));
	/// assert_eq!(db.host, Some("localhost".to_string()));
	/// assert_eq!(db.port, Some(5432));
	/// ```
	pub fn postgresql(
		name: impl Into<String>,
		user: impl Into<String>,
		password: impl Into<String>,
		host: impl Into<String>,
		port: u16,
	) -> Self {
		Self {
			engine: "reinhardt.db.backends.postgresql".to_string(),
			name: name.into(),
			user: Some(user.into()),
			password: Some(password.into()),
			host: Some(host.into()),
			port: Some(port),
			options: HashMap::new(),
		}
	}
	/// Create a MySQL database configuration
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_conf::settings::DatabaseConfig;
	///
	/// let db = DatabaseConfig::mysql("mydb", "root", "password123", "localhost", 3306);
	///
	/// assert_eq!(db.engine, "reinhardt.db.backends.mysql");
	/// assert_eq!(db.name, "mydb");
	/// assert_eq!(db.user, Some("root".to_string()));
	/// assert_eq!(db.password, Some("password123".to_string()));
	/// assert_eq!(db.host, Some("localhost".to_string()));
	/// assert_eq!(db.port, Some(3306));
	/// ```
	pub fn mysql(
		name: impl Into<String>,
		user: impl Into<String>,
		password: impl Into<String>,
		host: impl Into<String>,
		port: u16,
	) -> Self {
		Self {
			engine: "reinhardt.db.backends.mysql".to_string(),
			name: name.into(),
			user: Some(user.into()),
			password: Some(password.into()),
			host: Some(host.into()),
			port: Some(port),
			options: HashMap::new(),
		}
	}

	/// Convert DatabaseConfig to DATABASE_URL string
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_conf::settings::DatabaseConfig;
	///
	/// let db = DatabaseConfig::sqlite("db.sqlite3");
	/// assert_eq!(db.to_url(), "sqlite:db.sqlite3");
	///
	/// let db = DatabaseConfig::postgresql("mydb", "user", "pass", "localhost", 5432);
	/// assert_eq!(db.to_url(), "postgresql://user:pass@localhost:5432/mydb");
	/// ```
	pub fn to_url(&self) -> String {
		// Determine the database scheme from engine
		// Handle both short names (e.g., "sqlite") and full backend paths (e.g., "reinhardt.db.backends.sqlite3")
		let scheme = if self.engine == "sqlite" || self.engine.contains("sqlite") {
			"sqlite"
		} else if self.engine == "postgresql"
			|| self.engine == "postgres"
			|| self.engine.contains("postgresql")
			|| self.engine.contains("postgres")
		{
			"postgresql"
		} else if self.engine == "mysql" || self.engine.contains("mysql") {
			"mysql"
		} else {
			// Default to sqlite for unknown engines
			"sqlite"
		};

		match scheme {
			"sqlite" => {
				if self.name == ":memory:" {
					"sqlite::memory:".to_string()
				} else {
					// Use sqlite: format for relative paths (will be converted to absolute in connect_database)
					// sqlite:/// is for absolute paths
					use std::path::Path;
					let path = Path::new(&self.name);
					if path.is_absolute() {
						// Absolute path: sqlite:///path/to/db.sqlite3
						format!("sqlite:///{}", self.name)
					} else {
						// Relative path: sqlite:db.sqlite3 (will be converted to absolute in connect_database)
						format!("sqlite:{}", self.name)
					}
				}
			}
			"postgresql" | "mysql" => {
				let mut url = format!("{}://", scheme);

				// Add user and password if available
				if let Some(user) = &self.user {
					url.push_str(user);
					if let Some(password) = &self.password {
						url.push(':');
						url.push_str(password);
					}
					url.push('@');
				}

				// Add host (default to localhost if not specified)
				let host = self.host.as_deref().unwrap_or("localhost");
				url.push_str(host);

				// Add port if available
				if let Some(port) = self.port {
					url.push(':');
					url.push_str(&port.to_string());
				}

				// Add database name
				url.push('/');
				url.push_str(&self.name);

				// Add query parameters if any
				if !self.options.is_empty() {
					let mut query_parts = Vec::new();
					for (key, value) in &self.options {
						query_parts.push(format!("{}={}", key, value));
					}
					url.push('?');
					url.push_str(&query_parts.join("&"));
				}

				url
			}
			_ => format!("sqlite://{}", self.name),
		}
	}
}

impl Default for DatabaseConfig {
	fn default() -> Self {
		Self::sqlite("db.sqlite3".to_string())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_settings_db_config_sqlite() {
		let db = DatabaseConfig::sqlite("test.db");
		assert_eq!(db.engine, "reinhardt.db.backends.sqlite3");
		assert_eq!(db.name, "test.db");
		assert!(db.user.is_none());
	}

	#[test]
	fn test_settings_db_config_postgresql() {
		let db = DatabaseConfig::postgresql("testdb", "user", "pass", "localhost", 5432);
		assert_eq!(db.engine, "reinhardt.db.backends.postgresql");
		assert_eq!(db.name, "testdb");
		assert_eq!(db.user, Some("user".to_string()));
		assert_eq!(db.port, Some(5432));
	}
}
