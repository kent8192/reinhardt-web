//! Compile canonical component style definitions from one Cargo package.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use cargo_metadata::{Metadata, MetadataCommand, Package, PackageId};
use quote::{ToTokens, quote};
use reinhardt_manouche::{CompiledStyle, StyleCompileContext, compile_style, serialize_css};
use sha2::{Digest, Sha256};
use syn::visit::Visit;
use syn::visit_mut::VisitMut;
use syn::{Expr, ItemConst, ItemFn, ItemMod, ItemStatic, Meta, StaticMutability, Type};
use walkdir::WalkDir;

/// Stable logical path used for generated component CSS.
pub const COMPONENT_STYLES_PATH: &str = "__reinhardt__/components.css";

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
}

impl StylePackageContext {
	/// Select a package from already loaded Cargo metadata.
	pub fn from_metadata(
		metadata: &Metadata,
		requested_package: Option<&str>,
	) -> Result<Self, String> {
		let package = if let Some(requested) = requested_package {
			let matches: Vec<&Package> = metadata
				.packages
				.iter()
				.filter(|package| package.name.as_str() == requested)
				.collect();
			match matches.as_slice() {
				[package] => *package,
				[] => return Err(format!("Cargo package `{requested}` was not found")),
				_ => {
					return Err(format!(
						"Cargo package name `{requested}` is ambiguous; select a unique package"
					));
				}
			}
		} else {
			metadata.root_package().ok_or_else(|| {
				"the Cargo workspace has no root package; pass --package <NAME>".to_string()
			})?
		};

		let package_manifest_path = package.manifest_path.clone().into_std_path_buf();
		let src_root = package_manifest_path
			.parent()
			.ok_or_else(|| "selected package manifest has no parent directory".to_string())?
			.join("src");
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
		})
	}

	/// Load Cargo metadata and select exactly one package.
	pub fn resolve(
		manifest_path: impl AsRef<Path>,
		requested_package: Option<&str>,
	) -> Result<Self, String> {
		let mut command = MetadataCommand::new();
		command.manifest_path(manifest_path.as_ref());
		let metadata = command
			.exec()
			.map_err(|error| format!("failed to load Cargo metadata: {error}"))?;
		Self::from_metadata(&metadata, requested_package)
	}
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

	/// Discover canonical definitions, compile them, and build stable outputs.
	pub fn extract(&self) -> Result<StyleBundle, String> {
		let source_files = source_files(&self.context.src_root)?;
		let mut definitions = Vec::new();
		let mut non_style_hasher = Sha256::new();

		for source_path in &source_files {
			let source = std::fs::read_to_string(source_path)
				.map_err(|error| format!("failed to read {}: {error}", source_path.display()))?;
			let file = syn::parse_file(&source)
				.map_err(|error| format!("failed to parse {}: {error}", source_path.display()))?;
			let mut scanner = DefinitionScanner::new(source_path);
			scanner.visit_file(&file);
			if let Some(error) = scanner.error {
				return Err(error);
			}
			for authored in scanner.definitions {
				let compile_context = StyleCompileContext {
					package_name: &self.context.package_name,
					package_version: &self.context.package_version,
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
			StyleBodyMarker.visit_file_mut(&mut normalized_file);
			let relative = source_path
				.strip_prefix(&self.context.src_root)
				.unwrap_or(source_path);
			non_style_hasher.update(relative.to_string_lossy().replace('\\', "/").as_bytes());
			non_style_hasher.update([0]);
			non_style_hasher.update(normalized_file.into_token_stream().to_string().as_bytes());
			non_style_hasher.update([0]);
		}

		definitions.sort_by(|left, right| {
			(
				self.context.package_name.as_str(),
				left.style_type_name.as_str(),
			)
				.cmp(&(
					self.context.package_name.as_str(),
					right.style_type_name.as_str(),
				))
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

fn source_files(src_root: &Path) -> Result<Vec<PathBuf>, String> {
	if !src_root.exists() {
		return Ok(Vec::new());
	}
	let mut files = Vec::new();
	for entry in WalkDir::new(src_root) {
		let entry =
			entry.map_err(|error| format!("failed to walk {}: {error}", src_root.display()))?;
		if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "rs") {
			files.push(entry.into_path());
		}
	}
	files.sort_by_key(|path| path.to_string_lossy().replace('\\', "/"));
	Ok(files)
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
	definitions: Vec<AuthoredDefinition>,
	error: Option<String>,
}

impl<'a> DefinitionScanner<'a> {
	fn new(source_path: &'a Path) -> Self {
		Self {
			source_path,
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

impl<'ast> Visit<'ast> for DefinitionScanner<'_> {
	fn visit_item_fn(&mut self, item: &'ast ItemFn) {
		if item.attrs.iter().any(|attribute| {
			attribute
				.path()
				.segments
				.last()
				.is_some_and(|segment| segment.ident == "style_def")
		}) {
			self.reject(
				item.sig.fn_token.span,
				"the style_def attribute is supported only on immutable static items",
			);
			return;
		}
		syn::visit::visit_item_fn(self, item);
	}

	fn visit_item_mod(&mut self, item: &'ast ItemMod) {
		if item.attrs.iter().any(|attribute| {
			attribute
				.path()
				.segments
				.last()
				.is_some_and(|segment| segment.ident == "style_def")
		}) {
			self.reject(
				item.mod_token.span,
				"the style_def attribute is supported only on immutable static items",
			);
			return;
		}
		syn::visit::visit_item_mod(self, item);
	}

	fn visit_item_static(&mut self, item: &'ast ItemStatic) {
		let style_attributes: Vec<_> = item
			.attrs
			.iter()
			.filter(|attribute| {
				attribute
					.path()
					.segments
					.last()
					.is_some_and(|segment| segment.ident == "style_def")
			})
			.collect();
		let style_macro = match item.expr.as_ref() {
			Expr::Macro(expression)
				if expression
					.mac
					.path
					.segments
					.last()
					.is_some_and(|segment| segment.ident == "style") =>
			{
				Some(&expression.mac)
			}
			_ => None,
		};
		if style_attributes.is_empty() && style_macro.is_none() {
			syn::visit::visit_item_static(self, item);
			return;
		}

		let exact_attribute = style_attributes.len() == 1
			&& matches!(&style_attributes[0].meta, Meta::Path(path) if path.segments.len() == 1 && path.is_ident("style_def"));
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
		let has_style_attribute = item.attrs.iter().any(|attribute| {
			attribute
				.path()
				.segments
				.last()
				.is_some_and(|segment| segment.ident == "style_def")
		});
		let has_style_macro = matches!(item.expr.as_ref(), Expr::Macro(expression) if expression.mac.path.segments.last().is_some_and(|segment| segment.ident == "style"));
		if has_style_attribute || has_style_macro {
			self.reject(
				item.const_token.span,
				"component styles must use an immutable static item",
			);
			return;
		}
		syn::visit::visit_item_const(self, item);
	}
}

struct StyleBodyMarker;

impl VisitMut for StyleBodyMarker {
	fn visit_expr_macro_mut(&mut self, expression: &mut syn::ExprMacro) {
		if expression.mac.path.segments.len() == 1 && expression.mac.path.is_ident("style") {
			expression.mac.tokens = quote! { __reinhardt_style_body_marker };
			return;
		}
		syn::visit_mut::visit_expr_macro_mut(self, expression);
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
