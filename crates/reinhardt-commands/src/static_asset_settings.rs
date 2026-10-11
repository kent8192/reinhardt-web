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
		let merged = load_project_settings(base_dir)?;
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

/// Load the merged settings of the project's active profile used for static
/// asset resolution (`settings/base.toml` plus `settings/<REINHARDT_ENV>.toml`).
///
/// Uses the builder's default shallow merge: a top-level table declared in the
/// profile file replaces the same table from `base.toml`.
pub(crate) fn load_project_settings(base_dir: &Path) -> Result<MergedSettings, String> {
	let profile_name = std::env::var("REINHARDT_ENV").unwrap_or_else(|_| "local".to_string());
	let settings_dir = base_dir.join("settings");
	SettingsBuilder::new()
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
		.map_err(|error| format!("failed to load static asset settings: {error}"))
}

/// Resolve the `response_headers` tables that `runserver` attaches to static
/// asset and SPA shell responses.
///
/// Reads `[static] response_headers` and then `[static_files] response_headers`;
/// for a header name present in both, the `[static_files]` value wins, matching
/// the section precedence of [`StaticAssetSettings::from_merged`]. Each table
/// maps a header name to a string value:
///
/// ```toml
/// [static_files.response_headers]
/// "Content-Security-Policy" = "default-src 'self'"
/// "X-Content-Type-Options" = "nosniff"
/// ```
///
/// A present but malformed table, header name, or header value is an error so
/// that a misconfigured security header never silently disappears. The
/// settings are merged table by table (see [`load_project_settings`]), so a
/// profile file that declares the section must repeat `response_headers`.
#[cfg(feature = "server")]
pub(crate) fn static_response_headers_from_merged(
	settings: &MergedSettings,
) -> Result<hyper::HeaderMap, String> {
	use hyper::header::{HeaderName, HeaderValue};

	let mut headers = hyper::HeaderMap::new();
	for section in ["static", "static_files"] {
		let Some(table) = settings
			.get_raw(section)
			.and_then(|value| value.get("response_headers"))
		else {
			continue;
		};
		let table = table.as_object().ok_or_else(|| {
			format!("`{section}.response_headers` must be a table mapping header names to strings")
		})?;
		for (name, value) in table {
			let header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|error| {
				format!("`{section}.response_headers` has an invalid header name `{name}`: {error}")
			})?;
			let value = value
				.as_str()
				.ok_or_else(|| format!("`{section}.response_headers.{name}` must be a string"))?;
			let header_value = HeaderValue::from_str(value).map_err(|error| {
				format!("`{section}.response_headers.{name}` has an invalid header value: {error}")
			})?;
			headers.insert(header_name, header_value);
		}
	}
	Ok(headers)
}

