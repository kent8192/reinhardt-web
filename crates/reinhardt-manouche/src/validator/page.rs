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
use std::collections::HashSet;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Expr, Result, Token};

use crate::core::{
	ImplicitPageCapture, PageAttr, PageBody, PageComponent, PageElement, PageElse, PageEvent,
	PageExpression, PageFor, PageIf, PageMacro, PageMacroForm, PageNode, PageParam, PageWatch,
	TypedNamedSlot, TypedPageAttr, TypedPageBody, TypedPageComponent, TypedPageElement,
	TypedPageElse, TypedPageFor, TypedPageIf, TypedPageMacro, TypedPageMacroForm, TypedPageNode,
	TypedPageWatch, types::AttrValue,
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
pub fn validate_page(ast: &PageMacro) -> Result<TypedPageMacro> {
	let form = match &ast.form {
		PageMacroForm::StrictClosure { params, body } => {
			enforce_strict_captures(body, params)?;
			TypedPageMacroForm::StrictClosure {
				params: params.clone(),
				body: transform_body(body, &[])?,
			}
		}
		PageMacroForm::ImplicitBody { body } => TypedPageMacroForm::ImplicitBody {
			captures: collect_free_idents(body, &[]),
			body: transform_body(body, &[])?,
		},
	};

	Ok(TypedPageMacro {
		head: ast.head.clone(),
		form,
		span: ast.span,
	})
}

fn collect_free_idents(body: &PageBody, params: &[PageParam]) -> Vec<ImplicitPageCapture> {
	let allowed: HashSet<String> = params.iter().map(|p| p.name.to_string()).collect();

	let mut collector = CaptureCollector {
		allowed,
		locals_stack: Vec::new(),
		seen: HashSet::new(),
		captures: Vec::new(),
	};
	collector.visit_body(body);
	collector.captures
}

fn enforce_strict_captures(body: &PageBody, params: &[PageParam]) -> Result<()> {
	if let Some(capture) = collect_free_idents(body, params).into_iter().next() {
		return Err(missing_param_error(&capture.ident));
	}
	Ok(())
}

struct CaptureCollector {
	allowed: HashSet<String>,
	locals_stack: Vec<HashSet<String>>,
	seen: HashSet<String>,
	captures: Vec<ImplicitPageCapture>,
}

impl CaptureCollector {
	fn is_known(&self, name: &str) -> bool {
		self.allowed.contains(name) || self.locals_stack.iter().any(|s| s.contains(name))
	}

	fn record_capture(&mut self, ident: &syn::Ident) {
		let name = ident.to_string();
		if self.seen.insert(name) {
			self.captures.push(ImplicitPageCapture {
				ident: ident.clone(),
				span: ident.span(),
			});
		}
	}

	fn visit_body(&mut self, body: &PageBody) {
		for node in &body.nodes {
			self.visit_node(node);
		}
	}

	fn visit_node(&mut self, node: &PageNode) {
		match node {
			PageNode::Element(el) => self.visit_element(el),
			PageNode::Text(_) => {}
			PageNode::Expression(expr) => self.visit_expression(expr),
			PageNode::If(if_node) => self.visit_if(if_node),
			PageNode::For(for_node) => self.visit_for(for_node),
			PageNode::Component(comp) => self.visit_component(comp),
			PageNode::Watch(watch) => self.visit_node(&watch.expr),
		}
	}

	fn visit_element(&mut self, el: &PageElement) {
		for attr in &el.attrs {
			self.visit_expr(&attr.value);
		}
		for event in &el.events {
			self.visit_expr(&event.handler);
		}
		for child in &el.children {
			self.visit_node(child);
		}
	}

	fn visit_expression(&mut self, expr: &PageExpression) {
		self.visit_expr(&expr.expr);
	}

	fn visit_if(&mut self, if_node: &PageIf) {
		self.visit_expr(&if_node.condition);
		for node in &if_node.then_branch {
			self.visit_node(node);
		}
		if let Some(else_branch) = &if_node.else_branch {
			match else_branch {
				PageElse::Block(nodes) => {
					for node in nodes {
						self.visit_node(node);
					}
				}
				PageElse::If(inner) => self.visit_if(inner),
			}
		}
	}

	fn visit_for(&mut self, for_node: &PageFor) {
		self.visit_expr(&for_node.iter);
		let mut locals = HashSet::new();
		collect_pat_idents(&for_node.pat, &mut locals);
		self.locals_stack.push(locals);
		if let Some(key) = &for_node.key {
			self.visit_expr(key);
		}
		for node in &for_node.body {
			self.visit_node(node);
		}
		self.locals_stack.pop();
	}

	fn visit_component(&mut self, comp: &PageComponent) {
		for arg in &comp.args {
			self.visit_expr(&arg.value);
		}
		for event in &comp.events {
			self.visit_expr(&event.handler);
		}
		if let Some(children) = &comp.children {
			for child in children {
				self.visit_node(child);
			}
		}
		for slot in &comp.named_slots {
			for child in &slot.children {
				self.visit_node(child);
			}
		}
	}

	fn visit_expr(&mut self, expr: &Expr) {
		let mut visitor = ExprIdentVisitor { collector: self };
		visitor.visit_expr(expr);
	}
}

struct ExprIdentVisitor<'a> {
	collector: &'a mut CaptureCollector,
}

