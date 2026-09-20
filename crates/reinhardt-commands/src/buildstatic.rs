//! One supported publication path for ordinary static assets and Pages bundles.
//!
//! Output always uses [`StaticAssetSettings::static_root`]. Materialized inputs
//! replace their corresponding compilation steps; dry-run never compiles or writes.

mod inputs;

pub use inputs::{CollectedStaticInputs, PagesBuildInputs};

use crate::{CollectStaticCommand, CollectStaticOptions, StaticAssetSettings};
use crate::{StyleFeatureSelection, StylePackageContext, WasmBuildConfig, WasmBuilder};
use clap::Parser;
use reinhardt_utils::staticfiles::StaticFilesConfig;
use reinhardt_utils::staticfiles::publication::{
	AssetBuildError, AssetClassifier, AssetInput, AssetMode, AssetPipeline, AssetPublisher,
	AssetRole, AssetUrlSnapshot, ManifestSnapshot,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Typed parser carried through the existing extensible management-command dispatcher.
#[derive(Debug, Parser)]
#[command(
	name = "buildstatic",
	about = "Publish one complete static asset generation to configured STATIC_ROOT"
)]
pub struct BuildStaticArgs {
	/// Build a Pages package with Cargo and wasm-bindgen.
	#[arg(long, conflicts_with = "pages_dir")]
	pub pages: bool,
	/// Import a complete materialized wasm-bindgen web output directory.
	#[arg(long, requires = "pages_entry")]
	pub pages_dir: Option<PathBuf>,
	/// JavaScript entry relative to --pages-dir.
	#[arg(long, requires = "pages_dir")]
	pub pages_entry: Option<String>,
	/// Import a selected legacy or version 2 static manifest instead of collecting sources.
	#[arg(long)]
	pub static_manifest: Option<PathBuf>,
	/// Logical collected HTML template to render for Pages navigation.
	#[arg(long)]
	pub pages_document: Option<String>,
	/// Additional logical stylesheet, in cascade order (repeatable).
	#[arg(long = "pages-style")]
	pub pages_styles: Vec<String>,
	/// Cargo package used for both Pages and component styles.
	#[arg(short, long)]
	pub package: Option<String>,
	/// Package features shared by the WASM build and style extraction.
	#[arg(long, value_delimiter = ',', conflicts_with = "all_features")]
	pub features: Vec<String>,
	/// Enable every package feature.
	#[arg(long)]
	pub all_features: bool,
	/// Compile release artifacts; independent of publication mode.
	#[arg(long)]
	pub release: bool,
	/// Compile a named Cargo profile instead of --release.
	#[arg(long, conflicts_with = "release")]
	pub profile: Option<String>,
	/// Publication mode: production or development. A root cannot mix modes.
	#[arg(long, default_value = "production", value_parser = parse_mode)]
	pub mode: AssetMode,
	/// Report discovered paths and pending generated checks without writing or compiling.
	#[arg(long)]
	pub dry_run: bool,
}

pub(crate) fn parse_mode(value: &str) -> Result<AssetMode, String> {
	match value {
		"production" => Ok(AssetMode::Production),
		"development" => Ok(AssetMode::Development),
		_ => Err("expected production or development".into()),
	}
}

impl BuildStaticArgs {
	/// Resolve command inputs against an explicit project directory, never a global cwd.
	pub fn into_request(self, project_dir: PathBuf) -> BuildStaticRequest {
		BuildStaticRequest {
			project_dir,
			mode: self.mode,
			dry_run: self.dry_run,
			pages: self
				.pages_dir
				.map(|directory| PagesSource::Directory {
					directory,
					entry: self.pages_entry.unwrap_or_default(),
				})
				.or_else(|| self.pages.then_some(PagesSource::Build)),
			static_manifest: self.static_manifest,
			pages_document: self.pages_document,
			pages_styles: self.pages_styles,
			package: self.package,
			features: self.features,
			all_features: self.all_features,
			release: self.release,
			profile: self.profile,
		}
	}
}

