//! Template integration for static files
//!
//! Provides integration between reinhardt-static and reinhardt-templates,
//! allowing templates to generate URLs to static files using the `static` filter.

use crate::{ManifestStaticFilesStorage, StaticFilesConfig};
use std::collections::HashMap;
use std::io;

/// Configuration for static files in templates
///
/// This is a simplified version of `reinhardt_templates::static_filters::StaticConfig`
/// that can be constructed from `StaticFilesConfig`.
#[derive(Debug, Clone)]
pub struct TemplateStaticConfig {
	/// Base URL for static files (e.g., "/static/")
	pub static_url: String,
	/// Whether to use hashed filenames from manifest
	pub use_manifest: bool,
	/// Manifest mapping original paths to hashed paths
	pub manifest: HashMap<String, String>,
}

impl From<&StaticFilesConfig> for TemplateStaticConfig {
	fn from(config: &StaticFilesConfig) -> Self {
		Self {
			static_url: config.static_url.clone(),
			use_manifest: false,
			manifest: HashMap::new(),
		}
	}
}

impl TemplateStaticConfig {
	/// Create a new template static configuration
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::template_integration::TemplateStaticConfig;
	///
	/// let config = TemplateStaticConfig::new("/static/".to_string());
	/// assert_eq!(config.static_url, "/static/");
	/// assert!(!config.use_manifest);
	/// ```
	pub fn new(static_url: String) -> Self {
		Self {
			static_url,
			use_manifest: false,
			manifest: HashMap::new(),
		}
	}

	/// Enable manifest-based hashed filenames
	///
	/// # Examples
	///
	/// ```rust
	/// use reinhardt_static::template_integration::TemplateStaticConfig;
	/// use std::collections::HashMap;
	///
	/// let mut manifest = HashMap::new();
	/// manifest.insert("css/style.css".to_string(), "css/style.abc123.css".to_string());
	///
	/// let config = TemplateStaticConfig::new("/static/".to_string())
	///     .with_manifest(manifest);
	///
	/// assert!(config.use_manifest);
	/// assert_eq!(config.manifest.len(), 1);
	/// ```
	pub fn with_manifest(mut self, manifest: HashMap<String, String>) -> Self {
		self.use_manifest = true;
		self.manifest = manifest;
		self
	}

	/// Load manifest from ManifestStaticFilesStorage
	///
	/// # Examples
	///
	/// ```rust,ignore
	/// use reinhardt_static::template_integration::TemplateStaticConfig;
	/// use reinhardt_static::ManifestStaticFilesStorage;
	/// use std::path::PathBuf;
	///
	/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
	/// let storage = ManifestStaticFilesStorage::new(
	///     PathBuf::from("/var/www/static"),
	///     "/static/"
	/// );
	///
	/// let config = TemplateStaticConfig::from_storage(&storage).await?;
	/// assert!(config.use_manifest);
	/// # Ok(())
	/// # }
	/// ```
	pub async fn from_storage(storage: &ManifestStaticFilesStorage) -> io::Result<Self> {
		// Load manifest from disk
		let manifest_path = storage.location.join(&storage.manifest_name);

		if !manifest_path.exists() {
			return Ok(Self {
				static_url: storage.base_url.clone(),
				use_manifest: false,
				manifest: HashMap::new(),
			});
		}

		let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;

		// Parse manifest JSON
		// The manifest is stored as a simple HashMap<String, String>
		let manifest: HashMap<String, String> = serde_json::from_str(&manifest_content)
			.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

		Ok(Self {
			static_url: storage.base_url.clone(),
			use_manifest: true,
			manifest,
		})
	}
}

#[cfg(feature = "templates-integration")]
impl From<TemplateStaticConfig> for reinhardt_templates::StaticConfig {
	fn from(config: TemplateStaticConfig) -> Self {
		Self {
			static_url: config.static_url,
			use_manifest: config.use_manifest,
			manifest: config.manifest,
		}
	}
}

