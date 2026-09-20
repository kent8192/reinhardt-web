//! Captured inputs, processor lifecycle, and immutable generation preparation.

use super::classify::{AssetClassifier, validate_assignments};
use super::inventory::{build_id, digest_reader};
use super::manifest::encode_manifest;
use super::model::*;
use super::representations;
use super::rewrite::BuiltinProcessor;
use reinhardt_core::types::static_assets::validate_asset_path;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use tempfile::TempDir;

/// An asset's materialized source, captured before publication starts (P0).
#[derive(Debug, Clone)]
pub enum ContentSource {
	/// Owned generated or virtual content.
	Bytes(Vec<u8>),
	/// A file confined to its explicitly selected parent directory.
	File(PathBuf),
	/// A finder-selected path confined to its declared source root.
	RootedFile {
		/// Authorized source root.
		root: PathBuf,
		/// Decoded relative source path.
		relative: String,
	},
}

/// A logical input before category assignment (P0).
#[derive(Debug, Clone)]
pub struct AssetInput {
	/// Stable original logical name.
	pub logical_path: String,
	/// Content to capture into private owned staging.
	pub source: ContentSource,
	/// Pages provenance takes precedence over extension classification.
	pub producer: AssetProducer,
	/// Immutable asset, map, or renderable entry template.
	pub role: AssetRole,
	/// Optional producer MIME override.
	pub mime: Option<String>,
}

impl AssetInput {
	/// Construct an ordinary generated asset with MIME inferred from its name.
	pub fn bytes(logical: &str, bytes: Vec<u8>) -> Self {
		Self {
			logical_path: logical.into(),
			source: ContentSource::Bytes(bytes),
			producer: AssetProducer::Static,
			role: AssetRole::Asset,
			mime: None,
		}
	}

	/// Construct a source file whose symlinks cannot escape the declared root.
	pub fn from_directory(root: PathBuf, relative: &str, logical: &str) -> Self {
		Self {
			logical_path: logical.into(),
			source: ContentSource::RootedFile {
				root,
				relative: relative.into(),
			},
			producer: AssetProducer::Static,
			role: AssetRole::Asset,
			mime: None,
		}
	}

	/// Mark an input as belonging to a complete Pages output set.
	pub fn with_producer(mut self, producer: AssetProducer) -> Self {
		self.producer = producer;
		self
	}

	/// Mark an entry document or explicit source map.
	pub fn with_role(mut self, role: AssetRole) -> Self {
		self.role = role;
		self
	}

	/// Override MIME for a producer-defined format.
	pub fn with_mime(mut self, mime: String) -> Self {
		self.mime = Some(mime);
		self
	}
}

/// Captured immutable input exposed to a custom processor (P0).
#[derive(Debug, Clone)]
pub struct PreparedAsset {
	pub(super) logical: String,
	pub(super) path: PathBuf,
	pub(super) producer: AssetProducer,
	pub(super) role: AssetRole,
	pub(super) mime: String,
}

impl PreparedAsset {
	/// Original logical name; during rewrite validation, the assigned relative name.
	pub fn logical_path(&self) -> &str {
		&self.logical
	}
	/// Borrow the producer identity.
	pub fn producer(&self) -> AssetProducer {
		self.producer
	}
	/// Borrow the asset role.
	pub fn role(&self) -> AssetRole {
		self.role
	}
	/// Borrow the decoded MIME.
	pub fn mime(&self) -> &str {
		&self.mime
	}
	/// Open a read-only stream without buffering an entire large asset.
	pub fn open(&self) -> Result<File, AssetBuildError> {
		File::open(&self.path).map_err(|e| AssetBuildError::io(&self.path, e))
	}
	/// Read a transformable asset into memory. Opaque copying uses streaming I/O.
	pub fn read(&self) -> Result<Vec<u8>, AssetBuildError> {
		let mut bytes = Vec::new();
		self.open()?
			.read_to_end(&mut bytes)
			.map_err(|e| AssetBuildError::io(&self.path, e))?;
		Ok(bytes)
	}
}

