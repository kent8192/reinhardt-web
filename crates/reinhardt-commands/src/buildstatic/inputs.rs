//! Read-only adapters for physical, collected, and complete Pages inputs.

use super::{BuildStaticPreview, invalid, io_error};
use reinhardt_utils::staticfiles::StaticFilesFinder;
use reinhardt_utils::staticfiles::publication::{
	AssetBuildError, AssetCategory, AssetClassifier, AssetInput, AssetMode, AssetProducer,
	AssetRole, ContentSource, DecodedAssetManifest, ManifestSnapshot, PagesEntrypoint,
	SnapshotOptions, analyze_asset_references, decode_manifest, validate_assignments,
};
use std::collections::BTreeMap;
use std::path::Path;

/// Discovered physical and owned virtual inputs; publication captures files before activation (P0).
#[derive(Debug, Default)]
pub struct CollectedStaticInputs {
	/// Stable logical inputs, including framework-generated CSS.
	pub inputs: Vec<AssetInput>,
	/// Already-published physical paths mapped back to original logical names.
	pub aliases: BTreeMap<String, String>,
	/// Source locations used for analyzing imported relative references.
	pub reference_bases: BTreeMap<String, String>,
	/// Entrypoints retained when importing a complete version 2 publication.
	pub entrypoints: BTreeMap<String, PagesEntrypoint>,
	/// Generated checks omitted by side-effect-free discovery.
	pub pending_checks: Vec<String>,
}

/// Owned complete Pages bundle with an entrypoint discovered from JavaScript syntax (P0).
#[derive(Debug)]
pub struct PagesBuildInputs {
	/// All captured companions, retaining their complete relative directory tree.
	pub inputs: Vec<AssetInput>,
	/// Actual JS/WASM relationship; no suffix-based filename guessing.
	pub entrypoint: PagesEntrypoint,
}

impl PagesBuildInputs {
	/// Capture materialized wasm-bindgen web output without invoking build tools.
	pub fn from_directory(directory: &Path, entry: &str) -> Result<Self, AssetBuildError> {
		AssetClassifier::default().classify(entry, AssetProducer::Pages)?;
		if !entry.ends_with(".js") && !entry.ends_with(".mjs") {
			return Err(invalid(entry, "Pages entry must be a .js or .mjs module"));
		}
		if !directory.is_dir() {
			return Err(invalid(
				directory.display().to_string(),
				"Pages output directory is missing",
			));
		}
		let files = StaticFilesFinder::new(vec![directory.into()])
			.find_all_checked(&[])
			.map_err(|e| io_error(directory, e))?;
		let mut inputs = BTreeMap::new();
		for (root, name) in files {
			let path = root.join(&name);
			let bytes = std::fs::read(&path).map_err(|e| io_error(&path, e))?;
			let mut input = AssetInput::bytes(&name, bytes).with_producer(AssetProducer::Pages);
			if name.ends_with(".d.ts") {
				input.mime = Some("text/plain; charset=utf-8".into());
			}
			inputs.insert(name, input);
		}
		let source = inputs
			.get(entry)
			.ok_or_else(|| invalid(entry, "selected Pages JavaScript entry does not exist"))?;
		let ContentSource::Bytes(bytes) = &source.source else {
			unreachable!("adapter captures bytes")
		};
		let references = analyze_asset_references(entry, "text/javascript", bytes)?;
		let targets: std::collections::BTreeSet<_> = references
			.iter()
			.filter(|r| r.target.ends_with(".wasm"))
			.map(|r| r.target.clone())
			.collect();
		if targets.len() != 1 {
			return Err(invalid(
				entry,
				"Pages entry must reference exactly one local WASM target through its module syntax",
			));
		}
		let wasm = targets.into_iter().next().expect("one target");
		let target = inputs
			.get(&wasm)
			.ok_or_else(|| invalid(&wasm, "JavaScript-referenced WASM output is missing"))?;
		let ContentSource::Bytes(bytes) = &target.source else {
			unreachable!("adapter captures bytes")
		};
		if !bytes.starts_with(b"\0asm\x01\0\0\0") {
			return Err(invalid(&wasm, "expected a WebAssembly version 1 binary"));
		}
		Ok(Self {
			inputs: inputs.into_values().collect(),
			entrypoint: PagesEntrypoint {
				javascript: entry.into(),
				wasm,
				styles: Vec::new(),
				document: None,
			},
		})
	}
}

pub(super) fn import_manifest(path: &Path) -> Result<CollectedStaticInputs, AssetBuildError> {
	let bytes = std::fs::read(path).map_err(|e| io_error(path, e))?;
	let decoded = decode_manifest(&bytes)?;
	let mut root = path
		.parent()
		.ok_or_else(|| invalid(path.display().to_string(), "manifest has no source root"))?;
	let mut result = CollectedStaticInputs::default();
	if let DecodedAssetManifest::V2(manifest) = &decoded {
		// Both the active pointer and its immutable generation-local copy are supported.
		if root.file_name().and_then(|n| n.to_str()) == Some(&manifest.build_id)
			&& root
				.parent()
				.and_then(Path::file_name)
				.and_then(|n| n.to_str())
				== Some("builds")
		{
			root = root
				.parent()
				.and_then(Path::parent)
				.expect("generation has a publication root");
		}
		let options = match manifest.mode {
			AssetMode::Production => SnapshotOptions::production(),
			AssetMode::Development => SnapshotOptions::development(),
		}
		.manifest_path(path.into());
		ManifestSnapshot::load(root, &options)?;
		result.entrypoints = manifest.entrypoints.clone();
	}
	for (logical, physical) in decoded.paths() {
		let mut input = AssetInput::from_directory(root.into(), physical, logical);
		if let DecodedAssetManifest::V2(manifest) = &decoded {
			let record = &manifest.assets[logical];
			input.producer = if record.category == AssetCategory::Pages {
				AssetProducer::Pages
			} else {
				AssetProducer::Static
			};
			input.role = record.role;
			input.mime = Some(record.mime.clone());
		}
		// Entry documents retain their logical render namespace until request rendering.
		if input.role != AssetRole::EntryDocument {
			result
				.reference_bases
				.insert(logical.clone(), physical.clone());
		}
		result.aliases.insert(physical.clone(), logical.clone());
		result.inputs.push(input);
	}
	Ok(result)
}

pub(super) fn preview(
	inputs: &CollectedStaticInputs,
) -> Result<BuildStaticPreview, AssetBuildError> {
	let mut preview = BuildStaticPreview::default();
	let mut assignments = Vec::new();
	for input in &inputs.inputs {
		let (_, path) = AssetClassifier::default().classify(&input.logical_path, input.producer)?;
		assignments.push((input.logical_path.clone(), path.clone()));
		preview.assignments.insert(input.logical_path.clone(), path);
		let source = match &input.source {
			ContentSource::Bytes(bytes) => format!("owned generated input ({} bytes)", bytes.len()),
			ContentSource::File(path) => path.display().to_string(),
			ContentSource::RootedFile { root, relative } => {
				root.join(relative).display().to_string()
			}
		};
		preview.sources.insert(input.logical_path.clone(), source);
	}
	if let Err(error) = validate_assignments(&assignments) {
		preview.conflicts.push(error.to_string());
	}
	Ok(preview)
}
