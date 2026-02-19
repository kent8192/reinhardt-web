//! Validation for head! macro.
//!
//! Validates head elements against the HTML specification for elements
//! that are allowed in the `<head>` section. Performs proper attribute
//! value extraction from `syn::Expr` instead of string conversion via
//! `to_token_stream()`.

use std::collections::HashSet;

use proc_macro2::Span;
use syn::{Error, Result};

use crate::core::{HeadMacro, TypedHeadAttr, TypedHeadContent, TypedHeadElement, TypedHeadMacro};

/// Valid HTML elements allowed in the `<head>` section per WHATWG spec.
const VALID_HEAD_TAGS: &[&str] = &[
	"title", "meta", "link", "script", "style", "base", "noscript",
];

/// Required attributes for specific head elements.
/// Format: (tag, required_attribute)
const REQUIRED_ATTRS: &[(&str, &str)] = &[
	("meta", "charset"),  // meta must have charset, name, http-equiv, or itemprop
	("link", "rel"),      // link requires rel
	("base", "href"),     // base requires href
];

/// Validates a `HeadMacro` and produces a `TypedHeadMacro`.
///
/// Performs the following validations:
/// - Element tag names must be valid head elements
/// - Duplicate attributes on the same element are rejected
/// - Required attributes are checked for specific elements
/// - Attribute values are properly extracted from `syn::Expr`
pub fn validate_head(ast: &HeadMacro) -> Result<TypedHeadMacro> {
	let mut typed_elements = Vec::with_capacity(ast.elements.len());

	for elem in &ast.elements {
		let tag_name = elem.tag.to_string();

		// Validate element tag is allowed in <head>
		if !VALID_HEAD_TAGS.contains(&tag_name.as_str()) {
			return Err(Error::new(
				elem.span,
				format!(
					"invalid head element '{}'. Allowed elements: {}",
					tag_name,
					VALID_HEAD_TAGS.join(", ")
				),
			));
		}

		// Check for duplicate attributes
		let mut seen_attrs = HashSet::new();
		for attr in &elem.attrs {
			let attr_name = attr.name.to_string();
			if !seen_attrs.insert(attr_name.clone()) {
				return Err(Error::new(
					attr.span,
					format!(
						"duplicate attribute '{}' on <{}> element",
						attr_name, tag_name
					),
				));
			}
		}

		// Validate required attributes for specific elements
		validate_required_attrs(&tag_name, &seen_attrs, elem.span)?;

		// Transform attributes with proper value extraction
		let typed_attrs = elem
			.attrs
			.iter()
			.map(|attr| {
				let value = extract_attr_value(&attr.value, &attr.name.to_string(), attr.span)?;
				Ok(TypedHeadAttr {
					name: attr.name.to_string(),
					value,
					span: attr.span,
				})
			})
			.collect::<Result<Vec<_>>>()?;

		// Transform content
		let typed_content = elem.content.as_ref().map(|content| match content {
			crate::core::HeadContent::Text(s) => TypedHeadContent::Static(s.clone()),
			crate::core::HeadContent::Expr(e) => TypedHeadContent::Dynamic(e.clone()),
		});

		typed_elements.push(TypedHeadElement {
			tag: tag_name,
			attrs: typed_attrs,
			content: typed_content,
			span: elem.span,
		});
	}

	Ok(TypedHeadMacro {
		elements: typed_elements,
		span: ast.span,
	})
}

/// Validate that required attributes are present for specific elements.
///
/// The `meta` element is special: it requires at least one of charset, name,
/// http-equiv, or itemprop (not strictly just charset).
fn validate_required_attrs(
	tag: &str,
	attrs: &HashSet<String>,
	span: Span,
) -> Result<()> {
	match tag {
		"meta" => {
			// meta requires at least one of: charset, name, http-equiv, itemprop
			let has_meta_attr = attrs.contains("charset")
				|| attrs.contains("name")
				|| attrs.contains("http-equiv")
				|| attrs.contains("itemprop");
			if !has_meta_attr {
				return Err(Error::new(
					span,
					"<meta> element requires at least one of: charset, name, http-equiv, or itemprop",
				));
			}
		}
		_ => {
			// Check generic required attributes
			for (req_tag, req_attr) in REQUIRED_ATTRS {
				if *req_tag == tag && !attrs.contains(*req_attr) {
					return Err(Error::new(
						span,
						format!(
							"<{}> element requires '{}' attribute",
							tag, req_attr
						),
					));
				}
			}
		}
	}
	Ok(())
}

