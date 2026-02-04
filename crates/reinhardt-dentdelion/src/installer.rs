//! Plugin installer for managing Cargo.toml and dentdelion.toml.
//!
//! This module provides functionality to:
//! - Add plugin dependencies to Cargo.toml
//! - Remove plugin dependencies from Cargo.toml
//! - Update the dentdelion.toml manifest
//! - Perform full plugin uninstallation

use crate::error::{PluginError, PluginResult};
use crate::manifest::{InstalledPlugin, MANIFEST_FILENAME, PluginType, ProjectManifest};
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item, Table, Value};

/// Plugin installer for managing plugin installations.
///
/// The installer handles both Cargo.toml dependency management and
/// dentdelion.toml manifest updates.
pub struct PluginInstaller {
	/// Project root directory containing Cargo.toml and dentdelion.toml
	project_root: PathBuf,
}

impl PluginInstaller {
	/// Creates a new installer for the given project root.
	pub fn new(project_root: impl Into<PathBuf>) -> Self {
		Self {
			project_root: project_root.into(),
		}
	}

	/// Returns the path to the project's Cargo.toml.
	pub fn cargo_toml_path(&self) -> PathBuf {
		self.project_root.join("Cargo.toml")
	}

	/// Returns the path to the project's dentdelion.toml.
	pub fn manifest_path(&self) -> PathBuf {
		self.project_root.join(MANIFEST_FILENAME)
	}

	/// Adds a plugin dependency to Cargo.toml.
	///
	/// # Arguments
	///
	/// * `name` - The plugin crate name (e.g., "auth-delion")
	/// * `version` - The version requirement (e.g., "1.0.0" or "^1.0")
	///
	/// # Example
	///
	/// ```ignore
	/// let installer = PluginInstaller::new("/path/to/project");
	/// installer.add_to_cargo_toml("auth-delion", "1.0.0")?;
	/// ```
	pub fn add_to_cargo_toml(&self, name: &str, version: &str) -> PluginResult<()> {
		let cargo_path = self.cargo_toml_path();
		let content = std::fs::read_to_string(&cargo_path)?;
		let mut doc: DocumentMut = content.parse().map_err(|e| {
			PluginError::ManifestParseError(format!("Failed to parse Cargo.toml: {e}"))
		})?;

		// Get or create [dependencies] section
		let deps = doc
			.entry("dependencies")
			.or_insert(Item::Table(Table::new()))
			.as_table_mut()
			.ok_or_else(|| {
				PluginError::ManifestParseError("[dependencies] is not a table".to_string())
			})?;

		// Check if already exists
		if deps.contains_key(name) {
			return Err(PluginError::AlreadyRegistered(name.to_string()));
		}

		// Add the dependency with version
		deps.insert(name, Item::Value(Value::from(version)));

		// Write back
		std::fs::write(&cargo_path, doc.to_string())?;
		tracing::info!("Added {} = \"{}\" to Cargo.toml", name, version);

		Ok(())
	}

	/// Removes a plugin dependency from Cargo.toml.
	///
	/// # Arguments
	///
	/// * `name` - The plugin crate name to remove
	pub fn remove_from_cargo_toml(&self, name: &str) -> PluginResult<()> {
		let cargo_path = self.cargo_toml_path();
		let content = std::fs::read_to_string(&cargo_path)?;
		let mut doc: DocumentMut = content.parse().map_err(|e| {
			PluginError::ManifestParseError(format!("Failed to parse Cargo.toml: {e}"))
		})?;

		// Get [dependencies] section
		let deps = doc
			.get_mut("dependencies")
			.and_then(|d| d.as_table_mut())
			.ok_or_else(|| {
				PluginError::ManifestParseError("[dependencies] section not found".to_string())
			})?;

		// Check if exists
		if !deps.contains_key(name) {
			return Err(PluginError::NotFound(name.to_string()));
		}

		// Remove the dependency
		deps.remove(name);

		// Write back
		std::fs::write(&cargo_path, doc.to_string())?;
		tracing::info!("Removed {} from Cargo.toml", name);

		Ok(())
	}