impl<'ast> Visit<'ast> for ExprIdentVisitor<'_> {
	fn visit_expr_path(&mut self, expr_path: &'ast syn::ExprPath) {
		if expr_path.qself.is_none() && expr_path.path.segments.len() == 1 {
			let segment = &expr_path.path.segments[0];
			let name = segment.ident.to_string();
			if is_value_ident(&name) && !self.collector.is_known(&name) {
				self.collector.record_capture(&segment.ident);
			}
		}
		visit::visit_expr_path(self, expr_path);
	}

	fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
		let mut locals = HashSet::new();
		for input in &closure.inputs {
			collect_pat_idents(input, &mut locals);
		}
		self.collector.locals_stack.push(locals);
		visit::visit_expr_closure(self, closure);
		self.collector.locals_stack.pop();
	}

	fn visit_expr_let(&mut self, let_expr: &'ast syn::ExprLet) {
		let mut locals = HashSet::new();
		collect_pat_idents(&let_expr.pat, &mut locals);
		self.collector.locals_stack.push(locals);
		visit::visit_expr_let(self, let_expr);
		self.collector.locals_stack.pop();
	}

	fn visit_expr_if(&mut self, if_expr: &'ast syn::ExprIf) {
		if let syn::Expr::Let(let_expr) = &*if_expr.cond {
			let mut locals = HashSet::new();
			collect_pat_idents(&let_expr.pat, &mut locals);
			self.visit_expr(&let_expr.expr);
			self.collector.locals_stack.push(locals);
			self.visit_block(&if_expr.then_branch);
			self.collector.locals_stack.pop();
			if let Some((_, else_branch)) = &if_expr.else_branch {
				self.visit_expr(else_branch);
			}
		} else {
			visit::visit_expr_if(self, if_expr);
		}
	}

	fn visit_arm(&mut self, arm: &'ast syn::Arm) {
		let mut locals = HashSet::new();
		collect_pat_idents(&arm.pat, &mut locals);
		self.collector.locals_stack.push(locals);
		if let Some((_, guard)) = &arm.guard {
			self.visit_expr(guard);
		}
		self.visit_expr(&arm.body);
		self.collector.locals_stack.pop();
	}

	fn visit_expr_for_loop(&mut self, for_loop: &'ast syn::ExprForLoop) {
		self.visit_expr(&for_loop.expr);
		let mut locals = HashSet::new();
		collect_pat_idents(&for_loop.pat, &mut locals);
		self.collector.locals_stack.push(locals);
		self.visit_block(&for_loop.body);
		self.collector.locals_stack.pop();
	}

	fn visit_expr_macro(&mut self, expr_macro: &'ast syn::ExprMacro) {
		if let Ok(args) = expr_macro
			.mac
			.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)
		{
			for arg in args {
				self.visit_expr(&arg);
			}
		}
	}

	fn visit_block(&mut self, block: &'ast syn::Block) {
		let mut pushed = 0_usize;
		for stmt in &block.stmts {
			match stmt {
				syn::Stmt::Local(local) => {
					if let Some(init) = &local.init {
						self.visit_expr(&init.expr);
						if let Some((_, diverge)) = &init.diverge {
							self.visit_expr(diverge);
						}
					}
					let mut locals = HashSet::new();
					collect_pat_idents(&local.pat, &mut locals);
					self.collector.locals_stack.push(locals);
					pushed += 1;
				}
				syn::Stmt::Item(_) => {}
				syn::Stmt::Expr(expr, _) => self.visit_expr(expr),
				syn::Stmt::Macro(mac) => visit::visit_stmt_macro(self, mac),
			}
		}
		for _ in 0..pushed {
			self.collector.locals_stack.pop();
		}
	}
}

fn collect_pat_idents(pat: &syn::Pat, out: &mut HashSet<String>) {
	match pat {
		syn::Pat::Ident(ident) => {
			out.insert(ident.ident.to_string());
		}
		syn::Pat::Tuple(tuple) => {
			for elem in &tuple.elems {
				collect_pat_idents(elem, out);
			}
		}
		syn::Pat::TupleStruct(tuple_struct) => {
			for elem in &tuple_struct.elems {
				collect_pat_idents(elem, out);
			}
		}
		syn::Pat::Struct(struct_pat) => {
			for field in &struct_pat.fields {
				collect_pat_idents(&field.pat, out);
			}
		}
		syn::Pat::Reference(reference) => collect_pat_idents(&reference.pat, out),
		syn::Pat::Type(typed) => collect_pat_idents(&typed.pat, out),
		syn::Pat::Or(or_pat) => {
			for case in &or_pat.cases {
				collect_pat_idents(case, out);
			}
		}
		syn::Pat::Slice(slice) => {
			for elem in &slice.elems {
				collect_pat_idents(elem, out);
			}
		}
		syn::Pat::Paren(paren) => collect_pat_idents(&paren.pat, out),
		_ => {}
	}
}

fn is_value_ident(name: &str) -> bool {
	let starts_lowercase = name
		.chars()
		.next()
		.map(|c| c.is_ascii_lowercase())
		.unwrap_or(false);
	let all_screaming = name
		.chars()
		.all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit());
	starts_lowercase && !all_screaming
}

fn missing_param_error(ident: &syn::Ident) -> syn::Error {
	syn::Error::new(
		ident.span(),
		format!("identifier `{ident}` used inside `page!` is not declared as a parameter"),
	)
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
		PageNode::For(for_node) => Ok(TypedPageNode::For(Box::new(transform_for(
			for_node,
			parent_tags,
		)?))),
		PageNode::Component(comp) => Ok(TypedPageNode::Component(transform_component(
			comp,
			parent_tags,
		)?)),
		PageNode::Watch(watch_node) => Ok(TypedPageNode::Watch(transform_watch(
			watch_node,
			parent_tags,
		)?)),
	}
}

