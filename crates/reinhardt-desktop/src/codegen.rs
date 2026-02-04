//! Code generation for desktop applications.
//!
//! This module provides `StaticHtmlVisitor` that implements `IRVisitor`
//! to generate static HTML from reinhardt-manouche IR.

use reinhardt_manouche::codegen::IRVisitor;
use reinhardt_manouche::ir::*;

/// Generates static HTML from component IR.
///
/// This visitor produces HTML strings that can be bundled
/// with the desktop application via `ProtocolHandler`.
pub struct StaticHtmlVisitor {
	output: String,
}

impl StaticHtmlVisitor {
	/// Self-closing (void) HTML elements.
	const VOID_ELEMENTS: &'static [&'static str] = &[
		"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
		"source", "track", "wbr",
	];

	/// Creates a new visitor.
	pub fn new() -> Self {
		Self {
			output: String::new(),
		}
	}

	/// Consumes the visitor and returns the generated HTML.
	pub fn into_html(self) -> String {
		self.output
	}

	/// Escapes HTML special characters.
	fn escape_html(s: &str) -> String {
		s.replace('&', "&amp;")
			.replace('<', "&lt;")
			.replace('>', "&gt;")
			.replace('"', "&quot;")
			.replace('\'', "&#39;")
	}
}

impl Default for StaticHtmlVisitor {
	fn default() -> Self {
		Self::new()
	}
}

impl IRVisitor for StaticHtmlVisitor {
	type Output = ();

	fn visit_component(&mut self, ir: &ComponentIR) -> Self::Output {
		for node in &ir.body {
			self.visit_node(node);
		}
	}

	fn visit_prop(&mut self, _ir: &PropIR) -> Self::Output {
		// Props are compile-time only, no output needed
	}

	fn visit_element(&mut self, ir: &ElementIR) -> Self::Output {
		self.output.push('<');
		self.output.push_str(&ir.tag);

		for attr in &ir.attributes {
			self.visit_attribute(attr);
		}

		if Self::VOID_ELEMENTS.contains(&ir.tag.as_str()) {
			self.output.push_str(" />");
		} else {
			self.output.push('>');
			for child in &ir.children {
				self.visit_node(child);
			}
			self.output.push_str("</");
			self.output.push_str(&ir.tag);
			self.output.push('>');
		}
	}

	fn visit_text(&mut self, ir: &TextIR) -> Self::Output {
		self.output.push_str(&Self::escape_html(&ir.content));
	}

	fn visit_expression(&mut self, _ir: &ExprIR) -> Self::Output {
		// Dynamic expressions cannot be rendered statically
		self.output.push_str("[expr]");
	}

	fn visit_conditional(&mut self, ir: &ConditionalIR) -> Self::Output {
		// For static HTML, render then_body only (assume true)
		for node in &ir.then_body {
			self.visit_node(node);
		}
	}

	fn visit_loop(&mut self, _ir: &LoopIR) -> Self::Output {
		// Loops require runtime data, output comment
		self.output.push_str("<!-- loop: requires runtime data -->");
	}

	fn visit_fragment(&mut self, ir: &[NodeIR]) -> Self::Output {
		for node in ir {
			self.visit_node(node);
		}
	}

	fn visit_component_call(&mut self, ir: &ComponentCallIR) -> Self::Output {
		// Component calls output a placeholder
		self.output
			.push_str(&format!("<!-- component: {} -->", ir.name));
	}

	fn visit_watch(&mut self, ir: &WatchIR) -> Self::Output {
		// Watch blocks just render their body
		for node in &ir.body {
			self.visit_node(node);
		}
	}

	fn visit_attribute(&mut self, ir: &AttributeIR) -> Self::Output {
		self.output.push(' ');
		self.output.push_str(&ir.name);

		match &ir.value {
			AttrValueIR::Static(value) => {
				self.output.push_str("=\"");
				self.output.push_str(&Self::escape_html(value));
				self.output.push('"');
			}
			AttrValueIR::Dynamic(_) => {
				self.output.push_str("=\"[dynamic]\"");
			}
			AttrValueIR::Flag => {
				// Boolean attributes don't need a value
			}
		}
	}

	fn visit_event(&mut self, _ir: &EventIR) -> Self::Output {
		// Events are handled by JavaScript, no HTML output
	}

	fn visit_form(&mut self, _ir: &FormIR) -> Self::Output {
		todo!("Form rendering not yet implemented")
	}

	fn visit_field(&mut self, _ir: &FieldIR) -> Self::Output {
		todo!("Field rendering not yet implemented")
	}

