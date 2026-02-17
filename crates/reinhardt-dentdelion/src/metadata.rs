//! Plugin metadata and dependency definitions.
//!
//! This module defines the metadata structures that describe plugins,
//! including their identity, version, dependencies, and capabilities.

use crate::capability::Capability;
use crate::error::{PluginError, PluginResult};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

/// Plugin metadata containing identification and dependency information.
///
/// This structure follows the pattern established by Cargo's package metadata,
/// adapted for the Dentdelion plugin system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
	/// Unique plugin identifier (e.g., "auth-delion").
	///
	/// Plugin names should follow the naming convention: `xxx-delion`
	/// where xxx describes the plugin's functionality.
	pub name: String,

	/// Plugin version following Semantic Versioning 2.0.0.
	#[serde(with = "version_serde")]
	pub version: Version,

	/// Human-readable description of the plugin.
	#[serde(default)]
	pub description: String,

	/// Plugin author(s).
	#[serde(default)]
	pub authors: Vec<String>,

	/// License identifier (SPDX format).
	#[serde(default)]
	pub license: String,

	/// Repository URL.
	#[serde(default)]
	pub repository: Option<String>,

	/// Homepage URL.
	#[serde(default)]
	pub homepage: Option<String>,

	/// Plugin dependencies on other plugins.
	#[serde(default)]
	pub dependencies: Vec<PluginDependency>,

	/// Capabilities provided by this plugin.
	#[serde(default)]
	pub provides: Vec<Capability>,

	/// Capabilities required from other plugins or the framework.
	#[serde(default)]
	pub requires: Vec<Capability>,

	/// Plugin keywords for search/discovery.
	#[serde(default)]
	pub keywords: Vec<String>,

	/// Plugin categories.
	#[serde(default)]
	pub categories: Vec<String>,
}

impl PluginMetadata {
	/// Creates a new PluginMetadataBuilder.
	///
	/// # Arguments
	///
	/// * `name` - Plugin name (should follow xxx-delion convention)
	/// * `version` - Plugin version string (semver format)
	///
	/// # Example
	///
	/// ```ignore
	/// use reinhardt_dentdelion::metadata::PluginMetadata;
	///
	/// let metadata = PluginMetadata::builder("auth-delion", "1.0.0")
	///     .description("JWT authentication plugin")
	///     .author("Plugin Author")
	///     .build()
	///     .unwrap();
	/// ```
	pub fn builder(name: impl Into<String>, version: impl AsRef<str>) -> PluginMetadataBuilder {
		PluginMetadataBuilder::new(name, version)
	}

	/// Validates the metadata.
	pub fn validate(&self) -> PluginResult<()> {
		// Validate name format (recommended: xxx-delion)
		if self.name.is_empty() {
			return Err(PluginError::InvalidManifest(
				"plugin name cannot be empty".to_string(),
			));
		}

		// Warn if not following naming convention (but don't error)
		if !self.name.ends_with("-delion") {
			tracing::warn!(
				"plugin '{}' does not follow the recommended naming convention (xxx-delion)",
				self.name
			);
		}

		Ok(())
	}

	/// Returns the qualified name (name@version).
	pub fn qualified_name(&self) -> String {
		format!("{}@{}", self.name, self.version)
	}
}

/// Plugin dependency specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
	/// Name of the required plugin.
	pub name: String,

	/// Semver version requirement (e.g., "^1.0.0", ">=2.0, <3.0").
	#[serde(with = "version_req_serde")]
	pub version_req: VersionReq,

	/// Whether this dependency is optional.
	#[serde(default)]
	pub optional: bool,

	/// Specific capabilities required from this dependency.
	#[serde(default)]
	pub required_capabilities: Vec<Capability>,
}

impl PluginDependency {
	/// Creates a new required dependency.
	pub fn new(name: impl Into<String>, version_req: impl AsRef<str>) -> PluginResult<Self> {
		Ok(Self {
			name: name.into(),
			version_req: VersionReq::parse(version_req.as_ref())
				.map_err(|e| PluginError::InvalidVersionReq(e.to_string()))?,
			optional: false,
			required_capabilities: Vec::new(),
		})
	}

	/// Creates a new optional dependency.
	pub fn optional(name: impl Into<String>, version_req: impl AsRef<str>) -> PluginResult<Self> {
		let mut dep = Self::new(name, version_req)?;
		dep.optional = true;
		Ok(dep)
	}

	/// Adds a required capability to this dependency.
	pub fn with_capability(mut self, capability: impl Into<Capability>) -> Self {
		self.required_capabilities.push(capability.into());
		self
	}

	/// Checks if a version satisfies this dependency.
	pub fn matches(&self, version: &Version) -> bool {
		self.version_req.matches(version)
	}
}

/// Builder pattern for PluginMetadata.
///
/// Matches Reinhardt's existing builder patterns used throughout the framework.
pub struct PluginMetadataBuilder {
	name: String,
	version: String,
	description: String,
	authors: Vec<String>,
	license: String,
	repository: Option<String>,
	homepage: Option<String>,
	dependencies: Vec<PluginDependency>,
	provides: Vec<Capability>,
	requires: Vec<Capability>,
	keywords: Vec<String>,
	categories: Vec<String>,
}

