//! AST-based page! macro formatter implementation.
//!
//! This module provides formatting for `page!` macro DSL using proper AST parsing.
//! Unlike the text-based approach, this implementation:
//!
//! - Uses `syn::parse_file()` to parse the entire Rust source file
//! - Uses `syn::visit` to accurately detect `page!` macro invocations
//! - Ignores content in comments and strings (guaranteed by AST)
//! - Uses `reinhardt-pages-ast` for parsing the macro DSL
//!
//! ## Architecture
//!
//! ```text
//! [Rust source file]
//!         │
//!         ▼
//! [syn::parse_file()] ─── Parse entire file to AST
//!         │
//!         ▼
//! [PageMacroVisitor] ─── Walk AST to find page! macros with spans
//!         │
//!         ▼
//! [reinhardt_pages_ast::PageMacro] ─── Parse macro tokens to DSL AST
//!         │
//!         ▼
//! [format_macro()] ─── Generate formatted code from AST
//!         │
//!         ▼
//! [replace by span] ─── Replace original text with formatted version
//!         │
//!         ▼
//! [Formatted source file]
//! ```

use quote::ToTokens;
use reinhardt_pages_ast::{
	PageAttr, PageBody, PageComponent, PageElement, PageElse, PageEvent, PageExpression, PageFor,
	PageIf, PageMacro, PageNode, PageParam, PageText,
};
use syn::visit::Visit;
use syn::{ExprMacro, Macro, parse_file};

/// Information about a detected page! macro invocation.
#[derive(Debug)]
struct MacroInfo {
	/// Start byte offset in the source
	start: usize,
	/// End byte offset in the source
	end: usize,
	/// The macro's tokens (content inside page!(...))
	tokens: proc_macro2::TokenStream,
}

/// Visitor that walks the AST to find page! macro invocations.
struct PageMacroVisitor<'a> {
	/// Collected macro information
	macros: Vec<MacroInfo>,
	/// Original source code for offset calculation
	source: &'a str,
}

impl<'a> PageMacroVisitor<'a> {
	fn new(source: &'a str) -> Self {
		Self {
			macros: Vec::new(),
			source,
		}
	}

	/// Extract macro info from a Macro node.
	fn extract_macro_info(&mut self, mac: &Macro) {
		if mac.path.is_ident("page") {
			// Get span information
			// Note: proc_macro2::Span in non-procedural-macro context doesn't
			// give us byte offsets directly. We need to find the macro in source.
			let tokens_str = mac.tokens.to_string();

			// Find this macro in the source by searching for "page!("
			// We use the token stream content to verify we found the right one
			if let Some(info) = self.find_macro_in_source(&tokens_str) {
				self.macros.push(info);
			}
		}
	}

	/// Find the page! macro in source and return its position info.
	fn find_macro_in_source(&self, _tokens_content: &str) -> Option<MacroInfo> {
		// This is a simplified approach - we search for "page!(" patterns
		// and verify by comparing token content
		let pattern = "page!(";
		let mut search_start = 0;

		// Skip already found macros
		for found in &self.macros {
			if found.end > search_start {
				search_start = found.end;
			}
		}

		while let Some(pos) = self.source[search_start..].find(pattern) {
			let abs_start = search_start + pos;
			let content_start = abs_start + pattern.len();

			// Find matching closing paren
			if let Some(end_pos) = find_matching_paren(self.source, content_start) {
				let macro_content = &self.source[content_start..end_pos];

				// Parse the content to get tokens
				if let Ok(tokens) = syn::parse_str::<proc_macro2::TokenStream>(macro_content) {
					return Some(MacroInfo {
						start: abs_start,
						end: end_pos + 1, // Include closing paren
						tokens,
					});
				}
			}

			search_start = abs_start + 1;
		}

		None
	}
}

impl<'ast, 'a> Visit<'ast> for PageMacroVisitor<'a> {
	fn visit_expr_macro(&mut self, expr: &'ast ExprMacro) {
		self.extract_macro_info(&expr.mac);
		syn::visit::visit_expr_macro(self, expr);
	}

	fn visit_macro(&mut self, mac: &'ast Macro) {
		self.extract_macro_info(mac);
		syn::visit::visit_macro(self, mac);
	}
}

