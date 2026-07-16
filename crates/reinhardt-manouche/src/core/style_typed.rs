//! Semantic type vocabulary for checked component styles.

use proc_macro2::Span;

use crate::FunctionSpec;

use super::{
	CssName, StyleBinaryOperator, StyleGlobalDeclaration, StyleMediaCondition, StyleSelectorList,
	StyleUnaryOperator, StyleUncheckedFunction, StyleValueLiteral, StyleVariableDeclaration,
};

/// A validated style definition retaining authored order for later compiler stages.
#[derive(Debug, Clone)]
pub struct TypedStyleMacro {
	/// Validated global bindings in source order.
	pub globals: Vec<TypedStyleGlobal>,
	/// Validated component variables in source order.
	pub variables: Vec<TypedStyleVariable>,
	/// Type-checked rules and media rules in authored source order.
	pub items: Vec<TypedStyleItem>,
	/// Unique local classes in first-occurrence order.
	pub classes: Vec<TypedStyleClass>,
	/// Variable source indices in deterministic dependency-first evaluation order.
	pub variable_evaluation_order: Vec<usize>,
	/// Span of the complete authored style definition.
	pub span: Span,
}

/// One validated global custom-property binding.
#[derive(Debug, Clone)]
pub struct TypedStyleGlobal {
	/// Authored declaration retained for diagnostics and lowering metadata.
	pub declaration: StyleGlobalDeclaration,
	/// Resolved semantic type of this global binding.
	pub value_type: SemanticType,
	/// Deterministic CSS custom-property suffix with underscores converted to hyphens.
	pub css_name: String,
	/// Zero-based position in the authored `globals` block.
	pub source_index: usize,
}

/// One validated component variable and its dependency metadata.
#[derive(Debug, Clone)]
pub struct TypedStyleVariable {
	/// Authored declaration retained for diagnostics and generated API metadata.
	pub declaration: StyleVariableDeclaration,
	/// Resolved closed runtime wrapper category.
	pub runtime_type: StyleRuntimeType,
	/// Resolved semantic contract of the variable.
	pub value_type: SemanticType,
	/// Type-checked default expression.
	pub default: TypedValueExpr,
	/// Deterministic CSS custom-property suffix with underscores converted to hyphens.
	pub css_name: String,
	/// Zero-based position in the authored `vars` block.
	pub source_index: usize,
	/// Referenced component-variable source indices without duplicates.
	pub dependency_indices: Vec<usize>,
	/// Zero-based position in dependency-first evaluation order.
	pub evaluation_index: usize,
}

/// Closed mapping from style DSL variable types to generated runtime wrappers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleRuntimeType {
	/// `Color` mapped to `CssColor`.
	Color,
	/// `Length` mapped to `CssLength`.
	Length,
	/// `LengthPercentage` mapped to `CssLengthPercentage`.
	LengthPercentage,
	/// `Percentage` mapped to `CssPercentage`.
	Percentage,
	/// `Angle` mapped to `CssAngle`.
	Angle,
	/// `Time` mapped to `CssTime`.
	Time,
	/// `Number` mapped to `CssNumber`.
	Number,
	/// `Integer` mapped to `CssInteger`.
	Integer,
}

impl StyleRuntimeType {
	/// Resolves one exact DSL type name into its closed runtime category.
	pub fn from_dsl_name(name: &str) -> Option<Self> {
		match name {
			"Color" => Some(Self::Color),
			"Length" => Some(Self::Length),
			"LengthPercentage" => Some(Self::LengthPercentage),
			"Percentage" => Some(Self::Percentage),
			"Angle" => Some(Self::Angle),
			"Time" => Some(Self::Time),
			"Number" => Some(Self::Number),
			"Integer" => Some(Self::Integer),
			_ => None,
		}
	}

	/// Returns the semantic expression type enforced by this runtime category.
	pub const fn semantic_type(self) -> SemanticType {
		match self {
			Self::Color => SemanticType::Color,
			Self::Length => SemanticType::Length,
			Self::LengthPercentage => SemanticType::LengthPercentage,
			Self::Percentage => SemanticType::Percentage,
			Self::Angle => SemanticType::Angle,
			Self::Time => SemanticType::Time,
			Self::Number => SemanticType::Number,
			Self::Integer => SemanticType::Integer,
		}
	}