/// Mutually exclusive Pages input sources (P0).
#[derive(Debug, Clone)]
pub enum PagesSource {
	/// Compile the selected project package into a private intermediate directory.
	Build,
	/// Import a complete already-built directory without invoking WASM tools.
	Directory {
		/// Source directory, resolved against the request's project directory.
		directory: PathBuf,
		/// JavaScript entry's relative logical path.
		entry: String,
	},
}

/// Publication inputs and compilation selection, independent of the output settings (P0).
#[derive(Debug, Clone)]
pub struct BuildStaticRequest {
	/// Explicit project base used for every relative input path.
	pub project_dir: PathBuf,
	/// Publication/cache contract; production by default.
	pub mode: AssetMode,
	/// Preview without downloads, compilers, staging, or activation.
	pub dry_run: bool,
	/// Optional Pages source; ordinary packaging needs no WASM toolchain.
	pub pages: Option<PagesSource>,
	/// Selected pre-collected manifest, replacing physical discovery and style extraction.
	pub static_manifest: Option<PathBuf>,
	/// Logical collected navigation template.
	pub pages_document: Option<String>,
	/// Additional logical CSS names in cascade order.
	pub pages_styles: Vec<String>,
	/// Cargo package shared by style and WASM compilation.
	pub package: Option<String>,
	/// Explicit package features.
	pub features: Vec<String>,
	/// Enable all package features.
	pub all_features: bool,
	/// Compile release artifacts independently of publication mode.
	pub release: bool,
	/// Optional named Cargo profile; mutually exclusive with release.
	pub profile: Option<String>,
}

impl BuildStaticRequest {
	/// Create an ordinary production collection request.
	pub fn new(project_dir: PathBuf) -> Self {
		Self {
			project_dir,
			mode: AssetMode::Production,
			dry_run: false,
			pages: None,
			static_manifest: None,
			pages_document: None,
			pages_styles: Vec::new(),
			package: None,
			features: Vec::new(),
			all_features: false,
			release: false,
			profile: None,
		}
	}
}

/// Side-effect-free discovery report; intentionally has no final build identifier (P0).
#[derive(Debug, Default)]
pub struct BuildStaticPreview {
	/// Known logical inputs and their materialized source descriptions.
	pub sources: BTreeMap<String, String>,
	/// Proposed generation-relative category paths, before generated output exists.
	pub assignments: BTreeMap<String, String>,
	/// Discovered logical/category collisions.
	pub conflicts: Vec<String>,
	/// Checks requiring real compilation, download, or complete reference analysis.
	pub pending_checks: Vec<String>,
}

/// Completed publication or a non-mutating preview (P0).
#[derive(Debug)]
pub enum BuildStaticResult {
	/// Verified complete generation, activated in the configured static root.
	Published(ManifestSnapshot),
	/// Discovery only; no purported final identity or completeness guarantee.
	DryRun(BuildStaticPreview),
}

/// Settings-bound orchestration reusing collection, style extraction, and the WASM builder (P0).
pub struct BuildStaticCommand {
	settings: StaticAssetSettings,
}

impl BuildStaticCommand {
	/// Select the configured publication destination and public URL prefix.
	pub fn new(settings: StaticAssetSettings) -> Self {
		Self { settings }
	}

	/// Capture, verify, and atomically publish a complete generation.
	pub fn execute(
		&self,
		request: BuildStaticRequest,
	) -> Result<BuildStaticResult, AssetBuildError> {
		self.execute_with_compiler(request, compile_inputs)
	}