impl PluginMetadataBuilder {
	/// Creates a new builder with required fields.
	pub fn new(name: impl Into<String>, version: impl AsRef<str>) -> Self {
		Self {
			name: name.into(),
			version: version.as_ref().to_string(),
			description: String::new(),
			authors: Vec::new(),
			license: String::new(),
			repository: None,
			homepage: None,
			dependencies: Vec::new(),
			provides: Vec::new(),
			requires: Vec::new(),
			keywords: Vec::new(),
			categories: Vec::new(),
		}
	}

	/// Sets the plugin description.
	pub fn description(mut self, desc: impl Into<String>) -> Self {
		self.description = desc.into();
		self
	}

	/// Adds an author.
	pub fn author(mut self, author: impl Into<String>) -> Self {
		self.authors.push(author.into());
		self
	}

	/// Sets the license.
	pub fn license(mut self, license: impl Into<String>) -> Self {
		self.license = license.into();
		self
	}

	/// Sets the repository URL.
	pub fn repository(mut self, repo: impl Into<String>) -> Self {
		self.repository = Some(repo.into());
		self
	}

	/// Sets the homepage URL.
	pub fn homepage(mut self, homepage: impl Into<String>) -> Self {
		self.homepage = Some(homepage.into());
		self
	}

	/// Adds a capability that this plugin provides.
	pub fn provides(mut self, capability: impl Into<Capability>) -> Self {
		self.provides.push(capability.into());
		self
	}

	/// Adds a capability that this plugin requires.
	pub fn requires(mut self, capability: impl Into<Capability>) -> Self {
		self.requires.push(capability.into());
		self
	}

	/// Adds a dependency on another plugin.
	pub fn depends_on(mut self, name: impl Into<String>, version_req: impl AsRef<str>) -> Self {
		if let Ok(dep) = PluginDependency::new(name, version_req) {
			self.dependencies.push(dep);
		}
		self
	}

	/// Adds an optional dependency on another plugin.
	pub fn optionally_depends_on(
		mut self,
		name: impl Into<String>,
		version_req: impl AsRef<str>,
	) -> Self {
		if let Ok(dep) = PluginDependency::optional(name, version_req) {
			self.dependencies.push(dep);
		}
		self
	}

	/// Adds a keyword for search/discovery.
	pub fn keyword(mut self, keyword: impl Into<String>) -> Self {
		self.keywords.push(keyword.into());
		self
	}

	/// Adds a category.
	pub fn category(mut self, category: impl Into<String>) -> Self {
		self.categories.push(category.into());
		self
	}

	/// Builds the PluginMetadata.
	///
	/// # Errors
	///
	/// Returns an error if the version string is invalid.
	pub fn build(self) -> PluginResult<PluginMetadata> {
		let version = Version::parse(&self.version)
			.map_err(|e| PluginError::InvalidVersion(e.to_string()))?;

		let metadata = PluginMetadata {
			name: self.name,
			version,
			description: self.description,
			authors: self.authors,
			license: self.license,
			repository: self.repository,
			homepage: self.homepage,
			dependencies: self.dependencies,
			provides: self.provides,
			requires: self.requires,
			keywords: self.keywords,
			categories: self.categories,
		};

		metadata.validate()?;
		Ok(metadata)
	}
}

/// Serde support for semver::Version.
mod version_serde {
	use semver::Version;
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	pub(super) fn serialize<S>(version: &Version, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		version.to_string().serialize(serializer)
	}

	pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Version, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		Version::parse(&s).map_err(serde::de::Error::custom)
	}
}

/// Serde support for semver::VersionReq.
mod version_req_serde {
	use semver::VersionReq;
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	pub(super) fn serialize<S>(version_req: &VersionReq, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		version_req.to_string().serialize(serializer)
	}

	pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<VersionReq, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		VersionReq::parse(&s).map_err(serde::de::Error::custom)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::capability::PluginCapability;
	use rstest::rstest;

	#[rstest]
	fn test_metadata_builder() {
		let metadata = PluginMetadata::builder("auth-delion", "1.0.0")
			.description("JWT authentication plugin")
			.author("Test Author")
			.license("MIT")
			.provides(PluginCapability::Auth)
			.provides(PluginCapability::Middleware)
			.depends_on("core-delion", "^0.1.0")
			.build()
			.unwrap();

		assert_eq!(metadata.name, "auth-delion");
		assert_eq!(metadata.version.to_string(), "1.0.0");
		assert_eq!(metadata.description, "JWT authentication plugin");
		assert_eq!(metadata.authors, vec!["Test Author"]);
		assert_eq!(metadata.license, "MIT");
		assert_eq!(metadata.provides.len(), 2);
		assert_eq!(metadata.dependencies.len(), 1);
	}

	#[rstest]
	fn test_dependency_matches() {
		let dep = PluginDependency::new("test-delion", "^1.0.0").unwrap();

		assert!(dep.matches(&Version::parse("1.0.0").unwrap()));
		assert!(dep.matches(&Version::parse("1.5.0").unwrap()));
		assert!(!dep.matches(&Version::parse("2.0.0").unwrap()));
		assert!(!dep.matches(&Version::parse("0.9.0").unwrap()));
	}

	#[rstest]
	fn test_qualified_name() {
		let metadata = PluginMetadata::builder("test-delion", "2.1.0")
			.build()
			.unwrap();
		assert_eq!(metadata.qualified_name(), "test-delion@2.1.0");
	}
}