/// A declared local dependency and processor-owned rewrite location (P0).
#[derive(Debug, Clone)]
pub struct AssetReference {
	/// Dependency resolved in the input's logical namespace.
	pub target: String,
	/// Original query and fragment, including their delimiters.
	pub suffix: String,
	/// Syntax-specific source span or structured edit location.
	pub site: serde_json::Value,
}

/// Captured asset paired with all declared references for its processor (P0).
#[derive(Debug, Clone)]
pub struct AnalyzedAsset {
	/// Immutable captured source.
	pub asset: PreparedAsset,
	/// Validated references in the original logical namespace.
	pub references: Vec<AssetReference>,
}

/// Final bytes and an optional update to an already-declared source map (P0).
#[derive(Debug)]
pub struct RewriteOutput {
	/// Rewritten canonical content.
	pub bytes: Vec<u8>,
	/// Logical map name and corrected map bytes; cannot introduce another asset.
	pub source_map: Option<(String, Vec<u8>)>,
}

/// Custom dependency discovery and reference rewriting (native-only, P0).
///
/// Preparation declares additional inputs before any paths are assigned. Analysis
/// resolves names relative to [`PreparedAsset::logical_path`]. Rewriting receives
/// all generation-relative paths, never the eventual build ID or publication root.
/// Multiple custom processors claiming one input are rejected. A processor's
/// analysis must also understand its rewritten output, for dependency validation.
///
/// ```rust
/// use reinhardt_utils::staticfiles::publication::*;
/// use std::collections::BTreeMap;
///
/// // A .assetref file contains one relative URL, understood by the application.
/// struct AssetLink;
/// impl AssetProcessor for AssetLink {
///     fn identity(&self) -> ProcessorIdentity {
///         ProcessorIdentity { id: "example.asset-link".into(), version: "1".into(),
///             options: serde_json::json!({}) }
///     }
///     fn matches(&self, asset: &PreparedAsset) -> bool {
///         asset.logical_path().ends_with(".assetref")
///     }
///     fn prepare(&self, _: &PreparedAsset) -> Result<Vec<AssetInput>, AssetBuildError> {
///         Ok(Vec::new())
///     }
///     fn analyze(&self, asset: &PreparedAsset) -> Result<Vec<AssetReference>, AssetBuildError> {
///         let bytes = asset.read()?;
///         let value = std::str::from_utf8(&bytes).map_err(|error| AssetBuildError::Input {
///             asset: asset.logical_path().into(), reason: error.to_string(),
///         })?;
///         Ok(resolve_asset_reference(asset.logical_path(), value)?.into_iter()
///             .map(|(target, suffix)| AssetReference { target, suffix,
///                 site: serde_json::Value::Null }).collect())
///     }
///     fn rewrite(&self, asset: &AnalyzedAsset, paths: &BTreeMap<String, String>)
///         -> Result<RewriteOutput, AssetBuildError>
///     {
///         let bytes = if let Some(reference) = asset.references.first() {
///             format!("{}{}", relative_asset_url(&paths[asset.asset.logical_path()],
///                 &paths[&reference.target])?, reference.suffix).into_bytes()
///         } else { asset.asset.read()? };
///         Ok(RewriteOutput { bytes, source_map: None })
///     }
/// }
/// let mut pipeline = AssetPipeline::new();
/// pipeline.register_processor(Box::new(AssetLink))?;
/// pipeline.add_input(AssetInput::bytes("logo.assetref", b"images/logo.svg".to_vec()))?;
/// pipeline.add_input(AssetInput::bytes("images/logo.svg", b"<svg/>".to_vec()))?;
/// let generation = pipeline.prepare(AssetMode::Production)?;
/// assert_eq!(generation.read_asset("logo.assetref")?, b"../vectors/logo.svg");
/// # Ok::<(), AssetBuildError>(())
/// ```
pub trait AssetProcessor: Send + Sync {
	/// Stable algorithm version and deterministic options.
	fn identity(&self) -> ProcessorIdentity;
	/// Whether this processor owns reference handling for the asset.
	fn matches(&self, asset: &PreparedAsset) -> bool;
	/// Declare additional outputs; emitting an existing logical name is an error.
	fn prepare(&self, asset: &PreparedAsset) -> Result<Vec<AssetInput>, AssetBuildError>;
	/// Declare local references before assignment.
	fn analyze(&self, asset: &PreparedAsset) -> Result<Vec<AssetReference>, AssetBuildError>;
	/// Rewrite references using the complete assigned path map.
	fn rewrite(
		&self,
		asset: &AnalyzedAsset,
		paths: &BTreeMap<String, String>,
	) -> Result<RewriteOutput, AssetBuildError>;
}