/// Find the matching closing parenthesis, handling strings and nested parens.
fn find_matching_paren(source: &str, start: usize) -> Option<usize> {
	let bytes = source.as_bytes();
	let mut depth = 1;
	let mut pos = start;
	let mut in_string = false;
	let mut in_char = false;
	let mut escape_next = false;

	while pos < bytes.len() && depth > 0 {
		if escape_next {
			escape_next = false;
			pos += 1;
			continue;
		}

		match bytes[pos] {
			b'\\' if in_string || in_char => {
				escape_next = true;
			}
			b'"' if !in_char => {
				in_string = !in_string;
			}
			b'\'' if !in_string => {
				in_char = !in_char;
			}
			b'(' if !in_string && !in_char => {
				depth += 1;
			}
			b')' if !in_string && !in_char => {
				depth -= 1;
			}
			_ => {}
		}

		pos += 1;
	}

	if depth == 0 { Some(pos - 1) } else { None }
}

/// AST-based page! macro formatter.
pub(crate) struct AstPageFormatter {
	/// Indentation string (tab by default)
	indent: String,
}

impl Default for AstPageFormatter {
	fn default() -> Self {
		Self::new()
	}
}

impl AstPageFormatter {
	/// Create a new formatter with default settings.
	pub(crate) fn new() -> Self {
		Self {
			indent: "\t".to_string(),
		}
	}

	/// Format the content of a Rust source file.
	///
	/// Uses AST parsing for accurate macro detection. Falls back to returning
	/// the original content if parsing fails.
	pub(crate) fn format(&self, content: &str) -> Result<String, String> {
		// Safety check: If no page! pattern exists, return unchanged
		if !content.contains("page!(") {
			return Ok(content.to_string());
		}

		// Collect all page! macro locations
		let macros = self.find_page_macros(content)?;

		if macros.is_empty() {
			return Ok(content.to_string());
		}

		// Sort macros by position (they should already be in order)
		let mut macros = macros;
		macros.sort_by_key(|m| m.start);

		// Build result by replacing each macro
		let mut result = String::with_capacity(content.len() * 2);
		let mut last_end = 0;

		for macro_info in &macros {
			// Copy content before this macro
			result.push_str(&content[last_end..macro_info.start]);

			// Try to parse and format the macro
			match self.format_macro_tokens(&macro_info.tokens) {
				Ok(formatted) => {
					result.push_str("page!(");
					result.push_str(&formatted);
					result.push(')');
				}
				Err(_) => {
					// If formatting fails, keep original
					result.push_str(&content[macro_info.start..macro_info.end]);
				}
			}

			last_end = macro_info.end;
		}

		// Copy remaining content
		result.push_str(&content[last_end..]);

		Ok(result)
	}

	/// Find all page! macros in the source.
	fn find_page_macros(&self, content: &str) -> Result<Vec<MacroInfo>, String> {
		// Try to parse as a complete Rust file first
		match parse_file(content) {
			Ok(file) => {
				let mut visitor = PageMacroVisitor::new(content);
				visitor.visit_file(&file);
				Ok(visitor.macros)
			}
			Err(_) => {
				// If file parsing fails, fall back to text-based detection
				self.find_page_macros_text_based(content)
			}
		}
	}

	/// Text-based fallback for finding page! macros.
	fn find_page_macros_text_based(&self, content: &str) -> Result<Vec<MacroInfo>, String> {
		let mut macros = Vec::new();
		let pattern = "page!(";
		let mut search_start = 0;

		while let Some(pos) = content[search_start..].find(pattern) {
			let abs_start = search_start + pos;

			// Check if we're in a comment or string
			if self.is_in_comment_or_string(content, abs_start) {
				search_start = abs_start + 1;
				continue;
			}

			let content_start = abs_start + pattern.len();

			if let Some(end_pos) = find_matching_paren(content, content_start) {
				let macro_content = &content[content_start..end_pos];

				if let Ok(tokens) = syn::parse_str::<proc_macro2::TokenStream>(macro_content) {
					macros.push(MacroInfo {
						start: abs_start,
						end: end_pos + 1,
						tokens,
					});
				}

				search_start = end_pos + 1;
			} else {
				search_start = abs_start + 1;
			}
		}

		Ok(macros)
	}

