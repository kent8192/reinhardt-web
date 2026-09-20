//! OXC AST reference sites; unrelated strings/comments are never edited.

use super::{Site, external, map_reference, reference};
use crate::staticfiles::publication::{AssetBuildError, AssetReference};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};

pub(super) fn analyze(logical: &str, source: &str) -> Result<Vec<AssetReference>, AssetBuildError> {
	let allocator = Allocator::default();
	let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
	if let Some(error) = parsed.errors.first() {
		return Err(AssetBuildError::input(
			logical,
			format!("JavaScript syntax: {error:?}"),
		));
	}
	let mut visitor = References {
		logical,
		source,
		references: Vec::new(),
		error: None,
	};
	visitor.visit_program(&parsed.program);
	if let Some(error) = visitor.error {
		return Err(error);
	}
	for comment in &parsed.program.comments {
		let span = comment.content_span();
		if let Some(reference) = map_reference(
			logical,
			&source[span.start as usize..span.end as usize],
			span.start as usize,
		)? {
			visitor.references.push(reference);
		}
	}
	Ok(visitor.references)
}

struct References<'s> {
	logical: &'s str,
	source: &'s str,
	references: Vec<AssetReference>,
	error: Option<AssetBuildError>,
}

impl References<'_> {
	fn fail(&mut self, span: Span, reason: &str) {
		if self.error.is_none() {
			let before = &self.source[..span.start as usize];
			let line = before.bytes().filter(|b| *b == b'\n').count() + 1;
			let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
			self.error = Some(AssetBuildError::input(
				self.logical,
				format!(
					"{reason} at {line}:{column}; use the generation-bound resolver or register a processor"
				),
			));
		}
	}
	fn literal(&mut self, value: &str, span: Span, esm: bool) {
		if esm && !value.starts_with(['.', '/']) && !external(value) {
			self.fail(
				span,
				"bare module specifiers require bundling or explicit resolution",
			);
			return;
		}
		match reference(
			self.logical,
			value,
			Site::JavaScript {
				start: span.start as usize,
				end: span.end as usize,
			},
		) {
			Ok(Some(reference)) => self.references.push(reference),
			Ok(None) => {}
			Err(error) => {
				if self.error.is_none() {
					self.error = Some(error);
				}
			}
		}
	}
}

impl<'a> Visit<'a> for References<'_> {
	fn visit_import_declaration(&mut self, node: &ImportDeclaration<'a>) {
		self.literal(node.source.value.as_str(), node.source.span, true);
		walk::walk_import_declaration(self, node);
	}
	fn visit_export_named_declaration(&mut self, node: &ExportNamedDeclaration<'a>) {
		if let Some(source) = &node.source {
			self.literal(source.value.as_str(), source.span, true);
		}
		walk::walk_export_named_declaration(self, node);
	}
	fn visit_export_all_declaration(&mut self, node: &ExportAllDeclaration<'a>) {
		self.literal(node.source.value.as_str(), node.source.span, true);
		walk::walk_export_all_declaration(self, node);
	}
	fn visit_import_expression(&mut self, node: &ImportExpression<'a>) {
		if let Expression::StringLiteral(value) = &node.source {
			self.literal(value.value.as_str(), value.span, true);
		} else {
			self.fail(
				node.span,
				"computed dynamic import cannot be relocated automatically",
			);
		}
		walk::walk_import_expression(self, node);
	}
	fn visit_new_expression(&mut self, node: &NewExpression<'a>) {
		let is_url = matches!(&node.callee, Expression::Identifier(name) if name.name == "URL");
		let meta_base = matches!(node.arguments.get(1), Some(Argument::StaticMemberExpression(member)) if member.property.name == "url" && matches!(&member.object, Expression::MetaProperty(meta) if meta.meta.name == "import" && meta.property.name == "meta"));
		if is_url && meta_base {
			if let Some(Argument::StringLiteral(value)) = node.arguments.first() {
				self.literal(value.value.as_str(), value.span, false);
			} else {
				self.fail(
					node.span,
					"computed import.meta.url dependency cannot be relocated automatically",
				);
			}
		}
		walk::walk_new_expression(self, node);
	}
	fn visit_call_expression(&mut self, node: &CallExpression<'a>) {
		if matches!(&node.callee, Expression::Identifier(name) if name.name == "require") {
			self.fail(
				node.span,
				"CommonJS require is not an ES-module publication contract",
			);
		}
		walk::walk_call_expression(self, node);
	}
}