struct RegisteredProcessor {
	identity: ProcessorIdentity,
	processor: Box<dyn AssetProcessor>,
}

struct ByteProcessor {
	identity: ProcessorIdentity,
	processor: Box<dyn crate::staticfiles::processing::Processor>,
	preserves_positions: bool,
}

/// An owned completed generation ready for filesystem publication (P0).
///
/// Dropping it removes its private captured intermediates through RAII.
#[derive(Debug)]
pub struct PreparedGeneration {
	_spool: TempDir,
	pub(super) manifest: AssetManifestV2,
	pub(super) manifest_bytes: Vec<u8>,
	pub(super) assets: BTreeMap<String, PreparedAsset>,
}

impl PreparedGeneration {
	/// Complete metadata generated from final transformed bytes.
	pub fn manifest(&self) -> &AssetManifestV2 {
		&self.manifest
	}
	/// Canonical identical bytes for active and generation-local manifests.
	pub fn manifest_bytes(&self) -> &[u8] {
		&self.manifest_bytes
	}
	/// Read one final asset by logical name.
	pub fn read_asset(&self, logical: &str) -> Result<Vec<u8>, AssetBuildError> {
		self.assets
			.get(logical)
			.ok_or_else(|| {
				AssetBuildError::input(logical, "asset is not in the prepared generation")
			})?
			.read()
	}
}

#[derive(Default)]
pub(super) struct InputNamespace {
	pub bases: BTreeMap<String, String>,
	pub aliases: BTreeMap<String, String>,
}

impl InputNamespace {
	pub(super) fn base<'a>(&'a self, logical: &'a str) -> &'a str {
		self.bases.get(logical).map_or(logical, String::as_str)
	}
	pub(super) fn logical<'a>(&'a self, physical: &'a str) -> &'a str {
		self.aliases.get(physical).map_or(physical, String::as_str)
	}
}

/// Prepare all static input types under one generation identity (P0).
pub struct AssetPipeline {
	inputs: BTreeMap<String, AssetInput>,
	classifier: AssetClassifier,
	processors: Vec<RegisteredProcessor>,
	byte_processors: Vec<ByteProcessor>,
	entrypoints: BTreeMap<String, PagesEntrypoint>,
	namespace: InputNamespace,
}

impl Default for AssetPipeline {
	fn default() -> Self {
		Self::new()
	}
}