	/// Returns the generated Rust wrapper type name.
	pub const fn wrapper_name(self) -> &'static str {
		match self {
			Self::Color => "CssColor",
			Self::Length => "CssLength",
			Self::LengthPercentage => "CssLengthPercentage",
			Self::Percentage => "CssPercentage",
			Self::Angle => "CssAngle",
			Self::Time => "CssTime",
			Self::Number => "CssNumber",
			Self::Integer => "CssInteger",
		}
	}
}

/// One type-checked top-level style item.
#[derive(Debug, Clone)]
pub enum TypedStyleItem {
	/// A type-checked style rule.
	Rule(TypedStyleRule),
	/// A type-checked media grouping rule.
	Media(TypedStyleMediaRule),
}

/// One type-checked item within a rule or grouping rule.
#[derive(Debug, Clone)]
pub enum TypedStyleRuleItem {
	/// A registered property declaration with a compatible typed value.
	Declaration(TypedStyleDeclaration),
	/// A nested style rule.
	Rule(TypedStyleRule),
	/// A nested media grouping rule.
	Media(TypedStyleMediaRule),
}

/// A type-checked property declaration.
#[derive(Debug, Clone)]
pub struct TypedStyleDeclaration {
	/// Registered canonical CSS property name.
	pub name: CssName,
	/// Type-checked value expression.
	pub value: TypedValueExpr,
	/// Span of the complete authored declaration.
	pub span: Span,
}

/// A style rule whose declarations have completed semantic validation.
#[derive(Debug, Clone)]
pub struct TypedStyleRule {
	/// Structurally validated selector list.
	pub selectors: StyleSelectorList,
	/// Type-checked declarations and nested rules in source order.
	pub items: Vec<TypedStyleRuleItem>,
	/// Span of the authored rule head.
	pub span: Span,
}

/// A media grouping rule whose nested declarations are type checked.
#[derive(Debug, Clone)]
pub struct TypedStyleMediaRule {
	/// Structurally parsed static media condition.
	pub condition: StyleMediaCondition,
	/// Type-checked nested rule items in source order.
	pub items: Vec<TypedStyleRuleItem>,
	/// Span of the authored at-rule.
	pub span: Span,
}

/// A type-checked value expression ready for structured CSS lowering.
#[derive(Debug, Clone)]
pub struct TypedValueExpr {
	/// Inferred semantic result type.
	pub value_type: SemanticType,
	/// Type-checked structural expression kind.
	pub kind: TypedValueExprKind,
	/// Whether this expression itself requires one outer `calc(...)` boundary.
	pub contains_arithmetic: bool,
	/// Span of the authored expression.
	pub span: Span,
}

impl TypedValueExpr {
	/// Returns whether this expression is an atomic unitless zero.
	pub fn is_contextual_zero(&self) -> bool {
		match &self.kind {
			TypedValueExprKind::Literal(StyleValueLiteral::Integer(number))
			| TypedValueExprKind::Literal(StyleValueLiteral::Number(number)) => number.contextual_zero,
			TypedValueExprKind::Unary { operand, .. } | TypedValueExprKind::Group(operand) => {
				operand.is_contextual_zero()
			}
			_ => false,
		}
	}
}

