//! Shared static asset settings resolution for collectstatic and runserver.

use std::path::{Path, PathBuf};

use reinhardt_conf::settings::builder::{MergedSettings, SettingsBuilder};
use reinhardt_conf::settings::profile::Profile;
use reinhardt_conf::settings::sources::{DefaultSource, LowPriorityEnvSource, TomlFileSource};
use serde::Deserialize;
use serde_json::Value;

/// Active URL, destination, and source directories for static assets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticAssetSettings {
	/// Public URL prefix.
	pub static_url: String,
	/// Collection destination.
	pub static_root: PathBuf,
	/// Additional physical source directories.
	pub staticfiles_dirs: Vec<PathBuf>,
}

#[derive(Debug, Default, Deserialize)]
struct StaticSection {
	url: Option<String>,
	root: Option<PathBuf>,
}

impl StaticAssetSettings {
	/// Load static asset settings from the project's active settings profile.
	pub fn from_project_dir(base_dir: &Path) -> Result<Self, String> {
		let profile_name = std::env::var("REINHARDT_ENV").unwrap_or_else(|_| "local".to_string());
		let settings_dir = base_dir.join("settings");
		let merged = SettingsBuilder::new()
			.profile(Profile::parse(&profile_name))
			.add_source(
				DefaultSource::new()
					.with_value("static_url", Value::String("/static/".to_string()))
					.with_value(
						"static_root",
						Value::String(base_dir.join("staticfiles").to_string_lossy().to_string()),
					)
					.with_value("staticfiles_dirs", Value::Array(Vec::new())),
			)
			.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
			.add_source(TomlFileSource::new(settings_dir.join("base.toml")))
			.add_source(TomlFileSource::new(
				settings_dir.join(format!("{profile_name}.toml")),
			))
			.build()
			.map_err(|error| format!("failed to load static asset settings: {error}"))?;
		Ok(Self::from_merged(&merged, base_dir))
	}

	/// Resolve modern `[static]`, typed `[static_files]`, and legacy flat keys.
	pub fn from_merged(settings: &MergedSettings, base_dir: &Path) -> Self {
		let mut resolved = Self {
			static_url: settings.get_or("static_url", "/static/".to_string()),
			static_root: settings
				.get::<PathBuf>("static_root")
				.unwrap_or_else(|_| base_dir.join("staticfiles")),
			staticfiles_dirs: settings.get_or("staticfiles_dirs", Vec::new()),
		};
		for key in ["static", "static_files"] {
			if let Ok(section) = settings.get::<StaticSection>(key) {
				if let Some(url) = section.url {
					resolved.static_url = url;
				}
				if let Some(root) = section.root {
					resolved.static_root = if root.is_absolute() {
						root
					} else {
						base_dir.join(root)
					};
				}
			}
		}
		resolved
	}
}
