//! Validation and transformation logic for page! macro AST.
//!
//! This module transforms the untyped AST from the parser into a typed AST,
//! while performing semantic validation and type checking.
//!
//! ## Validation Rules
//!
//! 1. **Event Handlers**: Must be closure expressions with 0 or 1 arguments
//! 2. **Attributes**: data-* and aria-* attributes must follow naming conventions
//! 3. **Element Nesting**: Void elements cannot have children, interactive elements cannot nest
//! 4. **Required Attributes**: img elements must have alt attributes (accessibility)
//! 5. **Attribute Types**: Certain attributes must be specific types (e.g., img src must be string literal)

use proc_macro2::Span;
use syn::{Expr, Result};

use reinhardt_pages_ast::{
	PageAttr, PageBody, PageComponent, PageElement, PageElse, PageEvent, PageMacro, PageNode,
	TypedPageAttr, TypedPageBody, TypedPageComponent, TypedPageElement, TypedPageElse,
	TypedPageFor, TypedPageIf, TypedPageMacro, TypedPageNode, types::AttrValue,
};

/// Validates and transforms the entire PageMacro AST into a typed AST.
///
/// This is the main entry point for validation. It performs semantic checks
/// and type transformations, returning a fully validated and typed AST.
///
/// # Errors
///
/// Returns a compilation error if any validation rule is violated.
///
/// # Returns
///
/// A `TypedPageMacro` with validated and type-safe attribute values.
pub(super) fn validate(ast: &PageMacro) -> Result<TypedPageMacro> {
	let typed_body = transform_body(&ast.body, &[])?;

	Ok(TypedPageMacro {
		params: ast.params.clone(),
		body: typed_body,
		span: ast.span,
	})
}

/// Transforms a PageBody into a TypedPageBody.
///
/// # Arguments
///
/// * `body` - The untyped body to transform
/// * `parent_tags` - Stack of parent element tag names (for nesting validation)
fn transform_body(body: &PageBody, parent_tags: &[String]) -> Result<TypedPageBody> {
	let nodes = transform_nodes(&body.nodes, parent_tags)?;
	Ok(TypedPageBody {
		nodes,
		span: body.span,
	})
}

/// Transforms a slice of page nodes into typed nodes recursively.
///
/// # Arguments
///
/// * `nodes` - The nodes to transform
/// * `parent_tags` - Stack of parent element tag names (for nesting validation)
fn transform_nodes(nodes: &[PageNode], parent_tags: &[String]) -> Result<Vec<TypedPageNode>> {
	let mut typed_nodes = Vec::new();

	for node in nodes {
		typed_nodes.push(transform_node(node, parent_tags)?);
	}

	Ok(typed_nodes)
}

/// Transforms a single PageNode into a TypedPageNode.
///
/// Dispatches to the appropriate transformation function based on node type.
fn transform_node(node: &PageNode, parent_tags: &[String]) -> Result<TypedPageNode> {
	match node {
		PageNode::Element(elem) => Ok(TypedPageNode::Element(transform_element(
			elem,
			parent_tags,
		)?)),
		PageNode::Text(text) => Ok(TypedPageNode::Text(text.clone())),
		PageNode::Expression(expr) => Ok(TypedPageNode::Expression(expr.clone())),
		PageNode::If(if_node) => Ok(TypedPageNode::If(transform_if(if_node, parent_tags)?)),
		PageNode::For(for_node) => Ok(TypedPageNode::For(transform_for(for_node, parent_tags)?)),
		PageNode::Component(comp) => Ok(TypedPageNode::Component(transform_component(
			comp,
			parent_tags,
		)?)),
	}
}

/// Transforms a PageIf node (if/else if/else).
///
/// Recursively validates all branches.
fn transform_if(
	if_node: &reinhardt_pages_ast::PageIf,
	parent_tags: &[String],
) -> Result<TypedPageIf> {
	// Transform then branch
	let then_branch = transform_nodes(&if_node.then_branch, parent_tags)?;

	// Transform else branch if present
	let else_branch = if let Some(else_br) = &if_node.else_branch {
		Some(transform_else(else_br, parent_tags)?)
	} else {
		None
	};

	Ok(TypedPageIf {
		condition: if_node.condition.clone(),
		then_branch,
		else_branch,
		span: if_node.span,
	})
}

/// Transforms a PageElse branch.
fn transform_else(else_branch: &PageElse, parent_tags: &[String]) -> Result<TypedPageElse> {
	match else_branch {
		PageElse::Block(nodes) => {
			let typed_nodes = transform_nodes(nodes, parent_tags)?;
			Ok(TypedPageElse::Block(typed_nodes))
		}
		PageElse::If(nested_if) => {
			// Recursively transform nested if
			let typed_if = transform_if(nested_if, parent_tags)?;
			Ok(TypedPageElse::If(Box::new(typed_if)))
		}
	}
}

