//! Conservative source-change classification for template hot patching.

use std::{
	collections::BTreeSet,
	fs,
	path::{Path, PathBuf},
};

use reinhardt_pages::hmr::{
	PatchGeneration, StaticTemplateNode, TemplateKey, TemplatePatch, TemplatePatchBatch,
};

use crate::{
	template_manifest::{
		ManifestError, ParsedTemplateSource, collect_template_source, source_id_for_path,
	},
	template_state::{CompiledBaseline, SourceBaseline, StaticOverlayStore},
};

/// Result of classifying a group of changed source files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateClassification {
	/// The source edit can be represented by static template patches.
	Patchable(TemplatePatchSet),
	/// The edit must go through the normal build and reload path.
	RebuildRequired(RebuildReason),
	/// Manouche could not parse or validate the changed template immediately.
	InvalidTemplate(TemplateDiagnostic),
}

/// Static patches for one source generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplatePatchSet {
	/// Client build expected by the patch.
	pub build_id: reinhardt_pages::hmr::CompiledBuildId,
	/// Client manifest expected by the patch.
	pub manifest_digest: [u8; 32],
	/// Monotonic source generation represented by the patch.
	pub generation: PatchGeneration,
	/// Replacement trees for changed templates.
	pub patches: Vec<TemplatePatch>,
}

impl TemplatePatchSet {
	/// Converts this classifier result into the wire-level patch batch.
	pub fn into_batch(self) -> TemplatePatchBatch {
		TemplatePatchBatch {
			build_id: self.build_id,
			manifest_digest: self.manifest_digest,
			generation: self.generation,
			patches: self.patches,
		}
	}
}

/// Conservative reason for requiring a normal rebuild.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebuildReason {
	/// A dynamic expression, event, capture, or ABI-bearing structure changed.
	DynamicAbiChanged,
	/// Page callsites were added, removed, reordered, or became ambiguous.
	CallsiteSetChanged,
	/// Code outside the page body or shared/SSR source changed.
	SharedOrSsrSourceChanged,
	/// A changed file cannot be matched to the loaded client source manifest.
	NoMatchingClientSource,
	/// The changed source has multiple indistinguishable callsite matches.
	AmbiguousCallsite,
	/// A static replacement would drop a direct dynamic attribute, control
	/// binding, or event listener that has no enclosing structural range.
	UnsafeUnplacedDynamicSlot,
	/// A static edit occurred inside a nested control-flow or component
	/// template that the current runtime cannot mount independently.
	NestedTemplateChanged,
	/// Static source changed inside a retained bound-element range that cannot
	/// safely receive a partial subtree patch yet.
	StaticContentOutsidePatchTree,
}

/// Immediate source diagnostic shown before fallback compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateDiagnostic {
	/// Project-relative source identifier.
	pub source_id: reinhardt_pages::hmr::SourceId,
	/// Parser or validator message.
	pub message: String,
	/// Byte offsets relative to the source snapshot, when known.
	pub relative_span: Option<(usize, usize)>,
}

/// Classifies changed Rust sources against the last successful client baseline.
pub fn classify_source_change(
	project_root: &Path,
	paths: &[PathBuf],
	baseline: &CompiledBaseline,
	overlays: &StaticOverlayStore,
) -> TemplateClassification {
	let mut seen_sources = BTreeSet::new();
	let mut patches = Vec::new();
	let generation = PatchGeneration(overlays.generation.0.saturating_add(1));

	for path in paths {
		let source_id = source_id_for_path(project_root, path);
		if !seen_sources.insert(source_id.clone()) {
			continue;
		}
		if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
			return TemplateClassification::RebuildRequired(
				RebuildReason::SharedOrSsrSourceChanged,
			);
		}
		let Some(source_baseline) = baseline.sources.get(&source_id) else {
			return TemplateClassification::RebuildRequired(RebuildReason::NoMatchingClientSource);
		};
		let source_path = absolute_path(project_root, path);
		let source = match fs::read_to_string(&source_path) {
			Ok(source) => source,
			Err(error) => {
				return TemplateClassification::InvalidTemplate(TemplateDiagnostic {
					source_id,
					message: format!("failed to read changed source: {error}"),
					relative_span: None,
				});
			}
		};
		let current = match collect_template_source(&source_id, &source) {
			Ok(parsed) => parsed,
			Err(error) => {
				return TemplateClassification::InvalidTemplate(diagnostic_from_error(
					source_id, error,
				));
			}
		};
		let previous = match collect_template_source(&source_id, &source_baseline.source_text) {
			Ok(parsed) => parsed,
			Err(error) => {
				return TemplateClassification::InvalidTemplate(diagnostic_from_error(
					source_id, error,
				));
			}
		};
		match classify_one_source(source_baseline, &previous, &current) {
			Ok(source_templates) => {
				for (key, descriptor) in source_templates {
					let previous_static_tree = overlays.overlays.get(&key).or_else(|| {
						source_baseline
							.descriptors
							.get(&key)
							.map(|value| &value.static_tree)
					});
					if previous_static_tree == Some(&descriptor.static_tree) {
						continue;
					}
					if has_unplaced_dynamic_slots(&descriptor) {
						return TemplateClassification::RebuildRequired(
							RebuildReason::UnsafeUnplacedDynamicSlot,
						);
					}
					patches.push(TemplatePatch::static_replacement(
						&descriptor,
						descriptor.static_tree.clone(),
					));
				}
			}
			Err(ClassificationFailure::Rebuild(reason)) => {
				return TemplateClassification::RebuildRequired(reason);
			}
		}
	}

	TemplateClassification::Patchable(TemplatePatchSet {
		build_id: baseline.build_id,
		manifest_digest: baseline.manifest_digest,
		generation,
		patches,
	})
}

