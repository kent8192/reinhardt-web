//! Compile canonical component style definitions from one Cargo package.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use cargo_metadata::{CargoOpt, DependencyKind, Metadata, MetadataCommand, Package, PackageId};
use quote::{ToTokens, quote};
use reinhardt_manouche::{CompiledStyle, StyleCompileContext, compile_style, serialize_css};
use sha2::{Digest, Sha256};
use syn::visit::Visit;
use syn::visit_mut::VisitMut;
use syn::{
	Attribute, Expr, Item, ItemConst, ItemFn, ItemMacro, ItemMod, ItemStatic, Lit, LitStr, Meta,
	StaticMutability, Type,
};

/// Stable logical path used for generated component CSS.
pub const COMPONENT_STYLES_PATH: &str = "__reinhardt__/components.css";

/// Cargo feature selection used when extracting Pages component styles.
///
/// The selection must match the package features enabled for the WASM Pages
/// build so `cfg(feature = "...")` definitions produce the same generated
/// API and stylesheet in both pipelines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyleFeatureSelection {
	features: Vec<String>,
	all_features: bool,
}

impl StyleFeatureSelection {
	/// Select an explicit set of Cargo package features.
	pub fn with_features<I, S>(features: I) -> Self
	where
		I: IntoIterator<Item = S>,
		S: Into<String>,
	{
		let mut features: Vec<String> = features.into_iter().map(Into::into).collect();
		features.sort();
		features.dedup();
		Self {
			features,
			all_features: false,
		}
	}

	/// Select every Cargo package feature.
	pub fn all_features() -> Self {
		Self {
			features: Vec::new(),
			all_features: true,
		}
	}

	/// Return the explicit Cargo features selected for the Pages build.
	pub fn features(&self) -> &[String] {
		&self.features
	}

	/// Return whether the Pages build enables every Cargo feature.
	pub fn all_features_enabled(&self) -> bool {
		self.all_features
	}

	fn apply_to_metadata(&self, command: &mut MetadataCommand, selected_package: &Package) {
		let selected_package_name = selected_package.name.to_string();
		let selected_features: Vec<String> = if self.all_features {
			selected_package.features.keys().cloned().collect()
		} else {
			self.features.clone()
		};
		let features: Vec<String> = selected_features
			.iter()
			.map(|feature| {
				if feature.contains('/') {
					feature.clone()
				} else {
					format!("{selected_package_name}/{feature}")
				}
			})
			.collect();
		if !features.is_empty() {
			command.features(CargoOpt::SomeFeatures(features));
		}
	}
}

/// The selected Cargo package used by every component-style subsystem.
#[derive(Debug, Clone)]
pub struct StylePackageContext {
	/// Selected Cargo package identifier.
	pub package_id: PackageId,
	/// Cargo package name.
	pub package_name: String,
	/// Cargo-resolved package version.
	pub package_version: String,
	/// Selected package manifest path.
	pub package_manifest_path: PathBuf,
	/// Workspace manifest path.
	pub workspace_manifest_path: PathBuf,
	/// Selected package source root.
	pub src_root: PathBuf,
	/// Cargo library target name used for the selected Pages WASM bundle.
	wasm_target_name: String,
	/// Cargo library targets compiled into the selected Pages WASM bundle.
	source_roots: Vec<StyleSourceRoot>,
}

/// One local Cargo package target that can contribute component styles.
#[derive(Debug, Clone)]
struct StyleSourceRoot {
	package_name: String,
	package_version: String,
	package_root: PathBuf,
	source_path: PathBuf,
	cfg: CfgEvaluator,
}

impl StylePackageContext {
	/// Return every local package root that contributes code to the Pages bundle.
	///
	/// The result includes the selected package and enabled local path dependencies,
	/// matching the style extraction graph used for generated CSS.
	pub fn source_package_roots(&self) -> impl Iterator<Item = &Path> {
		self.source_roots
			.iter()
			.map(|source| source.package_root.as_path())
	}

	/// Return the selected package's cdylib target name used by wasm-bindgen.
	pub fn wasm_target_name(&self) -> &str {
		&self.wasm_target_name
	}

	/// Select a package from already loaded Cargo metadata.
	pub fn from_metadata(
		metadata: &Metadata,
		requested_package: Option<&str>,
	) -> Result<Self, String> {
		let package = select_workspace_package(metadata, requested_package)?;

		let package_manifest_path = package.manifest_path.clone().into_std_path_buf();
		let src_root = package_manifest_path
			.parent()
			.ok_or_else(|| "selected package manifest has no parent directory".to_string())?
			.join("src");
		let source_roots = style_source_roots(metadata, package)?;
		let wasm_target_name = package
			.targets
			.iter()
			.find(|target| {
				(target.is_cdylib() || target.is_lib())
					&& target
						.crate_types
						.iter()
						.any(|kind| matches!(kind, cargo_metadata::CrateType::CDyLib))
			})
			.or_else(|| package.targets.iter().find(|target| target.is_cdylib()))
			.or_else(|| package.targets.iter().find(|target| target.is_lib()))
			.map(|target| target.name.to_string())
			.unwrap_or_else(|| package.name.to_string());
		Ok(Self {
			package_id: package.id.clone(),
			package_name: package.name.to_string(),
			package_version: package.version.to_string(),
			package_manifest_path,
			workspace_manifest_path: metadata
				.workspace_root
				.join("Cargo.toml")
				.into_std_path_buf(),
			src_root,
			wasm_target_name,
			source_roots,
		})
	}

	/// Load Cargo metadata and select exactly one package.
	pub fn resolve(
		manifest_path: impl AsRef<Path>,
		requested_package: Option<&str>,
	) -> Result<Self, String> {
		Self::resolve_with_features(
			manifest_path,
			requested_package,
			StyleFeatureSelection::default(),
		)
	}

	/// Load Cargo metadata with the feature selection used for the Pages build.
	pub fn resolve_with_features(
		manifest_path: impl AsRef<Path>,
		requested_package: Option<&str>,
		feature_selection: StyleFeatureSelection,
	) -> Result<Self, String> {
		let manifest_path = manifest_path.as_ref();
		let initial_metadata = load_wasm_metadata(manifest_path)?;
		if !feature_selection.all_features && feature_selection.features.is_empty() {
			return Self::from_metadata(&initial_metadata, requested_package);
		}

		let selected_package = select_workspace_package(&initial_metadata, requested_package)?;
		let selected_package_name = selected_package.name.to_string();
		let mut command = wasm_metadata_command(manifest_path);
		feature_selection.apply_to_metadata(&mut command, selected_package);
		let metadata = command
			.exec()
			.map_err(|error| format!("failed to load Cargo metadata: {error}"))?;
		Self::from_metadata(&metadata, Some(&selected_package_name))
	}
}