/// Transforms a PageFor node.
fn transform_for(
	for_node: &reinhardt_pages_ast::PageFor,
	parent_tags: &[String],
) -> Result<TypedPageFor> {
	let body = transform_nodes(&for_node.body, parent_tags)?;

	Ok(TypedPageFor {
		pat: for_node.pat.clone(),
		iter: for_node.iter.clone(),
		body,
		span: for_node.span,
	})
}

/// Transforms a PageComponent node.
///
/// Recursively transforms the component's children (if any).
fn transform_component(comp: &PageComponent, parent_tags: &[String]) -> Result<TypedPageComponent> {
	// Transform children if present
	let typed_children = if let Some(children) = &comp.children {
		Some(transform_nodes(children, parent_tags)?)
	} else {
		None
	};

	Ok(TypedPageComponent {
		name: comp.name.clone(),
		args: comp.args.clone(),
		children: typed_children,
		span: comp.span,
	})
}

/// Transforms and validates an element and its children.
///
/// Performs the following checks:
/// - Event handler validation
/// - Attribute transformation and validation
/// - Element nesting rules
/// - Required attributes
/// - HTML specification compliance (Phase 2)
fn transform_element(elem: &PageElement, parent_tags: &[String]) -> Result<TypedPageElement> {
	let tag = elem.tag.to_string();

	// 1. Validate events (unchanged from untyped version)
	for event in &elem.events {
		validate_event_handler(event)?;
	}

	// 2. Transform and validate attributes
	let typed_attrs = transform_attrs(&elem.attrs, &tag)?;

	// 3. Validate element nesting
	validate_element_nesting(elem, parent_tags)?;

	// 4. Validate required attributes (using typed attrs)
	validate_required_attributes(&tag, &typed_attrs, elem.span)?;

	// 5. Recursively transform children
	let mut child_tags = parent_tags.to_vec();
	child_tags.push(tag.clone());
	let typed_children = transform_nodes(&elem.children, &child_tags)?;

	// Create typed element
	let typed_element = TypedPageElement {
		tag: elem.tag.clone(),
		attrs: typed_attrs,
		events: elem.events.clone(),
		children: typed_children,
		span: elem.span,
	};

	// 6. Validate against HTML specification (Phase 2)
	super::html_spec::validate_against_spec(&typed_element)?;

	Ok(typed_element)
}

/// Transforms attributes from untyped to typed, with validation.
///
/// This function converts `Expr` attribute values into `AttrValue`,
/// enabling type-specific validation.
fn transform_attrs(attrs: &[PageAttr], element_tag: &str) -> Result<Vec<TypedPageAttr>> {
	let mut typed_attrs = Vec::new();

	for attr in attrs {
		// Validate attribute naming conventions (data-*, aria-*)
		validate_attribute(attr, element_tag)?;

		// Transform to typed value
		let typed_value = AttrValue::from_expr(attr.value.clone());

		// Validate attribute type for specific elements/attributes
		validate_attr_type(&attr.name.to_string(), &typed_value, element_tag, attr.span)?;

		typed_attrs.push(TypedPageAttr {
			name: attr.name.clone(),
			value: typed_value,
			span: attr.span,
		});
	}

	Ok(typed_attrs)
}

/// Validates attribute type for specific elements and attributes.
///
/// # Phase 1 Implementation
///
/// Currently validates:
/// - `img` element `src` attribute must be a string literal and non-empty
///
/// Future phases will add more element-specific validations.
fn validate_attr_type(
	attr_name: &str,
	value: &AttrValue,
	element_tag: &str,
	span: Span,
) -> Result<()> {
	// Phase 1: img element src attribute validation
	if element_tag == "img" && attr_name == "src" {
		// Must be a string literal
		if !value.is_string_literal() {
			return Err(syn::Error::new(
				span,
				"Element <img> 'src' attribute must be a string literal",
			));
		}

		// Must not be empty
		if let Some(src_value) = value.as_string()
			&& src_value.trim().is_empty()
		{
			return Err(syn::Error::new(
				span,
				"Element <img> 'src' attribute must not be empty",
			));
		}
	}

	Ok(())
}

/// Validates event handlers.
///
/// # Rules
///
/// - If the handler is a closure, it must have 0 or 1 arguments
/// - Other expressions (variable references, Callback::new(), etc.) are allowed
///   and will be type-checked by the Rust compiler
///
/// # Errors
///
/// Returns a compilation error if the handler is a closure with more than 1 argument.
fn validate_event_handler(event: &PageEvent) -> Result<()> {
	// Only validate argument count for closure expressions
	// Other expressions (variables, method calls, etc.) are allowed
	// and will be type-checked by the Rust compiler
	if let Expr::Closure(closure) = &event.handler {
		let arg_count = closure.inputs.len();
		if arg_count > 1 {
			return Err(syn::Error::new_spanned(
				&event.handler,
				format!(
					"Event handler closure must have 0 or 1 arguments, but this closure has {} arguments",
					arg_count
				),
			));
		}
	}

	// All other expression types are allowed:
	// - Variable references: let handler = |_| {}; @click: handler
	// - Callback::new(): @click: Callback::new(|_| {})
	// - Method calls: @click: handler.clone()
	// Type checking for these will be done by the Rust compiler

	Ok(())
}