impl AssetPipeline {
	/// Create a pipeline with standard classification and opaque-file support.
	pub fn new() -> Self {
		Self {
			inputs: BTreeMap::new(),
			classifier: AssetClassifier::default(),
			processors: Vec::new(),
			byte_processors: Vec::new(),
			entrypoints: BTreeMap::new(),
			namespace: InputNamespace::default(),
		}
	}
	/// Register one unique logical input.
	pub fn add_input(&mut self, input: AssetInput) -> Result<(), AssetBuildError> {
		validate_asset_path(&input.logical_path)?;
		if self.inputs.contains_key(&input.logical_path) {
			return Err(AssetBuildError::Collision {
				first: input.logical_path.clone(),
				second: input.logical_path.clone(),
				path: input.logical_path,
			});
		}
		self.inputs.insert(input.logical_path.clone(), input);
		Ok(())
	}
	/// Borrow custom extension mapping configuration before preparation.
	pub fn classifier_mut(&mut self) -> &mut AssetClassifier {
		&mut self.classifier
	}
	/// Register a versioned custom processor; duplicate identities are errors.
	pub fn register_processor(
		&mut self,
		processor: Box<dyn AssetProcessor>,
	) -> Result<(), AssetBuildError> {
		let identity = processor.identity();
		if identity.id.is_empty()
			|| identity.version.is_empty()
			|| identity.id == self.classifier.identity().id
			|| identity.id == BuiltinProcessor.identity().id
			|| identity.id == representations::identity().id
			|| self.processors.iter().any(|p| p.identity.id == identity.id)
			|| self
				.byte_processors
				.iter()
				.any(|p| p.identity.id == identity.id)
		{
			return Err(AssetBuildError::manifest(format!(
				"empty or duplicate processor identity {:?}",
				identity.id
			)));
		}
		self.processors.push(RegisteredProcessor {
			identity,
			processor,
		});
		Ok(())
	}
	/// Adapt an existing asynchronous byte processor before reference discovery.
	///
	/// Identity and options must describe all output-affecting behavior. Mapped code
	/// may change only when `preserves_source_positions` is true and each generated
	/// line retains its UTF-16 length. Use [`AssetProcessor`] to supply corrected maps
	/// for other transformations. A scoped runtime allows use from async callers.
	pub fn register_byte_processor(
		&mut self,
		processor: Box<dyn crate::staticfiles::processing::Processor>,
		mut identity: ProcessorIdentity,
		preserves_source_positions: bool,
	) -> Result<(), AssetBuildError> {
		if identity.id.is_empty()
			|| identity.version.is_empty()
			|| [
				self.classifier.identity().id,
				BuiltinProcessor.identity().id,
				representations::identity().id,
			]
			.contains(&identity.id)
			|| self.processors.iter().any(|p| p.identity.id == identity.id)
			|| self
				.byte_processors
				.iter()
				.any(|p| p.identity.id == identity.id)
		{
			return Err(AssetBuildError::manifest(format!(
				"empty or duplicate processor identity {:?}",
				identity.id
			)));
		}
		identity.options = serde_json::json!({"options":identity.options,"preserves_source_positions":preserves_source_positions});
		self.byte_processors.push(ByteProcessor {
			identity,
			processor,
			preserves_positions: preserves_source_positions,
		});
		Ok(())
	}
	/// Declare one Pages entry with ordered styles and an optional document.
	pub fn set_entrypoint(
		&mut self,
		name: &str,
		entrypoint: PagesEntrypoint,
	) -> Result<(), AssetBuildError> {
		if name.is_empty() || self.entrypoints.contains_key(name) {
			return Err(AssetBuildError::input(
				name,
				"entrypoint is empty or duplicated",
			));
		}
		self.entrypoints.insert(name.into(), entrypoint);
		Ok(())
	}
	/// Declare an imported manifest's physical-to-logical reference alias.
	pub fn add_alias(&mut self, published: &str, logical: &str) -> Result<(), AssetBuildError> {
		validate_asset_path(published)?;
		validate_asset_path(logical)?;
		if let Some(existing) = self.namespace.aliases.get(published)
			&& existing != logical
		{
			return Err(AssetBuildError::Collision {
				first: existing.clone(),
				second: logical.into(),
				path: published.into(),
			});
		}
		self.namespace
			.aliases
			.insert(published.into(), logical.into());
		Ok(())
	}

	/// Use a previously published source location when analyzing imported references.
	/// Aliases translate targets back to their stable logical names before rewriting.
	pub fn add_reference_base(
		&mut self,
		logical: &str,
		source_path: &str,
	) -> Result<(), AssetBuildError> {
		validate_asset_path(logical)?;
		validate_asset_path(source_path)?;
		if self.namespace.bases.contains_key(logical) {
			return Err(AssetBuildError::input(logical, "duplicate reference base"));
		}
		self.namespace
			.bases
			.insert(logical.into(), source_path.into());
		Ok(())
	}

