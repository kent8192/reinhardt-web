//! Shared static asset settings resolution for collectstatic and runserver.

use std::path::{Path, PathBuf};

use reinhardt_conf::settings::builder::MergedSettings;
use serde::Deserialize;

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
