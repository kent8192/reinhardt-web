//! `SessionConfig`: cookie name, TTL, and cookie-attribute knobs.

use std::time::Duration;

/// Session configuration
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SessionConfig {
	/// Cookie name
	pub cookie_name: String,
	/// Session TTL
	pub ttl: Duration,
	/// HTTPS-only cookie
	pub secure: bool,
	/// HttpOnly flag
	pub http_only: bool,
	/// SameSite attribute
	pub same_site: Option<String>,
	/// Domain
	pub domain: Option<String>,
	/// Path
	pub path: String,
}

impl SessionConfig {
	/// Create a new configuration
	///
	/// # Examples
	///
	/// ```
	/// use std::time::Duration;
	/// use reinhardt_middleware::session::SessionConfig;
	///
	/// let config = SessionConfig::new("sessionid".to_string(), Duration::from_secs(3600));
	/// assert_eq!(config.cookie_name, "sessionid");
	/// assert_eq!(config.ttl, Duration::from_secs(3600));
	/// ```
	pub fn new(cookie_name: String, ttl: Duration) -> Self {
		Self {
			cookie_name,
			ttl,
			secure: true,
			http_only: true,
			same_site: Some("Lax".to_string()),
			domain: None,
			path: "/".to_string(),
		}
	}

	/// Enable secure cookie
	///
	/// # Examples
	///
	/// ```
	/// use std::time::Duration;
	/// use reinhardt_middleware::session::SessionConfig;
	///
	/// let config = SessionConfig::new("sessionid".to_string(), Duration::from_secs(3600))
	///     .with_secure();
	/// assert!(config.secure);
	/// ```
	pub fn with_secure(mut self) -> Self {
		self.secure = true;
		self
	}

	/// Set HttpOnly flag
	///
	/// # Examples
	///
	/// ```
	/// use std::time::Duration;
	/// use reinhardt_middleware::session::SessionConfig;
	///
	/// let config = SessionConfig::new("sessionid".to_string(), Duration::from_secs(3600))
	///     .with_http_only(false);
	/// assert!(!config.http_only);
	/// ```
	pub fn with_http_only(mut self, http_only: bool) -> Self {
		self.http_only = http_only;
		self
	}

	/// Set SameSite attribute
	///
	/// # Examples
	///
	/// ```
	/// use std::time::Duration;
	/// use reinhardt_middleware::session::SessionConfig;
	///
	/// let config = SessionConfig::new("sessionid".to_string(), Duration::from_secs(3600))
	///     .with_same_site("Strict".to_string());
	/// ```
	pub fn with_same_site(mut self, same_site: String) -> Self {
		self.same_site = Some(same_site);
		self
	}

	/// Set domain
	///
	/// # Examples
	///
	/// ```
	/// use std::time::Duration;
	/// use reinhardt_middleware::session::SessionConfig;
	///
	/// let config = SessionConfig::new("sessionid".to_string(), Duration::from_secs(3600))
	///     .with_domain("example.com".to_string());
	/// ```
	pub fn with_domain(mut self, domain: String) -> Self {
		self.domain = Some(domain);
		self
	}

	/// Set path
	///
	/// # Examples
	///
	/// ```
	/// use std::time::Duration;
	/// use reinhardt_middleware::session::SessionConfig;
	///
	/// let config = SessionConfig::new("sessionid".to_string(), Duration::from_secs(3600))
	///     .with_path("/app".to_string());
	/// assert_eq!(config.path, "/app");
	/// ```
	pub fn with_path(mut self, path: String) -> Self {
		self.path = path;
		self
	}

}

impl Default for SessionConfig {
	fn default() -> Self {
		Self::new("sessionid".to_string(), Duration::from_secs(3600))
	}
}
