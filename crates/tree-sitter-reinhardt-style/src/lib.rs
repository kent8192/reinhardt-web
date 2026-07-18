//! tree-sitter grammar for the Reinhardt `style!` DSL.

#![warn(missing_docs)]

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
	fn tree_sitter_reinhardt_style() -> *const ();
}

/// The tree-sitter language function for the Reinhardt `style!` DSL grammar.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_reinhardt_style) };

/// The grammar node type metadata.
pub const NODE_TYPES: &str = include_str!("node-types.json");

#[cfg(test)]
mod tests {
	use rstest::rstest;
	use tree_sitter::{Node, Parser};

	fn parse(source: &str) -> tree_sitter::Tree {
		let mut parser = Parser::new();
		parser
			.set_language(&super::LANGUAGE.into())
			.expect("load style DSL grammar");
		parser
			.parse(source, None)
			.expect("parser should produce a tree")
	}

	fn count_kind(node: Node<'_>, kind: &str) -> usize {
		let mut count = usize::from(node.kind() == kind);
		let mut cursor = node.walk();
		for child in node.children(&mut cursor) {
			count += count_kind(child, kind);
		}
		count
	}

	#[rstest]
	fn parser_loads() {
		// Arrange
		let mut parser = tree_sitter::Parser::new();

		// Act
		let result = parser.set_language(&super::LANGUAGE.into());

		// Assert
		assert_eq!(result, Ok(()));
	}

	#[rstest]
	fn representative_style_has_semantic_formatter_nodes() {
		// Arrange
		let source = r#"
			globals { border: Color; }
			vars { accent: Color = red; }
			.card {
				color: vars.accent;
				&:hover { color: blue; }
				@media (max-width: 640px) { padding: 1rem; }
			}
		"#;

		// Act
		let tree = parse(source);
		let root = tree.root_node();

		// Assert
		assert!(!root.has_error(), "parse tree: {}", root.to_sexp());
		assert_eq!(count_kind(root, "definition_block"), 2);
		assert_eq!(count_kind(root, "style_rule"), 2);
		assert_eq!(count_kind(root, "media_rule"), 1);
		assert_eq!(count_kind(root, "property_declaration"), 3);
	}

	#[rstest]
	#[case(".card { color: red; }")]
	#[case(".card, .panel { color: red; }")]
	#[case(".card { &:hover { color: red; } }")]
	#[case(".card { &[data-state=\"open\"] { color: red; } }")]
	#[case(".card { &.featured { color: red; } }")]
	#[case(".card { &:is(button) { color: red; } }")]
	#[case(".card { > h5 { color: red; } }")]
	#[case(".card { + .card { color: red; } }")]
	#[case(".card { ~ .card { color: red; } }")]
	#[case(".card { .label { color: red; } }")]
	#[case(".card { button { color: red; } }")]
	fn parses_every_selector_shape(#[case] source: &str) {
		// Arrange and Act
		let tree = parse(source);

		// Assert
		assert!(
			!tree.root_node().has_error(),
			"parse tree: {}",
			tree.root_node().to_sexp()
		);
	}

	#[rstest]
	#[case("1rem")]
	#[case("15%")]
	#[case("#ff00aa")]
	#[case("globals.surface_secondary")]
	#[case("100% - vars.gutter * 2")]
	#[case("Color::rgb(20%, 30%, 40%)")]
	#[case("vars.accent.mix(white, 15%)")]
	#[case("(1px, solid, globals.border)")]
	#[case("[stop(red, 0%), stop(black, 100%)]")]
	#[case("unchecked_fn!(paint(my_worklet))")]
	fn parses_every_value_shape(#[case] value: &str) {
		// Arrange
		let source = format!(".card {{ color: {value}; }}");

		// Act
		let tree = parse(&source);

		// Assert
		assert!(
			!tree.root_node().has_error(),
			"parse tree: {}",
			tree.root_node().to_sexp()
		);
	}
}
