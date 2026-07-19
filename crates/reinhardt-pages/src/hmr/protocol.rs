//! Target-neutral protocol values for development template hot patching.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Stable identity for a source file in a compiled template manifest.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceId(pub String);

/// Stable callsite identity for one `page!` template.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TemplateKey {
	/// Source file containing the template.
	pub source_id: SourceId,
	/// One-based source line of the macro callsite.
	pub line: u32,
	/// One-based source column of the macro callsite.
	pub column: u32,
	/// Deterministic index for a nested template at the callsite.
	pub nested_template_index: u32,
}

/// Stable identity for a dynamic slot within a template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DynamicSlotId(pub u32);

/// Hash of the dynamic expressions and their semantic roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DynamicAbiHash(pub [u8; 32]);

/// Identity of the compiled WASM build serving a client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompiledBuildId(pub [u8; 32]);

/// Monotonic generation assigned to a source change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PatchGeneration(pub u64);

/// Static and dynamic metadata emitted for one compiled template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateDescriptor {
	/// Stable template identity.
	pub key: TemplateKey,
	/// Compiled dynamic ABI hash.
	pub abi_hash: DynamicAbiHash,
	/// Compiled static template tree.
	pub static_tree: StaticTemplateNode,
	/// Dynamic slots in deterministic source order.
	pub slots: Vec<DynamicSlotDescriptor>,
	/// Nested templates in deterministic source order.
	pub nested: Vec<TemplateDescriptor>,
}

/// Derives the deterministic development identity for a compiled descriptor set.
///
/// Both the commands watcher and the loaded WASM compute this value from the
/// same descriptor data. Static overlays never participate, so a browser keeps
/// the identity of the last successful compiled build while it receives
/// compatible template patches.
pub fn template_manifest_identity(
	descriptors: impl IntoIterator<Item = TemplateDescriptor>,
) -> (CompiledBuildId, [u8; 32]) {
	let mut descriptors = descriptors.into_iter().collect::<Vec<_>>();
	descriptors.sort_by(|left, right| left.key.cmp(&right.key));
	let manifest_digest: [u8; 32] = Sha256::digest(
		serde_json::to_vec(&descriptors).expect("template descriptors are serializable"),
	)
	.into();
	let mut build_hasher = Sha256::new();
	build_hasher.update(b"reinhardt-page-hot-reload-v1");
	build_hasher.update(manifest_digest);
	let build_id = CompiledBuildId(build_hasher.finalize().into());
	(build_id, manifest_digest)
}

/// Compiler-side description of one dynamic slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicSlotDescriptor {
	/// Stable slot identity.
	pub slot_id: DynamicSlotId,
	/// Semantic role assigned by template lowering.
	pub semantic_kind: String,
	/// Canonicalized dynamic Rust tokens used for ABI comparison.
	pub canonical_tokens: Vec<String>,
}

/// Replacement static tree for one compatible template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplatePatch {
	/// Stable template identity.
	pub key: TemplateKey,
	/// Dynamic ABI required by the patch.
	pub abi_hash: DynamicAbiHash,
	/// Replacement static tree.
	pub static_tree: StaticTemplateNode,
	/// Locations where retained dynamic slots must be inserted.
	pub placements: Vec<SlotPlacement>,
}

impl TemplatePatch {
	/// Creates a static replacement patch for a known compiled descriptor.
	///
	/// The dynamic ABI always comes from the compiled descriptor. Placement
	/// paths are derived solely from the replacement static tree, which keeps
	/// command-side classification and future-mount replay on the same wire
	/// representation.
	pub fn static_replacement(
		descriptor: &TemplateDescriptor,
		static_tree: StaticTemplateNode,
	) -> Self {
		Self {
			key: descriptor.key.clone(),
			abi_hash: descriptor.abi_hash,
			placements: static_slot_placements(&static_tree),
			static_tree,
		}
	}
}

/// Atomic collection of template patches for one source generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplatePatchBatch {
	/// Compiled build expected by the patch.
	pub build_id: CompiledBuildId,
	/// Digest of the compiled template manifest.
	pub manifest_digest: [u8; 32],
	/// Source generation represented by the patch.
	pub generation: PatchGeneration,
	/// Compatible template replacements.
	pub patches: Vec<TemplatePatch>,
}

