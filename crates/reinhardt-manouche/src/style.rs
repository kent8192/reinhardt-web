//! Checked style diagnostics and normative registries.

pub mod compiler;
pub(crate) mod css_ir;
pub mod diagnostic;
pub mod registry;
pub mod scope;
pub mod serializer;

pub use compiler::{CompiledStyle, compile_style};
pub use css_ir::CssStylesheet;
pub use diagnostic::{StyleDiagnostic, StyleDiagnosticKind, StyleRelatedLabel};
pub use registry::{
	ArgumentConstraints, ArityPolicy, FunctionResult, FunctionSpec, GrammarMember,
	LoweringStrategy, PropertyFamily, PropertySpec, ReservedFunction, UnitCategory, UnitSpec,
	ValueGrammar, function_specs, property_specs, registry_reference_text, unit_specs,
};
pub use scope::{ScopedClass, ScopedVariable, StyleCompileContext, StyleScope};
pub use serializer::serialize_css;

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::{function_specs, property_specs, registry_reference_text, unit_specs};
	use crate::{
		Direction, KeywordDomain, NumericConstraint, NumericDimension, SemanticType,
		TypeConstraint, function_specs as root_function_specs,
		property_specs as root_property_specs,
		registry_reference_text as root_registry_reference_text, unit_specs as root_unit_specs,
	};

	#[rstest]
	fn style_registry_surfaces_are_consistent_at_module_and_crate_root() {
		// Arrange and Act
		let module_reference = registry_reference_text();
		let root_reference = root_registry_reference_text();

		// Assert
		assert_eq!(property_specs(), root_property_specs());
		assert_eq!(unit_specs(), root_unit_specs());
		assert_eq!(function_specs(), root_function_specs());
		assert_eq!(module_reference, root_reference);
	}

	#[rstest]
	fn semantic_types_have_an_explicit_crate_root_surface() {
		// Arrange
		let domain = KeywordDomain {
			name: "test",
			keywords: &["value"],
			produced_type: SemanticType::Keyword,
		};

		// Act
		let actual = (
			Direction::Right.as_css(),
			domain.produced_type,
			NumericDimension::Length,
			NumericConstraint::Joined,
			TypeConstraint::Exact(SemanticType::Color),
		);

		// Assert
		assert_eq!(
			actual,
			(
				"to right",
				SemanticType::Keyword,
				NumericDimension::Length,
				NumericConstraint::Joined,
				TypeConstraint::Exact(SemanticType::Color),
			)
		);
	}
}