/// The type-checked structural forms of one style value.
#[derive(Debug, Clone)]
pub enum TypedValueExprKind {
	/// A validated literal retained losslessly.
	Literal(StyleValueLiteral),
	/// A resolved package-global custom-property reference.
	GlobalReference(TypedGlobalReference),
	/// A resolved component-variable reference.
	VariableReference(TypedVariableReference),
	/// A validated direction associated value.
	Direction(Direction),
	/// A signed numeric expression.
	Unary {
		/// Authored sign operator.
		operator: StyleUnaryOperator,
		/// Type-checked signed operand.
		operand: Box<TypedValueExpr>,
	},
	/// A type-checked arithmetic expression.
	Binary {
		/// Type-checked left operand.
		left: Box<TypedValueExpr>,
		/// Authored arithmetic operator.
		operator: StyleBinaryOperator,
		/// Type-checked right operand.
		right: Box<TypedValueExpr>,
	},
	/// A checked registry function, constructor, method, or structural helper.
	Function(TypedFunctionCall),
	/// One explicitly grouped expression.
	Group(Box<TypedValueExpr>),
	/// A typed space-separated sequence.
	SpaceSequence(Vec<TypedValueExpr>),
	/// A typed comma-separated list.
	CommaList(Vec<TypedValueExpr>),
	/// The validated opaque whole-value function escape.
	UncheckedFunction(StyleUncheckedFunction),
}

/// One resolved global custom-property reference.
#[derive(Debug, Clone)]
pub struct TypedGlobalReference {
	/// Authored binding name.
	pub name: String,
	/// Kebab-case external custom-property suffix.
	pub css_name: String,
	/// Source-order index in the global block.
	pub source_index: usize,
}

/// One resolved component-variable reference.
#[derive(Debug, Clone)]
pub struct TypedVariableReference {
	/// Authored binding name.
	pub name: String,
	/// Kebab-case scoped custom-property suffix.
	pub css_name: String,
	/// Source-order index in the variable block.
	pub source_index: usize,
	/// Declared semantic contract of the referenced variable.
	pub value_type: SemanticType,
}

/// One checked call carrying its single-registry lowering metadata.
#[derive(Debug, Clone)]
pub struct TypedFunctionCall {
	/// Immutable registry entry used to validate the call.
	pub spec: FunctionSpec,
	/// Optional checked receiver for a method call.
	pub receiver: Option<Box<TypedValueExpr>>,
	/// Checked arguments in authored order.
	pub arguments: Vec<TypedValueExpr>,
}

/// Metadata for one unique authored local class.
#[derive(Debug, Clone)]
pub struct TypedStyleClass {
	/// CSS class spelling exactly as authored.
	pub authored_name: String,
	/// Generated ordinary Rust method name.
	pub accessor: String,
	/// Span of the authored class name.
	pub span: Span,
}

/// Numeric dimensions tracked by style expression validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericDimension {
	/// A unitless finite scalar.
	Number,
	/// A unitless integer scalar.
	Integer,
	/// A CSS length.
	Length,
	/// A CSS length or percentage.
	LengthPercentage,
	/// A CSS percentage.
	Percentage,
	/// A CSS angle.
	Angle,
	/// A CSS duration.
	Time,
	/// A CSS grid fraction.
	GridFraction,
}

/// Semantic value categories produced by checked style expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SemanticType {
	/// A CSS color.
	Color,
	/// A CSS length without a percentage.
	Length,
	/// A CSS length or percentage.
	LengthPercentage,
	/// A CSS percentage.
	Percentage,
	/// A CSS angle.
	Angle,
	/// A CSS duration.
	Time,
	/// A finite unitless scalar.
	Number,
	/// A unitless integer.
	Integer,
	/// A CSS grid fraction.
	GridFraction,
	/// A quoted CSS string.
	QuotedString,
	/// A validated CSS custom identifier.
	CustomIdentifier,
	/// A member of a registered keyword domain.
	Keyword,
	/// A linear-gradient direction.
	Direction,
	/// A typed gradient stop.
	GradientStop,
	/// A typed CSS image.
	Image,
	/// One typed transform function.
	TransformFunction,
	/// A typed space-separated sequence.
	SpaceSequence,
	/// A typed comma-separated list.
	CommaList,
	/// A typed slash-separated pair.
	SlashPair,
	/// A deliberately opaque whole value.
	Unchecked,
}

