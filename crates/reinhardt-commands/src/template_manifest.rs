//! Source collection and compiler-to-wire template manifest conversion.

use std::{
	collections::{BTreeMap, BTreeSet},
	fmt,
	path::{Component, Path, PathBuf},
};

use proc_macro2::Span;
use quote::{ToTokens, quote};
use reinhardt_manouche::{
	core::TypedPageMacro,
	hot_reload::{
		CompilerDynamicSlotSignature, CompilerSlotKind, CompilerStaticTemplateNode,
		ManoucheHotReloadTemplate,
	},
	parser::parse_page,
	validator::validate_page,
};
use reinhardt_pages::hmr::{
	DynamicAbiHash, DynamicSlotDescriptor, SourceId, StaticTemplateNode, TemplateDescriptor,
	TemplateKey, template_manifest_identity,
};
use syn::{spanned::Spanned, visit::Visit, visit_mut::VisitMut};

/// A parsed `page!` invocation and its compiler-side descriptor.
#[derive(Debug, Clone)]
pub struct CollectedTemplate {
	/// The source callsite identity.
	pub key: TemplateKey,
	/// The descriptor generated from the validated Manouche AST.
	pub descriptor: TemplateDescriptor,
	/// Canonical macro tokens used to detect static content hidden behind a
	/// retained bound-element range.
	pub source_fingerprint: String,
}

/// Parsed information needed to classify one Rust source file.
#[derive(Debug, Clone)]
pub struct ParsedTemplateSource {
	/// Normalized Rust syntax with page macro bodies replaced by a placeholder.
	pub outside_page_fingerprint: String,
	/// Page invocations in lexical source order.
	pub templates: Vec<CollectedTemplate>,
}

/// Error raised while collecting or lowering a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestError {
	/// Human-readable parser or lowering message.
	pub message: String,
	/// Byte offsets in the supplied source, when a span was available.
	pub relative_span: Option<(usize, usize)>,
}

impl fmt::Display for ManifestError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(&self.message)
	}
}

impl std::error::Error for ManifestError {}

/// Collects and lowers every `page!` invocation in one Rust source file.
pub fn collect_template_source(
	source_id: &SourceId,
	source: &str,
) -> Result<ParsedTemplateSource, ManifestError> {
	let syntax = syn::parse_file(source).map_err(|error| ManifestError {
		message: error.to_string(),
		relative_span: span_to_offsets(source, error.span()),
	})?;

	let mut collector = PageMacroCollector {
		source,
		source_id,
		templates: Vec::new(),
		errors: Vec::new(),
	};
	collector.visit_file(&syntax);
	if let Some(error) = collector.errors.into_iter().next() {
		return Err(error);
	}

	let mut fingerprint_syntax = syntax.clone();
	PageBodyPlaceholder.visit_file_mut(&mut fingerprint_syntax);
	let outside_page_fingerprint = fingerprint_syntax.into_token_stream().to_string();

	Ok(ParsedTemplateSource {
		outside_page_fingerprint,
		templates: collector.templates,
	})
}