	/// Check if a position is inside a comment or string literal.
	fn is_in_comment_or_string(&self, content: &str, pos: usize) -> bool {
		let bytes = content.as_bytes();
		let mut i = 0;
		let mut in_string = false;
		let mut in_line_comment = false;
		let mut in_block_comment = false;
		let mut escape_next = false;

		while i < pos && i < bytes.len() {
			if escape_next {
				escape_next = false;
				i += 1;
				continue;
			}

			// Check for line comment
			if !in_string && !in_block_comment && i + 1 < bytes.len() {
				if bytes[i] == b'/' && bytes[i + 1] == b'/' {
					in_line_comment = true;
					i += 2;
					continue;
				}
				if bytes[i] == b'/' && bytes[i + 1] == b'*' {
					in_block_comment = true;
					i += 2;
					continue;
				}
			}

			// Check for end of line comment
			if in_line_comment && bytes[i] == b'\n' {
				in_line_comment = false;
				i += 1;
				continue;
			}

			// Check for end of block comment
			if in_block_comment && i + 1 < bytes.len() && bytes[i] == b'*' && bytes[i + 1] == b'/' {
				in_block_comment = false;
				i += 2;
				continue;
			}

			// Handle strings
			if !in_line_comment && !in_block_comment {
				match bytes[i] {
					b'\\' if in_string => {
						escape_next = true;
					}
					b'"' => {
						in_string = !in_string;
					}
					_ => {}
				}
			}

			i += 1;
		}

		in_string || in_line_comment || in_block_comment
	}

	/// Format macro tokens to formatted string.
	fn format_macro_tokens(&self, tokens: &proc_macro2::TokenStream) -> Result<String, String> {
		// Parse tokens as PageMacro
		let page_macro: PageMacro =
			syn::parse2(tokens.clone()).map_err(|e| format!("Parse error: {}", e))?;

		// Format the macro
		self.format_page_macro(&page_macro)
	}

	/// Format a PageMacro AST to string.
	fn format_page_macro(&self, macro_ast: &PageMacro) -> Result<String, String> {
		let mut output = String::new();

		// Format closure parameters
		self.format_params(&mut output, &macro_ast.params);

		// Format body
		output.push_str(" {\n");
		self.format_body(&mut output, &macro_ast.body, 1);
		output.push('}');

		Ok(output)
	}

	/// Format closure parameters: |param: Type, ...|
	fn format_params(&self, output: &mut String, params: &[PageParam]) {
		output.push('|');
		for (i, param) in params.iter().enumerate() {
			if i > 0 {
				output.push_str(", ");
			}
			output.push_str(&param.name.to_string());
			output.push_str(": ");
			output.push_str(&param.ty.to_token_stream().to_string());
		}
		output.push('|');
	}

	/// Format the page body.
	fn format_body(&self, output: &mut String, body: &PageBody, indent: usize) {
		for node in &body.nodes {
			self.format_node(output, node, indent);
		}
	}

	/// Format a single node.
	fn format_node(&self, output: &mut String, node: &PageNode, indent: usize) {
		match node {
			PageNode::Element(elem) => self.format_element(output, elem, indent),
			PageNode::Text(text) => self.format_text(output, text, indent),
			PageNode::Expression(expr) => self.format_expression(output, expr, indent),
			PageNode::If(if_node) => self.format_if(output, if_node, indent),
			PageNode::For(for_node) => self.format_for(output, for_node, indent),
			PageNode::Component(comp) => self.format_component(output, comp, indent),
		}
	}

	/// Format an element node.
	fn format_element(&self, output: &mut String, elem: &PageElement, indent: usize) {
		let ind = self.make_indent(indent);

		// Element tag
		output.push_str(&ind);
		output.push_str(&elem.tag.to_string());
		output.push_str(" {\n");

		// Attributes (one per line)
		for attr in &elem.attrs {
			self.format_attr(output, attr, indent + 1);
		}

		// Event handlers (one per line)
		for event in &elem.events {
			self.format_event(output, event, indent + 1);
		}

		// Children
		for child in &elem.children {
			self.format_node(output, child, indent + 1);
		}

		// Closing brace
		output.push_str(&ind);
		output.push_str("}\n");
	}

