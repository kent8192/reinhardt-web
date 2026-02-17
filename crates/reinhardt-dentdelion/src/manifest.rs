//! Plugin manifest parser for dentdelion.toml.
//!
//! This module handles parsing and manipulation of the project's plugin
//! manifest file (dentdelion.toml).

use crate::error::{PluginError, PluginResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// The manifest filename.
pub const MANIFEST_FILENAME: &str = "dentdelion.toml";

/// Project plugin manifest (dentdelion.toml).
///
/// This is the main configuration file for plugins in a Reinhardt project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
	/// Dentdelion configuration section.
	pub dentdelion: DentdelionConfig,

	/// Installed plugins.
	#[serde(default)]
	pub plugins: Vec<InstalledPlugin>,

	/// Plugin-specific configuration.
	#[serde(default)]
	pub plugin_config: HashMap<String, toml::Table>,
}

impl ProjectManifest {
	/// Loads the manifest from a file.
	pub fn load(path: impl AsRef<Path>) -> PluginResult<Self> {
		let path = path.as_ref();
		let content = std::fs::read_to_string(path)
			.map_err(|e| PluginError::ManifestNotFound(e.to_string()))?;

		toml::from_str(&content).map_err(|e| PluginError::ManifestParseError(e.to_string()))
	}

	/// Loads the manifest from a project directory.
	pub fn load_from_project(project_root: impl AsRef<Path>) -> PluginResult<Self> {
		let manifest_path = project_root.as_ref().join(MANIFEST_FILENAME);
		Self::load(manifest_path)
	}

	/// Creates a default manifest.
	pub fn default_manifest() -> Self {
		Self {
			dentdelion: DentdelionConfig::default(),
			plugins: Vec::new(),
			plugin_config: HashMap::new(),
		}
	}

	/// Saves the manifest to a file.
	pub fn save(&self, path: impl AsRef<Path>) -> PluginResult<()> {
		let content = toml::to_string_pretty(self)
			.map_err(|e| PluginError::ManifestParseError(e.to_string()))?;
		std::fs::write(path, content)?;
		Ok(())
	}

	/// Saves the manifest to the project directory.
	pub fn save_to_project(&self, project_root: impl AsRef<Path>) -> PluginResult<()> {
		let manifest_path = project_root.as_ref().join(MANIFEST_FILENAME);
		self.save(manifest_path)
	}

	/// Gets an installed plugin by name.
	pub fn get_plugin(&self, name: &str) -> Option<&InstalledPlugin> {
		self.plugins.iter().find(|p| p.name == name)
	}

	/// Gets a mutable reference to an installed plugin by name.
	pub fn get_plugin_mut(&mut self, name: &str) -> Option<&mut InstalledPlugin> {
		self.plugins.iter_mut().find(|p| p.name == name)
	}

	/// Checks if a plugin is installed.
	pub fn is_installed(&self, name: &str) -> bool {
		self.plugins.iter().any(|p| p.name == name)
	}

	/// Adds a plugin to the manifest.
	pub fn add_plugin(&mut self, plugin: InstalledPlugin) {
		// Remove existing entry if present
		self.plugins.retain(|p| p.name != plugin.name);
		self.plugins.push(plugin);
	}

	/// Removes a plugin from the manifest.
	pub fn remove_plugin(&mut self, name: &str) -> Option<InstalledPlugin> {
		let idx = self.plugins.iter().position(|p| p.name == name)?;
		Some(self.plugins.remove(idx))
	}

	/// Gets plugin configuration.
	pub fn get_plugin_config(&self, name: &str) -> Option<&toml::Table> {
		self.plugin_config.get(name)
	}

	/// Sets plugin configuration.
	pub fn set_plugin_config(&mut self, name: impl Into<String>, config: toml::Table) {
		self.plugin_config.insert(name.into(), config);
	}

	/// Removes plugin configuration.
	pub fn remove_plugin_config(&mut self, name: &str) -> Option<toml::Table> {
		self.plugin_config.remove(name)
	}

	/// Returns all enabled plugins.
	pub fn enabled_plugins(&self) -> impl Iterator<Item = &InstalledPlugin> {
		self.plugins.iter().filter(|p| p.enabled)
	}

	/// Returns all disabled plugins.
	pub fn disabled_plugins(&self) -> impl Iterator<Item = &InstalledPlugin> {
		self.plugins.iter().filter(|p| !p.enabled)
	}

	/// Returns all static plugins.
	pub fn static_plugins(&self) -> impl Iterator<Item = &InstalledPlugin> {
		self.plugins
			.iter()
			.filter(|p| p.plugin_type == PluginType::Static)
	}

	/// Returns all WASM plugins.
	pub fn wasm_plugins(&self) -> impl Iterator<Item = &InstalledPlugin> {
		self.plugins
			.iter()
			.filter(|p| p.plugin_type == PluginType::Wasm)
	}
}