	/// Capture, classify, rewrite, and hash a coherent generation without publishing.
	pub fn prepare(self, mode: AssetMode) -> Result<PreparedGeneration, AssetBuildError> {
		let spool = tempfile::Builder::new()
			.prefix("reinhardt-assets-")
			.tempdir()
			.map_err(|e| AssetBuildError::io(std::env::temp_dir(), e))?;
		let mut captured = BTreeMap::new();
		for input in self.inputs.values().cloned() {
			let asset = capture(input, &spool, captured.len())?;
			captured.insert(asset.logical.clone(), asset);
		}
		let mut variants = BTreeMap::new();
		let mut processed = BTreeSet::new();
		loop {
			representations::prepare(&mut captured, &mut variants, &spool, &self.classifier)?;
			let Some(logical) = captured
				.keys()
				.find(|key| !processed.contains(*key) && !variants.contains_key(*key))
				.cloned()
			else {
				break;
			};
			processed.insert(logical.clone());
			let asset = &captured[&logical];
			if let Some(processor) = self.processor_for(asset)? {
				for input in processor.prepare(asset)? {
					if captured.contains_key(&input.logical_path) {
						return Err(AssetBuildError::Collision {
							first: logical.clone(),
							second: input.logical_path.clone(),
							path: input.logical_path,
						});
					}
					let asset = capture(input, &spool, captured.len())?;
					captured.insert(asset.logical.clone(), asset);
				}
			}
		}
		process_bytes(&self.byte_processors, &captured, &variants, &self.namespace)?;
		let mut analyses = BTreeMap::new();
		for (logical, asset) in &captured {
			if variants.contains_key(logical) {
				analyses.insert(logical.clone(), Vec::new());
				continue;
			}
			let mut references = match self.processor_for(asset)? {
				Some(processor) => {
					let mut input = asset.clone();
					input.logical = self.namespace.base(logical).into();
					processor.analyze(&input)?
				}
				None => Vec::new(),
			};
			for reference in &mut references {
				if let Some(original) = self.namespace.aliases.get(&reference.target) {
					reference.target = original.clone();
				}
				if !captured.contains_key(&reference.target) {
					return Err(AssetBuildError::Reference {
						asset: logical.clone(),
						target: reference.target.clone(),
						reason: "missing logical dependency; include the referenced input".into(),
					});
				}
			}
			analyses.insert(logical.clone(), references);
		}
		let mut paths = BTreeMap::new();
		let mut categories = BTreeMap::new();
		for (logical, asset) in &captured {
			let (category, path) = self.classifier.classify(logical, asset.producer)?;
			paths.insert(logical.clone(), path);
			categories.insert(logical.clone(), category);
		}
		validate_assignments(
			&paths
				.iter()
				.map(|(k, v)| (k.clone(), v.clone()))
				.collect::<Vec<_>>(),
		)?;
		let mut map_updates = BTreeMap::new();
		for (logical, asset) in &captured {
			if variants.contains_key(logical) {
				continue;
			}
			if let Some(processor) = self.processor_for(asset)? {
				let analyzed = AnalyzedAsset {
					asset: asset.clone(),
					references: analyses[logical].clone(),
				};
				let mut output = processor.rewrite(&analyzed, &paths)?;
				if output.source_map.is_none() {
					output.source_map = representations::maps::rewrite(
						&analyzed,
						&output.bytes,
						&paths,
						&captured,
						&processor.identity().id,
						&self.namespace,
					)?;
				}
				std::fs::write(&asset.path, &output.bytes)
					.map_err(|e| AssetBuildError::io(&asset.path, e))?;
				if let Some((name, bytes)) = output.source_map
					&& (!captured
						.get(&name)
						.is_some_and(|map| map.role == AssetRole::SourceMap)
						|| map_updates.insert(name.clone(), bytes).is_some())
				{
					return Err(AssetBuildError::input(
						logical,
						format!("undeclared or multiply rewritten source map {name:?}"),
					));
				}
			}
		}
		for (logical, bytes) in map_updates {
			let path = &captured[&logical].path;
			std::fs::write(path, bytes).map_err(|e| AssetBuildError::io(path, e))?;
		}
		let reverse: BTreeMap<&str, &str> = paths
			.iter()
			.map(|(logical, path)| (path.as_str(), logical.as_str()))
			.collect();
		for (logical, asset) in &captured {
			if variants.contains_key(logical) {
				continue;
			}
			if let Some(processor) = self.processor_for(asset)? {
				let mut relocated = asset.clone();
				if asset.role != AssetRole::EntryDocument {
					relocated.logical = paths[logical].clone();
				}
				for reference in processor.analyze(&relocated)? {
					let original = if asset.role == AssetRole::EntryDocument {
						Some(
							self.namespace
								.aliases
								.get(&reference.target)
								.unwrap_or(&reference.target)
								.as_str(),
						)
					} else {
						reverse.get(reference.target.as_str()).copied()
					};
					if !original
						.is_some_and(|name| analyses[logical].iter().any(|r| r.target == name))
					{
						return Err(AssetBuildError::Reference {
							asset: logical.clone(),
							target: reference.target,
							reason: "rewrite introduced an undeclared or unresolved dependency"
								.into(),
						});
					}
				}
			}
		}
		representations::regenerate(&captured, &variants)?;
		let mut assets = BTreeMap::new();
		for (logical, asset) in &captured {
			let (size, sha256) = digest_reader(asset.open()?, &asset.path)?;
			let dependencies = analyses[logical]
				.iter()
				.map(|r| r.target.clone())
				.collect::<BTreeSet<_>>()
				.into_iter()
				.collect();
			assets.insert(
				logical.clone(),
				AssetRecord {
					category: categories[logical],
					role: asset.role,
					mime: asset.mime.clone(),
					size,
					sha256,
					dependencies,
					variants: variants
						.iter()
						.filter(|(_, v)| &v.parent == logical)
						.map(|(name, _)| name.clone())
						.collect(),
					encoding: variants.get(logical).map(|v| v.encoding),
					parent: variants.get(logical).map(|v| v.parent.clone()),
					document: if asset.role == AssetRole::EntryDocument
						&& !variants.contains_key(logical)
					{
						Some(super::rewrite::compile_document(
							logical,
							std::str::from_utf8(&asset.read()?)
								.map_err(|e| AssetBuildError::input(logical, e.to_string()))?,
						)?)
					} else {
						None
					},
				},
			);
		}
		let mut pipeline = vec![
			self.classifier.identity(),
			BuiltinProcessor.identity(),
			representations::identity(),
		];
		pipeline.extend(self.processors.iter().map(|p| p.identity.clone()));
		pipeline.extend(self.byte_processors.iter().map(|p| p.identity.clone()));
		let build_id = build_id(mode, &paths, &assets, &self.entrypoints, &pipeline)?;
		let paths = paths
			.into_iter()
			.map(|(logical, path)| (logical, format!("builds/{build_id}/{path}")))
			.collect();
		let manifest = AssetManifestV2 {
			version: "2.0".into(),
			mode,
			build_id,
			paths,
			assets,
			entrypoints: self.entrypoints,
			pipeline,
		};
		let manifest_bytes = encode_manifest(&manifest)?;
		Ok(PreparedGeneration {
			_spool: spool,
			manifest,
			manifest_bytes,
			assets: captured,
		})
	}