	fn execute_with_compiler(
		&self,
		request: BuildStaticRequest,
		compiler: impl FnOnce(
			&BuildStaticRequest,
			bool,
			bool,
		) -> Result<CompiledInputs, AssetBuildError>,
	) -> Result<BuildStaticResult, AssetBuildError> {
		if !request.project_dir.is_absolute() {
			return Err(invalid("project_dir", "use an absolute project directory"));
		}
		if request.all_features && !request.features.is_empty() {
			return Err(invalid(
				"features",
				"choose explicit features or all-features",
			));
		}
		if request.release && request.profile.is_some() {
			return Err(invalid("profile", "choose --release or a named profile"));
		}
		let base = &request.project_dir;
		let root = absolute(base, &self.settings.static_root);
		if self.settings.static_root.as_os_str().is_empty() {
			return Err(invalid("STATIC_ROOT", "configure a nonempty static root"));
		}
		if root.join("staticfiles.json").exists() {
			return Err(invalid(
				"staticfiles.json",
				"destination contains a competing legacy manifest; import it with --static-manifest into a separate configured STATIC_ROOT, or explicitly archive it before migration",
			));
		}
		let projection = AssetUrlSnapshot::new(
			"0".repeat(64),
			self.settings.static_url.clone(),
			BTreeMap::new(),
		)?;
		let build_pages = matches!(request.pages, Some(PagesSource::Build));
		let extract_styles = request.static_manifest.is_none()
			&& (build_pages || request.package.is_some() || base.join("Cargo.toml").is_file());
		let compiled = if !request.dry_run && (build_pages || extract_styles) {
			compiler(&request, extract_styles, build_pages)?
		} else {
			CompiledInputs::default()
		};
		let mut collected = if let Some(manifest) = &request.static_manifest {
			inputs::import_manifest(&absolute(base, manifest))?
		} else {
			let mut collector = CollectStaticCommand::new(
				StaticFilesConfig {
					static_root: root.clone(),
					static_url: projection.static_url().into(),
					staticfiles_dirs: self
						.settings
						.staticfiles_dirs
						.iter()
						.map(|p| absolute(base, p))
						.collect(),
					media_url: None,
				},
				CollectStaticOptions {
					dry_run: request.dry_run,
					verbosity: 0,
					..Default::default()
				},
			);
			collector.set_source_base(base.clone());
			for asset in compiled.styles {
				collector.add_virtual_asset(asset);
			}
			let mut collected = collector.collect_inputs().map_err(|e| io_error(&root, e))?;
			// A Pages entry document commonly lives beside the application crate
			// (for example, `dashboard/index.html`) rather than in a configured
			// static source directory. Include that explicit input in the same
			// generation so `--pages-document` never falls back to an unrelated
			// legacy copy or requires a second publication step.
			if let Some(document) = &request.pages_document
				&& !collected
					.inputs
					.iter()
					.any(|input| input.logical_path == *document)
			{
				let source = absolute(base, Path::new(document));
				if source.is_file() {
					AssetClassifier::default().classify(
						document,
						reinhardt_utils::staticfiles::publication::AssetProducer::Static,
					)?;
					collected.inputs.push(
						AssetInput::from_directory(base.clone(), document, document)
							.with_role(AssetRole::EntryDocument),
					);
				}
			}
			collected
		};
		let pages = match &request.pages {
			Some(PagesSource::Directory { directory, entry }) => Some(
				PagesBuildInputs::from_directory(&absolute(base, directory), entry)?,
			),
			_ => compiled.pages,
		};
		let mut entrypoint = pages.as_ref().map(|p| p.entrypoint.clone());
		if let Some(pages) = pages {
			collected.inputs.extend(pages.inputs);
		}
		if let Some(entry) = &mut entrypoint {
			entry.styles.extend(request.pages_styles.iter().cloned());
			if collected
				.inputs
				.iter()
				.any(|input| input.logical_path == crate::COMPONENT_STYLES_PATH)
				&& !entry
					.styles
					.iter()
					.any(|name| name == crate::COMPONENT_STYLES_PATH)
			{
				entry.styles.push(crate::COMPONENT_STYLES_PATH.into());
			}
			entry.document = request.pages_document.clone();
		} else if !build_pages
			&& (request.pages_document.is_some() || !request.pages_styles.is_empty())
		{
			return Err(invalid(
				"Pages",
				"pages-document/pages-style requires Pages inputs",
			));
		}
		if let Some(document) = &request.pages_document {
			if let Some(input) = collected
				.inputs
				.iter_mut()
				.find(|i| &i.logical_path == document)
			{
				input.role = AssetRole::EntryDocument;
			} else if let Some(manifest) = &request.static_manifest {
				// Legacy collectstatic wrote the explicit index beside, but outside, its map.
				let selected = absolute(base, manifest);
				let source = selected.parent().expect("absolute manifest has parent");
				AssetClassifier::default().classify(
					document,
					reinhardt_utils::staticfiles::publication::AssetProducer::Static,
				)?;
				collected.inputs.push(
					AssetInput::from_directory(source.into(), document, document)
						.with_role(AssetRole::EntryDocument),
				);
			} else {
				return Err(invalid(
					document,
					"selected entry document was not collected",
				));
			}
		}
		if request.dry_run {
			let mut preview = inputs::preview(&collected)?;
			if build_pages {
				preview
					.pending_checks
					.push("Pages compilation and its complete JS/WASM dependency graph".into());
			}
			if extract_styles {
				preview
					.pending_checks
					.push("component style extraction using the selected package/features".into());
			}
			preview.pending_checks.extend(collected.pending_checks);
			preview.pending_checks.push("final reference closure, representations, content hashes, and publication readiness".into());
			return Ok(BuildStaticResult::DryRun(preview));
		}
		let mut pipeline = AssetPipeline::new();
		for (logical, source) in collected.reference_bases {
			pipeline.add_reference_base(&logical, &source)?;
		}
		for (published, logical) in &collected.aliases {
			pipeline.add_alias(published, logical)?;
		}
		let prefix = url::Url::parse(projection.static_url())
			.ok()
			.map(|u| u.path().to_owned())
			.unwrap_or_else(|| projection.static_url().to_owned());
		let prefix = percent_encoding::percent_decode_str(&prefix)
			.decode_utf8()
			.map_err(|error| invalid("STATIC_URL", error.to_string()))?;
		let prefix = prefix.trim_start_matches('/');
		for input in collected.inputs {
			pipeline.add_alias(
				&format!("{prefix}{}", input.logical_path),
				&input.logical_path,
			)?;
			pipeline.add_input(input)?;
		}
		for (physical, logical) in collected.aliases {
			pipeline.add_alias(&format!("{prefix}{physical}"), &logical)?;
		}
		for (name, entry) in collected.entrypoints {
			pipeline.set_entrypoint(&name, entry)?;
		}
		if let Some(entry) = entrypoint {
			pipeline.set_entrypoint("default", entry)?;
		}
		let prepared = pipeline.prepare(request.mode)?;
		AssetPublisher::new(root)
			.publish(prepared)
			.map(BuildStaticResult::Published)
	}
}