impl SemanticType {
	/// Every semantic category in stable declaration order.
	pub const ALL: &'static [Self] = &[
		Self::Color,
		Self::Length,
		Self::LengthPercentage,
		Self::Percentage,
		Self::Angle,
		Self::Time,
		Self::Number,
		Self::Integer,
		Self::GridFraction,
		Self::QuotedString,
		Self::CustomIdentifier,
		Self::Keyword,
		Self::Direction,
		Self::GradientStop,
		Self::Image,
		Self::TransformFunction,
		Self::SpaceSequence,
		Self::CommaList,
		Self::SlashPair,
		Self::Unchecked,
	];

	/// Returns the numeric dimension when this category participates in arithmetic.
	pub const fn numeric_dimension(self) -> Option<NumericDimension> {
		match self {
			Self::Length => Some(NumericDimension::Length),
			Self::LengthPercentage => Some(NumericDimension::LengthPercentage),
			Self::Percentage => Some(NumericDimension::Percentage),
			Self::Angle => Some(NumericDimension::Angle),
			Self::Time => Some(NumericDimension::Time),
			Self::Number => Some(NumericDimension::Number),
			Self::Integer => Some(NumericDimension::Integer),
			Self::GridFraction => Some(NumericDimension::GridFraction),
			Self::Color
			| Self::QuotedString
			| Self::CustomIdentifier
			| Self::Keyword
			| Self::Direction
			| Self::GradientStop
			| Self::Image
			| Self::TransformFunction
			| Self::SpaceSequence
			| Self::CommaList
			| Self::SlashPair
			| Self::Unchecked => None,
		}
	}
}

/// A named, immutable set of accepted CSS keywords.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeywordDomain {
	/// A stable descriptive name used by references and diagnostics.
	pub name: &'static str,
	/// The complete accepted keyword set.
	pub keywords: &'static [&'static str],
	/// The semantic type produced by a member of this domain.
	pub produced_type: SemanticType,
}

/// Cross-argument numeric constraints used by registered functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericConstraint {
	/// A channel accepts either a number or a percentage.
	NumberOrPercentage,
	/// A percentage must fall within an inclusive range when its value is known.
	PercentageRange {
		/// The inclusive lower bound.
		minimum: i16,
		/// The inclusive upper bound.
		maximum: i16,
	},
	/// All participating numeric values must join through the numeric type lattice.
	Joined,
}

/// A structural argument or receiver constraint in the function registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeConstraint {
	/// The value must have one exact semantic type.
	Exact(SemanticType),
	/// The value participates in a numeric constraint.
	Numeric(NumericConstraint),
	/// The value must be a comma list with a typed element and minimum length.
	CommaList {
		/// The required element type.
		element: SemanticType,
		/// The minimum accepted number of elements.
		min: usize,
	},
	/// Any checked type is accepted and retained by generic lowering.
	Any,
}

/// Directions accepted by `linear_gradient`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
	/// Toward the top edge.
	Top,
	/// Toward the top-right corner.
	TopRight,
	/// Toward the right edge.
	Right,
	/// Toward the bottom-right corner.
	BottomRight,
	/// Toward the bottom edge.
	Bottom,
	/// Toward the bottom-left corner.
	BottomLeft,
	/// Toward the left edge.
	Left,
	/// Toward the top-left corner.
	TopLeft,
}