/// Merge override header maps over a base map.
///
/// For every header name an override contains, its values replace all values
/// of that name gathered so far; other headers are kept. Later overrides win.
#[cfg(feature = "server")]
pub(crate) fn merge_static_response_headers(
	mut base: hyper::HeaderMap,
	overrides: impl IntoIterator<Item = hyper::HeaderMap>,
) -> hyper::HeaderMap {
	for headers in overrides {
		// `HeaderMap::extend` replaces every existing value of each overriding
		// name and keeps repeated overriding values intact.
		base.extend(headers);
	}
	base
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

#[cfg(all(test, feature = "server"))]
mod response_header_tests {
	use super::*;
	use hyper::HeaderMap;
	use hyper::header::{self, HeaderValue};
	use rstest::*;

	/// Merge one TOML document into settings the way `runserver` loads them.
	fn merged_from_toml(toml: &str) -> MergedSettings {
		let dir = tempfile::tempdir().unwrap();
		let path = dir.path().join("base.toml");
		std::fs::write(&path, toml).unwrap();
		SettingsBuilder::new()
			.add_source(TomlFileSource::new(&path))
			.build()
			.unwrap()
	}

	fn header_map(entries: &[(header::HeaderName, &'static str)]) -> HeaderMap {
		let mut headers = HeaderMap::new();
		for (name, value) in entries {
			headers.append(name.clone(), HeaderValue::from_static(value));
		}
		headers
	}

	#[rstest]
	fn settings_table_becomes_header_map() {
		// Arrange
		let settings = merged_from_toml(
			"[static_files.response_headers]\n\
			 \"Content-Security-Policy\" = \"default-src 'self'\"\n\
			 \"X-Frame-Options\" = \"DENY\"\n",
		);

		// Act
		let headers = static_response_headers_from_merged(&settings).unwrap();

		// Assert
		assert_eq!(
			headers,
			header_map(&[
				(header::CONTENT_SECURITY_POLICY, "default-src 'self'"),
				(header::X_FRAME_OPTIONS, "DENY"),
			])
		);
	}

	#[rstest]
	fn static_files_section_overrides_static_section_per_name() {
		// Arrange
		let settings = merged_from_toml(
			"[static.response_headers]\n\
			 \"X-Frame-Options\" = \"SAMEORIGIN\"\n\
			 \"Referrer-Policy\" = \"no-referrer\"\n\
			 [static_files.response_headers]\n\
			 \"X-Frame-Options\" = \"DENY\"\n",
		);

		// Act
		let headers = static_response_headers_from_merged(&settings).unwrap();

		// Assert
		assert_eq!(
			headers,
			header_map(&[
				(header::X_FRAME_OPTIONS, "DENY"),
				(header::REFERRER_POLICY, "no-referrer"),
			])
		);
	}

	#[rstest]
	#[case::no_static_section("[core]\ndebug = true\n")]
	#[case::static_section_without_headers("[static_files]\nurl = \"/assets/\"\n")]
	fn absent_response_headers_resolve_to_empty_map(#[case] toml: &str) {
		// Arrange
		let settings = merged_from_toml(toml);

		// Act
		let headers = static_response_headers_from_merged(&settings).unwrap();

		// Assert
		assert!(headers.is_empty());
	}

	#[rstest]
	#[case::not_a_table(
		"[static_files]\nresponse_headers = \"nosniff\"\n",
		"`static_files.response_headers` must be a table mapping header names to strings"
	)]
	#[case::non_string_value(
		"[static_files.response_headers]\n\"X-Frame-Options\" = 1\n",
		"`static_files.response_headers.X-Frame-Options` must be a string"
	)]
	#[case::invalid_name(
		"[static.response_headers]\n\"Bad Header\" = \"x\"\n",
		"`static.response_headers` has an invalid header name `Bad Header`: invalid HTTP header name"
	)]
	#[case::invalid_value(
		"[static_files.response_headers]\n\"X-Frame-Options\" = \"DENY\\nX-Injected: 1\"\n",
		"`static_files.response_headers.X-Frame-Options` has an invalid header value: failed to parse header value"
	)]
	fn malformed_response_headers_are_rejected(#[case] toml: &str, #[case] expected: &str) {
		// Arrange
		let settings = merged_from_toml(toml);

		// Act
		let error = static_response_headers_from_merged(&settings).unwrap_err();

		// Assert
		assert_eq!(error, expected);
	}

	#[rstest]
	fn hook_headers_override_settings_headers_per_name() {
		// Arrange
		let settings = header_map(&[
			(header::CONTENT_SECURITY_POLICY, "default-src 'self'"),
			(header::X_FRAME_OPTIONS, "SAMEORIGIN"),
		]);
		let first_hook = header_map(&[
			(header::X_FRAME_OPTIONS, "DENY"),
			(header::STRICT_TRANSPORT_SECURITY, "max-age=60"),
		]);
		let second_hook = header_map(&[
			(header::STRICT_TRANSPORT_SECURITY, "max-age=31536000"),
			(header::STRICT_TRANSPORT_SECURITY, "includeSubDomains"),
		]);

		// Act
		let merged = merge_static_response_headers(settings, [first_hook, second_hook]);

		// Assert
		assert_eq!(
			merged,
			header_map(&[
				(header::CONTENT_SECURITY_POLICY, "default-src 'self'"),
				(header::X_FRAME_OPTIONS, "DENY"),
				(header::STRICT_TRANSPORT_SECURITY, "max-age=31536000"),
				(header::STRICT_TRANSPORT_SECURITY, "includeSubDomains"),
			])
		);
	}

	#[rstest]
	fn empty_hook_headers_keep_settings_headers() {
		// Arrange
		let settings = header_map(&[(header::X_CONTENT_TYPE_OPTIONS, "nosniff")]);

		// Act
		let merged = merge_static_response_headers(settings.clone(), [HeaderMap::new()]);

		// Assert
		assert_eq!(merged, settings);
	}
}