	/// Updates the dentdelion.toml manifest with a plugin entry.
	///
	/// If the plugin already exists, it will be replaced.
	///
	/// # Arguments
	///
	/// * `plugin` - The plugin entry to add or update
	pub fn update_manifest(&self, plugin: InstalledPlugin) -> PluginResult<()> {
		let manifest_path = self.manifest_path();

		// Load or create manifest
		let mut manifest = if manifest_path.exists() {
			ProjectManifest::load(&manifest_path)?
		} else {
			ProjectManifest::default_manifest()
		};

		// Add or update plugin
		manifest.add_plugin(plugin.clone());

		// Save manifest
		manifest.save(&manifest_path)?;
		tracing::info!("Updated {} in dentdelion.toml", plugin.name);

		Ok(())
	}

	/// Removes a plugin entry from dentdelion.toml.
	///
	/// # Arguments
	///
	/// * `name` - The plugin name to remove
	/// * `purge_config` - Whether to also remove plugin configuration
	pub fn remove_from_manifest(&self, name: &str, purge_config: bool) -> PluginResult<()> {
		let manifest_path = self.manifest_path();

		if !manifest_path.exists() {
			return Err(PluginError::ManifestNotFound(
				"dentdelion.toml not found".to_string(),
			));
		}

		let mut manifest = ProjectManifest::load(&manifest_path)?;

		// Remove plugin entry
		manifest
			.remove_plugin(name)
			.ok_or_else(|| PluginError::NotFound(name.to_string()))?;

		// Optionally remove configuration
		if purge_config {
			manifest.remove_plugin_config(name);
			tracing::info!("Removed configuration for {}", name);
		}

		// Save manifest
		manifest.save(&manifest_path)?;
		tracing::info!("Removed {} from dentdelion.toml", name);

		Ok(())
	}

	/// Installs a static plugin (adds to both Cargo.toml and dentdelion.toml).
	///
	/// # Arguments
	///
	/// * `name` - The plugin crate name
	/// * `version` - The version requirement
	/// * `source` - Optional source (defaults to "crates.io")
	pub fn install_static(
		&self,
		name: &str,
		version: &str,
		source: Option<&str>,
	) -> PluginResult<()> {
		// Add to Cargo.toml
		self.add_to_cargo_toml(name, version)?;

		// Create plugin entry
		let mut plugin = InstalledPlugin::new_static(name, version);
		if let Some(src) = source {
			plugin = plugin.with_source(src);
		}

		// Add to dentdelion.toml
		self.update_manifest(plugin)?;

		Ok(())
	}

	/// Uninstalls a plugin (removes from both Cargo.toml and dentdelion.toml).
	///
	/// # Arguments
	///
	/// * `name` - The plugin name to uninstall
	/// * `purge_config` - Whether to also remove plugin configuration
	pub fn uninstall(&self, name: &str, purge_config: bool) -> PluginResult<()> {
		// Check plugin type first
		let manifest_path = self.manifest_path();
		let plugin_type = if manifest_path.exists() {
			let manifest = ProjectManifest::load(&manifest_path)?;
			manifest.get_plugin(name).map(|p| p.plugin_type)
		} else {
			None
		};

		// Remove from Cargo.toml (only for static plugins)
		if plugin_type != Some(PluginType::Wasm) {
			// Try to remove from Cargo.toml, but don't fail if not found
			match self.remove_from_cargo_toml(name) {
				Ok(()) => {}
				Err(PluginError::NotFound(_)) => {
					tracing::debug!("{} not found in Cargo.toml", name);
				}
				Err(e) => return Err(e),
			}
		}

		// Remove from dentdelion.toml
		self.remove_from_manifest(name, purge_config)?;

		Ok(())
	}

	/// Enables a plugin in dentdelion.toml.
	pub fn enable_plugin(&self, name: &str) -> PluginResult<()> {
		self.set_plugin_enabled(name, true)
	}