/// Collects the successful client-only `page!` manifest used by the active
/// development WASM build.
///
/// The watcher deliberately scans only source roots that it already owns and
/// only the client-only ownership paths accepted by the rebuild classifier.
/// Shared and SSR-visible sources always take the normal rebuild path and do
/// not participate in a compile-free template baseline.
pub fn collect_client_baseline(
	project_root: &Path,
	source_roots: &[PathBuf],
) -> Result<crate::CompiledBaseline, ManifestError> {
	let mut source_paths = BTreeSet::new();
	for root in source_roots {
		for entry in walkdir::WalkDir::new(root)
			.follow_links(false)
			.into_iter()
			.filter_map(Result::ok)
		{
			let path = entry.path();
			if entry.file_type().is_file()
				&& path.extension().and_then(|extension| extension.to_str()) == Some("rs")
				&& is_client_only_source(path)
			{
				source_paths.insert(path.to_path_buf());
			}
		}
	}

	let mut sources = BTreeMap::new();
	let mut descriptors = Vec::new();
	for path in source_paths {
		let source_id = source_id_for_path(project_root, &path);
		let source = std::fs::read_to_string(&path).map_err(|error| ManifestError {
			message: format!("failed to read {}: {error}", path.display()),
			relative_span: None,
		})?;
		let parsed = collect_template_source(&source_id, &source)?;
		if parsed.templates.is_empty() {
			continue;
		}
		let callsites = parsed
			.templates
			.iter()
			.map(|template| template.key.clone())
			.collect::<Vec<_>>();
		let source_descriptors = parsed
			.templates
			.into_iter()
			.map(|template| {
				descriptors.push(template.descriptor.clone());
				(template.key, template.descriptor)
			})
			.collect::<BTreeMap<_, _>>();
		sources.insert(
			source_id,
			crate::SourceBaseline {
				source_text: source,
				callsites,
				descriptors: source_descriptors,
			},
		);
	}

	let (build_id, manifest_digest) = template_manifest_identity(descriptors);
	Ok(crate::CompiledBaseline {
		build_id,
		manifest_digest,
		sources,
	})
}

/// Normalizes a source path into the same project-relative form used in wire
/// template keys.
pub fn source_id_for_path(project_root: &Path, path: &Path) -> SourceId {
	let relative = path
		.strip_prefix(project_root)
		.map(Path::to_path_buf)
		.unwrap_or_else(|_| path.to_path_buf());
	let normalized = relative
		.components()
		.filter_map(|component| match component {
			Component::Normal(value) => Some(value.to_string_lossy()),
			_ => None,
		})
		.collect::<Vec<_>>()
		.join("/");
	SourceId(normalized)
}

fn is_client_only_source(path: &Path) -> bool {
	let normalized = path.to_string_lossy().replace('\\', "/");
	let wrapped = format!("/{normalized}");
	wrapped.ends_with("/src/client.rs")
		|| wrapped.contains("/src/client/")
		|| (wrapped.contains("/src/apps/") && wrapped.contains("/client/"))
}

struct PageMacroCollector<'a> {
	source: &'a str,
	source_id: &'a SourceId,
	templates: Vec<CollectedTemplate>,
	errors: Vec<ManifestError>,
}

impl<'ast> Visit<'ast> for PageMacroCollector<'_> {
	fn visit_macro(&mut self, macro_node: &'ast syn::Macro) {
		if is_page_macro(&macro_node.path)
			&& let Err(error) = self.collect_macro(macro_node)
		{
			self.errors.push(error);
		}
		syn::visit::visit_macro(self, macro_node);
	}
}

impl PageMacroCollector<'_> {
	fn collect_macro(&mut self, macro_node: &syn::Macro) -> Result<(), ManifestError> {
		let parsed = parse_page(macro_node.tokens.clone()).map_err(|error| ManifestError {
			message: error.to_string(),
			relative_span: span_to_offsets(self.source, error.span()),
		})?;
		let typed = validate_page(&parsed).map_err(|error| ManifestError {
			message: error.to_string(),
			relative_span: span_to_offsets(self.source, error.span()),
		})?;
		let descriptor =
			descriptor_for_macro(self.source_id, macro_node, &typed).map_err(|message| {
				ManifestError {
					message,
					relative_span: span_to_offsets(self.source, macro_node.span()),
				}
			})?;
		self.templates.push(CollectedTemplate {
			key: descriptor.key.clone(),
			descriptor,
			source_fingerprint: macro_node.tokens.to_string(),
		});
		Ok(())
	}
}

struct PageBodyPlaceholder;

impl VisitMut for PageBodyPlaceholder {
	fn visit_macro_mut(&mut self, macro_node: &mut syn::Macro) {
		if is_page_macro(&macro_node.path) {
			macro_node.tokens = quote! { __reinhardt_page_body__ };
			return;
		}
		syn::visit_mut::visit_macro_mut(self, macro_node);
	}
}