	fn processor_for(
		&self,
		asset: &PreparedAsset,
	) -> Result<Option<&dyn AssetProcessor>, AssetBuildError> {
		let matched: Vec<_> = self
			.processors
			.iter()
			.filter(|p| p.processor.matches(asset))
			.collect();
		if matched.len() > 1 {
			return Err(AssetBuildError::input(
				&asset.logical,
				"multiple custom processors claim reference handling",
			));
		}
		Ok(matched.first().map(|p| p.processor.as_ref()).or_else(|| {
			BuiltinProcessor
				.matches(asset)
				.then_some(&BuiltinProcessor as &dyn AssetProcessor)
		}))
	}
}

fn process_bytes(
	processors: &[ByteProcessor],
	captured: &BTreeMap<String, PreparedAsset>,
	variants: &BTreeMap<String, representations::Representation>,
	namespace: &InputNamespace,
) -> Result<(), AssetBuildError> {
	if processors.is_empty() {
		return Ok(());
	}
	std::thread::scope(|scope| {
		scope.spawn(|| {
		let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| AssetBuildError::input("byte processors", e.to_string()))?;
		runtime.block_on(async {
			for registered in processors {
				for (logical, asset) in captured {
					if variants.contains_key(logical) || !registered.processor.can_process(std::path::Path::new(logical)) { continue; }
					let before = asset.read()?;
					let mapped = if BuiltinProcessor.matches(asset) {
                        let mut input = asset.clone();
                        input.logical = namespace.base(logical).into();
                        BuiltinProcessor.analyze(&input)?.iter().any(|r| captured.get(namespace.logical(&r.target)).is_some_and(|map| map.role == AssetRole::SourceMap))
                    } else { false };
					let after = registered.processor.process(&before, std::path::Path::new(logical)).await.map_err(|e| AssetBuildError::Processor { processor: registered.identity.id.clone(), asset: logical.clone(), reason: e.to_string() })?;
					if mapped && before != after {
						let lengths = |bytes: &[u8]| std::str::from_utf8(bytes).ok().map(|text| text.split('\n').map(|line| line.encode_utf16().count()).collect::<Vec<_>>());
						if !registered.preserves_positions || lengths(&before).is_none() || lengths(&before) != lengths(&after) {
							return Err(AssetBuildError::Processor { processor: registered.identity.id.clone(), asset: logical.clone(), reason: "cannot preserve source-map positions; use an AssetProcessor returning a corrected map".into() });
						}
					}
					std::fs::write(&asset.path, after).map_err(|e| AssetBuildError::io(&asset.path, e))?;
				}
			}
			Ok(())
		})
	}).join().map_err(|_| AssetBuildError::input("byte processors", "processor thread panicked"))?
	})
}

