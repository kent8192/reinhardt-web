//! Shared static asset settings resolution for collectstatic and runserver.

use std::path::{Path, PathBuf};

use reinhardt_conf::settings::builder::BuildError;
use reinhardt_conf::settings::builder::{MergedSettings, SettingsBuilder};
use reinhardt_conf::settings::profile::Profile;
use reinhardt_conf::settings::scoped::ScopedSettings;
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
	/// Resolve only the static input paths selected for an asset command.
	/// Explicit malformed values fail instead of silently using defaults.
	pub fn from_scoped(settings: &ScopedSettings) -> Result<Self, BuildError> {
		let base_dir = settings
			.first_present_path::<PathBuf>(&[&["core", "base_dir"], &["base_dir"]])?
			.unwrap_or(std::env::current_dir().map_err(|error| {
				BuildError::Deserialization(format!("cannot find project directory: {error}"))
			})?);
		let static_url = settings
			.first_present_path::<String>(&[
				&["static_files", "url"],
				&["static", "url"],
				&["static_url"],
			])?
			.unwrap_or_else(|| "/static/".to_owned());
		let absolute_url = url::Url::parse(&static_url)
			.is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some());
		if !static_url.ends_with('/') || !(static_url.starts_with('/') || absolute_url) {
			return Err(BuildError::Deserialization(
				"static URL must be a slash-prefixed path or HTTP(S) URL ending with `/` (settings path `static.url`)".to_owned(),
			));
		}
		let static_root = settings
			.first_present_path::<PathBuf>(&[
				&["static_files", "root"],
				&["static", "root"],
				&["static_root"],
			])?
			.ok_or_else(|| {
				BuildError::Deserialization(
				"missing required static output path `static.root`; configure it in settings TOML"
					.to_owned(),
			)
			})?;
		if static_root.as_os_str().is_empty() {
			return Err(BuildError::Deserialization(
				"static output path `static.root` must not be empty".to_owned(),
			));
		}
		let staticfiles_dirs = settings
			.optional_path::<Vec<PathBuf>>(&["staticfiles_dirs"])?
			.unwrap_or_default();
		let relative_to_base = |path: PathBuf| {
			if path.is_absolute() {
				path
			} else {
				base_dir.join(path)
			}
		};
		Ok(Self {
			static_url,
			static_root: relative_to_base(static_root),
			staticfiles_dirs: staticfiles_dirs.into_iter().map(relative_to_base).collect(),
		})
	}
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
		if resolved.static_root.is_relative() {
			resolved.static_root = base_dir.join(&resolved.static_root);
		}
		for source in &mut resolved.staticfiles_dirs {
			if source.is_relative() {
				*source = base_dir.join(&*source);
			}
		}
		resolved
	}
}

impl crate::capabilities::SettingsView for StaticAssetSettings {
	const NAME: &'static str = "static asset settings";

	fn resolve(settings: &ScopedSettings, alias: Option<&str>) -> Result<Self, BuildError> {
		if alias.is_some() {
			return Err(BuildError::Deserialization(
				"static asset settings do not accept an alias".to_owned(),
			));
		}
		Self::from_scoped(settings)
	}
}

#[cfg(test)]
mod scoped_tests {
	use super::*;
	use rstest::*;
	use std::fs;

	#[rstest]
	fn static_view_ignores_runtime_secret_but_validates_selected_root() {
		let dir = tempfile::tempdir().unwrap();
		let config = dir.path().join("settings.toml");
		fs::write(&config, "[core]\nsecret_key = \"${REINHARDT_SCOPED_MISSING_SECRET_6336}\"\n[static]\nurl = \"/assets/\"\nroot = \"dist\"\n").unwrap();
		let settings = SettingsBuilder::new()
			.add_source(TomlFileSource::new(&config))
			.build_scoped()
			.unwrap();
		let static_settings = StaticAssetSettings::from_scoped(&settings).unwrap();
		assert_eq!(static_settings.static_url, "/assets/");
		assert_eq!(
			static_settings.static_root,
			std::env::current_dir().unwrap().join("dist")
		);

		fs::write(&config, "[static]\nurl = \"/assets/\"\nroot = 7\n").unwrap();
		let settings = SettingsBuilder::new()
			.add_source(TomlFileSource::new(&config))
			.build_scoped()
			.unwrap();
		assert!(StaticAssetSettings::from_scoped(&settings).is_err());
	}

	#[rstest]
	fn missing_static_root_has_actionable_error() {
		let settings = SettingsBuilder::new().build_scoped().unwrap();
		let error = StaticAssetSettings::from_scoped(&settings).unwrap_err();
		assert!(error.to_string().contains("static.root"));
	}

	#[rstest]
	fn flat_base_dir_resolves_relative_static_paths() {
		let project = tempfile::tempdir().unwrap();
		let settings = SettingsBuilder::new()
			.add_source(
				DefaultSource::new()
					.with_value(
						"base_dir",
						Value::String(project.path().to_string_lossy().into_owned()),
					)
					.with_value("static", serde_json::json!({"root": "dist"}))
					.with_value("staticfiles_dirs", serde_json::json!(["assets"])),
			)
			.build_scoped()
			.unwrap();
		let static_settings = StaticAssetSettings::from_scoped(&settings).unwrap();
		assert_eq!(static_settings.static_root, project.path().join("dist"));
		assert_eq!(
			static_settings.staticfiles_dirs,
			vec![project.path().join("assets")]
		);
	}
}
