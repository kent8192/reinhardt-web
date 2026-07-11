//! Development ownership and last-good replacement for generated component CSS.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use tempfile::{NamedTempFile, TempDir};

use crate::{COMPONENT_STYLES_PATH, StyleExtractor, StyleFingerprints, StylePackageContext};

/// Result of comparing a newly compiled style bundle with the last-good state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentStyleStageResult {
	/// No relevant output changed.
	Unchanged,
	/// Only CSS bytes changed, so a browser stylesheet refresh is sufficient.
	CssOnly,
	/// Ordinary Rust or the generated component API changed.
	RustOrApiChanged,
	/// Compilation failed and the last-good snapshot was retained.
	Failed,
}

/// RAII owner for the temporary development static root.
#[derive(Debug)]
pub struct GeneratedStyleAssets {
	root: TempDir,
}

impl GeneratedStyleAssets {
	/// Create an owned static root and write the initial stylesheet atomically.
	pub fn new(css: &[u8]) -> Result<Self, String> {
		let root = tempfile::tempdir()
			.map_err(|error| format!("failed to create component style asset root: {error}"))?;
		let assets = Self { root };
		assets.replace(css)?;
		Ok(assets)
	}

	/// Root directory mounted before application-owned static sources.
	pub fn root(&self) -> &Path {
		self.root.path()
	}

	/// Exact generated stylesheet path inside the owned root.
	pub fn stylesheet_path(&self) -> PathBuf {
		self.root.path().join(COMPONENT_STYLES_PATH)
	}

	/// Atomically replace the generated stylesheet bytes.
	pub fn replace(&self, css: &[u8]) -> Result<(), String> {
		let destination = self.stylesheet_path();
		let parent = destination
			.parent()
			.ok_or_else(|| "component stylesheet destination has no parent".to_string())?;
		std::fs::create_dir_all(parent)
			.map_err(|error| format!("failed to create component stylesheet directory: {error}"))?;
		let mut temporary = NamedTempFile::new_in(parent).map_err(|error| {
			format!("failed to create component stylesheet temporary file: {error}")
		})?;
		temporary
			.write_all(css)
			.map_err(|error| format!("failed to write component stylesheet: {error}"))?;
		temporary
			.as_file()
			.sync_all()
			.map_err(|error| format!("failed to sync component stylesheet: {error}"))?;
		temporary.persist(&destination).map_err(|error| {
			format!(
				"failed to replace component stylesheet atomically: {}",
				error.error
			)
		})?;
		Ok(())
	}
}

/// Last-good development compiler state retained for the server lifetime.
#[derive(Debug)]
pub struct ComponentStyleState {
	manifest_path: PathBuf,
	requested_package: Option<String>,
	context: StylePackageContext,
	extractor: StyleExtractor,
	fingerprints: StyleFingerprints,
	assets: GeneratedStyleAssets,
}

impl ComponentStyleState {
	/// Resolve, compile, and write the initial stylesheet before server startup.
	pub fn initialize(
		manifest_path: impl Into<PathBuf>,
		requested_package: Option<String>,
	) -> Result<Self, String> {
		let manifest_path = manifest_path.into();
		let context = StylePackageContext::resolve(&manifest_path, requested_package.as_deref())?;
		let extractor = StyleExtractor::new(context.clone());
		let bundle = extractor.extract()?;
		let assets = GeneratedStyleAssets::new(&bundle.css)?;
		let fingerprints = bundle.fingerprints;
		Ok(Self {
			manifest_path,
			requested_package,
			context,
			extractor,
			fingerprints,
			assets,
		})
	}

	/// Owned development static root.
	pub fn generated_root(&self) -> &Path {
		self.assets.root()
	}

	/// Selected package context shared with production collection when requested.
	pub fn package_context(&self) -> &StylePackageContext {
		&self.context
	}

	/// Consume compiler state while retaining only the Send-safe RAII asset owner.
	pub fn into_generated_assets(self) -> GeneratedStyleAssets {
		self.assets
	}

	/// Stable public stylesheet URL for the supplied static prefix.
	pub fn stylesheet_url(&self, static_url: &str) -> String {
		join_static_url(static_url, COMPONENT_STYLES_PATH)
	}