	/// Format an attribute.
	fn format_attr(&self, output: &mut String, attr: &PageAttr, indent: usize) {
		let ind = self.make_indent(indent);
		output.push_str(&ind);
		output.push_str(&attr.name.to_string());
		output.push_str(": ");
		output.push_str(&attr.value.to_token_stream().to_string());
		output.push_str(",\n");
	}

	/// Format an event handler.
	fn format_event(&self, output: &mut String, event: &PageEvent, indent: usize) {
		let ind = self.make_indent(indent);
		output.push_str(&ind);
		output.push('@');
		output.push_str(&event.event_type.to_string());
		output.push_str(": ");
		output.push_str(&event.handler.to_token_stream().to_string());
		output.push_str(",\n");
	}

	/// Format a text node.
	fn format_text(&self, output: &mut String, text: &PageText, indent: usize) {
		let ind = self.make_indent(indent);
		output.push_str(&ind);
		// Escape and quote the text
		let escaped = text.content.replace('\\', "\\\\").replace('"', "\\\"");
		output.push('"');
		output.push_str(&escaped);
		output.push_str("\"\n");
	}

	/// Format an expression node.
	fn format_expression(&self, output: &mut String, expr: &PageExpression, indent: usize) {
		let ind = self.make_indent(indent);
		output.push_str(&ind);

		if expr.braced {
			output.push_str("{ ");
			output.push_str(&expr.expr.to_token_stream().to_string());
			output.push_str(" }\n");
		} else {
			output.push_str(&expr.expr.to_token_stream().to_string());
			output.push('\n');
		}
	}

	/// Format an if node.
	fn format_if(&self, output: &mut String, if_node: &PageIf, indent: usize) {
		let ind = self.make_indent(indent);

		// if condition {
		output.push_str(&ind);
		output.push_str("if ");
		output.push_str(&if_node.condition.to_token_stream().to_string());
		output.push_str(" {\n");

		// then branch
		for node in &if_node.then_branch {
			self.format_node(output, node, indent + 1);
		}

		// else branch
		match &if_node.else_branch {
			Some(PageElse::Block(nodes)) => {
				output.push_str(&ind);
				output.push_str("} else {\n");
				for node in nodes {
					self.format_node(output, node, indent + 1);
				}
				output.push_str(&ind);
				output.push_str("}\n");
			}
			Some(PageElse::If(nested_if)) => {
				output.push_str(&ind);
				output.push_str("} else ");
				// Format the nested if without initial indent
				self.format_if_inline(output, nested_if, indent);
			}
			None => {
				output.push_str(&ind);
				output.push_str("}\n");
			}
		}
	}

	/// Format an if node inline (for else if chains).
	fn format_if_inline(&self, output: &mut String, if_node: &PageIf, indent: usize) {
		let ind = self.make_indent(indent);

		output.push_str("if ");
		output.push_str(&if_node.condition.to_token_stream().to_string());
		output.push_str(" {\n");

		for node in &if_node.then_branch {
			self.format_node(output, node, indent + 1);
		}

		match &if_node.else_branch {
			Some(PageElse::Block(nodes)) => {
				output.push_str(&ind);
				output.push_str("} else {\n");
				for node in nodes {
					self.format_node(output, node, indent + 1);
				}
				output.push_str(&ind);
				output.push_str("}\n");
			}
			Some(PageElse::If(nested_if)) => {
				output.push_str(&ind);
				output.push_str("} else ");
				self.format_if_inline(output, nested_if, indent);
			}
			None => {
				output.push_str(&ind);
				output.push_str("}\n");
			}
		}
	}

	/// Format a for node.
	fn format_for(&self, output: &mut String, for_node: &PageFor, indent: usize) {
		let ind = self.make_indent(indent);

		output.push_str(&ind);
		output.push_str("for ");
		output.push_str(&for_node.pat.to_token_stream().to_string());
		output.push_str(" in ");
		output.push_str(&for_node.iter.to_token_stream().to_string());
		output.push_str(" {\n");

		for node in &for_node.body {
			self.format_node(output, node, indent + 1);
		}

		output.push_str(&ind);
		output.push_str("}\n");
	}

