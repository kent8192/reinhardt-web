//! Advanced settings and configuration
//!
//! This module provides a flexible configuration system inspired by Django's settings.
//! Settings can be loaded from environment variables, configuration files, or code.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// `AdvancedSettings` (deprecated since 0.1.0-rc.16) and its impl block were
// removed in 0.2.0 per Issue #4520. The individual fragment types below
// (`DatabaseSettings`, `CacheSettings`, `SessionSettings`, etc.) are the
// canonical building blocks composed via `ProjectSettings`.

/// Database settings
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSettings {
	/// Database connection URL (e.g., `"sqlite::memory:"`, `"postgres://..."`)
	pub url: String,
	/// Maximum number of connections in the pool.
	pub max_connections: u32,
	/// Minimum number of idle connections to maintain.
	pub min_connections: u32,
	/// Connection timeout in seconds.
	pub connect_timeout: u64,
	/// Idle connection timeout in seconds before eviction.
	pub idle_timeout: u64,
}

impl Default for DatabaseSettings {
	fn default() -> Self {
		Self {
			url: "sqlite::memory:".to_string(),
			max_connections: 10,
			min_connections: 1,
			connect_timeout: 30,
			idle_timeout: 600,
		}
	}
}

/// Cache settings
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
	/// Cache backend type (e.g., `"memory"`, `"redis"`, `"database"`).
	pub backend: String,
	/// Backend-specific connection location or URL.
	pub location: Option<String>,
	/// Default cache entry timeout in seconds.
	pub timeout: u64,
}

impl Default for CacheSettings {
	fn default() -> Self {
		Self {
			backend: "memory".to_string(),
			location: None,
			timeout: 300,
		}
	}
}

/// Session settings
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSettings {
	/// Session storage engine (e.g., `"cookie"`, `"database"`, `"redis"`).
	pub engine: String,
	/// Name of the session cookie.
	pub cookie_name: String,
	/// Maximum age of the session cookie in seconds.
	pub cookie_age: u64,
	/// Whether to set the `Secure` flag on the session cookie.
	pub cookie_secure: bool,
	/// Whether to set the `HttpOnly` flag on the session cookie.
	pub cookie_httponly: bool,
	/// `SameSite` attribute for the session cookie (e.g., `"lax"`, `"strict"`, `"none"`).
	pub cookie_samesite: String,
}

impl Default for SessionSettings {
	fn default() -> Self {
		Self {
			engine: "cookie".to_string(),
			cookie_name: "sessionid".to_string(),
			cookie_age: 1209600, // 2 weeks
			cookie_secure: false,
			cookie_httponly: true,
			cookie_samesite: "lax".to_string(),
		}
	}
}

/// CORS settings
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsSettings {
	/// Allowed origin domains (use `"*"` for any origin).
	pub allow_origins: Vec<String>,
	/// Allowed HTTP methods.
	pub allow_methods: Vec<String>,
	/// Allowed HTTP request headers.
	pub allow_headers: Vec<String>,
	/// Whether to allow credentials (cookies, authorization headers).
	pub allow_credentials: bool,
	/// Maximum age (in seconds) for preflight response caching.
	pub max_age: u64,
}

impl Default for CorsSettings {
	fn default() -> Self {
		Self {
			allow_origins: vec!["*".to_string()],
			allow_methods: vec![
				"GET".to_string(),
				"POST".to_string(),
				"PUT".to_string(),
				"PATCH".to_string(),
				"DELETE".to_string(),
			],
			allow_headers: vec!["*".to_string()],
			allow_credentials: false,
			max_age: 3600,
		}
	}
}

/// Static files settings
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticSettings {
	/// URL prefix for serving static files (e.g., `"/static/"`).
	pub url: String,
	/// Root directory for collected static files.
	pub root: PathBuf,
}

impl Default for StaticSettings {
	fn default() -> Self {
		Self {
			url: "/static/".to_string(),
			root: PathBuf::from("static"),
		}
	}
}