/// Dentdelion configuration section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DentdelionConfig {
	/// Manifest format version.
	#[serde(default = "default_format_version")]
	pub format_version: String,

	/// WASM plugins storage directory (relative to project root).
	#[serde(default = "default_wasm_dir")]
	pub wasm_dir: String,

	/// Framework version requirement.
	#[serde(default)]
	pub framework_version: Option<String>,
}

impl Default for DentdelionConfig {
	fn default() -> Self {
		Self {
			format_version: default_format_version(),
			wasm_dir: default_wasm_dir(),
			framework_version: None,
		}
	}
}

fn default_format_version() -> String {
	"1.0".to_string()
}

fn default_wasm_dir() -> String {
	".dentdelion/plugins".to_string()
}

/// Installed plugin entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
	/// Plugin name.
	pub name: String,

	/// Plugin type (static or wasm).
	#[serde(rename = "type")]
	pub plugin_type: PluginType,

	/// Plugin version.
	pub version: String,

	/// Whether the plugin is enabled.
	#[serde(default = "default_enabled")]
	pub enabled: bool,

	/// Plugin source (crates.io, github, local path).
	#[serde(default)]
	pub source: Option<String>,

	/// WASM-specific configuration.
	#[serde(default)]
	pub wasm: Option<WasmPluginConfig>,
}

fn default_enabled() -> bool {
	true
}

impl InstalledPlugin {
	/// Creates a new static plugin entry.
	pub fn new_static(name: impl Into<String>, version: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			plugin_type: PluginType::Static,
			version: version.into(),
			enabled: true,
			source: Some("crates.io".to_string()),
			wasm: None,
		}
	}

	/// Creates a new WASM plugin entry.
	pub fn new_wasm(name: impl Into<String>, version: impl Into<String>) -> Self {
		Self {
			name: name.into(),
			plugin_type: PluginType::Wasm,
			version: version.into(),
			enabled: true,
			source: Some("crates.io".to_string()),
			wasm: Some(WasmPluginConfig::default()),
		}
	}

	/// Sets the source.
	pub fn with_source(mut self, source: impl Into<String>) -> Self {
		self.source = Some(source.into());
		self
	}

	/// Sets the enabled state.
	pub fn with_enabled(mut self, enabled: bool) -> Self {
		self.enabled = enabled;
		self
	}

	/// Sets WASM configuration.
	pub fn with_wasm_config(mut self, config: WasmPluginConfig) -> Self {
		self.wasm = Some(config);
		self
	}
}

/// Plugin type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
	/// Static plugin (Rust crate).
	Static,
	/// Dynamic plugin (WASM).
	Wasm,
}

impl std::fmt::Display for PluginType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Static => write!(f, "static"),
			Self::Wasm => write!(f, "wasm"),
		}
	}
}

/// WASM plugin configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPluginConfig {
	/// Memory limit in megabytes.
	#[serde(default = "default_memory_limit")]
	pub memory_limit_mb: u32,

	/// Capabilities granted to this plugin.
	#[serde(default)]
	pub capabilities: Vec<String>,

	/// Execution timeout in seconds.
	#[serde(default = "default_timeout")]
	pub timeout_secs: u32,
}

fn default_memory_limit() -> u32 {
	128
}

fn default_timeout() -> u32 {
	30
}

impl Default for WasmPluginConfig {
	fn default() -> Self {
		Self {
			memory_limit_mb: default_memory_limit(),
			capabilities: Vec::new(),
			timeout_secs: default_timeout(),
		}
	}
}

impl WasmPluginConfig {
	/// Sets the memory limit.
	pub fn with_memory_limit(mut self, limit_mb: u32) -> Self {
		self.memory_limit_mb = limit_mb;
		self
	}

	/// Adds a capability.
	pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
		self.capabilities.push(capability.into());
		self
	}

	/// Sets the timeout.
	pub fn with_timeout(mut self, timeout_secs: u32) -> Self {
		self.timeout_secs = timeout_secs;
		self
	}
}

/// Plugin author manifest (dentdelion-plugin.toml).
///
/// This manifest is included in plugin packages to describe the plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
	/// Plugin metadata.
	pub plugin: PluginManifestInfo,

	/// Compatibility requirements.
	#[serde(default)]
	pub compatibility: CompatibilityInfo,

	/// Plugin dependencies.
	#[serde(default)]
	pub dependencies: HashMap<String, String>,

	/// Hooks provided by this plugin.
	#[serde(default)]
	pub hooks: HooksInfo,

	/// Configuration schema.
	#[serde(default)]
	pub config_schema: Option<ConfigSchema>,

	/// WASM-specific configuration (for WASM plugins).
	#[serde(default)]
	pub wasm: Option<WasmManifestConfig>,
}

