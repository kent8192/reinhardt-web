//! Compiled template baselines and development static overlays.

use std::collections::BTreeMap;

use reinhardt_pages::hmr::{
	CompiledBuildId, PatchGeneration, SourceId, StaticTemplateNode, TemplateDescriptor, TemplateKey,
};

/// The last successfully compiled client template manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledBaseline {
	/// Identity of the client build that produced this manifest.
	pub build_id: CompiledBuildId,
	/// Digest of the compiled template manifest.
	pub manifest_digest: [u8; 32],
	/// Successful source snapshots and their descriptors.
	pub sources: BTreeMap<SourceId, SourceBaseline>,
}

impl CompiledBaseline {
	/// Creates an empty baseline for a successful client build.
	pub fn new(build_id: CompiledBuildId, manifest_digest: [u8; 32]) -> Self {
		Self {
			build_id,
			manifest_digest,
			sources: BTreeMap::new(),
		}
	}
}

/// Successful compilation data for one source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBaseline {
	/// Source snapshot used to produce the client build.
	pub source_text: String,
	/// Top-level page macro keys in lexical order.
	pub callsites: Vec<TemplateKey>,
	/// Top-level descriptors keyed by their compiled callsite identity.
	pub descriptors: BTreeMap<TemplateKey, TemplateDescriptor>,
}

/// Mutable development overlay state, separate from the compiled baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticOverlayStore {
	/// Generation represented by the currently installed overlays.
	pub generation: PatchGeneration,
	/// Current static replacements keyed by compiled template identity.
	pub overlays: BTreeMap<TemplateKey, StaticTemplateNode>,
}

impl Default for StaticOverlayStore {
	fn default() -> Self {
		Self {
			generation: PatchGeneration(0),
			overlays: BTreeMap::new(),
		}
	}
}

impl StaticOverlayStore {
	/// Creates an empty overlay store at generation zero.
	pub fn new() -> Self {
		Self::default()
	}

	/// Installs a patch's static trees after the runtime accepts the generation.
	pub fn install(
		&mut self,
		generation: PatchGeneration,
		patches: &[reinhardt_pages::hmr::TemplatePatch],
	) {
		self.generation = generation;
		for patch in patches {
			self.overlays
				.insert(patch.key.clone(), patch.static_tree.clone());
		}
	}

	/// Clears all overlays while retaining the latest acknowledged generation.
	pub fn clear(&mut self, generation: PatchGeneration) {
		self.generation = generation;
		self.overlays.clear();
	}
}