fn capture(
	input: AssetInput,
	spool: &TempDir,
	index: usize,
) -> Result<PreparedAsset, AssetBuildError> {
	validate_asset_path(&input.logical_path)?;
	let path = spool.path().join(index.to_string());
	let mut output = File::create(&path).map_err(|e| AssetBuildError::io(&path, e))?;
	match input.source {
		ContentSource::Bytes(bytes) => output
			.write_all(&bytes)
			.map_err(|e| AssetBuildError::io(&path, e))?,
		ContentSource::File(source) => {
			let parent = source
				.parent()
				.filter(|p| !p.as_os_str().is_empty())
				.ok_or_else(|| {
					AssetBuildError::input(
						&input.logical_path,
						"file source must have an explicit parent directory",
					)
				})?;
			let name = source.file_name().ok_or_else(|| {
				AssetBuildError::input(&input.logical_path, "file source requires a filename")
			})?;
			let root = cap_std::fs::Dir::open_ambient_dir(parent, cap_std::ambient_authority())
				.map_err(|e| AssetBuildError::io(parent, e))?;
			let mut reader = root
				.open(name)
				.map_err(|e| AssetBuildError::io(&source, e))?;
			std::io::copy(&mut reader, &mut output).map_err(|e| AssetBuildError::io(&source, e))?;
		}
		ContentSource::RootedFile { root, relative } => {
			validate_asset_path(&relative)?;
			let directory = cap_std::fs::Dir::open_ambient_dir(&root, cap_std::ambient_authority())
				.map_err(|e| AssetBuildError::io(&root, e))?;
			let mut reader = directory
				.open(&relative)
				.map_err(|e| AssetBuildError::io(root.join(&relative), e))?;
			std::io::copy(&mut reader, &mut output)
				.map_err(|e| AssetBuildError::io(root.join(&relative), e))?;
		}
	}
	let role = if input.role == AssetRole::Asset
		&& input.logical_path.to_ascii_lowercase().ends_with(".map")
	{
		AssetRole::SourceMap
	} else {
		input.role
	};
	let mime = input
		.mime
		.unwrap_or_else(|| inferred_mime(&input.logical_path, role));
	let mime = mime
		.parse::<mime_guess::Mime>()
		.map_err(|e| AssetBuildError::input(&input.logical_path, format!("invalid MIME: {e}")))?
		.to_string();
	Ok(PreparedAsset {
		logical: input.logical_path,
		path,
		producer: input.producer,
		role,
		mime,
	})
}

pub(super) fn inferred_mime(logical: &str, role: AssetRole) -> String {
	if role == AssetRole::SourceMap {
		"application/json".into()
	} else {
		mime_guess::from_path(logical)
			.first_or_octet_stream()
			.essence_str()
			.into()
	}
}