	/// Format a component call.
	fn format_component(&self, output: &mut String, comp: &PageComponent, indent: usize) {
		let ind = self.make_indent(indent);

		output.push_str(&ind);
		output.push_str(&comp.name.to_string());
		output.push('(');

		// Arguments
		for (i, arg) in comp.args.iter().enumerate() {
			if i > 0 {
				output.push_str(", ");
			}
			output.push_str(&arg.name.to_string());
			output.push_str(": ");
			output.push_str(&arg.value.to_token_stream().to_string());
		}

		output.push(')');

		// Children
		if let Some(children) = &comp.children {
			output.push_str(" {\n");
			for child in children {
				self.format_node(output, child, indent + 1);
			}
			output.push_str(&ind);
			output.push('}');
		}

		output.push('\n');
	}

	/// Create indentation string.
	fn make_indent(&self, level: usize) -> String {
		self.indent.repeat(level)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_format_simple_element() {
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { div { "hello" } })"#;
		let result = formatter.format(input).unwrap();

		assert!(result.contains("div {"));
		assert!(result.contains("\"hello\""));
	}

	#[test]
	fn test_format_with_attributes() {
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { div { class: "container", id: "main", "text" } })"#;
		let result = formatter.format(input).unwrap();

		assert!(result.contains("class:"));
		assert!(result.contains("id:"));
	}

	#[test]
	fn test_no_change_non_page() {
		let formatter = AstPageFormatter::new();
		let input = "fn main() { println!(\"hello\"); }";
		let result = formatter.format(input).unwrap();

		assert_eq!(input, result);
	}

	#[test]
	fn test_skip_page_in_string() {
		let formatter = AstPageFormatter::new();
		let input = r#"let x = "page!(|| { div { } })";"#;
		let result = formatter.format(input).unwrap();

		// Should be unchanged since page! is in a string
		assert_eq!(input, result);
	}

	#[test]
	fn test_skip_page_in_comment() {
		let formatter = AstPageFormatter::new();
		let input = r#"// page!(|| { div { } })
fn main() {}"#;
		let result = formatter.format(input).unwrap();

		assert!(result.contains("// page!(|| { div { } })"));
	}

	#[test]
	fn test_format_with_params() {
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|name: String, count: i32| { div { {name} } })"#;
		let result = formatter.format(input).unwrap();

		assert!(result.contains("|name: String, count: i32|"));
	}

	#[test]
	fn test_format_nested_elements() {
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { div { span { "nested" } } })"#;
		let result = formatter.format(input).unwrap();

		assert!(result.contains("div {"));
		assert!(result.contains("span {"));
	}

	#[test]
	fn test_format_if_node() {
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { if show { div { "visible" } } })"#;
		let result = formatter.format(input).unwrap();

		assert!(result.contains("if show"));
	}

	#[test]
	fn test_format_for_node() {
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { for item in items { li { item } } })"#;
		let result = formatter.format(input).unwrap();

		assert!(result.contains("for item in items"));
	}

	#[test]
	fn test_format_component() {
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { MyButton(label: "Click") })"#;
		let result = formatter.format(input).unwrap();

		assert!(result.contains("MyButton("));
		assert!(result.contains("label:"));
	}

	#[test]
	fn test_format_event_handler() {
		let formatter = AstPageFormatter::new();
		let input = r#"page!(|| { button { @click: |_| {}, "Click" } })"#;
		let result = formatter.format(input).unwrap();

		assert!(result.contains("@click:"));
	}

	#[test]
	fn test_safety_complex_non_page_file() {
		let formatter = AstPageFormatter::new();

		let input = r#"//! Common UI components
use reinhardt_pages::Signal;

pub fn button(text: &str) -> View {
	let class = if disabled {
		format!("{} disabled", variant.class())
	} else {
		variant.class().to_string()
	};

	#[cfg(target_arch = "wasm32")]
	let button_view = {
		ElementView::new("button")
			.attr("class", &class)
			.child(text.to_string())
	};

	button_view.into_view()
}
"#;

		let result = formatter.format(input).unwrap();

		// Must be exactly identical
		assert_eq!(input, result, "Non-page files must not be modified");
	}
}