/// Transforms a PageIf node (if/else if/else).
///
/// Recursively validates all branches.
fn transform_if(if_node: &PageIf, parent_tags: &[String]) -> Result<TypedPageIf> {
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
fn transform_for(for_node: &PageFor, parent_tags: &[String]) -> Result<TypedPageFor> {
	let body = transform_nodes(&for_node.body, parent_tags)?;

	Ok(TypedPageFor {
		pat: for_node.pat.clone(),
		iter: for_node.iter.clone(),
		key: for_node.key.clone(),
		body,
		span: for_node.span,
	})
}

/// Transforms a PageWatch node.
fn transform_watch(watch_node: &PageWatch, parent_tags: &[String]) -> Result<TypedPageWatch> {
	let inner = transform_node(&watch_node.expr, parent_tags)?;

	Ok(TypedPageWatch {
		expr: Box::new(inner),
		span: watch_node.span,
	})
}

/// Transforms a PageComponent node.
///
/// Recursively transforms the component's children (if any) and named slots.
fn transform_component(comp: &PageComponent, parent_tags: &[String]) -> Result<TypedPageComponent> {
	// Validate component event handlers (same as element events)
	for event in &comp.events {
		validate_event_handler(event)?;
	}

	// Transform children if present
	let typed_children = if let Some(children) = &comp.children {
		Some(transform_nodes(children, parent_tags)?)
	} else {
		None
	};

	let typed_named_slots: Vec<TypedNamedSlot> = comp
		.named_slots
		.iter()
		.map(|slot| {
			Ok(TypedNamedSlot {
				name: slot.name.clone(),
				children: transform_nodes(&slot.children, parent_tags)?,
				span: slot.span,
			})
		})
		.collect::<Result<Vec<_>>>()?;

	Ok(TypedPageComponent {
		name: comp.name.clone(),
		invocation_form: comp.invocation_form,
		args: comp.args.clone(),
		events: comp.events.clone(),
		children: typed_children,
		named_slots: typed_named_slots,
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

	// 7. Validate accessibility requirements (Phase 5)
	validate_accessibility(
		&tag,
		&typed_element.attrs,
		&typed_element.children,
		elem.span,
	)?;

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

/// Checks if an attribute is a URL attribute for the given element.
fn is_url_attribute(attr_name: &str, element_tag: &str, url_attrs: &[(&str, &str)]) -> bool {
	for (url_attr, applicable_tags) in url_attrs {
		if attr_name == *url_attr {
			for tag in applicable_tags.split(',').map(|s| s.trim()) {
				if tag == element_tag {
					return true;
				}
			}
		}
	}
	false
}

/// Validates enumerated attribute values.
///
/// Checks if a string literal attribute value is one of the allowed values
/// for enumerated attributes (like `input[type]`, `button[type]`, etc.).
///
/// # Parameters
///
/// * `attr_name` - The attribute name
/// * `value` - The attribute value
/// * `element_tag` - The element tag name
/// * `span` - The span for error reporting
///
/// # Returns
///
/// Ok if validation passes, Err with descriptive error message otherwise.
fn validate_enum_attr(
	attr_name: &str,
	value: &AttrValue,
	element_tag: &str,
	span: Span,
) -> Result<()> {
	let Some(enum_spec) = super::html_spec::get_enum_attr_spec(element_tag, attr_name) else {
		return Ok(());
	};

	let Some(str_value) = value.as_string() else {
		return Ok(()); // Dynamic expressions are OK
	};

	if !enum_spec.valid_values.contains(&str_value.as_str()) {
		return Err(syn::Error::new(
			span,
			format!(
				"Invalid value '{}' for attribute '{}' on element <{}>.\n\
				Valid values are: {}\n\n\
				Examples:\n\
				  Correct:   {}=\"{}\"\n\
				  Incorrect: {}=\"{}\"",
				str_value,
				attr_name,
				element_tag,
				enum_spec.valid_values.join("\", \""),
				attr_name,
				enum_spec.valid_values[0],
				attr_name,
				str_value
			),
		));
	}

	Ok(())
}

/// Checks if children nodes contain meaningful content.
///
/// Returns true if any child contains non-whitespace text or is a dynamic expression.
fn has_meaningful_content(children: &[TypedPageNode]) -> bool {
	for child in children {
		match child {
			TypedPageNode::Text(text) => {
				if !text.content.trim().is_empty() {
					return true;
				}
			}
			TypedPageNode::Element(elem) => {
				if has_meaningful_content(&elem.children) {
					return true;
				}
			}
			TypedPageNode::Expression(_)
			| TypedPageNode::Component(_)
			| TypedPageNode::If(_)
			| TypedPageNode::For(_)
			| TypedPageNode::Watch(_) => {
				// Dynamic content - assume it will have meaningful content at runtime
				return true;
			}
		}
	}
	false
}

/// Validates button element accessibility requirements.
///
/// Button elements must have either:
/// - Text content (direct or nested)
/// - aria-label attribute
/// - aria-labelledby attribute
fn validate_button_accessibility(
	attrs: &[TypedPageAttr],
	children: &[TypedPageNode],
	span: Span,
) -> Result<()> {
	// Check for aria-label or aria-labelledby attributes
	let has_aria_label = attrs.iter().any(|attr| {
		let name = attr.name.to_string();
		name == "aria_label" || name == "aria_labelledby"
	});

	if has_aria_label {
		return Ok(());
	}

	// Check for text content
	if !has_meaningful_content(children) {
		return Err(syn::Error::new(
			span,
			"Element <button> requires accessible text.\n\
			Either provide text content or use 'aria_label' attribute.\n\n\
			Examples:\n\
			  Correct:   button { \"Click me\" }\n\
			  Correct:   button { aria_label: \"Close\" }\n\
			  Correct:   button { span { \"Submit\" } }\n\
			  Incorrect: button {}",
		));
	}

	Ok(())
}

/// Validates accessibility requirements for elements.
///
/// Currently validates:
/// - button elements: Must have text content or aria-label
fn validate_accessibility(
	tag: &str,
	attrs: &[TypedPageAttr],
	children: &[TypedPageNode],
	span: Span,
) -> Result<()> {
	if tag == "button" {
		validate_button_accessibility(attrs, children, span)?
	}
	Ok(())
}

/// Validates attribute type for specific elements and attributes.
///
/// # Validation Rules
///
/// - Boolean attributes must have dynamic expressions only (no literals)
/// - Numeric attributes must have integer literals or dynamic expressions (no strings/floats/booleans)
/// - URL attributes are checked for dangerous schemes (javascript:, data:, vbscript:) for XSS prevention
/// - Enumerated attributes are validated against allowed values (`input[type]`, `button[type]`, etc.)
/// - `img` element `src` attribute: when given as a string literal it must be non-empty;
///   dynamic expressions (e.g. `resolve_static(...)`) are allowed and deferred to runtime
///
/// Future phases will add accessibility checks.
fn validate_attr_type(
	attr_name: &str,
	value: &AttrValue,
	element_tag: &str,
	span: Span,
) -> Result<()> {
	// Boolean attributes validation - must use dynamic expressions only
	const BOOLEAN_ATTRS: &[&str] = &[
		"allowfullscreen",
		"async",
		"autofocus",
		"autoplay",
		"checked",
		"controls",
		"default",
		"defer",
		"disabled",
		"formnovalidate",
		"hidden",
		"inert",
		"ismap",
		"itemscope",
		"loop",
		"multiple",
		"muted",
		"nomodule",
		"novalidate",
		"open",
		"playsinline",
		"readonly",
		"required",
		"reversed",
		"selected",
		"truespeed",
	];

	// Numeric attributes that must have integer literal or dynamic values
	const NUMERIC_ATTRS: &[&str] = &[
		"maxlength",
		"minlength",
		"size",
		"min",
		"max",
		"step",
		"rows",
		"cols",
		"colspan",
		"rowspan",
		"tabindex",
	];

	// URL attributes that should be validated for dangerous schemes
	// Each entry is (attribute_name, applicable_element_tags)
	const URL_ATTRS: &[(&str, &str)] = &[
		("href", "a, area, link"),
		("action", "form"),
		("formaction", "button, input"),
		("src", "iframe, video, audio, source, script, embed, img"),
	];

	// Dangerous URL schemes that should be blocked for security (XSS prevention)
	const DANGEROUS_URL_SCHEMES: &[&str] = &["javascript:", "data:", "vbscript:"];

	if BOOLEAN_ATTRS.contains(&attr_name) {
		// 1. String literals are prohibited
		if value.is_string_literal() {
			return Err(syn::Error::new(
				span,
				format!(
					"Boolean attribute '{}' cannot have a string literal value.\n\
					HTML boolean attributes represent true/false by their presence/absence:\n\
					  - Attribute present = true\n\
					  - Attribute absent = false\n\n\
					Use a variable or expression for dynamic boolean values:\n\
					  Correct:   {}: is_disabled\n\
					  Correct:   {}: state.is_active()\n\
					  Incorrect: {}: \"true\"\n\
					  Incorrect: {}: \"disabled\"",
					attr_name, attr_name, attr_name, attr_name, attr_name
				),
			));
		}

		// 2. Boolean literal `false` is prohibited (omit the attribute instead).
		//    `true` is allowed to support standalone syntax (e.g., `required`
		//    which the parser desugars to `required: true`).
		if let AttrValue::BoolLit(lit) = value
			&& !lit.value()
		{
			return Err(syn::Error::new(
				span,
				format!(
					"Boolean attribute '{}' cannot be set to `false`.\n\
						To disable a boolean attribute, omit it entirely:\n\
						  - Attribute present = true (e.g., `{0}` or `{0}: true`)\n\
						  - Attribute absent = false (just remove `{0}`)\n\n\
						Use a variable or expression for dynamic boolean values:\n\
						  Correct:   {0}: is_disabled\n\
						  Correct:   {0}: state.is_active()",
					attr_name
				),
			));
		}

		// 3. Numeric literals are prohibited
		if value.is_int_literal() || value.is_float_literal() {
			return Err(syn::Error::new(
				span,
				format!(
					"Boolean attribute '{}' cannot have a numeric literal value.\n\
					HTML boolean attributes represent true/false by their presence/absence:\n\
					  - Attribute present = true\n\
					  - Attribute absent = false\n\n\
					Use a variable or expression for dynamic boolean values:\n\
					  Correct:   {}: is_disabled\n\
					  Correct:   {}: state.is_active()\n\
					  Incorrect: {}: 1\n\
					  Incorrect: {}: 0",
					attr_name, attr_name, attr_name, attr_name, attr_name
				),
			));
		}

		// 4. Dynamic expressions are OK (no check needed)
	}

	// Numeric attributes validation - must be integer literal or dynamic
	if NUMERIC_ATTRS.contains(&attr_name) {
		match value {
			AttrValue::StringLit(_) => {
				return Err(syn::Error::new(
					span,
					format!(
						"Attribute '{}' must be an integer literal or dynamic expression, not a string literal.\n\n\
						Examples:\n\
						  Incorrect: {}=\"100\"  // String literal is not allowed\n\
						  Correct:   {}=100      // Integer literal\n\
						  Correct:   {}=max_len  // Dynamic expression",
						attr_name, attr_name, attr_name, attr_name
					),
				));
			}
			AttrValue::FloatLit(_) => {
				return Err(syn::Error::new(
					span,
					format!(
						"Attribute '{}' must be an integer, not a floating-point number.\n\n\
						Examples:\n\
						  Incorrect: {}=100.0   // Float literal is not allowed\n\
						  Correct:   {}=100     // Integer literal",
						attr_name, attr_name, attr_name
					),
				));
			}
			AttrValue::BoolLit(_) => {
				return Err(syn::Error::new(
					span,
					format!(
						"Attribute '{}' must be an integer, not a boolean.\n\n\
						Use an integer literal or dynamic expression instead.",
						attr_name
					),
				));
			}
			AttrValue::IntLit(_) | AttrValue::Dynamic(_) => {
				// OK: Integer literal or dynamic expression
			}
		}
	}

	// URL attributes safety check - prevent XSS attacks
	if is_url_attribute(attr_name, element_tag, URL_ATTRS)
		&& let Some(url_str) = value.as_string()
	{
		// Check for empty strings first
		if url_str.trim().is_empty() {
			return Err(syn::Error::new(
				span,
				format!(
					"URL attribute '{}' cannot be empty.\n\n\
						Provide a valid URL or use a dynamic expression.",
					attr_name
				),
			));
		}

		// Check for dangerous schemes (case-insensitive)
		let url_lower = url_str.to_lowercase();
		for scheme in DANGEROUS_URL_SCHEMES {
			if url_lower.starts_with(scheme) {
				return Err(syn::Error::new(
					span,
					format!(
						"Dangerous URL scheme detected in attribute '{}'.\n\
							The '{}' scheme can be used for XSS (Cross-Site Scripting) attacks.\n\n\
							Security risk: This URL could execute arbitrary JavaScript code.\n\n\
							Use safe URL schemes instead:\n\
							  - https://example.com\n\
							  - /path/to/resource\n\
							  - #anchor\n\
							  - mailto:user@example.com",
						attr_name,
						scheme.trim_end_matches(':')
					),
				));
			}
		}
	}
	// Dynamic expressions are OK (runtime validation recommended)

	// Enumerated attributes validation - check if value is in allowed list
	validate_enum_attr(attr_name, value, element_tag, span)?;

	// Note: emptiness of a string-literal `<img src="">` is rejected by the
	// generic URL-attribute check above (URL_ATTRS includes ("src", img)).
	// Dynamic expressions (e.g. `resolve_static(...)`) are deferred to runtime.

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

/// Validates that a `data-*` attribute suffix is non-empty, starts with a
/// lowercase letter, and contains only lowercase letters, digits, or hyphens.
///
/// Returns an error referencing `attr_name_token` with `html_name` in the message.
fn validate_data_attr_suffix(
	attr_name_token: &syn::Ident,
	html_name: &str,
	suffix: &str,
) -> Result<()> {
	if suffix.is_empty()
		|| !suffix
			.chars()
			.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
	{
		return Err(syn::Error::new_spanned(
			attr_name_token,
			format!(
				"Invalid data attribute name '{}'. Must match pattern: data-[a-z][a-z0-9-]*",
				html_name
			),
		));
	}
	// First character must be a lowercase letter
	if !suffix.chars().next().unwrap().is_ascii_lowercase() {
		return Err(syn::Error::new_spanned(
			attr_name_token,
			format!(
				"Invalid data attribute name '{}'. Must start with a lowercase letter after 'data-'",
				html_name
			),
		));
	}
	Ok(())
}

/// Validates that an `aria-*` attribute suffix is non-empty and contains only
/// lowercase letters or hyphens.
///
/// Returns an error referencing `attr_name_token` with `html_name` in the message.
fn validate_aria_attr_suffix(
	attr_name_token: &syn::Ident,
	html_name: &str,
	suffix: &str,
) -> Result<()> {
	if suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
		return Err(syn::Error::new_spanned(
			attr_name_token,
			format!(
				"Invalid aria attribute name '{}'. Must match pattern: aria-[a-z-]+",
				html_name
			),
		));
	}
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
		let suffix = &html_name[5..]; // Skip "data-"
		validate_data_attr_suffix(&attr.name, &html_name, suffix)?;
	}

	// Validate aria-* attributes
	if attr_name.starts_with("aria_") {
		let html_name = attr.html_name();
		let suffix = &html_name[5..]; // Skip "aria-"
		validate_aria_attr_suffix(&attr.name, &html_name, suffix)?;
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
	use crate::core::{PageExpression, PageText, TypedPageElement};
	use quote::quote;
	use rstest::rstest;
	use syn::parse_quote;

	fn parse(input: proc_macro2::TokenStream) -> PageMacro {
		syn::parse2(input).expect("input must be a valid page! macro")
	}

	fn implicit_capture_names(ast: &PageMacro) -> Vec<String> {
		validate_page(ast)
			.unwrap()
			.implicit_captures()
			.iter()
			.map(|capture| capture.ident.to_string())
			.collect()
	}

	#[rstest]
	fn test_implicit_body_records_captures() {
		// Arrange
		let ast = parse(quote! {
			{ div { {outer_count.get()} } }
		});

		// Act
		let captures = implicit_capture_names(&ast);

		// Assert
		assert_eq!(captures, vec!["outer_count"]);
	}

	#[rstest]
	fn test_implicit_body_records_macro_argument_captures() {
		// Arrange
		let ast = parse(quote! {
			{ p { {format!("value={}", outer_value)} } }
		});

		// Act
		let captures = implicit_capture_names(&ast);

		// Assert
		assert_eq!(captures, vec!["outer_value"]);
	}

	#[rstest]
	fn test_implicit_body_records_page_for_iter_and_key_captures() {
		// Arrange
		let ast = parse(quote! {
			{
				ul {
					for item in items @key(route_id.to_string()) {
						li { {item.clone()} }
					}
				}
			}
		});

		// Act
		let captures = implicit_capture_names(&ast);

		// Assert
		assert_eq!(captures, vec!["items", "route_id"]);
		assert!(!captures.iter().any(|capture| capture == "item"));
	}

	#[rstest]
	fn test_implicit_body_treats_page_for_key_pattern_as_local() {
		// Arrange
		let ast = parse(quote! {
			{
				ul {
					for item in items @key(item.clone()) {
						li { {item.clone()} }
					}
				}
			}
		});

		// Act
		let captures = implicit_capture_names(&ast);

		// Assert
		assert_eq!(captures, vec!["items"]);
	}

	#[rstest]
	fn test_strict_closure_rejects_implicit_capture() {
		// Arrange
		let ast = parse(quote! {
			|| { div { {outer_count.get()} } }
		});

		// Act
		let result = validate_page(&ast);

		// Assert
		let err = result.expect_err("strict closure must reject implicit capture");
		assert!(
			err.to_string().contains("outer_count"),
			"diagnostic should name the missing parameter: {err}"
		);
	}

	#[rstest]
	fn test_validate_valid_closure() {
		// Arrange
		let event = PageEvent {
			event_type: syn::Ident::new("click", proc_macro2::Span::call_site()),
			handler: parse_quote!(|_| {}),
			span: proc_macro2::Span::call_site(),
		};

		// Act
		let result = validate_event_handler(&event);

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_closure_with_one_arg() {
		// Arrange
		let event = PageEvent {
			event_type: syn::Ident::new("click", proc_macro2::Span::call_site()),
			handler: parse_quote!(|e| {
				handle_click(e);
			}),
			span: proc_macro2::Span::call_site(),
		};

		// Act
		let result = validate_event_handler(&event);

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_closure_too_many_args() {
		// Arrange
		let event = PageEvent {
			event_type: syn::Ident::new("click", proc_macro2::Span::call_site()),
			handler: parse_quote!(|a, b, c| {}),
			span: proc_macro2::Span::call_site(),
		};

		// Act
		let result = validate_event_handler(&event);

		// Assert
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("0 or 1 arguments"));
	}

	#[rstest]
	fn test_validate_valid_data_attribute() {
		// Arrange
		let attr = PageAttr {
			name: syn::Ident::new("data_testid", proc_macro2::Span::call_site()),
			value: parse_quote!("test"),
			span: proc_macro2::Span::call_site(),
		};

		// Act
		let result = validate_attribute(&attr, "div");

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_invalid_data_attribute() {
		// Arrange
		let attr = PageAttr {
			name: syn::Ident::new("data_", proc_macro2::Span::call_site()),
			value: parse_quote!("test"),
			span: proc_macro2::Span::call_site(),
		};

		// Act
		let result = validate_attribute(&attr, "div");

		// Assert
		assert!(result.is_err());
	}

	#[rstest]
	fn test_validate_valid_aria_attribute() {
		// Arrange
		let attr = PageAttr {
			name: syn::Ident::new("aria_label", proc_macro2::Span::call_site()),
			value: parse_quote!("Navigation"),
			span: proc_macro2::Span::call_site(),
		};

		// Act
		let result = validate_attribute(&attr, "div");

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_void_element_with_children() {
		// Arrange
		let mut elem = PageElement::new(
			syn::Ident::new("input", proc_macro2::Span::call_site()),
			proc_macro2::Span::call_site(),
		);
		elem.children.push(PageNode::Text(PageText {
			content: "text".to_string(),
			span: proc_macro2::Span::call_site(),
		}));

		// Act
		let result = validate_element_nesting(&elem, &[]);

		// Assert
		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("cannot have children")
		);
	}

	#[rstest]
	fn test_nested_interactive_elements() {
		// Arrange
		let elem = PageElement::new(
			syn::Ident::new("button", proc_macro2::Span::call_site()),
			proc_macro2::Span::call_site(),
		);

		// Act
		let result = validate_element_nesting(&elem, &["a".to_string()]);

		// Assert
		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("cannot be nested inside another interactive element")
		);
	}

	#[rstest]
	fn test_img_missing_alt() {
		// Arrange
		let elem = PageElement::new(
			syn::Ident::new("img", proc_macro2::Span::call_site()),
			proc_macro2::Span::call_site(),
		);

		// Act
		let result = validate_required_attributes("img", &[], elem.span);

		// Assert
		assert!(result.is_err());
		assert!(
			result
				.unwrap_err()
				.to_string()
				.contains("requires 'alt' attribute")
		);
	}

	#[rstest]
	fn test_transform_attrs_with_string_lit() {
		// Arrange
		let attrs = vec![PageAttr {
			name: syn::Ident::new("src", proc_macro2::Span::call_site()),
			value: parse_quote!("/image.png"),
			span: proc_macro2::Span::call_site(),
		}];

		// Act
		let result = transform_attrs(&attrs, "img");

		// Assert
		assert!(result.is_ok());
		let typed_attrs = result.unwrap();
		assert_eq!(typed_attrs.len(), 1);
		assert!(typed_attrs[0].value.is_string_literal());
	}

	#[rstest]
	fn test_transform_attrs_with_dynamic() {
		// Arrange
		let attrs = vec![PageAttr {
			name: syn::Ident::new("src", proc_macro2::Span::call_site()),
			value: parse_quote!(image_url),
			span: proc_macro2::Span::call_site(),
		}];

		// Act
		let result = transform_attrs(&attrs, "div");

		// Assert
		assert!(result.is_ok());
		let typed_attrs = result.unwrap();
		assert_eq!(typed_attrs.len(), 1);
		assert!(typed_attrs[0].value.is_dynamic());
	}

	#[rstest]
	fn test_validate_attr_type_img_src_literal() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("/image.png"));

		// Act
		let result = validate_attr_type("src", &value, "img", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_attr_type_img_src_dynamic() {
		// Arrange: dynamic expressions (function calls, identifiers) are accepted
		// because their value can only be validated at runtime.
		let value = AttrValue::from_expr(parse_quote!(resolve_static("images/poll.svg")));

		// Act
		let result = validate_attr_type("src", &value, "img", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_attr_type_img_src_empty() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(""));

		// Act
		let result = validate_attr_type("src", &value, "img", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("cannot be empty"));
	}

	// Boolean attribute tests - string literals are prohibited
	#[rstest]
	fn test_validate_boolean_attr_string_literal() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("disabled"));

		// Act
		let result =
			validate_attr_type("disabled", &value, "button", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Boolean attribute"));
		assert!(err_msg.contains("cannot have a string literal value"));
	}

	#[rstest]
	fn test_validate_boolean_attr_string_empty() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(""));

		// Act
		let result =
			validate_attr_type("disabled", &value, "button", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Boolean attribute"));
		assert!(err_msg.contains("cannot have a string literal value"));
	}

	#[rstest]
	fn test_validate_boolean_attr_bool_literal_true() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(true));

		// Act
		let result =
			validate_attr_type("disabled", &value, "button", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_boolean_attr_bool_literal_false() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(false));

		// Act
		let result = validate_attr_type("checked", &value, "input", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("cannot be set to `false`"));
	}

	#[rstest]
	fn test_validate_boolean_attr_int_literal() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(1));

		// Act
		let result =
			validate_attr_type("disabled", &value, "button", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Boolean attribute"));
		assert!(err_msg.contains("cannot have a numeric literal value"));
	}

	#[rstest]
	fn test_validate_boolean_attr_float_literal() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(1.0));

		// Act
		let result =
			validate_attr_type("required", &value, "input", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Boolean attribute"));
		assert!(err_msg.contains("cannot have a numeric literal value"));
	}

	#[rstest]
	fn test_validate_boolean_attr_dynamic_variable() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(is_disabled));

		// Act
		let result =
			validate_attr_type("disabled", &value, "button", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_boolean_attr_dynamic_function() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(is_disabled()));

		// Act
		let result = validate_attr_type("checked", &value, "input", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_boolean_attr_dynamic_conditional() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(if condition { true } else { false }));

		// Act
		let result =
			validate_attr_type("readonly", &value, "input", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	// Numeric attribute tests - string and float literals are prohibited
	#[rstest]
	fn test_validate_numeric_attr_string_literal() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("100"));

		// Act
		let result =
			validate_attr_type("maxlength", &value, "input", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Attribute"));
		assert!(err_msg.contains("must be an integer literal or dynamic expression"));
	}

	#[rstest]
	fn test_validate_numeric_attr_float_literal() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(100.0));

		// Act
		let result = validate_attr_type("rows", &value, "textarea", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Attribute"));
		assert!(err_msg.contains("must be an integer, not a floating-point number"));
	}

	#[rstest]
	fn test_validate_numeric_attr_bool_literal() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(true));

		// Act
		let result = validate_attr_type("min", &value, "input", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Attribute"));
		assert!(err_msg.contains("must be an integer, not a boolean"));
	}

	#[rstest]
	fn test_validate_numeric_attr_int_literal() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(100));

		// Act
		let result =
			validate_attr_type("maxlength", &value, "input", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_numeric_attr_dynamic_variable() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(max_len));

		// Act
		let result =
			validate_attr_type("maxlength", &value, "input", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_numeric_attr_dynamic_function() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(get_max_len()));

		// Act
		let result = validate_attr_type("cols", &value, "textarea", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	// URL attribute tests - dangerous schemes and empty strings are prohibited
	#[rstest]
	fn test_validate_url_attr_javascript_scheme() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("javascript:alert('xss')"));

		// Act
		let result = validate_attr_type("href", &value, "a", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Dangerous URL scheme"));
		assert!(err_msg.contains("javascript"));
		assert!(err_msg.contains("XSS"));
	}

	#[rstest]
	fn test_validate_url_attr_data_scheme() {
		// Arrange
		let value =
			AttrValue::from_expr(parse_quote!("data:text/html,<script>alert('xss')</script>"));

		// Act
		let result = validate_attr_type("href", &value, "a", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Dangerous URL scheme"));
		assert!(err_msg.contains("data"));
	}

	#[rstest]
	fn test_validate_url_attr_vbscript_scheme() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("vbscript:msgbox('xss')"));

		// Act
		let result = validate_attr_type("href", &value, "a", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Dangerous URL scheme"));
		assert!(err_msg.contains("vbscript"));
	}

	#[rstest]
	fn test_validate_url_attr_case_insensitive() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("JavaScript:alert(1)"));

		// Act
		let result = validate_attr_type("href", &value, "a", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Dangerous URL scheme"));
		assert!(err_msg.contains("javascript"));
	}

	#[rstest]
	fn test_validate_url_attr_empty_string() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(""));

		// Act
		let result = validate_attr_type("action", &value, "form", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("cannot be empty"));
	}

	#[rstest]
	fn test_validate_url_attr_whitespace_string() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("   "));

		// Act
		let result = validate_attr_type("href", &value, "a", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("cannot be empty"));
	}

	#[rstest]
	fn test_validate_url_attr_https_scheme() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("https://example.com"));

		// Act
		let result = validate_attr_type("href", &value, "a", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_url_attr_relative_path() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("/path/to/page"));

		// Act
		let result = validate_attr_type("action", &value, "form", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_url_attr_anchor() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("#section"));

		// Act
		let result = validate_attr_type("href", &value, "a", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_url_attr_dynamic_variable() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(url_var));

		// Act
		let result = validate_attr_type("href", &value, "a", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_url_attr_img_src_javascript_blocked() {
		// Arrange
		// img src should block dangerous URL schemes like javascript:
		let value = AttrValue::from_expr(parse_quote!("javascript:alert(1)"));

		// Act
		let result = validate_attr_type("src", &value, "img", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(
			err_msg.contains("Dangerous URL scheme"),
			"Expected dangerous URL scheme error, got: {}",
			err_msg
		);
	}

	#[rstest]
	fn test_validate_url_attr_img_src_data_scheme_blocked() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("data:text/html,<script>alert(1)</script>"));

		// Act
		let result = validate_attr_type("src", &value, "img", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(
			err_msg.contains("Dangerous URL scheme"),
			"Expected dangerous URL scheme error, got: {}",
			err_msg
		);
	}

	#[rstest]
	fn test_validate_url_attr_img_src_vbscript_blocked() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("vbscript:MsgBox('XSS')"));

		// Act
		let result = validate_attr_type("src", &value, "img", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(
			err_msg.contains("Dangerous URL scheme"),
			"Expected dangerous URL scheme error, got: {}",
			err_msg
		);
	}

	#[rstest]
	fn test_validate_url_attr_img_src_safe_url_passes() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("https://example.com/image.png"));

		// Act
		let result = validate_attr_type("src", &value, "img", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_url_attr_img_src_relative_path_passes() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("/images/photo.jpg"));

		// Act
		let result = validate_attr_type("src", &value, "img", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	// Enumerated attribute tests - invalid values are prohibited
	#[rstest]
	fn test_validate_enum_attr_input_type_invalid() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("invalid"));

		// Act
		let result = validate_attr_type("type", &value, "input", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Invalid value"));
		assert!(err_msg.contains("invalid"));
		assert!(err_msg.contains("input"));
	}

	#[rstest]
	fn test_validate_enum_attr_input_type_text() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("text"));

		// Act
		let result = validate_attr_type("type", &value, "input", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_enum_attr_input_type_email() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("email"));

		// Act
		let result = validate_attr_type("type", &value, "input", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_enum_attr_button_type_invalid() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("invalid"));

		// Act
		let result = validate_attr_type("type", &value, "button", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Invalid value"));
		assert!(err_msg.contains("button"));
	}

	#[rstest]
	fn test_validate_enum_attr_button_type_submit() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("submit"));

		// Act
		let result = validate_attr_type("type", &value, "button", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_enum_attr_form_method_invalid() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("put"));

		// Act
		let result = validate_attr_type("method", &value, "form", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Invalid value"));
		assert!(err_msg.contains("put"));
	}

	#[rstest]
	fn test_validate_enum_attr_form_method_post() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("post"));

		// Act
		let result = validate_attr_type("method", &value, "form", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_enum_attr_form_enctype_invalid() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("invalid"));

		// Act
		let result = validate_attr_type("enctype", &value, "form", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("Invalid value"));
	}

	#[rstest]
	fn test_validate_enum_attr_form_enctype_multipart() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("multipart/form-data"));

		// Act
		let result = validate_attr_type("enctype", &value, "form", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_enum_attr_script_type_module() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!("module"));

		// Act
		let result = validate_attr_type("type", &value, "script", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_enum_attr_dynamic_variable() {
		// Arrange
		let value = AttrValue::from_expr(parse_quote!(input_type));

		// Act
		let result = validate_attr_type("type", &value, "input", proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	// Accessibility tests - button elements must have text or aria-label
	#[rstest]
	fn test_validate_button_accessibility_empty() {
		// Arrange
		let attrs = vec![];
		let children = vec![];

		// Act
		let result =
			validate_button_accessibility(&attrs, &children, proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("requires accessible text"));
		assert!(err_msg.contains("aria_label"));
	}

	#[rstest]
	fn test_validate_button_accessibility_whitespace_only() {
		// Arrange
		let attrs = vec![];
		let children = vec![TypedPageNode::Text(PageText {
			content: "   ".to_string(),
			span: proc_macro2::Span::call_site(),
		})];

		// Act
		let result =
			validate_button_accessibility(&attrs, &children, proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_err());
	}

	#[rstest]
	fn test_validate_button_accessibility_with_text() {
		// Arrange
		let attrs = vec![];
		let children = vec![TypedPageNode::Text(PageText {
			content: "Click me".to_string(),
			span: proc_macro2::Span::call_site(),
		})];

		// Act
		let result =
			validate_button_accessibility(&attrs, &children, proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_button_accessibility_with_nested_text() {
		// Arrange
		let attrs = vec![];
		let children = vec![TypedPageNode::Element(TypedPageElement {
			tag: syn::Ident::new("span", proc_macro2::Span::call_site()),
			attrs: vec![],
			events: vec![],
			children: vec![TypedPageNode::Text(PageText {
				content: "Submit".to_string(),
				span: proc_macro2::Span::call_site(),
			})],
			span: proc_macro2::Span::call_site(),
		})];

		// Act
		let result =
			validate_button_accessibility(&attrs, &children, proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_button_accessibility_with_aria_label() {
		// Arrange
		let attrs = vec![TypedPageAttr {
			name: syn::Ident::new("aria_label", proc_macro2::Span::call_site()),
			value: AttrValue::from_expr(parse_quote!("Close")),
			span: proc_macro2::Span::call_site(),
		}];
		let children = vec![];

		// Act
		let result =
			validate_button_accessibility(&attrs, &children, proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_validate_button_accessibility_with_expression() {
		// Arrange
		let attrs = vec![];
		let children = vec![TypedPageNode::Expression(PageExpression {
			expr: parse_quote!(button_text),
			braced: true,
			span: proc_macro2::Span::call_site(),
		})];

		// Act
		let result =
			validate_button_accessibility(&attrs, &children, proc_macro2::Span::call_site());

		// Assert
		assert!(result.is_ok());
	}
}