/// Plugin manifest information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifestInfo {
	/// Plugin name.
	pub name: String,

	/// Display name.
	#[serde(default)]
	pub display_name: Option<String>,

	/// Plugin description.
	#[serde(default)]
	pub description: String,

	/// Plugin version.
	pub version: String,

	/// Plugin type.
	#[serde(rename = "type")]
	pub plugin_type: PluginType,

	/// License.
	#[serde(default)]
	pub license: String,

	/// Authors.
	#[serde(default)]
	pub authors: Vec<String>,

	/// Repository URL.
	#[serde(default)]
	pub repository: Option<String>,

	/// Keywords.
	#[serde(default)]
	pub keywords: Vec<String>,
}

/// Compatibility requirements.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompatibilityInfo {
	/// Minimum framework version.
	#[serde(default)]
	pub framework_version: Option<String>,

	/// Minimum Rust version.
	#[serde(default)]
	pub rust_version: Option<String>,
}

/// Hooks information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HooksInfo {
	/// Capabilities provided by this plugin.
	#[serde(default)]
	pub provides: Vec<String>,

	/// Lifecycle hooks implemented.
	#[serde(default)]
	pub lifecycle: Vec<String>,
}

/// Configuration schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSchema {
	/// Required configuration keys.
	#[serde(default)]
	pub required: Vec<String>,

	/// Field definitions.
	#[serde(default)]
	pub fields: HashMap<String, ConfigFieldSchema>,
}

/// Configuration field schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFieldSchema {
	/// Field type.
	#[serde(rename = "type")]
	pub field_type: String,

	/// Field description.
	#[serde(default)]
	pub description: Option<String>,

	/// Default value.
	#[serde(default)]
	pub default: Option<toml::Value>,

	/// Allowed values (enum).
	#[serde(rename = "enum", default)]
	pub allowed_values: Option<Vec<String>>,

	/// Minimum value (for numbers).
	#[serde(default)]
	pub min: Option<i64>,

	/// Maximum value (for numbers).
	#[serde(default)]
	pub max: Option<i64>,
}

/// WASM manifest configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmManifestConfig {
	/// Required capabilities.
	#[serde(default)]
	pub required_capabilities: Vec<String>,

	/// Default memory limit in MB.
	#[serde(default = "default_memory_limit")]
	pub default_memory_mb: u32,

	/// Maximum memory limit in MB.
	#[serde(default)]
	pub max_memory_mb: Option<u32>,

	/// Exported functions.
	#[serde(default)]
	pub exported_functions: Vec<String>,
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_parse_project_manifest() {
		let toml_content = r#"
[dentdelion]
format_version = "1.0"
wasm_dir = ".dentdelion/plugins"

[[plugins]]
name = "auth-delion"
type = "static"
version = "0.2.1"
enabled = true

[[plugins]]
name = "rate-limit-delion"
type = "wasm"
version = "1.0.0"
enabled = true

[plugins.wasm]
memory_limit_mb = 256
capabilities = ["http_request", "logging"]

[plugin_config.auth-delion]
algorithm = "HS256"
token_expiry_hours = 24
"#;

		let manifest: ProjectManifest = toml::from_str(toml_content).unwrap();

		assert_eq!(manifest.dentdelion.format_version, "1.0");
		assert_eq!(manifest.plugins.len(), 2);

		let auth = manifest.get_plugin("auth-delion").unwrap();
		assert_eq!(auth.plugin_type, PluginType::Static);
		assert!(auth.enabled);

		let config = manifest.get_plugin_config("auth-delion").unwrap();
		assert_eq!(config.get("algorithm").unwrap().as_str().unwrap(), "HS256");
	}

	#[rstest]
	fn test_create_manifest() {
		let mut manifest = ProjectManifest::default_manifest();

		manifest.add_plugin(InstalledPlugin::new_static("auth-delion", "1.0.0"));
		manifest.add_plugin(
			InstalledPlugin::new_wasm("rate-limit-delion", "2.0.0")
				.with_wasm_config(WasmPluginConfig::default().with_capability("logging")),
		);

		assert_eq!(manifest.plugins.len(), 2);
		assert!(manifest.is_installed("auth-delion"));
		assert!(manifest.is_installed("rate-limit-delion"));
	}

	#[rstest]
	fn test_enabled_disabled_plugins() {
		let mut manifest = ProjectManifest::default_manifest();

		manifest.add_plugin(InstalledPlugin::new_static("a-delion", "1.0.0").with_enabled(true));
		manifest.add_plugin(InstalledPlugin::new_static("b-delion", "1.0.0").with_enabled(false));
		manifest.add_plugin(InstalledPlugin::new_static("c-delion", "1.0.0").with_enabled(true));

		let enabled: Vec<_> = manifest.enabled_plugins().collect();
		let disabled: Vec<_> = manifest.disabled_plugins().collect();

		assert_eq!(enabled.len(), 2);
		assert_eq!(disabled.len(), 1);
		assert_eq!(disabled[0].name, "b-delion");
	}
}
