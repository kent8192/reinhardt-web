//! tree-sitter grammar for the Reinhardt `form!` DSL.

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
}
