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

	fn visit_form(&mut self, ir: &FormIR) -> Self::Output {
		self.output.push_str("<form");

		// Add name attribute
		self.output.push_str(" name=\"");
		self.output.push_str(&Self::escape_html(&ir.name));
		self.output.push('"');

		// Add action
		match &ir.action {
			FormActionIR::Url(url) => {
				self.output.push_str(" action=\"");
				self.output.push_str(&Self::escape_html(url));
				self.output.push('"');
			}
			FormActionIR::ServerFn(name) => {
				self.output.push_str(" data-server-fn=\"");
				self.output.push_str(&Self::escape_html(name));
				self.output.push('"');
			}
		}

		// Add method
		self.output.push_str(" method=\"");
		match ir.method {
			FormMethodIR::Get => self.output.push_str("get"),
			FormMethodIR::Post => self.output.push_str("post"),
			FormMethodIR::Put => self.output.push_str("put"),
			FormMethodIR::Patch => self.output.push_str("patch"),
			FormMethodIR::Delete => self.output.push_str("delete"),
		}
		self.output.push('"');

		// Add styling class if present
		if let Some(class) = &ir.styling.class {
			self.output.push_str(" class=\"");
			self.output.push_str(&Self::escape_html(class));
			self.output.push('"');
		}

		// Add additional styling attrs
		for (name, value) in &ir.styling.attrs {
			self.output.push(' ');
			self.output.push_str(name);
			self.output.push_str("=\"");
			self.output.push_str(&Self::escape_html(value));
			self.output.push('"');
		}

		self.output.push('>');

		// Render fields
		for field in &ir.fields {
			self.visit_field(field);
		}

		self.output.push_str("</form>");
	}

	fn visit_field(&mut self, ir: &FieldIR) -> Self::Output {
		// Wrap field in a div
		self.output.push_str("<div class=\"form-field\">");

		// Render label if present
		if let Some(label) = &ir.label {
			self.output.push_str("<label for=\"");
			self.output.push_str(&Self::escape_html(&ir.name));
			self.output.push_str("\">");
			self.output.push_str(&Self::escape_html(label));
			if ir.required {
				self.output.push_str(" <span class=\"required\">*</span>");
			}
			self.output.push_str("</label>");
		}

		// Render input based on widget type
		match &ir.widget.widget_type {
			WidgetTypeIR::TextInput => {
				self.output.push_str("<input type=\"text\"");
			}
			WidgetTypeIR::PasswordInput => {
				self.output.push_str("<input type=\"password\"");
			}
			WidgetTypeIR::TextArea => {
				self.output.push_str("<textarea");
			}
			WidgetTypeIR::Select => {
				self.output.push_str("<select");
			}
			WidgetTypeIR::Checkbox => {
				self.output.push_str("<input type=\"checkbox\"");
			}
			WidgetTypeIR::Radio => {
				self.output.push_str("<input type=\"radio\"");
			}
			WidgetTypeIR::FileInput => {
				self.output.push_str("<input type=\"file\"");
			}
			WidgetTypeIR::DateInput => {
				self.output.push_str("<input type=\"date\"");
			}
			WidgetTypeIR::DateTimeInput => {
				self.output.push_str("<input type=\"datetime-local\"");
			}
			WidgetTypeIR::Hidden => {
				self.output.push_str("<input type=\"hidden\"");
			}
			WidgetTypeIR::Custom(tag) => {
				self.output.push('<');
				self.output.push_str(tag);
			}
		}

		// Add name and id
		self.output.push_str(" name=\"");
		self.output.push_str(&Self::escape_html(&ir.name));
		self.output.push_str("\" id=\"");
		self.output.push_str(&Self::escape_html(&ir.name));
		self.output.push('"');

		// Add required if needed
		if ir.required {
			self.output.push_str(" required");
		}

		// Add widget attrs
		for (name, value) in &ir.widget.attrs {
			self.output.push(' ');
			self.output.push_str(name);
			self.output.push_str("=\"");
			self.output.push_str(&Self::escape_html(value));
			self.output.push('"');
		}

		// Close the input element
		match &ir.widget.widget_type {
			WidgetTypeIR::TextArea => {
				self.output.push_str("></textarea>");
			}
			WidgetTypeIR::Select => {
				self.output.push_str("></select>");
			}
			_ => {
				self.output.push_str(" />");
			}
		}

		self.output.push_str("</div>");
	}

	fn visit_head(&mut self, ir: &HeadIR) -> Self::Output {
		for element in &ir.elements {
			self.visit_head_element(element);
		}
	}

	fn visit_head_element(&mut self, ir: &HeadElementIR) -> Self::Output {
		match ir {
			HeadElementIR::Title(title) => {
				self.output.push_str("<title>");
				self.output.push_str(&Self::escape_html(&title.content));
				self.output.push_str("</title>");
			}
			HeadElementIR::Meta(meta) => {
				self.output.push_str("<meta");
				for (name, value) in &meta.attrs {
					self.output.push(' ');
					self.output.push_str(name);
					self.output.push_str("=\"");
					self.output.push_str(&Self::escape_html(value));
					self.output.push('"');
				}
				self.output.push_str(" />");
			}
			HeadElementIR::Link(link) => {
				self.output.push_str("<link rel=\"");
				self.output.push_str(&Self::escape_html(&link.rel));
				self.output.push_str("\" href=\"");
				self.output.push_str(&Self::escape_html(&link.href));
				self.output.push('"');
				for (name, value) in &link.attrs {
					self.output.push(' ');
					self.output.push_str(name);
					self.output.push_str("=\"");
					self.output.push_str(&Self::escape_html(value));
					self.output.push('"');
				}
				self.output.push_str(" />");
			}
			HeadElementIR::Script(script) => {
				self.output.push_str("<script");
				if let Some(src) = &script.src {
					self.output.push_str(" src=\"");
					self.output.push_str(&Self::escape_html(src));
					self.output.push('"');
				}
				if script.is_async {
					self.output.push_str(" async");
				}
				if script.defer {
					self.output.push_str(" defer");
				}
				if script.is_module {
					self.output.push_str(" type=\"module\"");
				}
				self.output.push('>');
				if let Some(content) = &script.content {
					self.output.push_str(content);
				}
				self.output.push_str("</script>");
			}
			HeadElementIR::Style(style) => {
				self.output.push_str("<style>");
				self.output.push_str(&style.content);
				self.output.push_str("</style>");
			}
		}
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

	// Head tests
	#[rstest]
	fn test_visit_head_with_title() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let head = HeadIR {
			elements: vec![HeadElementIR::Title(TitleIR {
				content: "My App".to_string(),
				span: Span::call_site(),
			})],
			span: Span::call_site(),
		};

		// Act
		visitor.visit_head(&head);

		// Assert
		assert_eq!(visitor.into_html(), "<title>My App</title>");
	}

	#[rstest]
	fn test_visit_head_with_meta() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let head = HeadIR {
			elements: vec![HeadElementIR::Meta(MetaIR {
				attrs: vec![("charset".to_string(), "utf-8".to_string())],
				span: Span::call_site(),
			})],
			span: Span::call_site(),
		};

		// Act
		visitor.visit_head(&head);

		// Assert
		assert_eq!(visitor.into_html(), r#"<meta charset="utf-8" />"#);
	}

	#[rstest]
	fn test_visit_head_with_link() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let head = HeadIR {
			elements: vec![HeadElementIR::Link(LinkIR {
				rel: "stylesheet".to_string(),
				href: "/styles.css".to_string(),
				attrs: vec![],
				span: Span::call_site(),
			})],
			span: Span::call_site(),
		};

		// Act
		visitor.visit_head(&head);

		// Assert
		assert_eq!(
			visitor.into_html(),
			r#"<link rel="stylesheet" href="/styles.css" />"#
		);
	}

	#[rstest]
	fn test_visit_head_with_script() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let head = HeadIR {
			elements: vec![HeadElementIR::Script(ScriptIR {
				src: Some("/app.js".to_string()),
				content: None,
				is_async: false,
				defer: true,
				is_module: true,
				span: Span::call_site(),
			})],
			span: Span::call_site(),
		};

		// Act
		visitor.visit_head(&head);

		// Assert
		assert_eq!(
			visitor.into_html(),
			r#"<script src="/app.js" defer type="module"></script>"#
		);
	}

	#[rstest]
	fn test_visit_head_with_style() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let head = HeadIR {
			elements: vec![HeadElementIR::Style(StyleIR {
				content: "body { margin: 0; }".to_string(),
				span: Span::call_site(),
			})],
			span: Span::call_site(),
		};

		// Act
		visitor.visit_head(&head);

		// Assert
		assert_eq!(visitor.into_html(), "<style>body { margin: 0; }</style>");
	}

	// Form tests
	#[rstest]
	fn test_visit_form_basic() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let form = FormIR {
			name: "login".to_string(),
			action: FormActionIR::Url("/login".to_string()),
			method: FormMethodIR::Post,
			fields: vec![],
			styling: FormStylingIR {
				class: None,
				attrs: vec![],
			},
			span: Span::call_site(),
		};

		// Act
		visitor.visit_form(&form);

		// Assert
		assert_eq!(
			visitor.into_html(),
			r#"<form name="login" action="/login" method="post"></form>"#
		);
	}

	#[rstest]
	fn test_visit_form_with_field() {
		// Arrange
		let mut visitor = StaticHtmlVisitor::new();
		let form = FormIR {
			name: "contact".to_string(),
			action: FormActionIR::Url("/contact".to_string()),
			method: FormMethodIR::Post,
			fields: vec![FieldIR {
				name: "email".to_string(),
				field_type: FieldTypeIR::EmailField,
				label: Some("Email".to_string()),
				required: true,
				validators: vec![],
				widget: WidgetIR {
					widget_type: WidgetTypeIR::TextInput,
					attrs: vec![("placeholder".to_string(), "you@example.com".to_string())],
				},
				span: Span::call_site(),
			}],
			styling: FormStylingIR {
				class: Some("form-horizontal".to_string()),
				attrs: vec![],
			},
			span: Span::call_site(),
		};

		// Act
		visitor.visit_form(&form);

		// Assert
		let html = visitor.into_html();
		assert!(html.contains(r#"<form name="contact""#));
		assert!(html.contains(r#"class="form-horizontal""#));
		assert!(html.contains(r#"<label for="email">Email"#));
		assert!(html.contains(r#"<span class="required">*</span>"#));
		assert!(html.contains(r#"<input type="text""#));
		assert!(html.contains(r#"name="email""#));
		assert!(html.contains(r#"required"#));
		assert!(html.contains(r#"placeholder="you@example.com""#));
	}
}