#[cfg(feature = "templates-integration")]
/// Initialize the global static configuration for templates
///
/// This should be called once at application startup to configure how
/// templates generate URLs to static files.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_static::{StaticFilesConfig, template_integration};
/// use std::path::PathBuf;
///
/// let config = StaticFilesConfig {
///     static_root: PathBuf::from("/var/www/static"),
///     static_url: "/static/".to_string(),
///     staticfiles_dirs: vec![],
///     media_url: None,
/// };
///
/// template_integration::init_template_static_config(&config);
/// ```
pub fn init_template_static_config(config: &StaticFilesConfig) {
	let template_config = TemplateStaticConfig::from(config);
	let reinhardt_config = reinhardt_templates::StaticConfig::from(template_config);
	reinhardt_templates::init_static_config(reinhardt_config);
}

#[cfg(feature = "templates-integration")]
/// Initialize the global static configuration with manifest support
///
/// This loads the manifest from storage and configures templates to use
/// hashed filenames for cache busting.
///
/// # Examples
///
/// ```rust,ignore
/// use reinhardt_static::{ManifestStaticFilesStorage, template_integration};
/// use std::path::PathBuf;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let storage = ManifestStaticFilesStorage::new(
///     PathBuf::from("/var/www/static"),
///     "/static/"
/// );
///
/// template_integration::init_template_static_config_with_manifest(&storage).await?;
/// # Ok(())
/// # }
/// ```
pub async fn init_template_static_config_with_manifest(
	storage: &ManifestStaticFilesStorage,
) -> io::Result<()> {
	let template_config = TemplateStaticConfig::from_storage(storage).await?;
	let reinhardt_config = reinhardt_templates::StaticConfig::from(template_config);
	reinhardt_templates::init_static_config(reinhardt_config);
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_template_static_config_new() {
		let config = TemplateStaticConfig::new("/static/".to_string());
		assert_eq!(config.static_url, "/static/");
		assert!(!config.use_manifest);
		assert!(config.manifest.is_empty());
	}

	#[test]
	fn test_template_static_config_with_manifest() {
		let mut manifest = HashMap::new();
		manifest.insert(
			"css/style.css".to_string(),
			"css/style.abc123.css".to_string(),
		);

		let config =
			TemplateStaticConfig::new("/static/".to_string()).with_manifest(manifest.clone());

		assert_eq!(config.static_url, "/static/");
		assert!(config.use_manifest);
		assert_eq!(config.manifest.len(), 1);
		assert_eq!(
			config.manifest.get("css/style.css"),
			Some(&"css/style.abc123.css".to_string())
		);
	}

	#[test]
	fn test_template_static_config_from_static_files_config() {
		let static_config = StaticFilesConfig {
			static_root: std::path::PathBuf::from("/var/www/static"),
			static_url: "/assets/".to_string(),
			staticfiles_dirs: vec![],
			media_url: None,
		};

		let template_config = TemplateStaticConfig::from(&static_config);
		assert_eq!(template_config.static_url, "/assets/");
		assert!(!template_config.use_manifest);
		assert!(template_config.manifest.is_empty());
	}

	#[tokio::test]
	async fn test_from_storage() {
		use tempfile::tempdir;

		let temp_dir = tempdir().unwrap();
		let static_root = temp_dir.path().to_path_buf();

		// Create manifest file (simple HashMap format)
		let manifest_content = r#"{
  "css/style.css": "css/style.abc123.css",
  "js/app.js": "js/app.def456.js"
}"#;

		std::fs::write(static_root.join("staticfiles.json"), manifest_content).unwrap();

		let storage = ManifestStaticFilesStorage::new(static_root, "/static/");
		let config = TemplateStaticConfig::from_storage(&storage).await.unwrap();

		assert_eq!(config.static_url, "/static/");
		assert!(config.use_manifest);
		assert_eq!(config.manifest.len(), 2);
		assert_eq!(
			config.manifest.get("css/style.css"),
			Some(&"css/style.abc123.css".to_string())
		);
		assert_eq!(
			config.manifest.get("js/app.js"),
			Some(&"js/app.def456.js".to_string())
		);
	}
}