	/// Recompile and atomically advance the last-good snapshot on success.
	pub fn refresh(&mut self, metadata_changed: bool) -> ComponentStyleStageResult {
		let candidate_context = if metadata_changed {
			match StylePackageContext::resolve(
				&self.manifest_path,
				self.requested_package.as_deref(),
			) {
				Ok(context) => context,
				Err(error) => {
					eprintln!("component style metadata refresh failed: {error}");
					return ComponentStyleStageResult::Failed;
				}
			}
		} else {
			self.context.clone()
		};
		let candidate_extractor = StyleExtractor::new(candidate_context.clone());
		let candidate = match candidate_extractor.extract() {
			Ok(bundle) => bundle,
			Err(error) => {
				eprintln!("component style compilation failed: {error}");
				return ComponentStyleStageResult::Failed;
			}
		};

		let old = self.fingerprints;
		let new = candidate.fingerprints;
		let result = if old == new {
			ComponentStyleStageResult::Unchanged
		} else if old.non_style_rust == new.non_style_rust && old.generated_api == new.generated_api
		{
			ComponentStyleStageResult::CssOnly
		} else {
			ComponentStyleStageResult::RustOrApiChanged
		};
		if old.css != new.css
			&& let Err(error) = self.assets.replace(&candidate.css)
		{
			eprintln!("component style asset replacement failed: {error}");
			return ComponentStyleStageResult::Failed;
		}
		self.context = candidate_context;
		self.extractor = candidate_extractor;
		self.fingerprints = new;
		result
	}
}

/// Join a configured static URL and a logical asset path without duplicate separators.
pub fn join_static_url(static_url: &str, logical_path: &str) -> String {
	format!(
		"{}/{}",
		static_url.trim_end_matches('/'),
		logical_path.trim_start_matches('/')
	)
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	#[rstest]
	#[case("/static/", "/static/__reinhardt__/components.css")]
	#[case("/assets", "/assets/__reinhardt__/components.css")]
	#[case(
		"https://cdn.example.test/static/",
		"https://cdn.example.test/static/__reinhardt__/components.css"
	)]
	fn joins_static_urls(#[case] prefix: &str, #[case] expected: &str) {
		assert_eq!(join_static_url(prefix, COMPONENT_STYLES_PATH), expected);
	}

	#[rstest]
	fn generated_assets_replace_bytes_and_accept_empty_output() {
		let assets = GeneratedStyleAssets::new(b"first").expect("create assets");
		assert_eq!(std::fs::read(assets.stylesheet_path()).unwrap(), b"first");
		assets.replace(b"").expect("replace assets");
		assert_eq!(std::fs::read(assets.stylesheet_path()).unwrap(), b"");
		assert!(assets.stylesheet_path().ends_with(COMPONENT_STYLES_PATH));
	}

	#[rstest]
	fn refresh_keeps_last_good_css_then_dispatches_css_only() {
		let directory = tempfile::tempdir().expect("create temporary package");
		std::fs::create_dir(directory.path().join("src")).unwrap();
		std::fs::write(
			directory.path().join("Cargo.toml"),
			"[package]\nname = \"style-refresh\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
		)
		.unwrap();
		let source_path = directory.path().join("src/lib.rs");
		std::fs::write(
			&source_path,
			"#[style_def] static STYLES: Styles = style! { .card { color: red; } };\n",
		)
		.unwrap();
		let mut state = ComponentStyleState::initialize(directory.path().join("Cargo.toml"), None)
			.expect("initialize style state");
		let output = state.generated_root().join(COMPONENT_STYLES_PATH);
		let last_good = std::fs::read(&output).unwrap();

		std::fs::write(
			&source_path,
			"#[style_def] static STYLES: Styles = style! { .card { color: not-a-color; } };\n",
		)
		.unwrap();
		assert_eq!(state.refresh(false), ComponentStyleStageResult::Failed);
		assert_eq!(std::fs::read(&output).unwrap(), last_good);

		std::fs::write(
			&source_path,
			"#[style_def] static STYLES: Styles = style! { .card { color: blue; } };\n",
		)
		.unwrap();
		assert_eq!(state.refresh(false), ComponentStyleStageResult::CssOnly);
		assert_ne!(std::fs::read(&output).unwrap(), last_good);
	}
}
