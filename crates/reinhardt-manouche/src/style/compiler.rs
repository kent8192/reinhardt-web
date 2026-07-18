//! Public end-to-end component-style compiler facade.

use proc_macro2::TokenStream;

use super::{
	CssStylesheet, ScopedClass, ScopedVariable, StyleCompileContext, StyleDiagnostic, StyleScope,
	css_ir::lower_style,
};
use crate::{TypedStyleGlobal, parser::parse_style, validator::validate_style};

/// One fully compiled component style consumed by macros and source extraction.
#[derive(Debug, Clone)]
pub struct CompiledStyle {
	/// Deterministic definition scope.
	pub scope: StyleScope,
	/// Generated scoped classes in first-occurrence order.
	pub classes: Vec<ScopedClass>,
	/// Typed global bindings in authored order.
	pub globals: Vec<TypedStyleGlobal>,
	/// Generated scoped component variables in authored order.
	pub variables: Vec<ScopedVariable>,
	/// Structured CSS ready for deterministic serialization.
	pub css: CssStylesheet,
}

/// Parses, validates, scopes, and lowers one complete `style!` body.
pub fn compile_style(
	input: TokenStream,
	context: &StyleCompileContext<'_>,
) -> Result<CompiledStyle, StyleDiagnostic> {
	let ast = parse_style(input).map_err(StyleDiagnostic::from_syn_error)?;
	let typed = validate_style(&ast)?;
	let lowered = lower_style(&typed, context)?;
	Ok(CompiledStyle {
		scope: lowered.scope,
		classes: lowered.classes,
		globals: lowered.globals,
		variables: lowered.variables,
		css: lowered.css,
	})
}

#[cfg(test)]
mod tests {
	use quote::quote;
	use rstest::rstest;

	use super::compile_style;
	use crate::{StyleCompileContext, StyleDiagnosticKind};

	#[rstest]
	fn syntax_errors_enter_the_stable_style_diagnostic_boundary() {
		// Arrange
		let input = quote! { .card { color: ; } };
		let context = StyleCompileContext {
			package_name: "poll-app",
			package_version: "0.4.0",
			style_type_name: "PollCardStyles",
		};

		// Act
		let diagnostic = compile_style(input, &context).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::Syntax {
				message: "expected a style value expression".into(),
			}
		);
	}
}