/// Validates attribute naming and values.
///
/// # Rules
///
/// - data-* attributes must match pattern: `data-[a-z][a-z0-9-]*`
/// - aria-* attributes must match pattern: `aria-[a-z-]+`
///
/// # Errors
///
/// Returns an error if naming conventions are violated.
fn validate_attribute(attr: &PageAttr, _element_tag: &str) -> Result<()> {
	let attr_name = attr.name.to_string();

	// Validate data-* attributes
	if attr_name.starts_with("data_") {
		let html_name = attr.html_name();
		// Check if all characters after "data-" are lowercase letters, digits, or hyphens
		let suffix = &html_name[5..]; // Skip "data-"
		if suffix.is_empty()
			|| !suffix
				.chars()
				.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
		{
			return Err(syn::Error::new_spanned(
				&attr.name,
				format!(
					"Invalid data attribute name '{}'. Must match pattern: data-[a-z][a-z0-9-]*",
					html_name
				),
			));
		}
		// Additionally check first character is lowercase letter
		if !suffix.chars().next().unwrap().is_ascii_lowercase() {
			return Err(syn::Error::new_spanned(
				&attr.name,
				format!(
					"Invalid data attribute name '{}'. Must start with a lowercase letter after 'data-'",
					html_name
				),
			));
		}
	}

	// Validate aria-* attributes
	if attr_name.starts_with("aria_") {
		let html_name = attr.html_name();
		// Check if all characters after "aria-" are lowercase letters or hyphens
		let suffix = &html_name[5..]; // Skip "aria-"
		if suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
			return Err(syn::Error::new_spanned(
				&attr.name,
				format!(
					"Invalid aria attribute name '{}'. Must match pattern: aria-[a-z-]+",
					html_name
				),
			));
		}
	}

	Ok(())
}

/// Validates element nesting rules.
///
/// # Rules
///
/// - Void elements (br, hr, img, input, etc.) cannot have children
/// - Interactive elements (button, a, label, etc.) cannot be nested inside other interactive elements
///
/// # Errors
///
/// Returns an error if nesting rules are violated.
fn validate_element_nesting(elem: &PageElement, parent_tags: &[String]) -> Result<()> {
	let tag = elem.tag.to_string();

	// Check if void element has children
	if elem.is_void() && !elem.children.is_empty() {
		return Err(syn::Error::new_spanned(
			&elem.tag,
			format!("Void element <{}> cannot have children", tag),
		));
	}

	// Check interactive element nesting
	let interactive_elements = ["button", "a", "label", "select", "textarea"];
	if interactive_elements.contains(&tag.as_str()) {
		for parent_tag in parent_tags {
			if interactive_elements.contains(&parent_tag.as_str()) {
				return Err(syn::Error::new_spanned(
					&elem.tag,
					format!(
						"Interactive element <{}> cannot be nested inside another interactive element <{}>",
						tag, parent_tag
					),
				));
			}
		}
	}

	Ok(())
}