/// Media files settings
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaSettings {
	/// URL prefix for serving user-uploaded media files (e.g., `"/media/"`).
	pub url: String,
	/// Root directory for user-uploaded media files.
	pub root: PathBuf,
}

impl Default for MediaSettings {
	fn default() -> Self {
		Self {
			url: "/media/".to_string(),
			root: PathBuf::from("media"),
		}
	}
}

/// Email settings
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSettings {
	/// Email backend type (e.g., `"smtp"`, `"console"`, `"file"`, `"memory"`).
	pub backend: String,
	/// SMTP server hostname.
	pub host: String,
	/// SMTP server port number.
	pub port: u16,
	/// Optional SMTP authentication username.
	pub username: Option<String>,
	/// Optional SMTP authentication password.
	pub password: Option<String>,
	/// Whether to use STARTTLS for the SMTP connection.
	pub use_tls: bool,
	/// Whether to use direct TLS/SSL for the SMTP connection.
	pub use_ssl: bool,
	/// Default sender email address for outgoing emails.
	pub from_email: String,

	/// List of (name, email) tuples for site administrators
	/// Used by mail_admins() helper
	#[serde(default)]
	pub admins: Vec<(String, String)>,

	/// List of (name, email) tuples for site managers
	/// Used by mail_managers() helper
	#[serde(default)]
	pub managers: Vec<(String, String)>,

	/// Email address for server error notifications
	#[serde(default = "default_server_email")]
	pub server_email: String,

	/// Prefix for email subjects (e.g., `"[Django]"`)
	#[serde(default)]
	pub subject_prefix: String,

	/// Connection timeout in seconds
	pub timeout: Option<u64>,

	/// Path to SSL certificate file
	pub ssl_certfile: Option<PathBuf>,

	/// Path to SSL key file
	pub ssl_keyfile: Option<PathBuf>,

	/// Directory path for file-based email backend.
	/// Required when backend is "file".
	#[serde(default)]
	pub file_path: Option<PathBuf>,
}

fn default_server_email() -> String {
	"root@localhost".to_string()
}

impl Default for EmailSettings {
	fn default() -> Self {
		Self {
			backend: "console".to_string(),
			host: "localhost".to_string(),
			port: 25,
			username: None,
			password: None,
			use_tls: false,
			use_ssl: false,
			from_email: "noreply@example.com".to_string(),
			admins: Vec::new(),
			managers: Vec::new(),
			server_email: default_server_email(),
			subject_prefix: String::new(),
			timeout: None,
			ssl_certfile: None,
			ssl_keyfile: None,
			file_path: None,
		}
	}
}

/// Logging settings
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
	/// Log level (e.g., `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`).
	pub level: String,
	/// Log output format (e.g., `"text"`, `"json"`).
	pub format: String,
}

impl Default for LoggingSettings {
	fn default() -> Self {
		Self {
			level: "info".to_string(),
			format: "text".to_string(),
		}
	}
}

/// Settings error
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
	/// An error occurred reading or writing a configuration file.
	#[error("File error: {0}")]
	FileError(String),

	/// An error occurred parsing configuration content.
	#[error("Parse error: {0}")]
	ParseError(String),

	/// A configuration value failed validation.
	#[error("Validation error: {0}")]
	ValidationError(String),

	/// The configuration file format is not supported.
	#[error("Unsupported format: {0}")]
	UnsupportedFormat(String),

	/// An error occurred during serialization or deserialization.
	#[error("Serialization error: {0}")]
	SerializationError(String),
}

// The `mod tests` that exercised `AdvancedSettings::default`, `validate`,
// `set`/`get`, etc. was removed in 0.2.0 per Issue #4520, alongside the
// deprecated `AdvancedSettings` struct itself. Replacement coverage for
// the surviving fragment types (`DatabaseSettings`, `CacheSettings`, etc.)
// lives in their dedicated test modules.