fn select_workspace_package<'a>(
	metadata: &'a Metadata,
	requested_package: Option<&str>,
) -> Result<&'a Package, String> {
	if let Some(requested) = requested_package {
		let matches: Vec<&Package> = metadata
			.workspace_packages()
			.into_iter()
			.filter(|package| package.name.as_str() == requested)
			.collect();
		match matches.as_slice() {
			[package] => Ok(*package),
			[] => Err(format!("Cargo package `{requested}` was not found")),
			_ => Err(format!(
				"Cargo package name `{requested}` is ambiguous; select a unique package"
			)),
		}
	} else {
		metadata.root_package().ok_or_else(|| {
			"the Cargo workspace has no root package; pass --package <NAME>".to_string()
		})
	}
}

fn wasm_metadata_command(manifest_path: &Path) -> MetadataCommand {
	let mut command = MetadataCommand::new();
	command.manifest_path(manifest_path);
	// Cargo still requires a valid working directory with an explicit manifest.
	// Pin it to the selected package so concurrent callers cannot leak a removed cwd.
	if let Some(package_root) = manifest_path
		.parent()
		.filter(|path| !path.as_os_str().is_empty())
	{
		command.current_dir(package_root);
	}
	command.other_options(vec![
		"--filter-platform".to_string(),
		"wasm32-unknown-unknown".to_string(),
	]);
	command
}

fn load_wasm_metadata(manifest_path: &Path) -> Result<Metadata, String> {
	wasm_metadata_command(manifest_path)
		.exec()
		.map_err(|error| format!("failed to load Cargo metadata: {error}"))
}

fn style_source_roots(
	metadata: &Metadata,
	selected_package: &Package,
) -> Result<Vec<StyleSourceRoot>, String> {
	let wasm_target = CfgEvaluator::new(BTreeSet::new())?.target;
	let resolve = metadata
		.resolve
		.as_ref()
		.ok_or_else(|| "Cargo metadata did not include a dependency graph".to_string())?;
	let mut pending = vec![selected_package.id.clone()];
	let mut visited = HashSet::new();
	let mut roots = Vec::new();

	while let Some(package_id) = pending.pop() {
		if !visited.insert(package_id.clone()) {
			continue;
		}
		let node = resolve
			.nodes
			.iter()
			.find(|node| node.id == package_id)
			.ok_or_else(|| format!("Cargo metadata omitted dependency node `{package_id}`"))?;
		for dependency in &node.deps {
			if dependency_is_enabled_for_wasm(dependency, &wasm_target) {
				pending.push(dependency.pkg.clone());
			}
		}

		let package = metadata
			.packages
			.iter()
			.find(|package| package.id == package_id)
			.ok_or_else(|| format!("Cargo metadata omitted package `{package_id}`"))?;
		if package.id != selected_package.id && package.source.is_some() {
			continue;
		}

		let enabled_features: BTreeSet<String> =
			node.features.iter().map(ToString::to_string).collect();
		let cfg = CfgEvaluator {
			target: wasm_target.clone(),
			features: enabled_features.clone(),
		};
		let package_root = package
			.manifest_path
			.parent()
			.ok_or_else(|| format!("package manifest has no parent: {}", package.manifest_path))?
			.to_path_buf()
			.into_std_path_buf();
		for target in package.targets.iter().filter(|target| {
			(target.is_lib() || target.is_cdylib())
				&& target
					.required_features
					.iter()
					.all(|feature| enabled_features.contains(feature))
		}) {
			roots.push(StyleSourceRoot {
				package_name: package.name.to_string(),
				package_version: package.version.to_string(),
				package_root: package_root.clone(),
				source_path: target.src_path.clone().into_std_path_buf(),
				cfg: cfg.clone(),
			});
		}
	}

	roots.sort_by(|left, right| {
		(
			left.package_name.as_str(),
			left.package_version.as_str(),
			left.source_path.as_os_str(),
		)
			.cmp(&(
				right.package_name.as_str(),
				right.package_version.as_str(),
				right.source_path.as_os_str(),
			))
	});
	roots.dedup_by(|left, right| left.source_path == right.source_path);
	Ok(roots)
}

fn dependency_is_enabled_for_wasm(
	dependency: &cargo_metadata::NodeDep,
	wasm_target: &CfgTarget,
) -> bool {
	dependency.dep_kinds.is_empty()
		|| dependency.dep_kinds.iter().any(|kind| {
			kind.kind == DependencyKind::Normal
				&& kind
					.target
					.as_ref()
					.is_none_or(|target| wasm_target.matches_platform(target))
		})
}

/// One canonical style definition and its checked compiler output.
#[derive(Debug, Clone)]
pub struct ExtractedStyleDefinition {
	/// Authored generated type name.
	pub style_type_name: String,
	/// Source file containing the definition.
	pub source_path: PathBuf,
	/// One-based source line.
	pub line: usize,
	/// One-based source column.
	pub column: usize,
	/// Shared semantic compiler output.
	pub compiled: CompiledStyle,
}

/// Independent fingerprints used by the development rebuild pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyleFingerprints {
	/// Rust source with style bodies replaced by a stable marker.
	pub non_style_rust: [u8; 32],
	/// Generated Rust API records.
	pub generated_api: [u8; 32],
	/// Final deterministic CSS bytes.
	pub css: [u8; 32],
}

/// Deterministic package-wide component stylesheet.
#[derive(Debug, Clone)]
pub struct StyleBundle {
	/// Serialized CSS bytes.
	pub css: Vec<u8>,
	/// Definitions in stable package/type order.
	pub definitions: Vec<ExtractedStyleDefinition>,
	/// Rebuild fingerprints for this source revision.
	pub fingerprints: StyleFingerprints,
}

/// Extracts and compiles component styles from a selected package.
#[derive(Debug, Clone)]
pub struct StyleExtractor {
	context: StylePackageContext,
}

impl StyleExtractor {
	/// Create an extractor using one previously resolved package context.
	pub fn new(context: StylePackageContext) -> Self {
		Self { context }
	}

	/// Return the selected package context.
	pub fn context(&self) -> &StylePackageContext {
		&self.context
	}

	/// Return whether `path` is a Rust source file scanned for component styles.
	///
	/// A failed scan is treated as not tracked so callers conservatively rebuild
	/// rather than skipping a reload for an uncertain path.
	#[cfg(feature = "pages")]
	pub(crate) fn tracks_source_path(&self, path: &Path) -> bool {
		self.context.source_roots.iter().any(|source_root| {
			source_files(source_root).is_ok_and(|source_paths| {
				source_paths.iter().any(|source_path| source_path == path)
			})
		})
	}