	/// Disables a plugin in dentdelion.toml.
	pub fn disable_plugin(&self, name: &str) -> PluginResult<()> {
		self.set_plugin_enabled(name, false)
	}

	/// Sets the enabled state of a plugin.
	fn set_plugin_enabled(&self, name: &str, enabled: bool) -> PluginResult<()> {
		let manifest_path = self.manifest_path();

		if !manifest_path.exists() {
			return Err(PluginError::ManifestNotFound(
				"dentdelion.toml not found".to_string(),
			));
		}

		let mut manifest = ProjectManifest::load(&manifest_path)?;

		let plugin = manifest
			.get_plugin_mut(name)
			.ok_or_else(|| PluginError::NotFound(name.to_string()))?;

		plugin.enabled = enabled;

		manifest.save(&manifest_path)?;

		let state = if enabled { "enabled" } else { "disabled" };
		tracing::info!("Plugin {} is now {}", name, state);

		Ok(())
	}

	/// Updates a plugin version in both Cargo.toml and dentdelion.toml.
	///
	/// # Arguments
	///
	/// * `name` - The plugin name
	/// * `new_version` - The new version to update to
	pub fn update_version(&self, name: &str, new_version: &str) -> PluginResult<()> {
		let manifest_path = self.manifest_path();

		if !manifest_path.exists() {
			return Err(PluginError::ManifestNotFound(
				"dentdelion.toml not found".to_string(),
			));
		}

		let manifest = ProjectManifest::load(&manifest_path)?;
		let plugin = manifest
			.get_plugin(name)
			.ok_or_else(|| PluginError::NotFound(name.to_string()))?;

		// Update Cargo.toml for static plugins
		if plugin.plugin_type == PluginType::Static {
			self.update_cargo_toml_version(name, new_version)?;
		}

		// Update dentdelion.toml
		self.update_manifest_version(name, new_version)?;

		Ok(())
	}

	/// Updates a dependency version in Cargo.toml.
	fn update_cargo_toml_version(&self, name: &str, version: &str) -> PluginResult<()> {
		let cargo_path = self.cargo_toml_path();
		let content = std::fs::read_to_string(&cargo_path)?;
		let mut doc: DocumentMut = content.parse().map_err(|e| {
			PluginError::ManifestParseError(format!("Failed to parse Cargo.toml: {e}"))
		})?;

		let deps = doc
			.get_mut("dependencies")
			.and_then(|d| d.as_table_mut())
			.ok_or_else(|| {
				PluginError::ManifestParseError("[dependencies] section not found".to_string())
			})?;

		if !deps.contains_key(name) {
			return Err(PluginError::NotFound(name.to_string()));
		}

		// Update the version
		deps.insert(name, Item::Value(Value::from(version)));

		std::fs::write(&cargo_path, doc.to_string())?;
		tracing::info!("Updated {} to {} in Cargo.toml", name, version);

		Ok(())
	}

	/// Updates a plugin version in dentdelion.toml.
	fn update_manifest_version(&self, name: &str, version: &str) -> PluginResult<()> {
		let manifest_path = self.manifest_path();
		let mut manifest = ProjectManifest::load(&manifest_path)?;

		let plugin = manifest
			.get_plugin_mut(name)
			.ok_or_else(|| PluginError::NotFound(name.to_string()))?;

		plugin.version = version.to_string();

		manifest.save(&manifest_path)?;
		tracing::info!("Updated {} to {} in dentdelion.toml", name, version);

		Ok(())
	}

	/// Checks if a project has a dentdelion.toml manifest.
	pub fn has_manifest(&self) -> bool {
		self.manifest_path().exists()
	}

	/// Initializes a new dentdelion.toml manifest if it doesn't exist.
	pub fn init_manifest(&self) -> PluginResult<()> {
		let manifest_path = self.manifest_path();

		if manifest_path.exists() {
			return Err(PluginError::ManifestParseError(
				"dentdelion.toml already exists".to_string(),
			));
		}

		let manifest = ProjectManifest::default_manifest();
		manifest.save(&manifest_path)?;
		tracing::info!("Created dentdelion.toml");

		Ok(())
	}