enum ClassificationFailure {
	Rebuild(RebuildReason),
}

fn classify_one_source(
	baseline: &SourceBaseline,
	previous: &ParsedTemplateSource,
	current: &ParsedTemplateSource,
) -> Result<Vec<(TemplateKey, reinhardt_pages::hmr::TemplateDescriptor)>, ClassificationFailure> {
	if baseline.callsites.len() != previous.templates.len()
		|| baseline.callsites.len() != current.templates.len()
	{
		return Err(ClassificationFailure::Rebuild(
			RebuildReason::CallsiteSetChanged,
		));
	}
	if has_duplicate_keys(&baseline.callsites)
		|| has_duplicate_keys(
			&previous
				.templates
				.iter()
				.map(|template| template.key.clone())
				.collect::<Vec<_>>(),
		) || has_duplicate_keys(
		&current
			.templates
			.iter()
			.map(|template| template.key.clone())
			.collect::<Vec<_>>(),
	) {
		return Err(ClassificationFailure::Rebuild(
			RebuildReason::AmbiguousCallsite,
		));
	}
	if previous.outside_page_fingerprint != current.outside_page_fingerprint {
		return Err(ClassificationFailure::Rebuild(
			RebuildReason::SharedOrSsrSourceChanged,
		));
	}

	let mut changed = Vec::new();
	for (index, baseline_key) in baseline.callsites.iter().enumerate() {
		let Some(baseline_descriptor) = baseline.descriptors.get(baseline_key) else {
			return Err(ClassificationFailure::Rebuild(
				RebuildReason::CallsiteSetChanged,
			));
		};
		let current_template = &current.templates[index];
		let previous_template = &previous.templates[index];
		let current_descriptor = &current_template.descriptor;
		if baseline_descriptor.abi_hash != current_descriptor.abi_hash {
			return Err(ClassificationFailure::Rebuild(
				RebuildReason::DynamicAbiChanged,
			));
		}
		if baseline_descriptor.nested != current_descriptor.nested {
			return Err(ClassificationFailure::Rebuild(
				RebuildReason::NestedTemplateChanged,
			));
		}
		if baseline_descriptor.static_tree == current_descriptor.static_tree
			&& previous_template.source_fingerprint != current_template.source_fingerprint
		{
			return Err(ClassificationFailure::Rebuild(
				RebuildReason::StaticContentOutsidePatchTree,
			));
		}
		let mut descriptor = current_descriptor.clone();
		descriptor.key = baseline_key.clone();
		changed.push((baseline_key.clone(), descriptor));
	}
	Ok(changed)
}

fn has_unplaced_dynamic_slots(descriptor: &reinhardt_pages::hmr::TemplateDescriptor) -> bool {
	let mut placed_slots = BTreeSet::new();
	collect_placed_slots(&descriptor.static_tree, &mut placed_slots);
	descriptor.slots.iter().any(|slot| {
		slot.semantic_kind == "head_expression" && !placed_slots.contains(&slot.slot_id)
	})
}

fn collect_placed_slots(
	node: &StaticTemplateNode,
	placed_slots: &mut BTreeSet<reinhardt_pages::hmr::DynamicSlotId>,
) {
	match node {
		StaticTemplateNode::Element { children, .. } => {
			for child in children {
				collect_placed_slots(child, placed_slots);
			}
		}
		StaticTemplateNode::Slot(slot_id) => {
			placed_slots.insert(*slot_id);
		}
		StaticTemplateNode::Text(_) => {}
	}
}

fn has_duplicate_keys(keys: &[TemplateKey]) -> bool {
	let mut unique = BTreeSet::new();
	keys.iter().any(|key| !unique.insert(key))
}

fn diagnostic_from_error(
	source_id: reinhardt_pages::hmr::SourceId,
	error: ManifestError,
) -> TemplateDiagnostic {
	TemplateDiagnostic {
		source_id,
		message: error.message,
		relative_span: error.relative_span,
	}
}

fn absolute_path(project_root: &Path, path: &Path) -> PathBuf {
	if path.is_absolute() {
		path.to_path_buf()
	} else {
		project_root.join(path)
	}
}