	/// Discover canonical definitions, compile them, and build stable outputs.
	pub fn extract(&self) -> Result<StyleBundle, String> {
		let mut definitions = Vec::new();
		let mut non_style_hasher = Sha256::new();

		for source_root in &self.context.source_roots {
			let source_files = source_files(source_root)?;
			for source_path in source_files {
				let source = std::fs::read_to_string(&source_path).map_err(|error| {
					format!("failed to read {}: {error}", source_path.display())
				})?;
				let file = syn::parse_file(&source).map_err(|error| {
					format!("failed to parse {}: {error}", source_path.display())
				})?;
				if !source_root.cfg.items_are_enabled(&file.attrs) {
					continue;
				}
				let mut scanner = DefinitionScanner::new(&source_path, &source_root.cfg);
				scanner.visit_file(&file);
				if let Some(error) = scanner.error {
					return Err(error);
				}
				for authored in scanner.definitions {
					let compile_context = StyleCompileContext {
						package_name: &source_root.package_name,
						package_version: &source_root.package_version,
						style_type_name: &authored.style_type_name,
					};
					let compiled =
						compile_style(authored.tokens, &compile_context).map_err(|error| {
							format!(
								"{}:{}:{}: {error}",
								source_path.display(),
								authored.line,
								authored.column
							)
						})?;
					definitions.push(ExtractedStyleDefinition {
						style_type_name: authored.style_type_name,
						source_path: source_path.clone(),
						line: authored.line,
						column: authored.column,
						compiled,
					});
				}

				let mut normalized_file = file;
				StyleBodyMarker {
					cfg: &source_root.cfg,
				}
				.visit_file_mut(&mut normalized_file);
				let relative = source_path
					.strip_prefix(&source_root.package_root)
					.unwrap_or(&source_path);
				non_style_hasher.update(source_root.package_name.as_bytes());
				non_style_hasher.update([0]);
				non_style_hasher.update(source_root.package_version.as_bytes());
				non_style_hasher.update([0]);
				non_style_hasher.update(relative.to_string_lossy().replace('\\', "/").as_bytes());
				non_style_hasher.update([0]);
				non_style_hasher.update(normalized_file.into_token_stream().to_string().as_bytes());
				non_style_hasher.update([0]);
			}
		}

		definitions.sort_by(|left, right| {
			left.compiled
				.scope
				.identity
				.cmp(&right.compiled.scope.identity)
		});
		validate_scopes(&definitions)?;

		let mut css = Vec::new();
		for definition in &definitions {
			css.extend_from_slice(serialize_css(&definition.compiled.css).as_bytes());
		}
		let fingerprints = StyleFingerprints {
			non_style_rust: non_style_hasher.finalize().into(),
			generated_api: generated_api_fingerprint(&definitions),
			css: Sha256::digest(&css).into(),
		};

		Ok(StyleBundle {
			css,
			definitions,
			fingerprints,
		})
	}
}

fn source_files(source_root: &StyleSourceRoot) -> Result<Vec<PathBuf>, String> {
	let mut files = BTreeSet::new();
	if !source_root.source_path.is_file() {
		return Ok(Vec::new());
	}
	let module_dir = source_root.source_path.parent().ok_or_else(|| {
		format!(
			"Cargo target root has no parent directory: {}",
			source_root.source_path.display()
		)
	})?;
	collect_source_file(
		&source_root.source_path,
		module_dir,
		&source_root.cfg,
		&mut files,
	)?;
	Ok(files.into_iter().collect())
}

fn collect_source_file(
	source_path: &Path,
	module_dir: &Path,
	cfg: &CfgEvaluator,
	files: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
	let source_path = source_path.to_path_buf();
	if files.contains(&source_path) {
		return Ok(());
	}
	let source = std::fs::read_to_string(&source_path)
		.map_err(|error| format!("failed to read {}: {error}", source_path.display()))?;
	let file = syn::parse_file(&source)
		.map_err(|error| format!("failed to parse {}: {error}", source_path.display()))?;
	if !cfg.items_are_enabled(&file.attrs) {
		return Ok(());
	}
	files.insert(source_path.clone());
	collect_module_items(&file.items, module_dir, &source_path, cfg, files)
}

fn collect_module_items(
	items: &[Item],
	module_dir: &Path,
	source_path: &Path,
	cfg: &CfgEvaluator,
	files: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
	for item in items {
		match item {
			Item::Mod(module) => {
				if !cfg.items_are_enabled(&module.attrs) {
					continue;
				}
				if let Some((_, nested_items)) = &module.content {
					collect_module_items(
						nested_items,
						&module_dir.join(module.ident.to_string()),
						source_path,
						cfg,
						files,
					)?;
					continue;
				}

				let module_source = external_module_source_path(module_dir, module, cfg)?;
				let nested_module_dir =
					module_directory(&module_source.path, module_source.has_path_attribute)?;
				collect_source_file(&module_source.path, &nested_module_dir, cfg, files)?;
			}
			Item::Macro(item_macro) if cfg.items_are_enabled(&item_macro.attrs) => {
				if let Some(include_path) = include_source_path(source_path, item_macro)? {
					let included_module_dir = include_path.parent().ok_or_else(|| {
						format!(
							"included source has no parent directory: {}",
							include_path.display()
						)
					})?;
					collect_source_file(&include_path, included_module_dir, cfg, files)?;
				}
			}
			_ => {}
		}
	}
	Ok(())
}

fn include_source_path(source_path: &Path, item: &ItemMacro) -> Result<Option<PathBuf>, String> {
	if item.mac.path.segments.len() != 1 || !item.mac.path.is_ident("include") {
		return Ok(None);
	}
	let Ok(included) = syn::parse2::<LitStr>(item.mac.tokens.clone()) else {
		return Ok(None);
	};
	let parent = source_path.parent().ok_or_else(|| {
		format!(
			"included source has no parent directory: {}",
			source_path.display()
		)
	})?;
	let resolved = parent.join(included.value());
	if !resolved.is_file() {
		return Err(format!(
			"{}: include! references missing source file {}",
			source_path.display(),
			resolved.display()
		));
	}
	Ok(Some(resolved))
}

/// One resolved external module source and its child-module resolution mode.
struct ModuleSource {
	path: PathBuf,
	has_path_attribute: bool,
}

fn external_module_source_path(
	module_dir: &Path,
	module: &ItemMod,
	cfg: &CfgEvaluator,
) -> Result<ModuleSource, String> {
	if let Some(path) = module_path_attribute(module, cfg)? {
		let resolved = module_dir.join(path);
		if resolved.is_file() {
			return Ok(ModuleSource {
				path: resolved,
				has_path_attribute: true,
			});
		}
		return Err(format!(
			"module `{}` references missing source file {}",
			module.ident,
			resolved.display()
		));
	}

	let flat = module_dir.join(format!("{}.rs", module.ident));
	let legacy = module_dir.join(module.ident.to_string()).join("mod.rs");
	match (flat.is_file(), legacy.is_file()) {
		(true, false) => Ok(ModuleSource {
			path: flat,
			has_path_attribute: false,
		}),
		(false, true) => Ok(ModuleSource {
			path: legacy,
			has_path_attribute: false,
		}),
		(false, false) => Err(format!(
			"module `{}` has no source file at {} or {}",
			module.ident,
			flat.display(),
			legacy.display()
		)),
		(true, true) => Err(format!(
			"module `{}` has ambiguous source files at {} and {}",
			module.ident,
			flat.display(),
			legacy.display()
		)),
	}
}