impl Direction {
	/// Returns the canonical CSS direction phrase.
	pub const fn as_css(self) -> &'static str {
		match self {
			Self::Top => "to top",
			Self::TopRight => "to top right",
			Self::Right => "to right",
			Self::BottomRight => "to bottom right",
			Self::Bottom => "to bottom",
			Self::BottomLeft => "to bottom left",
			Self::Left => "to left",
			Self::TopLeft => "to top left",
		}
	}
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::{Direction, NumericDimension, SemanticType, StyleRuntimeType};

	#[rstest]
	fn semantic_type_vocabulary_is_explicit_and_complete() {
		// Arrange
		let expected = [
			SemanticType::Color,
			SemanticType::Length,
			SemanticType::LengthPercentage,
			SemanticType::Percentage,
			SemanticType::Angle,
			SemanticType::Time,
			SemanticType::Number,
			SemanticType::Integer,
			SemanticType::GridFraction,
			SemanticType::QuotedString,
			SemanticType::CustomIdentifier,
			SemanticType::Keyword,
			SemanticType::Direction,
			SemanticType::GradientStop,
			SemanticType::Image,
			SemanticType::TransformFunction,
			SemanticType::SpaceSequence,
			SemanticType::CommaList,
			SemanticType::SlashPair,
			SemanticType::Unchecked,
		];

		// Act
		let actual = SemanticType::ALL;

		// Assert
		assert_eq!(actual, &expected);
	}

	#[rstest]
	#[case(SemanticType::Number, Some(NumericDimension::Number))]
	#[case(SemanticType::Integer, Some(NumericDimension::Integer))]
	#[case(SemanticType::Length, Some(NumericDimension::Length))]
	#[case(
		SemanticType::LengthPercentage,
		Some(NumericDimension::LengthPercentage)
	)]
	#[case(SemanticType::Percentage, Some(NumericDimension::Percentage))]
	#[case(SemanticType::Angle, Some(NumericDimension::Angle))]
	#[case(SemanticType::Time, Some(NumericDimension::Time))]
	#[case(SemanticType::GridFraction, Some(NumericDimension::GridFraction))]
	#[case(SemanticType::Color, None)]
	#[case(SemanticType::Unchecked, None)]
	fn semantic_type_reports_numeric_dimension(
		#[case] semantic_type: SemanticType,
		#[case] expected: Option<NumericDimension>,
	) {
		// Arrange and Act
		let actual = semantic_type.numeric_dimension();

		// Assert
		assert_eq!(actual, expected);
	}

	#[rstest]
	#[case(Direction::Top, "to top")]
	#[case(Direction::TopRight, "to top right")]
	#[case(Direction::Right, "to right")]
	#[case(Direction::BottomRight, "to bottom right")]
	#[case(Direction::Bottom, "to bottom")]
	#[case(Direction::BottomLeft, "to bottom left")]
	#[case(Direction::Left, "to left")]
	#[case(Direction::TopLeft, "to top left")]
	fn direction_has_exact_css_lowering(#[case] direction: Direction, #[case] expected: &str) {
		// Arrange and Act
		let actual = direction.as_css();

		// Assert
		assert_eq!(actual, expected);
	}

	#[rstest]
	#[case("Color", StyleRuntimeType::Color, SemanticType::Color, "CssColor")]
	#[case("Length", StyleRuntimeType::Length, SemanticType::Length, "CssLength")]
	#[case(
		"LengthPercentage",
		StyleRuntimeType::LengthPercentage,
		SemanticType::LengthPercentage,
		"CssLengthPercentage"
	)]
	#[case(
		"Percentage",
		StyleRuntimeType::Percentage,
		SemanticType::Percentage,
		"CssPercentage"
	)]
	#[case("Angle", StyleRuntimeType::Angle, SemanticType::Angle, "CssAngle")]
	#[case("Time", StyleRuntimeType::Time, SemanticType::Time, "CssTime")]
	#[case("Number", StyleRuntimeType::Number, SemanticType::Number, "CssNumber")]
	#[case(
		"Integer",
		StyleRuntimeType::Integer,
		SemanticType::Integer,
		"CssInteger"
	)]
	fn runtime_type_mapping_is_closed_and_exact(
		#[case] dsl_name: &str,
		#[case] expected_runtime: StyleRuntimeType,
		#[case] expected_semantic: SemanticType,
		#[case] expected_wrapper: &str,
	) {
		// Arrange and Act
		let runtime_type = StyleRuntimeType::from_dsl_name(dsl_name).unwrap();

		// Assert
		assert_eq!(runtime_type, expected_runtime);
		assert_eq!(runtime_type.semantic_type(), expected_semantic);
		assert_eq!(runtime_type.wrapper_name(), expected_wrapper);
	}

	#[rstest]
	fn runtime_type_mapping_rejects_types_outside_the_mvp() {
		// Arrange and Act
		let runtime_type = StyleRuntimeType::from_dsl_name("GridFraction");

		// Assert
		assert_eq!(runtime_type, None);
	}
}