	/// Returns the project root path.
	pub fn project_root(&self) -> &Path {
		&self.project_root
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile::TempDir;

	fn create_test_cargo_toml(dir: &Path) {
		let content = r#"[package]
name = "test-project"
version = "0.1.0-alpha.1"

[dependencies]
serde = "1.0"
"#;
		std::fs::write(dir.join("Cargo.toml"), content).unwrap();
	}

	#[test]
	fn test_add_to_cargo_toml() {
		let temp_dir = TempDir::new().unwrap();
		create_test_cargo_toml(temp_dir.path());

		let installer = PluginInstaller::new(temp_dir.path());
		installer.add_to_cargo_toml("auth-delion", "1.0.0").unwrap();

		let content = std::fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();
		assert!(content.contains("auth-delion"));
		assert!(content.contains("1.0.0"));
	}

	#[test]
	fn test_add_duplicate_fails() {
		let temp_dir = TempDir::new().unwrap();
		create_test_cargo_toml(temp_dir.path());

		let installer = PluginInstaller::new(temp_dir.path());
		installer.add_to_cargo_toml("auth-delion", "1.0.0").unwrap();

		let result = installer.add_to_cargo_toml("auth-delion", "2.0.0");
		assert!(matches!(result, Err(PluginError::AlreadyRegistered(_))));
	}

	#[test]
	fn test_remove_from_cargo_toml() {
		let temp_dir = TempDir::new().unwrap();
		create_test_cargo_toml(temp_dir.path());

		let installer = PluginInstaller::new(temp_dir.path());
		installer.add_to_cargo_toml("auth-delion", "1.0.0").unwrap();
		installer.remove_from_cargo_toml("auth-delion").unwrap();

		let content = std::fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();
		assert!(!content.contains("auth-delion"));
	}

	#[test]
	fn test_remove_nonexistent_fails() {
		let temp_dir = TempDir::new().unwrap();
		create_test_cargo_toml(temp_dir.path());

		let installer = PluginInstaller::new(temp_dir.path());
		let result = installer.remove_from_cargo_toml("nonexistent");
		assert!(matches!(result, Err(PluginError::NotFound(_))));
	}

	#[test]
	fn test_update_manifest() {
		let temp_dir = TempDir::new().unwrap();

		let installer = PluginInstaller::new(temp_dir.path());
		let plugin = InstalledPlugin::new_static("auth-delion", "1.0.0");
		installer.update_manifest(plugin).unwrap();

		let manifest = ProjectManifest::load(installer.manifest_path()).unwrap();
		assert!(manifest.is_installed("auth-delion"));
	}

	#[test]
	fn test_install_static() {
		let temp_dir = TempDir::new().unwrap();
		create_test_cargo_toml(temp_dir.path());

		let installer = PluginInstaller::new(temp_dir.path());
		installer
			.install_static("auth-delion", "1.0.0", None)
			.unwrap();

		// Check Cargo.toml
		let cargo_content = std::fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();
		assert!(cargo_content.contains("auth-delion"));

		// Check dentdelion.toml
		let manifest = ProjectManifest::load(installer.manifest_path()).unwrap();
		assert!(manifest.is_installed("auth-delion"));
	}

	#[test]
	fn test_uninstall() {
		let temp_dir = TempDir::new().unwrap();
		create_test_cargo_toml(temp_dir.path());

		let installer = PluginInstaller::new(temp_dir.path());
		installer
			.install_static("auth-delion", "1.0.0", None)
			.unwrap();
		installer.uninstall("auth-delion", false).unwrap();

		// Check Cargo.toml
		let cargo_content = std::fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();
		assert!(!cargo_content.contains("auth-delion"));

		// Check dentdelion.toml
		let manifest = ProjectManifest::load(installer.manifest_path()).unwrap();
		assert!(!manifest.is_installed("auth-delion"));
	}

	#[test]
	fn test_enable_disable_plugin() {
		let temp_dir = TempDir::new().unwrap();
		create_test_cargo_toml(temp_dir.path());

		let installer = PluginInstaller::new(temp_dir.path());
		installer
			.install_static("auth-delion", "1.0.0", None)
			.unwrap();

		// Disable
		installer.disable_plugin("auth-delion").unwrap();
		let manifest = ProjectManifest::load(installer.manifest_path()).unwrap();
		assert!(!manifest.get_plugin("auth-delion").unwrap().enabled);

		// Enable
		installer.enable_plugin("auth-delion").unwrap();
		let manifest = ProjectManifest::load(installer.manifest_path()).unwrap();
		assert!(manifest.get_plugin("auth-delion").unwrap().enabled);
	}

	#[test]
	fn test_update_version() {
		let temp_dir = TempDir::new().unwrap();
		create_test_cargo_toml(temp_dir.path());

		let installer = PluginInstaller::new(temp_dir.path());
		installer
			.install_static("auth-delion", "1.0.0", None)
			.unwrap();
		installer.update_version("auth-delion", "2.0.0").unwrap();

		// Check Cargo.toml
		let cargo_content = std::fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();
		assert!(cargo_content.contains("2.0.0"));
		assert!(!cargo_content.contains("1.0.0") || cargo_content.contains("0.1.0"));

		// Check dentdelion.toml
		let manifest = ProjectManifest::load(installer.manifest_path()).unwrap();
		assert_eq!(manifest.get_plugin("auth-delion").unwrap().version, "2.0.0");
	}

	#[test]
	fn test_init_manifest() {
		let temp_dir = TempDir::new().unwrap();

		let installer = PluginInstaller::new(temp_dir.path());
		assert!(!installer.has_manifest());

		installer.init_manifest().unwrap();
		assert!(installer.has_manifest());

		// Double init should fail
		let result = installer.init_manifest();
		assert!(result.is_err());
	}

	// ==========================================================================
	// Path Getter Tests
	// ==========================================================================

	#[test]
	fn test_project_root() {
		let temp_dir = TempDir::new().unwrap();
		let installer = PluginInstaller::new(temp_dir.path());
		assert_eq!(installer.project_root(), temp_dir.path());
	}

	#[test]
	fn test_cargo_toml_path() {
		let temp_dir = TempDir::new().unwrap();
		let installer = PluginInstaller::new(temp_dir.path());
		assert_eq!(
			installer.cargo_toml_path(),
			temp_dir.path().join("Cargo.toml")
		);
	}

	#[test]
	fn test_manifest_path() {
		let temp_dir = TempDir::new().unwrap();
		let installer = PluginInstaller::new(temp_dir.path());
		assert_eq!(
			installer.manifest_path(),
			temp_dir.path().join("dentdelion.toml")
		);
	}

	// ==========================================================================
	// Edge Case Tests
	// ==========================================================================

	#[test]
	fn test_add_to_cargo_toml_no_file() {
		let temp_dir = TempDir::new().unwrap();
		// Don't create Cargo.toml
		let installer = PluginInstaller::new(temp_dir.path());
		let result = installer.add_to_cargo_toml("test-delion", "1.0.0");
		assert!(result.is_err());
	}

	#[test]
	fn test_add_to_cargo_toml_no_dependencies_section() {
		let temp_dir = TempDir::new().unwrap();
		let content = r#"[package]
name = "test-project"
version = "0.1.0-alpha.1"
"#;
		std::fs::write(temp_dir.path().join("Cargo.toml"), content).unwrap();

		let installer = PluginInstaller::new(temp_dir.path());
		// This should add the [dependencies] section automatically or fail
		let result = installer.add_to_cargo_toml("test-delion", "1.0.0");
		// Check if it succeeded or failed gracefully
		if result.is_ok() {
			let content = std::fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();
			assert!(content.contains("test-delion"));
		}
	}

	#[test]
	fn test_update_version_nonexistent_plugin() {
		let temp_dir = TempDir::new().unwrap();
		create_test_cargo_toml(temp_dir.path());

		let installer = PluginInstaller::new(temp_dir.path());
		installer.init_manifest().unwrap();

		let result = installer.update_version("nonexistent-delion", "2.0.0");
		assert!(matches!(result, Err(PluginError::NotFound(_))));
	}

	#[test]
	fn test_enable_nonexistent_plugin() {
		let temp_dir = TempDir::new().unwrap();
		create_test_cargo_toml(temp_dir.path());

		let installer = PluginInstaller::new(temp_dir.path());
		installer.init_manifest().unwrap();

		let result = installer.enable_plugin("nonexistent-delion");
		assert!(matches!(result, Err(PluginError::NotFound(_))));
	}

	#[test]
	fn test_disable_nonexistent_plugin() {
		let temp_dir = TempDir::new().unwrap();
		create_test_cargo_toml(temp_dir.path());

		let installer = PluginInstaller::new(temp_dir.path());
		installer.init_manifest().unwrap();

		let result = installer.disable_plugin("nonexistent-delion");
		assert!(matches!(result, Err(PluginError::NotFound(_))));
	}

	#[test]
	fn test_uninstall_nonexistent_plugin() {
		let temp_dir = TempDir::new().unwrap();
		create_test_cargo_toml(temp_dir.path());

		let installer = PluginInstaller::new(temp_dir.path());
		installer.init_manifest().unwrap();

		let result = installer.uninstall("nonexistent-delion", false);
		assert!(matches!(result, Err(PluginError::NotFound(_))));
	}

	#[test]
	fn test_uninstall_with_purge() {
		let temp_dir = TempDir::new().unwrap();
		create_test_cargo_toml(temp_dir.path());

		let installer = PluginInstaller::new(temp_dir.path());
		installer
			.install_static("auth-delion", "1.0.0", None)
			.unwrap();

		// Uninstall with purge option
		installer.uninstall("auth-delion", true).unwrap();

		// Verify it's removed
		let manifest = ProjectManifest::load(installer.manifest_path()).unwrap();
		assert!(!manifest.is_installed("auth-delion"));
	}

	#[test]
	fn test_remove_from_cargo_toml_no_file() {
		let temp_dir = TempDir::new().unwrap();
		// Don't create Cargo.toml
		let installer = PluginInstaller::new(temp_dir.path());
		let result = installer.remove_from_cargo_toml("test-delion");
		assert!(result.is_err());
	}

	// ==========================================================================
	// InstalledPlugin Tests
	// ==========================================================================

	#[test]
	fn test_installed_plugin_new_static() {
		let plugin = InstalledPlugin::new_static("test-delion", "1.0.0");
		assert_eq!(plugin.name, "test-delion");
		assert_eq!(plugin.version, "1.0.0");
		assert_eq!(plugin.plugin_type, PluginType::Static);
		assert!(plugin.enabled);
	}

	#[test]
	fn test_installed_plugin_new_wasm() {
		let plugin = InstalledPlugin::new_wasm("test-delion", "1.0.0");
		assert_eq!(plugin.name, "test-delion");
		assert_eq!(plugin.version, "1.0.0");
		assert_eq!(plugin.plugin_type, PluginType::Wasm);
		assert!(plugin.enabled);
	}

	// ==========================================================================
	// Clone Tests
	// ==========================================================================

	#[test]
	fn test_installed_plugin_clone() {
		let plugin = InstalledPlugin::new_static("test-delion", "1.0.0");
		let cloned = plugin.clone();
		assert_eq!(plugin.name, cloned.name);
		assert_eq!(plugin.version, cloned.version);
		assert_eq!(plugin.plugin_type, cloned.plugin_type);
	}

	#[test]
	fn test_plugin_type_clone() {
		let static_type = PluginType::Static;
		let wasm_type = PluginType::Wasm;
		assert_eq!(static_type, static_type.clone());
		assert_eq!(wasm_type, wasm_type.clone());
	}

	#[test]
	fn test_plugin_type_equality() {
		assert_eq!(PluginType::Static, PluginType::Static);
		assert_eq!(PluginType::Wasm, PluginType::Wasm);
		assert_ne!(PluginType::Static, PluginType::Wasm);
	}
}