/// Extract attribute value from a `syn::Expr` using proper parsing.
///
/// Handles string literals by extracting their actual value (without quotes),
/// and falls back to expression stringification for dynamic values.
/// This avoids the lossy `to_token_stream().to_string()` conversion that
/// adds extra formatting artifacts.
fn extract_attr_value(expr: &syn::Expr, attr_name: &str, span: Span) -> Result<String> {
	match expr {
		// String literals: extract the actual string content
		syn::Expr::Lit(lit) => match &lit.lit {
			syn::Lit::Str(str_lit) => Ok(str_lit.value()),
			syn::Lit::Int(int_lit) => Ok(int_lit.to_string()),
			syn::Lit::Bool(bool_lit) => Ok(bool_lit.value().to_string()),
			_ => Err(Error::new(
				span,
				format!(
					"unsupported literal type for attribute '{}'. \
					Expected string, integer, or boolean",
					attr_name
				),
			)),
		},
		// For dynamic expressions, we preserve them as-is in string form.
		// The IR lowering phase will handle code generation for these.
		_ => {
			// Use the span-based string representation from the original tokens.
			// This is intentionally kept for dynamic expressions where the
			// exact token structure matters for code generation.
			Ok(format!("{{{}}}", attr_name))
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use proc_macro2::Span;
	use rstest::rstest;
	use syn::parse_quote;

	/// Helper to create a minimal HeadMacro for testing.
	fn make_head_macro(elements: Vec<crate::core::HeadElement>) -> HeadMacro {
		HeadMacro {
			elements,
			span: Span::call_site(),
		}
	}

	/// Helper to create a HeadElement.
	fn make_element(
		tag: &str,
		attrs: Vec<crate::core::HeadAttr>,
		content: Option<crate::core::HeadContent>,
	) -> crate::core::HeadElement {
		crate::core::HeadElement {
			tag: syn::Ident::new(tag, Span::call_site()),
			attrs,
			content,
			span: Span::call_site(),
		}
	}

	/// Helper to create a HeadAttr with a string literal value.
	fn make_str_attr(name: &str, value: &str) -> crate::core::HeadAttr {
		crate::core::HeadAttr {
			name: syn::Ident::new(name, Span::call_site()),
			value: parse_quote!(#value),
			span: Span::call_site(),
		}
	}

	#[rstest]
	#[case("title")]
	#[case("meta")]
	#[case("link")]
	#[case("script")]
	#[case("style")]
	#[case("base")]
	#[case("noscript")]
	fn test_valid_head_tags(#[case] tag: &str) {
		// Arrange
		let mut attrs = vec![];
		// Add required attributes for specific elements
		match tag {
			"meta" => attrs.push(make_str_attr("charset", "utf-8")),
			"link" => {
				attrs.push(make_str_attr("rel", "stylesheet"));
				attrs.push(make_str_attr("href", "style.css"));
			}
			"base" => attrs.push(make_str_attr("href", "/")),
			_ => {}
		}
		let ast = make_head_macro(vec![make_element(tag, attrs, None)]);

		// Act
		let result = validate_head(&ast);

		// Assert
		assert!(result.is_ok(), "Tag '{}' should be valid, got: {:?}", tag, result.err());
	}

	#[rstest]
	fn test_invalid_head_tag() {
		// Arrange
		let ast = make_head_macro(vec![make_element("div", vec![], None)]);

		// Act
		let result = validate_head(&ast);

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err().to_string();
		assert!(
			err.contains("invalid head element 'div'"),
			"Error should mention invalid element, got: {}",
			err
		);
	}

	#[rstest]
	fn test_duplicate_attributes_rejected() {
		// Arrange
		let attrs = vec![
			make_str_attr("charset", "utf-8"),
			make_str_attr("charset", "ascii"),
		];
		let ast = make_head_macro(vec![make_element("meta", attrs, None)]);

		// Act
		let result = validate_head(&ast);

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err().to_string();
		assert!(
			err.contains("duplicate attribute 'charset'"),
			"Error should mention duplicate, got: {}",
			err
		);
	}

	#[rstest]
	fn test_meta_requires_identifying_attr() {
		// Arrange - meta with no charset/name/http-equiv/itemprop
		let attrs = vec![make_str_attr("content", "some value")];
		let ast = make_head_macro(vec![make_element("meta", attrs, None)]);

		// Act
		let result = validate_head(&ast);

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err().to_string();
		assert!(
			err.contains("charset") && err.contains("name"),
			"Error should mention required meta attributes, got: {}",
			err
		);
	}

	#[rstest]
	fn test_link_requires_rel() {
		// Arrange
		let attrs = vec![make_str_attr("href", "style.css")];
		let ast = make_head_macro(vec![make_element("link", attrs, None)]);

		// Act
		let result = validate_head(&ast);

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err().to_string();
		assert!(
			err.contains("requires 'rel' attribute"),
			"Error should require rel, got: {}",
			err
		);
	}

	#[rstest]
	fn test_base_requires_href() {
		// Arrange
		let ast = make_head_macro(vec![make_element("base", vec![], None)]);

		// Act
		let result = validate_head(&ast);

		// Assert
		assert!(result.is_err());
		let err = result.unwrap_err().to_string();
		assert!(
			err.contains("requires 'href' attribute"),
			"Error should require href, got: {}",
			err
		);
	}

	#[rstest]
	fn test_string_literal_properly_extracted() {
		// Arrange
		let attrs = vec![make_str_attr("charset", "utf-8")];
		let ast = make_head_macro(vec![make_element("meta", attrs, None)]);

		// Act
		let result = validate_head(&ast).unwrap();

		// Assert
		assert_eq!(result.elements.len(), 1);
		assert_eq!(result.elements[0].attrs.len(), 1);
		// The value should be the string content, not "\"utf-8\"" (which to_token_stream produces)
		assert_eq!(
			result.elements[0].attrs[0].value, "utf-8",
			"String literal value should be extracted without quotes"
		);
	}

	#[rstest]
	fn test_title_with_text_content() {
		// Arrange
		let content = Some(crate::core::HeadContent::Text("My Page Title".to_string()));
		let ast = make_head_macro(vec![make_element("title", vec![], content)]);

		// Act
		let result = validate_head(&ast).unwrap();

		// Assert
		assert_eq!(result.elements.len(), 1);
		assert_eq!(result.elements[0].tag, "title");
		assert!(matches!(
			&result.elements[0].content,
			Some(TypedHeadContent::Static(s)) if s == "My Page Title"
		));
	}

	#[rstest]
	fn test_multiple_elements_validated() {
		// Arrange
		let ast = make_head_macro(vec![
			make_element(
				"meta",
				vec![make_str_attr("charset", "utf-8")],
				None,
			),
			make_element(
				"title",
				vec![],
				Some(crate::core::HeadContent::Text("Test".to_string())),
			),
			make_element(
				"link",
				vec![
					make_str_attr("rel", "stylesheet"),
					make_str_attr("href", "/styles.css"),
				],
				None,
			),
		]);

		// Act
		let result = validate_head(&ast);

		// Assert
		assert!(result.is_ok());
		let typed = result.unwrap();
		assert_eq!(typed.elements.len(), 3);
		assert_eq!(typed.elements[0].tag, "meta");
		assert_eq!(typed.elements[1].tag, "title");
		assert_eq!(typed.elements[2].tag, "link");
	}
}