/// Executable-code-free static template tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaticTemplateNode {
	/// Static element with static attributes and child nodes.
	Element {
		/// Element tag name.
		tag: String,
		/// Static attributes in source order.
		static_attrs: Vec<(String, String)>,
		/// Static and dynamic child positions.
		children: Vec<StaticTemplateNode>,
	},
	/// Literal text node.
	Text(String),
	/// Retained dynamic slot position.
	Slot(DynamicSlotId),
}

/// Path where a retained dynamic slot is placed in a static tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotPlacement {
	/// Retained slot identity.
	pub slot_id: DynamicSlotId,
	/// Child indexes from the template root to the slot position.
	pub path: Vec<u32>,
}

/// Computes every retained dynamic-slot placement in a static template tree.
pub fn static_slot_placements(node: &StaticTemplateNode) -> Vec<SlotPlacement> {
	let mut placements = Vec::new();
	let mut path = Vec::new();
	collect_static_slot_placements(node, &mut path, &mut placements);
	placements
}

fn collect_static_slot_placements(
	node: &StaticTemplateNode,
	path: &mut Vec<u32>,
	placements: &mut Vec<SlotPlacement>,
) {
	match node {
		StaticTemplateNode::Slot(slot_id) => placements.push(SlotPlacement {
			slot_id: *slot_id,
			path: path.clone(),
		}),
		StaticTemplateNode::Element { children, .. } => {
			for (index, child) in children.iter().enumerate() {
				path.push(index as u32);
				collect_static_slot_placements(child, path, placements);
				path.pop();
			}
		}
		StaticTemplateNode::Text(_) => {}
	}
}

/// Template identity sent by a browser when opening an HMR connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientHello {
	/// Build currently loaded by the browser.
	pub build_id: CompiledBuildId,
	/// Template manifest currently loaded by the browser.
	pub manifest_digest: [u8; 32],
	/// Dynamic ABI known for each compiled template.
	pub abi_hashes: Vec<(TemplateKey, DynamicAbiHash)>,
}

/// Reason a client could not apply a template patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchRejection {
	/// The browser is running a different compiled build.
	StaleBuild,
	/// The patch references a template absent from the client manifest.
	UnknownTemplate,
	/// The dynamic ABI differs from the compiled template.
	DynamicAbiMismatch,
	/// A retained slot could not be placed in the replacement tree.
	PlacementFailure,
	/// Transactional DOM replacement failed.
	TransactionFailure,
	/// A newer source generation superseded the patch.
	SupersededGeneration,
}

/// Build artifact targets that can participate in fallback compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildTarget {
	/// Native server artifact.
	Server,
	/// Browser WASM artifact.
	Wasm,
}

/// Source of a normalized development build diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticTarget {
	/// Template parsing or classification.
	Template,
	/// Native server Rust compilation.
	ServerRustc,
	/// Browser WASM Rust compilation.
	WasmRustc,
	/// wasm-bindgen artifact generation.
	WasmBindgen,
	/// Another build tool or phase.
	Other,
}

/// Severity of a normalized development build diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
	/// Compilation error.
	Error,
	/// Compiler warning.
	Warning,
	/// Supplemental note.
	Note,
	/// Suggested remediation.
	Help,
}

/// Project-relative source span attached to a build diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticSpan {
	/// Project-relative source file name.
	pub file_name: String,
	/// One-based starting line.
	pub line_start: u32,
	/// One-based ending line.
	pub line_end: u32,
	/// One-based starting column.
	pub column_start: u32,
	/// One-based ending column.
	pub column_end: u32,
	/// Whether this is the compiler's primary span.
	pub is_primary: bool,
	/// Optional compiler label for the span.
	pub label: Option<String>,
}

/// Serializable compiler diagnostic sent to development clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildDiagnostic {
	/// Source generation that produced the diagnostic.
	pub generation: PatchGeneration,
	/// Build phase that produced the diagnostic.
	pub target: DiagnosticTarget,
	/// Diagnostic severity.
	pub level: DiagnosticLevel,
	/// Primary compiler message.
	pub message: String,
	/// Optional compiler diagnostic code.
	pub code: Option<String>,
	/// Sanitized rendered compiler output.
	pub rendered: String,
	/// Sanitized project-relative source spans.
	pub relative_spans: Vec<DiagnosticSpan>,
}
