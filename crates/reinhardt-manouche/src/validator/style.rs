//! Semantic validation for component-scoped styles.

mod dependency;
mod expression;
mod selector;

use crate::{StyleDiagnostic, StyleMacro, TypedStyleMacro};

/// Validates style namespaces and selectors into an owned compiler intermediate.
pub fn validate_style(ast: &StyleMacro) -> Result<TypedStyleMacro, StyleDiagnostic> {
	let bindings = dependency::validate_bindings(ast)?;
	let classes = selector::validate_selectors(ast)?;
	expression::validate_expressions(ast, bindings, classes)
}