#[derive(Default)]
struct CompiledInputs {
	styles: Vec<crate::VirtualStaticAsset>,
	pages: Option<PagesBuildInputs>,
}

fn compile_inputs(
	request: &BuildStaticRequest,
	styles: bool,
	pages: bool,
) -> Result<CompiledInputs, AssetBuildError> {
	let selection = if request.all_features {
		StyleFeatureSelection::all_features()
	} else {
		StyleFeatureSelection::with_features(request.features.iter().cloned())
	};
	let context = StylePackageContext::resolve_with_features(
		request.project_dir.join("Cargo.toml"),
		request.package.as_deref(),
		selection,
	)
	.map_err(|e| invalid("Pages package", e))?;
	let mut output = CompiledInputs::default();
	if styles {
		let bundle = crate::StyleExtractor::new(context.clone())
			.extract()
			.map_err(|e| invalid("component styles", e.to_string()))?;
		output.styles.push(crate::VirtualStaticAsset {
			logical_path: crate::COMPONENT_STYLES_PATH.into(),
			bytes: bundle.css,
		});
	}
	if pages {
		if !context.has_cdylib_target() {
			return Err(invalid(
				&context.package_name,
				"Pages requires a cdylib target",
			));
		}
		let private = tempfile::Builder::new()
			.prefix("reinhardt-pages-build-")
			.tempdir()
			.map_err(|e| io_error(&std::env::temp_dir(), e))?;
		let mut builder = WasmBuilder::new(
			WasmBuildConfig::new(&request.project_dir)
				.output_dir(private.path())
				.release(request.release)
				.package(&context.package_name)
				.target_name(context.wasm_target_name()),
		)
		.features(request.features.iter().cloned())
		.all_features(request.all_features);
		if let Some(profile) = &request.profile {
			builder = builder.profile(profile);
		}
		let build = builder
			.build()
			.map_err(|e| invalid("Pages build", e.to_string()))?;
		let entry = build
			.js_file
			.file_name()
			.and_then(|n| n.to_str())
			.ok_or_else(|| invalid("Pages build", "invalid JavaScript entry filename"))?;
		output.pages = Some(
			build
				.publication_inputs(entry)
				.map_err(|e| invalid(entry, e.to_string()))?,
		);
	}
	Ok(output)
}

