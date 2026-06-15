//! tree-sitter grammar for the Reinhardt `page!` DSL.

#![warn(missing_docs)]

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
	fn tree_sitter_reinhardt_page() -> *const ();
}

/// The tree-sitter language function for the Reinhardt `page!` DSL grammar.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_reinhardt_page) };

/// The grammar node type metadata.
pub const NODE_TYPES: &str = include_str!("node-types.json");

#[cfg(test)]
mod tests {
	#[test]
	fn parser_loads() {
		let mut parser = tree_sitter::Parser::new();
		parser
			.set_language(&super::LANGUAGE.into())
			.expect("load page DSL grammar");
	}

	fn parse(source: &str) -> tree_sitter::Tree {
		let mut parser = tree_sitter::Parser::new();
		parser.set_language(&super::LANGUAGE.into()).unwrap();
		parser.parse(source, None).unwrap()
	}

	fn has_node_kind(node: tree_sitter::Node<'_>, expected_kind: &str) -> bool {
		if node.kind() == expected_kind {
			return true;
		}

		let mut cursor = node.walk();
		node.children(&mut cursor)
			.any(|child| has_node_kind(child, expected_kind))
	}

	fn count_node_kind(node: tree_sitter::Node<'_>, expected_kind: &str) -> usize {
		let self_count = usize::from(node.kind() == expected_kind);
		let mut cursor = node.walk();
		self_count
			+ node
				.children(&mut cursor)
				.map(|child| count_node_kind(child, expected_kind))
				.sum::<usize>()
	}

	fn has_element_starting_with(node: tree_sitter::Node<'_>, source: &str, prefix: &str) -> bool {
		if node.kind() == "element"
			&& node
				.utf8_text(source.as_bytes())
				.is_ok_and(|text| text.trim_start().starts_with(prefix))
		{
			return true;
		}

		let mut cursor = node.walk();
		node.children(&mut cursor)
			.any(|child| has_element_starting_with(child, source, prefix))
	}