fn module_path_attribute(module: &ItemMod, cfg: &CfgEvaluator) -> Result<Option<PathBuf>, String> {
	let mut selected = None;
	for attribute in &module.attrs {
		let Some(path) = active_path_attribute(&attribute.meta, cfg, module)? else {
			continue;
		};
		if selected.replace(path).is_some() {
			return Err(format!(
				"module `{}` has multiple active path attributes",
				module.ident
			));
		}
	}
	Ok(selected)
}

fn active_path_attribute(
	meta: &Meta,
	cfg: &CfgEvaluator,
	module: &ItemMod,
) -> Result<Option<PathBuf>, String> {
	if meta.path().is_ident("path") {
		let Meta::NameValue(name_value) = meta else {
			return Err(format!(
				"module `{}` has an invalid path attribute",
				module.ident
			));
		};
		let Expr::Lit(expression) = &name_value.value else {
			return Err(format!(
				"module `{}` has an invalid path attribute",
				module.ident
			));
		};
		let Lit::Str(path) = &expression.lit else {
			return Err(format!(
				"module `{}` has an invalid path attribute",
				module.ident
			));
		};
		return Ok(Some(PathBuf::from(path.value())));
	}
	if !meta.path().is_ident("cfg_attr") {
		return Ok(None);
	}

	let Meta::List(list) = meta else {
		return Err(format!(
			"module `{}` has an invalid cfg_attr path attribute",
			module.ident
		));
	};
	let arguments = list
		.parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
		.map_err(|_| {
			format!(
				"module `{}` has an invalid cfg_attr path attribute",
				module.ident
			)
		})?;
	let Some(condition) = arguments.first() else {
		return Err(format!(
			"module `{}` has an invalid cfg_attr path attribute",
			module.ident
		));
	};
	if !cfg.predicate_is_enabled(condition) {
		return Ok(None);
	}

	let mut selected = None;
	for nested in arguments.iter().skip(1) {
		let Some(path) = active_path_attribute(nested, cfg, module)? else {
			continue;
		};
		if selected.replace(path).is_some() {
			return Err(format!(
				"module `{}` has multiple active path attributes",
				module.ident
			));
		}
	}
	Ok(selected)
}

fn module_directory(source_path: &Path, has_path_attribute: bool) -> Result<PathBuf, String> {
	let parent = source_path.parent().ok_or_else(|| {
		format!(
			"module source has no parent directory: {}",
			source_path.display()
		)
	})?;
	if has_path_attribute || source_path.file_name().is_some_and(|name| name == "mod.rs") {
		return Ok(parent.to_path_buf());
	}
	let stem = source_path
		.file_stem()
		.ok_or_else(|| format!("module source has no file stem: {}", source_path.display()))?;
	Ok(parent.join(stem))
}

/// Evaluates `cfg` predicates for declarations compiled into the Pages WASM target and build profile.
#[derive(Debug, Clone)]
struct CfgEvaluator {
	target: CfgTarget,
	features: BTreeSet<String>,
}

/// One target-specific Rust compiler configuration.
#[derive(Debug, Clone)]
struct CfgTarget {
	flags: BTreeSet<String>,
	key_values: BTreeMap<String, BTreeSet<String>>,
}

const WASM_TARGET: &str = "wasm32-unknown-unknown";

impl CfgEvaluator {
	fn new(features: BTreeSet<String>) -> Result<Self, String> {
		Self::with_debug_assertions(features, cfg!(debug_assertions))
	}

	fn with_debug_assertions(
		features: BTreeSet<String>,
		debug_assertions: bool,
	) -> Result<Self, String> {
		let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
		let target = CfgTarget::from_rustc(&rustc, WASM_TARGET, debug_assertions)?;
		Ok(Self { target, features })
	}

	fn items_are_enabled(&self, attributes: &[Attribute]) -> bool {
		self.target.items_are_enabled(attributes, &self.features)
	}

	fn predicate_is_enabled(&self, predicate: &Meta) -> bool {
		self.target.predicate_is_enabled(predicate, &self.features)
	}

	fn active_style_def_attributes(&self, attributes: &[Attribute]) -> Vec<Meta> {
		let mut active = Vec::new();
		for attribute in attributes {
			self.collect_active_attributes(&attribute.meta, &mut active);
		}
		active.into_iter().filter(is_style_def_attribute).collect()
	}

	fn collect_active_attributes(&self, meta: &Meta, active: &mut Vec<Meta>) {
		if !meta.path().is_ident("cfg_attr") {
			active.push(meta.clone());
			return;
		}
		let Meta::List(list) = meta else {
			return;
		};
		let Ok(arguments) = list
			.parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
		else {
			return;
		};
		let Some(condition) = arguments.first() else {
			return;
		};
		if self.predicate_is_enabled(condition) {
			for generated in arguments.iter().skip(1) {
				self.collect_active_attributes(generated, active);
			}
		}
	}
}

impl CfgTarget {
	fn matches_platform(&self, platform: &cargo_metadata::cargo_platform::Platform) -> bool {
		platform.matches(WASM_TARGET, &self.cargo_platform_cfgs())
	}

	fn cargo_platform_cfgs(&self) -> Vec<cargo_metadata::cargo_platform::Cfg> {
		let mut cfgs = Vec::new();
		for flag in &self.flags {
			// These aliases are used while scanning Pages source, but Cargo does
			// not expose them as target cfgs for dependency predicates.
			if matches!(flag.as_str(), "client" | "wasm") {
				continue;
			}
			if let Ok(cfg) = flag.parse() {
				cfgs.push(cfg);
			}
		}
		for (key, values) in &self.key_values {
			for value in values {
				if let Ok(cfg) = format!("{key} = {value:?}").parse() {
					cfgs.push(cfg);
				}
			}
		}
		cfgs
	}

	fn from_rustc(
		rustc: &std::ffi::OsStr,
		target: &str,
		debug_assertions: bool,
	) -> Result<Self, String> {
		let mut command = Command::new(rustc);
		// `rustc --print cfg` does not need project files. Use a stable directory
		// instead of inheriting a process cwd that another caller may remove.
		command.current_dir(std::env::temp_dir());
		command.args(["--print", "cfg", "--target", target]);
		if !debug_assertions {
			command.args(["-C", "debug-assertions=no"]);
		}
		let output = command.output().map_err(|error| {
			format!("failed to query Rust compiler configuration for {target}: {error}")
		})?;
		if !output.status.success() {
			return Err(format!(
				"failed to query Rust compiler configuration for {target}: {}",
				String::from_utf8_lossy(&output.stderr).trim()
			));
		}

		let mut flags = BTreeSet::new();
		let mut key_values: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
		for line in String::from_utf8_lossy(&output.stdout).lines() {
			let line = line.trim();
			if let Some((key, raw_value)) = line.split_once('=') {
				let Ok(value) = syn::parse_str::<LitStr>(raw_value) else {
					continue;
				};
				key_values
					.entry(key.to_string())
					.or_default()
					.insert(value.value());
			} else if !line.is_empty() {
				flags.insert(line.to_string());
			}
		}

		// Pages templates expose these aliases from their build scripts for
		// client-only modules. Component CSS always mirrors the WASM build, so
		// evaluate the aliases alongside the built-in WASM target cfgs.
		if target == "wasm32-unknown-unknown" {
			flags.insert("client".to_string());
			flags.insert("wasm".to_string());
		}

		Ok(Self { flags, key_values })
	}