pub(super) fn absolute(base: &Path, path: &Path) -> PathBuf {
	if path.is_absolute() {
		path.into()
	} else {
		base.join(path)
	}
}

pub(super) fn invalid(asset: impl Into<String>, reason: impl Into<String>) -> AssetBuildError {
	AssetBuildError::Input {
		asset: asset.into(),
		reason: reason.into(),
	}
}

pub(super) fn io_error(path: &Path, source: std::io::Error) -> AssetBuildError {
	AssetBuildError::Io {
		path: path.into(),
		source,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	// The collaborator covers package resolution, style compilation, Cargo and bindgen
	// together, without changing process-global PATH or environment variables.
	#[rstest::rstest]
	#[case(false)]
	#[case(true)]
	fn materialized_inputs_and_dry_run_never_enter_the_compiler(#[case] dry_run: bool) {
		// Arrange
		let root = tempfile::tempdir().unwrap();
		let pages = root.path().join("pages");
		std::fs::create_dir(&pages).unwrap();
		std::fs::write(
			pages.join("app.js"),
			"export const wasm = new URL('app.wasm',import.meta.url);",
		)
		.unwrap();
		std::fs::write(pages.join("app.wasm"), b"\0asm\x01\0\0\0").unwrap();
		std::fs::write(root.path().join("manifest.json"), br#"{"paths":{}}"#).unwrap();
		let calls = std::cell::Cell::new(0);
		let command = BuildStaticCommand::new(StaticAssetSettings {
			static_root: root.path().join("output"),
			static_url: "/static/".into(),
			staticfiles_dirs: Vec::new(),
		});
		let mut request = BuildStaticRequest::new(root.path().into());
		request.dry_run = dry_run;
		request.pages = Some(if dry_run {
			PagesSource::Build
		} else {
			PagesSource::Directory {
				directory: pages,
				entry: "app.js".into(),
			}
		});
		request.static_manifest = Some("manifest.json".into());
		// Act
		let result = command.execute_with_compiler(request, |_, _, _| {
			calls.set(calls.get() + 1);
			Err(invalid("compiler probe", "compiler must not execute"))
		});
		// Assert
		assert!(result.is_ok(), "{result:?}");
		assert_eq!(calls.get(), 0);
	}

	#[test]
	fn pages_document_can_live_at_the_project_root() {
		// Arrange
		let root = tempfile::tempdir().unwrap();
		let pages = root.path().join("wasm-dist");
		std::fs::create_dir(&pages).unwrap();
		std::fs::write(
			pages.join("app.js"),
			"export default ({module_or_path}) => module_or_path;\nconst wasm = new URL('app_bg.wasm', import.meta.url);",
		)
		.unwrap();
		std::fs::write(pages.join("app_bg.wasm"), b"\0asm\x01\0\0\0").unwrap();
		std::fs::write(
			root.path().join("index.html"),
			"<!doctype html><script type=\"module\" src=\"app.js\"></script>",
		)
		.unwrap();
		let output = root.path().join("static");
		let command = BuildStaticCommand::new(StaticAssetSettings {
			static_root: output.clone(),
			static_url: "/static/".into(),
			staticfiles_dirs: Vec::new(),
		});
		let mut request = BuildStaticRequest::new(root.path().into());
		request.pages = Some(PagesSource::Directory {
			directory: pages,
			entry: "app.js".into(),
		});
		request.pages_document = Some("index.html".into());

		// Act
		let result = command.execute(request).unwrap();

		// Assert
		let BuildStaticResult::Published(snapshot) = result else {
			panic!("expected a published generation");
		};
		assert_eq!(
			snapshot.manifest().entrypoints["default"]
				.document
				.as_deref(),
			Some("index.html")
		);
		assert!(snapshot.manifest().paths["index.html"].ends_with("/other/index.html"));
		assert!(output.join("manifest.json").is_file());
	}
}