	fn visit_head(&mut self, _ir: &HeadIR) -> Self::Output {
		todo!("Head rendering not yet implemented")
	}

	fn visit_head_element(&mut self, _ir: &HeadElementIR) -> Self::Output {
		todo!("HeadElement rendering not yet implemented")
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use proc_macro2::Span;
	use rstest::rstest;

	#[rstest]
	fn test_static_html_visitor_new() {
		// Act
		let visitor = StaticHtmlVisitor::new();

		// Assert
		assert!(visitor.output.is_empty());
	}

	// Task 5: visit_text tests
	#[rstest]
	fn test_visit_text_plain() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let text = TextIR {
			content: "Hello, World!".to_string(),
			span: Span::call_site(),
		};

		// Act
		visitor.visit_text(&text);

		// Assert
		assert_eq!(visitor.into_html(), "Hello, World!");
	}

	#[rstest]
	fn test_visit_text_with_html_entities() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let text = TextIR {
			content: "<script>alert('xss')</script>".to_string(),
			span: Span::call_site(),
		};

		// Act
		visitor.visit_text(&text);

		// Assert
		assert_eq!(
			visitor.into_html(),
			"&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;"
		);
	}

	// Task 6: visit_element tests
	#[rstest]
	fn test_visit_element_simple() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let element = ElementIR {
			tag: "div".to_string(),
			attributes: vec![],
			events: vec![],
			children: vec![],
			span: Span::call_site(),
		};

		// Act
		visitor.visit_element(&element);

		// Assert
		assert_eq!(visitor.into_html(), "<div></div>");
	}

	#[rstest]
	fn test_visit_element_with_children() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let element = ElementIR {
			tag: "p".to_string(),
			attributes: vec![],
			events: vec![],
			children: vec![NodeIR::Text(TextIR {
				content: "Hello".to_string(),
				span: Span::call_site(),
			})],
			span: Span::call_site(),
		};

		// Act
		visitor.visit_element(&element);

		// Assert
		assert_eq!(visitor.into_html(), "<p>Hello</p>");
	}

	#[rstest]
	fn test_visit_element_self_closing() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let element = ElementIR {
			tag: "br".to_string(),
			attributes: vec![],
			events: vec![],
			children: vec![],
			span: Span::call_site(),
		};

		// Act
		visitor.visit_element(&element);

		// Assert
		assert_eq!(visitor.into_html(), "<br />");
	}

	// Task 7: visit_attribute tests
	#[rstest]
	fn test_visit_element_with_static_attribute() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let element = ElementIR {
			tag: "div".to_string(),
			attributes: vec![AttributeIR {
				name: "class".to_string(),
				value: AttrValueIR::Static("container".to_string()),
				span: Span::call_site(),
			}],
			events: vec![],
			children: vec![],
			span: Span::call_site(),
		};

		// Act
		visitor.visit_element(&element);

		// Assert
		assert_eq!(visitor.into_html(), r#"<div class="container"></div>"#);
	}

	#[rstest]
	fn test_visit_element_with_flag_attribute() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let element = ElementIR {
			tag: "input".to_string(),
			attributes: vec![
				AttributeIR {
					name: "type".to_string(),
					value: AttrValueIR::Static("checkbox".to_string()),
					span: Span::call_site(),
				},
				AttributeIR {
					name: "checked".to_string(),
					value: AttrValueIR::Flag,
					span: Span::call_site(),
				},
			],
			events: vec![],
			children: vec![],
			span: Span::call_site(),
		};

		// Act
		visitor.visit_element(&element);

		// Assert
		assert_eq!(visitor.into_html(), r#"<input type="checkbox" checked />"#);
	}

	// Task 8: visit_component test
	#[rstest]
	fn test_visit_component_generates_full_html() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let component = ComponentIR {
			props: vec![],
			body: vec![
				NodeIR::Element(ElementIR {
					tag: "h1".to_string(),
					attributes: vec![],
					events: vec![],
					children: vec![NodeIR::Text(TextIR {
						content: "Welcome".to_string(),
						span: Span::call_site(),
					})],
					span: Span::call_site(),
				}),
				NodeIR::Element(ElementIR {
					tag: "p".to_string(),
					attributes: vec![],
					events: vec![],
					children: vec![NodeIR::Text(TextIR {
						content: "Hello, Desktop!".to_string(),
						span: Span::call_site(),
					})],
					span: Span::call_site(),
				}),
			],
			span: Span::call_site(),
		};

		// Act
		visitor.visit_component(&component);

		// Assert
		assert_eq!(
			visitor.into_html(),
			"<h1>Welcome</h1><p>Hello, Desktop!</p>"
		);
	}
}