	fn items_are_enabled(&self, attributes: &[Attribute], features: &BTreeSet<String>) -> bool {
		attributes
			.iter()
			.all(|attribute| self.attribute_is_enabled(attribute, features))
	}

	fn attribute_is_enabled(&self, attribute: &Attribute, features: &BTreeSet<String>) -> bool {
		if attribute.path().is_ident("cfg") {
			return attribute
				.parse_args::<Meta>()
				.map(|predicate| self.predicate_is_enabled(&predicate, features))
				.unwrap_or(false);
		}
		if attribute.path().is_ident("cfg_attr") {
			return self.cfg_attr_is_enabled(&attribute.meta, features);
		}
		true
	}

	fn cfg_attr_is_enabled(&self, meta: &Meta, features: &BTreeSet<String>) -> bool {
		let Meta::List(list) = meta else {
			return false;
		};
		let Ok(arguments) = list
			.parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
		else {
			return false;
		};
		let Some(condition) = arguments.first() else {
			return false;
		};
		if !self.predicate_is_enabled(condition, features) {
			return true;
		}
		arguments
			.iter()
			.skip(1)
			.all(|nested| self.generated_attribute_is_enabled(nested, features))
	}

	fn generated_attribute_is_enabled(&self, meta: &Meta, features: &BTreeSet<String>) -> bool {
		if meta.path().is_ident("cfg") {
			let Meta::List(list) = meta else {
				return false;
			};
			return list
				.parse_args::<Meta>()
				.map(|predicate| self.predicate_is_enabled(&predicate, features))
				.unwrap_or(false);
		}
		if meta.path().is_ident("cfg_attr") {
			return self.cfg_attr_is_enabled(meta, features);
		}
		true
	}

