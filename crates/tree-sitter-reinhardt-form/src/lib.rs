//! tree-sitter grammar for the Reinhardt `form!` DSL.

#![warn(missing_docs)]

use tree_sitter_language::LanguageFn;

unsafe extern "C" {
	fn tree_sitter_reinhardt_form() -> *const ();
}

/// The tree-sitter language function for the Reinhardt `form!` DSL grammar.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_reinhardt_form) };

/// The grammar node type metadata.
pub const NODE_TYPES: &str = include_str!("node-types.json");

#[cfg(test)]
mod tests {
	#[test]
	fn parser_loads() {
		let mut parser = tree_sitter::Parser::new();
		parser
			.set_language(&super::LANGUAGE.into())
			.expect("load form DSL grammar");
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
		let source = "// comment\nname: \"User\"";

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
	fn form_dsl_basic_structure() {
		// Arrange
		let mut parser = tree_sitter::Parser::new();
		parser.set_language(&super::LANGUAGE.into()).unwrap();
		let source = r#"{ name: "User", fields { email { label: "Email" } } }"#;

		// Act
		let tree = parser.parse(source, None).unwrap();

		// Assert
		assert!(
			!tree.root_node().has_error(),
			"parse tree: {}",
			tree.root_node().to_sexp()
		);
	}

	// -- DSL with embedded comments -------------------------------------------

	#[test]
	fn form_dsl_with_block_comment() {
		// Arrange
		let mut parser = tree_sitter::Parser::new();
		parser.set_language(&super::LANGUAGE.into()).unwrap();
		let source = r#"{ /* comment */ name: "User" }"#;

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
