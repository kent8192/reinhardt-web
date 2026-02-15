//! Custom URL scheme configuration.
//!
//! This module provides configuration helpers for custom URL schemes (e.g., `myapp://`).

/// Custom URL scheme configuration.
///
/// Custom URL schemes allow apps to be opened via URLs like `myapp://path/to/content`.
/// Unlike Universal Links / App Links, custom schemes require client-side configuration
/// and cannot be verified server-side.
///
/// # Example
///
/// ```rust
/// use reinhardt_deeplink::CustomSchemeConfig;
///
/// let config = CustomSchemeConfig::builder()
///     .scheme("myapp")
///     .host("open")
///     .path("/products/*")
///     .build();
///
/// // Generates URL template: myapp://open/products/*
/// ```
#[derive(Debug, Clone, Default)]
pub struct CustomSchemeConfig {
	/// Configured custom schemes.
	pub schemes: Vec<CustomScheme>,
}

/// Individual custom URL scheme.
#[derive(Debug, Clone)]
pub struct CustomScheme {
	/// The scheme name (e.g., `myapp` for `myapp://`).
	pub name: String,

	/// Optional hosts that this scheme handles.
	pub hosts: Vec<String>,

	/// URL paths that this scheme handles.
	pub paths: Vec<String>,
}

impl CustomScheme {
	/// Generates a URL template for this scheme.
	///
	/// # Example
	///
	/// ```rust
	/// use reinhardt_deeplink::CustomScheme;
	///
	/// let scheme = CustomScheme {
	///     name: "myapp".to_string(),
	///     hosts: vec!["open".to_string()],
	///     paths: vec!["/products/*".to_string()],
	/// };
	///
	/// assert_eq!(scheme.url_template(), "myapp://open/products/*");
	/// ```
	pub fn url_template(&self) -> String {
		let host = self.hosts.first().map(String::as_str).unwrap_or("");
		let path = self.paths.first().map(String::as_str).unwrap_or("");
		format!("{}://{}{}", self.name, host, path)
	}
}

impl CustomSchemeConfig {
	/// Creates a new builder for custom scheme configuration.
	pub fn builder() -> CustomSchemeBuilder {
		CustomSchemeBuilder::new()
	}
}

/// Builder for custom URL scheme configuration.
#[derive(Debug, Default)]
pub struct CustomSchemeBuilder {
	name: Option<String>,
	hosts: Vec<String>,
	paths: Vec<String>,
}

impl CustomSchemeBuilder {
	/// Creates a new builder.
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the scheme name.
	///
	/// # Arguments
	///
	/// * `name` - The scheme name (e.g., `myapp` for `myapp://`)
	pub fn scheme(mut self, name: impl Into<String>) -> Self {
		self.name = Some(name.into());
		self
	}

	/// Adds a host for the scheme.
	///
	/// Custom URL schemes can optionally include a host component.
	pub fn host(mut self, host: impl Into<String>) -> Self {
		self.hosts.push(host.into());
		self
	}

	/// Adds multiple hosts.
	pub fn hosts(mut self, hosts: &[&str]) -> Self {
		self.hosts.extend(hosts.iter().map(|s| (*s).to_string()));
		self
	}

	/// Adds a path pattern.
	pub fn path(mut self, path: impl Into<String>) -> Self {
		self.paths.push(path.into());
		self
	}

	/// Adds multiple path patterns.
	pub fn paths(mut self, paths: &[&str]) -> Self {
		self.paths.extend(paths.iter().map(|s| (*s).to_string()));
		self
	}

	/// Builds the custom scheme configuration.
	pub fn build(self) -> CustomSchemeConfig {
		let scheme = CustomScheme {
			name: self.name.unwrap_or_default(),
			hosts: self.hosts,
			paths: self.paths,
		};

		CustomSchemeConfig {
			schemes: vec![scheme],
		}
	}
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	#[rstest]
	fn test_url_template_with_host_and_path() {
		let scheme = CustomScheme {
			name: "myapp".to_string(),
			hosts: vec!["open".to_string()],
			paths: vec!["/products/*".to_string()],
		};

		assert_eq!(scheme.url_template(), "myapp://open/products/*");
	}

	#[rstest]
	fn test_url_template_without_host() {
		let scheme = CustomScheme {
			name: "myapp".to_string(),
			hosts: Vec::new(),
			paths: vec!["/products".to_string()],
		};

		assert_eq!(scheme.url_template(), "myapp:///products");
	}

	#[rstest]
	fn test_url_template_without_path() {
		let scheme = CustomScheme {
			name: "myapp".to_string(),
			hosts: vec!["open".to_string()],
			paths: Vec::new(),
		};

		assert_eq!(scheme.url_template(), "myapp://open");
	}

	#[rstest]
	fn test_builder() {
		let config = CustomSchemeConfig::builder()
			.scheme("myapp")
			.host("open")
			.paths(&["/products/*", "/users/*"])
			.build();

		assert_eq!(config.schemes.len(), 1);
		assert_eq!(config.schemes[0].name, "myapp");
		assert_eq!(config.schemes[0].hosts, vec!["open"]);
		assert_eq!(config.schemes[0].paths, vec!["/products/*", "/users/*"]);
	}
}