	fn predicate_is_enabled(&self, predicate: &Meta, features: &BTreeSet<String>) -> bool {
		match predicate {
			Meta::Path(path) => path
				.get_ident()
				.is_some_and(|ident| self.flags.contains(&ident.to_string())),
			Meta::NameValue(name_value) => {
				let Some(key) = name_value.path.get_ident() else {
					return false;
				};
				let Expr::Lit(expression) = &name_value.value else {
					return false;
				};
				let Lit::Str(value) = &expression.lit else {
					return false;
				};
				if key == "feature" {
					features.contains(&value.value())
				} else {
					self.key_values
						.get(&key.to_string())
						.is_some_and(|values| values.contains(&value.value()))
				}
			}
			Meta::List(list) => {
				let Some(operator) = list.path.get_ident() else {
					return false;
				};
				let Ok(predicates) = list.parse_args_with(
					syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
				) else {
					return false;
				};
				match operator.to_string().as_str() {
					"all" => predicates
						.iter()
						.all(|predicate| self.predicate_is_enabled(predicate, features)),
					"any" => predicates
						.iter()
						.any(|predicate| self.predicate_is_enabled(predicate, features)),
					"not" if predicates.len() == 1 => {
						!self.predicate_is_enabled(&predicates[0], features)
					}
					_ => false,
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::fs;
	use std::path::PathBuf;

	fn write_test_package(root: &Path, source: &str) -> PathBuf {
		fs::create_dir_all(root.join("src")).expect("create source directory");
		fs::write(
			root.join("Cargo.toml"),
			"[package]\nname = \"style-test-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
		)
		.expect("write package manifest");
		fs::write(root.join("src/lib.rs"), source).expect("write package source");
		root.join("Cargo.toml")
	}

	#[test]
	fn wasm_release_cfgs_enable_not_debug_assertions_items() {
		let evaluator = CfgEvaluator::with_debug_assertions(BTreeSet::new(), false)
			.expect("load release WASM cfgs");
		let release_only: ItemStatic =
			syn::parse_str("#[cfg(not(debug_assertions))] static RELEASE: () = ();")
				.expect("parse release-only static");
		let debug_only: ItemStatic =
			syn::parse_str("#[cfg(debug_assertions)] static DEBUG: () = ();")
				.expect("parse debug-only static");

		assert!(evaluator.items_are_enabled(&release_only.attrs));
		assert!(!evaluator.items_are_enabled(&debug_only.attrs));
	}

	#[test]
	fn source_collection_ignores_nonliteral_include_expressions() {
		// Arrange
		let directory = tempfile::tempdir().expect("create temporary source tree");
		let source_root = directory.path().join("src");
		fs::create_dir_all(&source_root).expect("create source root");
		let source_path = source_root.join("lib.rs");
		fs::write(
			&source_path,
			"include!(concat!(env!(\"OUT_DIR\"), \"/bindings.rs\"));\n",
		)
		.expect("write source");
		let evaluator = CfgEvaluator::new(BTreeSet::new()).expect("load WASM cfgs");
		let mut files = BTreeSet::new();

		// Act
		collect_source_file(&source_path, &source_root, &evaluator, &mut files)
			.expect("nonliteral include should not prevent style extraction");

		// Assert
		assert_eq!(files, BTreeSet::from([source_path]));
	}

	#[test]
	fn source_collection_resolves_submodules_relative_to_included_file() {
		// Arrange
		let directory = tempfile::tempdir().expect("create temporary source tree");
		let source_root = directory.path().join("src");
		let nested_root = source_root.join("nested");
		fs::create_dir_all(&nested_root).expect("create nested source root");
		let source_path = source_root.join("lib.rs");
		let included_path = nested_root.join("mods.rs");
		let child_path = nested_root.join("child.rs");
		fs::write(&source_path, "include!(\"nested/mods.rs\");\n").expect("write source");
		fs::write(&included_path, "mod child;\n").expect("write included source");
		fs::write(&child_path, "pub const CHILD: () = ();\n").expect("write child source");
		let evaluator = CfgEvaluator::new(BTreeSet::new()).expect("load WASM cfgs");
		let mut files = BTreeSet::new();

		// Act
		collect_source_file(&source_path, &source_root, &evaluator, &mut files)
			.expect("included child module should resolve");

		// Assert
		assert_eq!(
			files,
			BTreeSet::from([source_path, included_path, child_path])
		);
	}

	#[test]
	fn style_extractor_collects_local_path_dependency_definitions() {
		// Arrange
		let directory = tempfile::tempdir().expect("create temporary workspace");
		let app = directory.path().join("app");
		let shared = directory.path().join("shared");
		fs::create_dir_all(app.join("src")).expect("create app source root");
		fs::create_dir_all(shared.join("src")).expect("create shared source root");
		fs::write(
			directory.path().join("Cargo.toml"),
			"[workspace]\nresolver = \"3\"\nmembers = [\"app\", \"shared\"]\n",
		)
		.expect("write workspace manifest");
		fs::write(
			app.join("Cargo.toml"),
			"[package]\nname = \"style-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nstyle-shared = { path = \"../shared\" }\n",
		)
		.expect("write app manifest");
		fs::write(app.join("src/lib.rs"), "pub fn app() {}\n").expect("write app source");
		fs::write(
			shared.join("Cargo.toml"),
			"[package]\nname = \"style-shared\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
		)
		.expect("write shared manifest");
		fs::write(
			shared.join("src/lib.rs"),
			"#[style_def]\nstatic SHARED: SharedStyles = style! { .shared { color: red; } };\n",
		)
		.expect("write shared source");

		// Act
		let context = StylePackageContext::resolve(app.join("Cargo.toml"), None)
			.expect("resolve selected app package");
		let bundle = StyleExtractor::new(context)
			.extract()
			.expect("extract local dependency styles");

		// Assert
		assert_eq!(bundle.definitions.len(), 1);
		assert_eq!(bundle.definitions[0].style_type_name, "SharedStyles");
	}

	#[test]
	fn style_extractor_ignores_unannotated_bare_style_macros() {
		// Arrange
		let directory = tempfile::tempdir().expect("create temporary package");
		let package = directory.path();
		fs::create_dir_all(package.join("src")).expect("create source directory");
		fs::write(
			package.join("Cargo.toml"),
			"[package]\nname = \"foreign-style-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
		)
		.expect("write package manifest");
		fs::write(
			package.join("src/lib.rs"),
			"static FOREIGN: ForeignStyles = style! { foreign_tokens };\n",
		)
		.expect("write source");

		// Act
		let context = StylePackageContext::resolve(package.join("Cargo.toml"), None)
			.expect("resolve package");
		let bundle = StyleExtractor::new(context)
			.extract()
			.expect("foreign bare style macros must not be interpreted as component styles");

		// Assert
		assert!(bundle.definitions.is_empty());
		assert!(bundle.css.is_empty());
	}

	#[test]
	fn unannotated_style_macro_bodies_contribute_to_the_rust_fingerprint() {
		// Arrange
		let directory = tempfile::tempdir().expect("create temporary package");
		let manifest = write_test_package(
			directory.path(),
			concat!(
				"#[style_def]\nstatic STYLES: CardStyles = style! { .card { color: red; } };\n",
				"static FOREIGN: ForeignStyles = style! { foreign_tokens };\n",
			),
		);
		let context = StylePackageContext::resolve(&manifest, None).expect("select package");
		let first = StyleExtractor::new(context.clone())
			.extract()
			.expect("extract initial styles");

		// Act
		fs::write(
			directory.path().join("src/lib.rs"),
			concat!(
				"#[style_def]\nstatic STYLES: CardStyles = style! { .card { color: red; } };\n",
				"static FOREIGN: ForeignStyles = style! { changed_foreign_tokens };\n",
			),
		)
		.expect("change foreign style macro body");
		let changed = StyleExtractor::new(context)
			.extract()
			.expect("extract changed styles");

		// Assert
		assert_ne!(
			first.fingerprints.non_style_rust,
			changed.fingerprints.non_style_rust
		);
		assert_eq!(first.fingerprints.css, changed.fingerprints.css);
	}

	#[test]
	fn active_cfg_attr_style_bodies_do_not_change_the_rust_fingerprint() {
		// Arrange
		let directory = tempfile::tempdir().expect("create temporary package");
		fs::create_dir_all(directory.path().join("src")).expect("create source directory");
		fs::write(
			directory.path().join("Cargo.toml"),
			"[package]\nname = \"cfg-attr-style-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[features]\ntheme = []\n",
		)
		.expect("write package manifest");
		let source_path = directory.path().join("src/lib.rs");
		fs::write(
			&source_path,
			"#[cfg_attr(feature = \"theme\", style_def)]\nstatic STYLES: CardStyles = style! { .card { color: red; } };\n",
		)
		.expect("write initial source");
		let manifest = directory.path().join("Cargo.toml");
		let context = StylePackageContext::resolve_with_features(
			&manifest,
			None,
			StyleFeatureSelection::with_features(["theme"]),
		)
		.expect("select package with the theme feature");
		let first = StyleExtractor::new(context.clone())
			.extract()
			.expect("extract initial styles");

		// Act
		fs::write(
			&source_path,
			"#[cfg_attr(feature = \"theme\", style_def)]\nstatic STYLES: CardStyles = style! { .card { color: blue; } };\n",
		)
		.expect("change style body");
		let changed = StyleExtractor::new(context)
			.extract()
			.expect("extract changed styles");

		// Assert
		assert_eq!(
			first.fingerprints.non_style_rust,
			changed.fingerprints.non_style_rust
		);
		assert_ne!(first.fingerprints.css, changed.fingerprints.css);
	}

	#[test]
	fn extracted_scope_matches_the_macro_generated_style_type_identity() {
		// Arrange
		let directory = tempfile::tempdir().expect("create temporary package");
		let manifest = write_test_package(
			directory.path(),
			"#[style_def]\nstatic STYLES: CardStyles = style! { .card { color: red; } };\n",
		);

		// Act
		let context = StylePackageContext::resolve(&manifest, None).expect("select package");
		let bundle = StyleExtractor::new(context)
			.extract()
			.expect("extract component styles");

		// Assert
		assert_eq!(
			bundle.definitions[0].compiled.scope.identity,
			"rstyle-v2\0style-test-app\00.1.0\0CardStyles"
		);
	}

	#[test]
	fn duplicate_module_local_style_types_are_rejected() {
		// Arrange
		let directory = tempfile::tempdir().expect("create temporary package");
		let manifest = write_test_package(
			directory.path(),
			r#"
mod card {
    #[style_def]
    static STYLES: Styles = style! { .card { color: red; } };
}

mod modal {
    #[style_def]
    static STYLES: Styles = style! { .modal { color: blue; } };
}
"#,
		);

		// Act
		let context = StylePackageContext::resolve(&manifest, None).expect("select package");
		let error = StyleExtractor::new(context)
			.extract()
			.expect_err("duplicate generated style types must not share one scope");

		// Assert
		assert!(error.contains("duplicate component style scope identity"));
	}

	#[test]
	fn selected_package_features_do_not_enable_matching_dependency_features() {
		// Arrange
		let directory = tempfile::tempdir().expect("create temporary workspace");
		let app = directory.path().join("app");
		let shared = directory.path().join("shared");
		fs::create_dir_all(app.join("src")).expect("create app source root");
		fs::create_dir_all(shared.join("src")).expect("create shared source root");
		fs::write(
			directory.path().join("Cargo.toml"),
			"[workspace]\nresolver = \"3\"\nmembers = [\"app\", \"shared\"]\n",
		)
		.expect("write workspace manifest");
		fs::write(
			app.join("Cargo.toml"),
			"[package]\nname = \"style-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[features]\ntheme = []\n\n[dependencies]\nstyle-shared = { path = \"../shared\" }\n",
		)
		.expect("write app manifest");
		fs::write(
			app.join("src/lib.rs"),
			"#[cfg(feature = \"theme\")]\n#[style_def]\nstatic APP: AppStyles = style! { .app { color: blue; } };\n",
		)
		.expect("write app source");
		fs::write(
			shared.join("Cargo.toml"),
			"[package]\nname = \"style-shared\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[features]\ntheme = []\n",
		)
		.expect("write shared manifest");
		fs::write(
			shared.join("src/lib.rs"),
			"#[cfg(feature = \"theme\")]\n#[style_def]\nstatic SHARED: SharedStyles = style! { .shared { color: red; } };\n",
		)
		.expect("write shared source");

		// Act
		let context = StylePackageContext::resolve_with_features(
			app.join("Cargo.toml"),
			None,
			StyleFeatureSelection::with_features(["theme"]),
		)
		.expect("resolve selected app features");
		let bundle = StyleExtractor::new(context)
			.extract()
			.expect("extract selected package styles");

		// Assert
		assert_eq!(bundle.definitions.len(), 1);
		assert_eq!(bundle.definitions[0].style_type_name, "AppStyles");
	}

	#[test]
	fn selected_package_all_features_do_not_enable_matching_dependency_features() {
		// Arrange
		let directory = tempfile::tempdir().expect("create temporary workspace");
		let app = directory.path().join("app");
		let shared = directory.path().join("shared");
		fs::create_dir_all(app.join("src")).expect("create app source root");
		fs::create_dir_all(shared.join("src")).expect("create shared source root");
		fs::write(
			directory.path().join("Cargo.toml"),
			"[workspace]\nresolver = \"3\"\nmembers = [\"app\", \"shared\"]\n",
		)
		.expect("write workspace manifest");
		fs::write(
			app.join("Cargo.toml"),
			"[package]\nname = \"all-features-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[features]\ntheme = []\n\n[dependencies]\nall-features-shared = { path = \"../shared\" }\n",
		)
		.expect("write app manifest");
		fs::write(
			app.join("src/lib.rs"),
			"#[cfg(feature = \"theme\")]\n#[style_def]\nstatic APP: AppStyles = style! { .app { color: blue; } };\n",
		)
		.expect("write app source");
		fs::write(
			shared.join("Cargo.toml"),
			"[package]\nname = \"all-features-shared\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[features]\ntheme = []\n",
		)
		.expect("write shared manifest");
		fs::write(
			shared.join("src/lib.rs"),
			"#[cfg(feature = \"theme\")]\n#[style_def]\nstatic SHARED: SharedStyles = style! { .shared { color: red; } };\n",
		)
		.expect("write shared source");

		// Act
		let context = StylePackageContext::resolve_with_features(
			app.join("Cargo.toml"),
			None,
			StyleFeatureSelection::all_features(),
		)
		.expect("resolve selected app features");
		let bundle = StyleExtractor::new(context)
			.extract()
			.expect("extract selected package styles");

		// Assert
		assert_eq!(bundle.definitions.len(), 1);
		assert_eq!(bundle.definitions[0].style_type_name, "AppStyles");
	}

	#[test]
	fn style_extractor_ignores_files_disabled_by_an_inner_wasm_cfg() {
		// Arrange
		let directory = tempfile::tempdir().expect("create temporary package");
		let package = directory.path();
		fs::create_dir_all(package.join("src")).expect("create source directory");
		fs::write(
			package.join("Cargo.toml"),
			"[package]\nname = \"inner-cfg-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
		)
		.expect("write package manifest");
		fs::write(package.join("src/lib.rs"), "mod host_only;\n").expect("write library source");
		fs::write(
			package.join("src/host_only.rs"),
			"#![cfg(not(target_family = \"wasm\"))]\n#[style_def]\nstatic HOST: HostStyles = style! { .host { color: red; } };\n",
		)
		.expect("write host-only style source");

		// Act
		let context = StylePackageContext::resolve(package.join("Cargo.toml"), None)
			.expect("resolve package");
		let bundle = StyleExtractor::new(context)
			.extract()
			.expect("extract component styles");

		// Assert
		assert!(bundle.definitions.is_empty());
		assert!(bundle.css.is_empty());
	}

	#[test]
	fn style_extractor_ignores_host_only_normal_dependencies() {
		// Arrange
		let directory = tempfile::tempdir().expect("create temporary workspace");
		let app = directory.path().join("app");
		let host_only = directory.path().join("host-only");
		fs::create_dir_all(app.join("src")).expect("create app source directory");
		fs::create_dir_all(host_only.join("src")).expect("create dependency source directory");
		fs::write(
			directory.path().join("Cargo.toml"),
			"[workspace]\nresolver = \"3\"\nmembers = [\"app\", \"host-only\"]\n",
		)
		.expect("write workspace manifest");
		fs::write(
			app.join("Cargo.toml"),
			"[package]\nname = \"target-filter-app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nhost-only = { path = \"../host-only\" }\n",
		)
		.expect("write app manifest");
		fs::write(app.join("src/lib.rs"), "pub fn app() {}\n").expect("write app source");
		fs::write(
			host_only.join("Cargo.toml"),
			"[package]\nname = \"host-only\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
		)
		.expect("write dependency manifest");
		fs::write(
			host_only.join("src/lib.rs"),
			"#[style_def]\nstatic HOST: HostDependencyStyles = style! { .host { color: red; } };\n",
		)
		.expect("write dependency style source");

		// Act
		let metadata = load_wasm_metadata(&app.join("Cargo.toml")).expect("load app metadata");
		let host_only_id = metadata
			.packages
			.iter()
			.find(|package| package.name.as_str() == "host-only")
			.expect("find host-only package")
			.id
			.to_string();
		let mut metadata_json = serde_json::to_value(metadata).expect("serialize metadata");
		let app_node = metadata_json["resolve"]["nodes"]
			.as_array_mut()
			.expect("metadata resolve nodes")
			.iter_mut()
			.find(|node| {
				node["deps"].as_array().is_some_and(|dependencies| {
					dependencies
						.iter()
						.any(|dependency| dependency["pkg"] == host_only_id)
				})
			})
			.expect("find app dependency node");
		let dependency = app_node["deps"]
			.as_array_mut()
			.expect("app dependencies")
			.iter_mut()
			.find(|dependency| dependency["pkg"] == host_only_id)
			.expect("find host-only dependency");
		// Cargo may prefilter this edge on the local host. Recreate the
		// target-bearing resolved edge to exercise the metadata contract.
		dependency["dep_kinds"][0]["target"] =
			serde_json::Value::String("cfg(not(target_arch = \"wasm32\"))".to_string());
		let metadata = serde_json::from_value(metadata_json).expect("deserialize target metadata");
		let context = StylePackageContext::from_metadata(&metadata, Some("target-filter-app"))
			.expect("resolve app package for WASM");
		let bundle = StyleExtractor::new(context)
			.extract()
			.expect("extract component styles");

		// Assert
		assert!(bundle.definitions.is_empty());
		assert!(bundle.css.is_empty());
	}
}

#[derive(Debug)]
struct AuthoredDefinition {
	style_type_name: String,
	tokens: proc_macro2::TokenStream,
	line: usize,
	column: usize,
}

struct DefinitionScanner<'a> {
	source_path: &'a Path,
	cfg: &'a CfgEvaluator,
	definitions: Vec<AuthoredDefinition>,
	error: Option<String>,
}

impl<'a> DefinitionScanner<'a> {
	fn new(source_path: &'a Path, cfg: &'a CfgEvaluator) -> Self {
		Self {
			source_path,
			cfg,
			definitions: Vec::new(),
			error: None,
		}
	}

	fn reject(&mut self, span: proc_macro2::Span, reason: &str) {
		if self.error.is_none() {
			let location = span.start();
			self.error = Some(format!(
				"{}:{}:{}: invalid component style canonical envelope: {reason}",
				self.source_path.display(),
				location.line,
				location.column + 1
			));
		}
	}
}

fn is_style_def_attribute(meta: &Meta) -> bool {
	meta.path()
		.segments
		.last()
		.is_some_and(|segment| segment.ident == "style_def")
}

fn is_style_def_path(path: &syn::Path) -> bool {
	path.segments
		.last()
		.is_some_and(|segment| segment.ident == "style_def")
}

impl<'ast> Visit<'ast> for DefinitionScanner<'_> {
	fn visit_item_fn(&mut self, item: &'ast ItemFn) {
		if !self.cfg.items_are_enabled(&item.attrs) {
			return;
		}
		if !self.cfg.active_style_def_attributes(&item.attrs).is_empty() {
			self.reject(
				item.sig.fn_token.span,
				"the style_def attribute is supported only on immutable static items",
			);
			return;
		}
		syn::visit::visit_item_fn(self, item);
	}

	fn visit_item_mod(&mut self, item: &'ast ItemMod) {
		if !self.cfg.items_are_enabled(&item.attrs) {
			return;
		}
		if !self.cfg.active_style_def_attributes(&item.attrs).is_empty() {
			self.reject(
				item.mod_token.span,
				"the style_def attribute is supported only on immutable static items",
			);
			return;
		}
		syn::visit::visit_item_mod(self, item);
	}

	fn visit_item_static(&mut self, item: &'ast ItemStatic) {
		if !self.cfg.items_are_enabled(&item.attrs) {
			return;
		}
		let style_attributes = self.cfg.active_style_def_attributes(&item.attrs);
		let style_macro = match item.expr.as_ref() {
			Expr::Macro(expression) if expression.mac.path.is_ident("style") => {
				Some(&expression.mac)
			}
			_ => None,
		};
		if style_attributes.is_empty() {
			syn::visit::visit_item_static(self, item);
			return;
		}

		let exact_attribute = style_attributes.len() == 1
			&& matches!(&style_attributes[0], Meta::Path(path) if is_style_def_path(path));
		let immutable = matches!(item.mutability, StaticMutability::None);
		let style_type = match item.ty.as_ref() {
			Type::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
				Some(path.path.segments[0].ident.to_string())
			}
			_ => None,
		};
		let bare_style =
			style_macro.filter(|mac| mac.path.segments.len() == 1 && mac.path.is_ident("style"));
		if !exact_attribute || !immutable || style_type.is_none() || bare_style.is_none() {
			self.reject(
				item.static_token.span,
				"expected `#[style_def] static NAME: StyleType = style! { ... };`",
			);
			return;
		}

		let location = item.static_token.span.start();
		self.definitions.push(AuthoredDefinition {
			style_type_name: style_type.expect("validated one-segment style type"),
			tokens: bare_style
				.expect("validated bare style macro")
				.tokens
				.clone(),
			line: location.line,
			column: location.column + 1,
		});
		syn::visit::visit_item_static(self, item);
	}

	fn visit_item_const(&mut self, item: &'ast ItemConst) {
		if !self.cfg.items_are_enabled(&item.attrs) {
			return;
		}
		let has_style_attribute = !self.cfg.active_style_def_attributes(&item.attrs).is_empty();
		if has_style_attribute {
			self.reject(
				item.const_token.span,
				"component styles must use an immutable static item",
			);
			return;
		}
		syn::visit::visit_item_const(self, item);
	}
}

struct StyleBodyMarker<'a> {
	cfg: &'a CfgEvaluator,
}

