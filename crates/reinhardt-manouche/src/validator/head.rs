//! Validation for head! macro.

use quote::ToTokens;
use syn::Result;

use crate::core::{HeadMacro, TypedHeadAttr, TypedHeadContent, TypedHeadElement, TypedHeadMacro};

/// Validates a `HeadMacro`.
///
/// Currently performs a simple pass-through validation since head elements
/// don't require complex type transformations like page or form elements.
/// Future enhancements may include:
/// - Validation of element names
/// - Validation of attribute combinations
/// - Validation of content structure
pub fn validate_head(ast: &HeadMacro) -> Result<TypedHeadMacro> {
	// Currently a no-op validator: creates a typed version from the untyped AST.
	// The typed version will be used by the IR lowering and code generation.
	Ok(TypedHeadMacro {
		elements: ast
			.elements
			.iter()
			.map(|elem| TypedHeadElement {
				tag: elem.tag.to_string(),
				attrs: elem
					.attrs
					.iter()
					.map(|attr| TypedHeadAttr {
						name: attr.name.to_string(),
						value: attr.value.to_token_stream().to_string(),
						span: attr.span,
					})
					.collect(),
				content: elem.content.as_ref().map(|content| match content {
					crate::core::HeadContent::Text(s) => TypedHeadContent::Static(s.clone()),
					crate::core::HeadContent::Expr(e) => TypedHeadContent::Dynamic(e.clone()),
				}),
				span: elem.span,
			})
			.collect(),
		span: ast.span,
	})
}