fn descriptor_for_macro(
	source_id: &SourceId,
	macro_node: &syn::Macro,
	typed: &TypedPageMacro,
) -> Result<TemplateDescriptor, String> {
	let lowered = reinhardt_manouche::hot_reload::lower_page_macro(typed)
		.map_err(|error| error.to_string())?;
	let span = macro_node.path.span();
	let start = span.start();
	let mut next_nested_index = 1;
	Ok(convert_template(
		&lowered,
		source_id,
		start.line as u32,
		(start.column + 1) as u32,
		0,
		&mut next_nested_index,
	))
}

fn convert_template(
	template: &ManoucheHotReloadTemplate,
	source_id: &SourceId,
	line: u32,
	column: u32,
	nested_template_index: u32,
	next_nested_index: &mut u32,
) -> TemplateDescriptor {
	let key = TemplateKey {
		source_id: source_id.clone(),
		line,
		column,
		nested_template_index,
	};
	let nested = template
		.nested
		.iter()
		.map(|nested_template| {
			let index = *next_nested_index;
			*next_nested_index = next_nested_index.saturating_add(1);
			convert_template(
				nested_template,
				source_id,
				line,
				column,
				index,
				next_nested_index,
			)
		})
		.collect();
	TemplateDescriptor {
		key,
		abi_hash: DynamicAbiHash(template.abi_hash.0),
		static_tree: convert_static_tree(&template.static_tree),
		slots: template.slots.iter().map(convert_slot).collect(),
		nested,
	}
}

fn convert_static_tree(node: &CompilerStaticTemplateNode) -> StaticTemplateNode {
	match node {
		CompilerStaticTemplateNode::Element {
			tag,
			static_attrs,
			children,
		} => StaticTemplateNode::Element {
			tag: tag.clone(),
			static_attrs: static_attrs.clone(),
			children: children.iter().map(convert_static_tree).collect(),
		},
		CompilerStaticTemplateNode::Text(text) => StaticTemplateNode::Text(text.clone()),
		CompilerStaticTemplateNode::Slot(slot_id) => {
			StaticTemplateNode::Slot(reinhardt_pages::hmr::DynamicSlotId(slot_id.0))
		}
	}
}

fn convert_slot(slot: &CompilerDynamicSlotSignature) -> DynamicSlotDescriptor {
	DynamicSlotDescriptor {
		slot_id: reinhardt_pages::hmr::DynamicSlotId(slot.slot_id.0),
		semantic_kind: semantic_kind_name(&slot.semantic_kind),
		canonical_tokens: slot.canonical_tokens.clone(),
	}
}

fn semantic_kind_name(kind: &CompilerSlotKind) -> String {
	match kind {
		CompilerSlotKind::Expression => "expression".to_owned(),
		CompilerSlotKind::DynamicAttribute { name } => format!("dynamic_attribute:{name}"),
		CompilerSlotKind::Event { name } => format!("event:{name}"),
		CompilerSlotKind::IfCondition => "if_condition".to_owned(),
		CompilerSlotKind::ForIteration => "for_iteration".to_owned(),
		CompilerSlotKind::ForKey => "for_key".to_owned(),
		CompilerSlotKind::ComponentInvocation => "component_invocation".to_owned(),
		CompilerSlotKind::HeadExpression => "head_expression".to_owned(),
	}
}

fn is_page_macro(path: &syn::Path) -> bool {
	path.segments
		.last()
		.is_some_and(|segment| segment.ident == "page")
}

fn span_to_offsets(source: &str, span: Span) -> Option<(usize, usize)> {
	let start = span.start();
	let end = span.end();
	Some((
		line_column_to_offset(source, start)?,
		line_column_to_offset(source, end)?,
	))
}

fn line_column_to_offset(source: &str, line: proc_macro2::LineColumn) -> Option<usize> {
	let mut offset = 0;
	for (index, line_text) in source.split_inclusive('\n').enumerate() {
		if index + 1 == line.line {
			return Some(offset + line.column.min(line_text.trim_end_matches('\n').len()));
		}
		offset += line_text.len();
	}
	if line.line == source.lines().count().saturating_add(1) {
		Some(source.len())
	} else {
		None
	}
}