	fn find_node_kind<'a>(
		node: tree_sitter::Node<'a>,
		expected_kind: &str,
	) -> Option<tree_sitter::Node<'a>> {
		if node.kind() == expected_kind {
			return Some(node);
		}

		let mut cursor = node.walk();
		node.children(&mut cursor)
			.find_map(|child| find_node_kind(child, expected_kind))
	}

	fn assert_has_node(source: &str, expected_node: &str) {
		let tree = parse(source);
		let sexp = tree.root_node().to_sexp();
		assert!(
			!tree.root_node().has_error(),
			"parse tree should not contain errors: {sexp}"
		);
		assert!(
			has_node_kind(tree.root_node(), expected_node),
			"expected node {expected_node} in parse tree: {sexp}"
		);
	}

	fn assert_control_flow_body(source: &str, control_flow_kind: &str, expected_body: &str) {
		let tree = parse(source);
		let sexp = tree.root_node().to_sexp();
		assert!(
			!tree.root_node().has_error(),
			"parse tree should not contain errors: {sexp}"
		);
		let control_flow = find_node_kind(tree.root_node(), control_flow_kind)
			.unwrap_or_else(|| panic!("expected {control_flow_kind} node in parse tree: {sexp}"));
		let mut cursor = control_flow.walk();
		let body = control_flow
			.children(&mut cursor)
			.find(|child| child.kind() == "block")
			.unwrap_or_else(|| {
				panic!("expected {control_flow_kind} body block in parse tree: {sexp}")
			});
		assert_eq!(
			body.utf8_text(source.as_bytes()).unwrap(),
			expected_body,
			"{control_flow_kind} should use the DSL body block, not a Rust block expression: {sexp}"
		);
	}

	// -- Block comment tests --------------------------------------------------

	#[test]
	fn block_comment_simple() {
		// Arrange
		let mut parser = tree_sitter::Parser::new();
		parser.set_language(&super::LANGUAGE.into()).unwrap();
		let source = "/* hello */";

		// Act
		let tree = parser.parse(source, None).unwrap();

		// Assert
		assert!(
			!tree.root_node().has_error(),
			"parse tree: {}",
			tree.root_node().to_sexp()
		);
	}

	#[test]
	fn block_comment_empty() {
		// Arrange
		let mut parser = tree_sitter::Parser::new();
		parser.set_language(&super::LANGUAGE.into()).unwrap();
		let source = "/**/";

		// Act
		let tree = parser.parse(source, None).unwrap();

		// Assert
		assert!(
			!tree.root_node().has_error(),
			"parse tree: {}",
			tree.root_node().to_sexp()
		);
	}

	#[test]
	fn block_comment_slash_star_slash_is_not_complete() {
		// Arrange
		let mut parser = tree_sitter::Parser::new();
		parser.set_language(&super::LANGUAGE.into()).unwrap();
		let source = "/*/ div { \"text\" }";

		// Act
		let tree = parser.parse(source, None).unwrap();

		// Assert — '/*/' should NOT close the comment, so the rest is consumed
		// as comment body
		let sexp = tree.root_node().to_sexp();
		// The entire input should be treated as an unterminated block comment
		// or error
		assert!(
			tree.root_node().has_error() || !sexp.contains("block_comment"),
			"/*/ should not be a valid block comment: {sexp}"
		);
	}

	#[test]
	fn block_comment_nested_stars() {
		// Arrange
		let mut parser = tree_sitter::Parser::new();
		parser.set_language(&super::LANGUAGE.into()).unwrap();
		let source = "/* nested ** stars */";

		// Act
		let tree = parser.parse(source, None).unwrap();

		// Assert
		assert!(
			!tree.root_node().has_error(),
			"parse tree: {}",
			tree.root_node().to_sexp()
		);
	}

	#[test]
	fn block_comment_unterminated_is_not_accepted() {
		// Arrange
		let mut parser = tree_sitter::Parser::new();
		parser.set_language(&super::LANGUAGE.into()).unwrap();
		let source = "/* unterminated comment";

		// Act
		let tree = parser.parse(source, None).unwrap();

		// Assert — the scanner rejects unterminated block comments, so
		// the text must NOT appear as a block_comment node. The grammar
		// may error-recover the input as fragments instead.
		let sexp = tree.root_node().to_sexp();
		assert!(
			!sexp.contains("block_comment"),
			"unterminated input should not produce a block_comment node: {sexp}"
		);
	}

	// -- Line comment tests ---------------------------------------------------

	#[test]
	fn line_comment_simple() {
		// Arrange
		let mut parser = tree_sitter::Parser::new();
		parser.set_language(&super::LANGUAGE.into()).unwrap();
		let source = "// this is a comment";

		// Act
		let tree = parser.parse(source, None).unwrap();

		// Assert
		assert!(
			!tree.root_node().has_error(),
			"parse tree: {}",
			tree.root_node().to_sexp()
		);
	}

	#[test]
	fn line_comment_only_captures_first_line() {
		// Arrange
		let mut parser = tree_sitter::Parser::new();
		parser.set_language(&super::LANGUAGE.into()).unwrap();
		let source = "// comment\ndiv { \"hello\" }";

		// Act
		let tree = parser.parse(source, None).unwrap();

		// Assert — the line comment should not swallow the second line
		let sexp = tree.root_node().to_sexp();
		assert!(
			sexp.contains("line_comment"),
			"expected a line_comment node: {sexp}"
		);
		// The second line should produce additional nodes beyond the comment
		let root = tree.root_node();
		assert!(
			root.child_count() > 1,
			"second line should produce separate nodes after the comment: {sexp}"
		);
	}

	// -- Basic DSL structure tests --------------------------------------------

	#[test]
	fn page_dsl_basic_structure() {
		// Arrange
		let mut parser = tree_sitter::Parser::new();
		parser.set_language(&super::LANGUAGE.into()).unwrap();
		let source = r#"|| { div { "hello" } }"#;

		// Act
		let tree = parser.parse(source, None).unwrap();

		// Assert
		assert!(
			!tree.root_node().has_error(),
			"parse tree: {}",
			tree.root_node().to_sexp()
		);
	}

	#[test]
	fn page_dsl_parses_semantic_element() {
		assert_has_node(r#"|| { div { "hello" } }"#, "element");
	}

	#[test]
	fn page_dsl_parses_typed_closure_args_with_nested_type_syntax() {
		let source = r#"|items: Vec<(usize, String)>| { div { "x" } }"#;

		assert_has_node(source, "closure_args");
		assert_has_node(source, "element");
	}

	#[test]
	fn page_dsl_parses_comma_separated_closure_args() {
		let source = r#"|handler1: Callback, handler2: Callback| { div { "x" } }"#;

		assert_has_node(source, "closure_args");
		assert_has_node(source, "element");
	}

	#[test]
	fn page_dsl_parses_lifetime_in_closure_args() {
		let source = r#"|item: &'a str| { div { "x" } }"#;

		assert_has_node(source, "closure_args");
		assert_has_node(source, "element");
	}

	#[test]
	fn page_dsl_parses_attribute_rustfmt_island() {
		assert_has_node(
			r#"|| { button { tabindex: (- 1_i32).to_string(), "Save" } }"#,
			"rustfmt_island",
		);
	}

	#[test]
	fn page_dsl_parses_numeric_attribute_rustfmt_island() {
		let source = r#"|| { button { tabindex: 0, "Save" } }"#;

		assert_has_node(source, "attribute");
		assert_has_node(source, "rustfmt_island");
	}

	#[test]
	fn page_dsl_parses_identifier_attribute_rustfmt_island() {
		let source = r#"|| { button { disabled: is_loading, "Save" } }"#;

		assert_has_node(source, "attribute");
		assert_has_node(source, "rustfmt_island");
	}

	#[test]
	fn page_dsl_parses_event_attribute_rustfmt_island() {
		let source =
			r#"|| { button { @click: |_| { set_count.update(|value| *value += 1); }, "Save" } }"#;

		assert_has_node(source, "event_attribute");
		assert_has_node(source, "rustfmt_island");
	}

	#[test]
	fn page_dsl_parses_event_attribute_rustfmt_island_without_trailing_comma() {
		let source = r#"|| { button { @click: |_| { set_count.update(|value| *value += 1); } } }"#;

		assert_has_node(source, "event_attribute");
		assert_has_node(source, "rustfmt_island");
	}

	#[test]
	fn page_dsl_parses_event_attribute_identifier_rustfmt_island_without_trailing_comma() {
		let source = r#"|| { button { @click: handle_click } }"#;

		assert_has_node(source, "event_attribute");
		assert_has_node(source, "rustfmt_island");
	}

	#[test]
	fn page_dsl_parses_interpolation_rustfmt_island() {
		let source = r#"|| { div { { count + 1 } } }"#;

		assert_has_node(source, "interpolation");
		assert_has_node(source, "rustfmt_island");
	}

	#[test]
	fn page_dsl_parses_control_flow_without_rustfmt_targeting_body() {
		let tree = parse(r#"|| { if show { div { "visible" } } else { span { "hidden" } } }"#);
		let sexp = tree.root_node().to_sexp();
		assert!(
			!tree.root_node().has_error(),
			"parse tree should not contain errors: {sexp}"
		);
		assert!(
			has_node_kind(tree.root_node(), "control_flow"),
			"expected control_flow node in parse tree: {sexp}"
		);
		assert!(
			has_node_kind(tree.root_node(), "element"),
			"control_flow body should keep DSL elements visible: {sexp}"
		);
	}

	#[test]
	fn page_dsl_parses_for_control_flow_with_parenthesized_iterator() {
		let source = r#"|| { for item in items.clone() { div { { item } } } }"#;

		assert_has_node(source, "control_flow");
	}

	#[test]
	fn page_dsl_parses_for_control_flow_with_keyed_iterator() {
		let source = r#"|| { for todo in todos @key(todo.id) { div { { todo.text } } } }"#;

		assert_has_node(source, "control_flow");
	}

	#[test]
	fn page_dsl_parses_if_control_flow_with_closure_block_in_head() {
		let source = r#"|| { if items.iter().any(|x| { x.active }) { div { "active" } } }"#;

		assert_has_node(source, "control_flow");
		assert_has_node(source, "if_control_flow");
	}

	#[test]
	fn page_dsl_parses_if_control_flow_with_block_expression_head() {
		let source = r#"|| { if { true } { div { "x" } } }"#;

		assert_control_flow_body(source, "if_control_flow", r#"{ div { "x" } }"#);
	}

	#[test]
	fn page_dsl_parses_match_control_flow_with_block_expression_head() {
		let source = r#"|| { match { value } { _ => div { "x" } } }"#;

		assert_control_flow_body(source, "match_control_flow", r#"{ _ => div { "x" } }"#);
	}

	#[test]
	fn page_dsl_parses_for_control_flow_with_block_expression_iterator() {
		let source = r#"|| { for item in { items } { div { { item } } } }"#;

		assert_control_flow_body(source, "for_control_flow", r#"{ div { { item } } }"#);
	}

	#[test]
	fn page_dsl_parses_control_flow_keyword_followed_by_newline() {
		let source = "|| { if\nshow { div { \"x\" } } }";

		assert_control_flow_body(source, "if_control_flow", r#"{ div { "x" } }"#);
	}

	#[test]
	fn page_dsl_parses_else_if_as_control_flow() {
		let source = r#"|| { if status == 0 { span { "Pending" } } else if status == 1 { span { "Processing" } } else { span { "Done" } } }"#;
		let tree = parse(source);
		let sexp = tree.root_node().to_sexp();
		assert!(
			!tree.root_node().has_error(),
			"parse tree should not contain errors: {sexp}"
		);
		assert!(
			count_node_kind(tree.root_node(), "control_flow") >= 2,
			"expected nested else-if control_flow nodes in parse tree: {sexp}"
		);
		assert!(
			!has_element_starting_with(tree.root_node(), source, "else if"),
			"else-if must not be exposed as an element node: {sexp}"
		);
	}

	#[test]
	fn page_dsl_parses_if_control_flow_without_else() {
		let tree = parse(r#"|| { if show { div { "visible" } } }"#);
		let sexp = tree.root_node().to_sexp();
		assert!(
			!tree.root_node().has_error(),
			"parse tree should not contain errors: {sexp}"
		);
		assert!(
			has_node_kind(tree.root_node(), "control_flow"),
			"expected control_flow node in parse tree: {sexp}"
		);
		assert!(
			has_node_kind(tree.root_node(), "element"),
			"control_flow body should keep DSL elements visible: {sexp}"
		);
	}

	#[test]
	fn page_dsl_parses_consecutive_elements_without_control_flow() {
		let tree = parse(r#"|| { div { "a" } span { "b" } }"#);
		let sexp = tree.root_node().to_sexp();
		assert!(
			!tree.root_node().has_error(),
			"parse tree should not contain errors: {sexp}"
		);
		assert_eq!(
			count_node_kind(tree.root_node(), "control_flow"),
			0,
			"ordinary adjacent elements must not parse as control_flow: {sexp}"
		);
		assert_eq!(
			count_node_kind(tree.root_node(), "element"),
			2,
			"expected both adjacent tags to remain element nodes: {sexp}"
		);
	}

	// -- DSL with embedded comments -------------------------------------------

	#[test]
	fn page_dsl_with_block_comment() {
		// Arrange
		let mut parser = tree_sitter::Parser::new();
		parser.set_language(&super::LANGUAGE.into()).unwrap();
		let source = r#"|| { /* comment */ div { "x" } }"#;

		// Act
		let tree = parser.parse(source, None).unwrap();

		// Assert
		let sexp = tree.root_node().to_sexp();
		assert!(!tree.root_node().has_error(), "parse tree: {sexp}");
		assert!(
			sexp.contains("block_comment"),
			"expected a block_comment node in the tree: {sexp}"
		);
	}
}