/// Validates required attributes for certain elements.
///
/// # Rules
///
/// - img elements must have an alt attribute (accessibility requirement)
/// - img elements must have a src attribute with a non-empty string literal
///
/// # Errors
///
/// Returns a compilation error if required attributes are missing or invalid.
fn validate_required_attributes(tag: &str, attrs: &[TypedPageAttr], span: Span) -> Result<()> {
	// img requires alt and src attributes
	if tag == "img" {
		// Check alt attribute (accessibility)
		let has_alt = attrs.iter().any(|attr| attr.name == "alt");
		if !has_alt {
			return Err(syn::Error::new(
				span,
				"Element <img> requires 'alt' attribute for accessibility",
			));
		}

		// Check src attribute (required for img)
		let has_src = attrs.iter().any(|attr| attr.name == "src");
		if !has_src {
			return Err(syn::Error::new(
				span,
				"Element <img> requires 'src' attribute",
			));
		}

		// Note: src type validation (must be string literal, non-empty) is done in validate_attr_type()
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use syn::parse_quote;

	#[test]
	fn test_validate_valid_closure() {
		let event = PageEvent {
			event_type: syn::Ident::new("click", proc_macro2::Span::call_site()),
			handler: parse_quote!(|_| {}),
			span: proc_macro2::Span::call_site(),
		};
		assert!(validate_event_handler(&event).is_ok());
	}

	#[test]
	fn test_validate_closure_with_one_arg() {
		let event = PageEvent {
			event_type: syn::Ident::new("click", proc_macro2::Span::call_site()),
			handler: parse_quote!(|e| {
				handle_click(e);
			}),
			span: proc_macro2::Span::call_site(),
		};
		assert!(validate_event_handler(&event).is_ok());
	}

	#[test]
	fn test_validate_closure_too_many_args() {
		let event = PageEvent {
			event_type: syn::Ident::new("click", proc_macro2::Span::call_site()),
			handler: parse_quote!(|a, b, c| {}),
			span: proc_macro2::Span::call_site(),
		};
		let result = validate_event_handler(&event);
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("0 or 1 arguments"));
	}

	#[test]
	fn test_validate_valid_data_attribute() {
		let attr = PageAttr {
			name: syn::Ident::new("data_testid", proc_macro2::Span::call_site()),
			value: parse_quote!("test"),
			span: proc_macro2::Span::call_site(),
		};
		assert!(validate_attribute(&attr, "div").is_ok());
	}

	#[test]
	fn test_validate_invalid_data_attribute() {
		let attr = PageAttr {
			name: syn::Ident::new("data_", proc_macro2::Span::call_site()),
			value: parse_quote!("test"),
			span: proc_macro2::Span::call_site(),
		};
		let result = validate_attribute(&attr, "div");
		assert!(result.is_err());
	}

	#[test]
	fn test_validate_valid_aria_attribute() {
		let attr = PageAttr {
			name: syn::Ident::new("aria_label", proc_macro2::Span::call_site()),
			value: parse_quote!("Navigation"),
			span: proc_macro2::Span::call_site(),
		};
		assert!(validate_attribute(&attr, "div").is_ok());
	}

	#[test]
	fn test_void_element_with_children() {
		let mut elem = PageElement::new(
			syn::Ident::new("input", proc_macro2::Span::call_site()),
			proc_macro2::Span::call_site(),
		);
		elem.children
			.push(PageNode::Text(reinhardt_pages_ast::PageText {
				content: "text".to_string(),
				span: proc_macro2::Span::call_site(),
			}));

		let result = validate_element_nesting(&elem, &[]);
		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("cannot have children")
		);
	}

	#[test]
	fn test_nested_interactive_elements() {
		let elem = PageElement::new(
			syn::Ident::new("button", proc_macro2::Span::call_site()),
			proc_macro2::Span::call_site(),
		);

		let result = validate_element_nesting(&elem, &["a".to_string()]);
		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("cannot be nested inside another interactive element")
		);
	}

	#[test]
	fn test_img_missing_alt() {
		let elem = PageElement::new(
			syn::Ident::new("img", proc_macro2::Span::call_site()),
			proc_macro2::Span::call_site(),
		);

		let result = validate_required_attributes("img", &[], elem.span);
		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("requires 'alt' attribute")
		);
	}

	#[test]
	fn test_transform_attrs_with_string_lit() {
		let attrs = vec![PageAttr {
			name: syn::Ident::new("src", proc_macro2::Span::call_site()),
			value: parse_quote!("/image.png"),
			span: proc_macro2::Span::call_site(),
		}];

		let result = transform_attrs(&attrs, "img");
		assert!(result.is_ok());
		let typed_attrs = result.unwrap();
		assert_eq!(typed_attrs.len(), 1);
		assert!(typed_attrs[0].value.is_string_literal());
	}

	#[test]
	fn test_transform_attrs_with_dynamic() {
		let attrs = vec![PageAttr {
			name: syn::Ident::new("src", proc_macro2::Span::call_site()),
			value: parse_quote!(image_url),
			span: proc_macro2::Span::call_site(),
		}];

		let result = transform_attrs(&attrs, "div");
		assert!(result.is_ok());
		let typed_attrs = result.unwrap();
		assert_eq!(typed_attrs.len(), 1);
		assert!(typed_attrs[0].value.is_dynamic());
	}

	#[test]
	fn test_validate_attr_type_img_src_literal() {
		let value = AttrValue::from_expr(parse_quote!("/image.png"));
		let result = validate_attr_type("src", &value, "img", proc_macro2::Span::call_site());
		assert!(result.is_ok());
	}

	#[test]
	fn test_validate_attr_type_img_src_dynamic() {
		let value = AttrValue::from_expr(parse_quote!(image_url));
		let result = validate_attr_type("src", &value, "img", proc_macro2::Span::call_site());
		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("must be a string literal")
		);
	}

	#[test]
	fn test_validate_attr_type_img_src_empty() {
		let value = AttrValue::from_expr(parse_quote!(""));
		let result = validate_attr_type("src", &value, "img", proc_macro2::Span::call_site());
		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("must not be empty")
		);
	}
}