impl VisitMut for StyleBodyMarker<'_> {
	fn visit_item_static_mut(&mut self, item: &mut ItemStatic) {
		let style_attributes = self.cfg.active_style_def_attributes(&item.attrs);
		if style_attributes.len() == 1
			&& matches!(&style_attributes[0], Meta::Path(path) if is_style_def_path(path))
			&& let Expr::Macro(expression) = item.expr.as_mut()
			&& expression.mac.path.segments.len() == 1
			&& expression.mac.path.is_ident("style")
		{
			expression.mac.tokens = quote! { __reinhardt_style_body_marker };
		}
		syn::visit_mut::visit_item_static_mut(self, item);
	}
}

fn validate_scopes(definitions: &[ExtractedStyleDefinition]) -> Result<(), String> {
	let mut identities = BTreeSet::new();
	let mut suffixes = BTreeMap::new();
	for definition in definitions {
		let scope = &definition.compiled.scope;
		if !identities.insert(scope.identity.clone()) {
			return Err(format!(
				"duplicate component style scope identity `{}`",
				scope.identity
			));
		}
		if let Some(existing) = suffixes.insert(scope.suffix.clone(), scope.identity.clone())
			&& existing != scope.identity
		{
			return Err(format!(
				"component style scope suffix collision `{}` between `{existing}` and `{}`",
				scope.suffix, scope.identity
			));
		}
	}
	Ok(())
}

fn generated_api_fingerprint(definitions: &[ExtractedStyleDefinition]) -> [u8; 32] {
	let mut hasher = Sha256::new();
	for definition in definitions {
		hasher.update(definition.compiled.scope.identity.as_bytes());
		hasher.update([0]);
		for class in &definition.compiled.classes {
			hasher.update(class.accessor.as_bytes());
			hasher.update([0]);
			hasher.update(class.css_name.as_bytes());
			hasher.update([0]);
		}
		for variable in &definition.compiled.variables {
			hasher.update(variable.authored_name.as_bytes());
			hasher.update([0]);
			hasher.update(variable.custom_property_name.as_bytes());
			hasher.update([0]);
			hasher.update(
				format!("{:?}:{}", variable.runtime_type, variable.source_index).as_bytes(),
			);
			hasher.update([0]);
		}
		hasher.update([255]);
	}
	hasher.finalize().into()
}
