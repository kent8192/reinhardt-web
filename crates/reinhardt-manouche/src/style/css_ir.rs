//! Structured CSS intermediate representation and style lowering.

use proc_macro2::TokenTree;

use super::{
	ScopedClass, ScopedVariable, StyleCompileContext, StyleDiagnostic, StyleDiagnosticKind,
	StyleScope,
};
use crate::{
	Direction, LoweringStrategy, StyleAttributeMatcher, StyleAttributeValue,
	StyleBinaryOperatorKind, StyleMediaCondition, StyleNumericUnit, StylePseudoSelector,
	StyleSelector, StyleSelectorCombinator, StyleSelectorKind, StyleSelectorList,
	StyleSimpleSelector, StyleUnaryOperatorKind, StyleUncheckedFunction, StyleValueLiteral,
	TypedFunctionCall, TypedStyleDeclaration, TypedStyleGlobal, TypedStyleItem, TypedStyleMacro,
	TypedStyleMediaRule, TypedStyleRule, TypedStyleRuleItem, TypedValueExpr, TypedValueExprKind,
};

/// One crate-internal fully scoped style handoff for the public compiler facade.
#[derive(Debug, Clone)]
pub(crate) struct LoweredStyle {
	/// Deterministic definition scope.
	pub(crate) scope: StyleScope,
	/// Generated class metadata in first-occurrence order.
	pub(crate) classes: Vec<ScopedClass>,
	/// Typed global metadata in authored order.
	pub(crate) globals: Vec<TypedStyleGlobal>,
	/// Generated variable metadata in authored order.
	pub(crate) variables: Vec<ScopedVariable>,
	/// Flat structured CSS ready for deterministic serialization.
	pub(crate) css: CssStylesheet,
}

/// A complete structured CSS stylesheet.
#[derive(Debug, Clone, Default)]
pub struct CssStylesheet {
	/// Style and grouping rules in final cascade order.
	pub(crate) rules: Vec<CssRule>,
	/// Type-checked component-variable defaults addressed by source index.
	pub(crate) variable_defaults: Vec<CssValue>,
}

/// One top-level or grouped CSS rule.
#[derive(Debug, Clone)]
pub(crate) enum CssRule {
	/// A flat selector rule.
	Style(CssStyleRule),
	/// A media grouping rule.
	Group(CssGroupingRule),
}

/// A structured media grouping rule.
#[derive(Debug, Clone)]
pub(crate) struct CssGroupingRule {
	/// Statically parsed media condition.
	pub(crate) condition: StyleMediaCondition,
	/// Nested flat style and grouping rules in cascade order.
	pub(crate) rules: Vec<CssRule>,
}

/// One flat CSS style rule.
#[derive(Debug, Clone)]
pub(crate) struct CssStyleRule {
	/// Fully expanded selector list.
	pub(crate) selectors: Vec<CssSelector>,
	/// One contiguous authored declaration run.
	pub(crate) declarations: Vec<CssDeclaration>,
}

/// One structured, fully scoped CSS selector.
#[derive(Debug, Clone)]
pub(crate) struct CssSelector {
	/// Compound-selector segments and their preceding combinators.
	pub(crate) segments: Vec<CssSelectorSegment>,
}

/// One compound selector and its relationship to the preceding segment.
#[derive(Debug, Clone)]
pub(crate) struct CssSelectorSegment {
	/// Relationship to the preceding segment, or a leading relative combinator.
	pub(crate) combinator: Option<StyleSelectorCombinator>,
	/// Same-element simple selectors in authored order.
	pub(crate) simple_selectors: Vec<CssSimpleSelector>,
}

/// One simple selector leaf in structured CSS IR.
#[derive(Debug, Clone)]
pub(crate) enum CssSimpleSelector {
	/// A fully scoped local class.
	Class(String),
	/// An unscoped type selector.
	Type(String),
	/// An unscoped ID selector.
	Id(String),
	/// The universal selector.
	Universal,
	/// A structured attribute selector.
	Attribute(CssAttributeSelector),
	/// A structured pseudo-class, pseudo-element, or pseudo-function.
	Pseudo(CssPseudoSelector),
}

/// One structured CSS attribute selector.
#[derive(Debug, Clone)]
pub(crate) struct CssAttributeSelector {
	/// Attribute name.
	pub(crate) name: String,
	/// Optional matcher operator.
	pub(crate) matcher: Option<StyleAttributeMatcher>,
	/// Optional matcher value.
	pub(crate) value: Option<CssAttributeValue>,
	/// Optional ASCII matching modifier.
	pub(crate) modifier: Option<String>,
}

/// One structured attribute-selector value.
#[derive(Debug, Clone)]
pub(crate) enum CssAttributeValue {
	/// An identifier value.
	Identifier(String),
	/// A decoded quoted string value.
	String(String),
}

/// One structured pseudo selector.
#[derive(Debug, Clone)]
pub(crate) struct CssPseudoSelector {
	/// Pseudo selector name without the leading colon.
	pub(crate) name: String,
	/// Whether the selector uses the pseudo-element `::` prefix.
	pub(crate) is_element: bool,
	/// Optional structured arguments.
	pub(crate) arguments: Option<CssPseudoArguments>,
}

/// Structured pseudo-function argument forms.
#[derive(Debug, Clone)]
pub(crate) enum CssPseudoArguments {
	/// A selector-list pseudo-function such as `:is` or `:has`.
	SelectorList(Vec<CssSelector>),
	/// An An+B formula with an optional structured `of` selector list.
	Nth {
		/// Losslessly retained validated formula tokens.
		formula_tokens: Vec<TokenTree>,
		/// Optional scoped selector list after `of`.
		selectors: Option<Vec<CssSelector>>,
	},
	/// Validated static tokens for a non-selector pseudo-function.
	RawTokens(Vec<TokenTree>),
}

/// One registered CSS declaration and its typed value.
#[derive(Debug, Clone)]
pub(crate) struct CssDeclaration {
	/// Canonical registered property name.
	pub(crate) property: String,
	/// Structured typed declaration value.
	pub(crate) value: CssValue,
}

/// One typed CSS value node.
#[derive(Debug, Clone)]
pub(crate) struct CssValue {
	/// Structured CSS value form.
	pub(crate) kind: CssValueKind,
}

/// Structured forms accepted in CSS value IR.
#[derive(Debug, Clone)]
pub(crate) enum CssValueKind {
	/// A normalized literal leaf.
	Literal(CssLiteral),
	/// An external custom-property reference.
	GlobalVariable {
		/// Complete CSS custom-property name.
		custom_property: String,
	},
	/// A scoped component variable and its recursively compiled fallback.
	ComponentVariable {
		/// Complete scoped CSS custom-property name.
		custom_property: String,
		/// Source index of the compiled fallback in the stylesheet arena.
		fallback_index: usize,
	},
	/// A checked gradient direction.
	Direction(Direction),
	/// A signed numeric value.
	Unary {
		/// Authored unary operator.
		operator: StyleUnaryOperatorKind,
		/// Structured operand.
		operand: Box<CssValue>,
	},
	/// A checked arithmetic operation.
	Binary {
		/// Structured left operand.
		left: Box<CssValue>,
		/// Authored arithmetic operator.
		operator: StyleBinaryOperatorKind,
		/// Structured right operand.
		right: Box<CssValue>,
	},
	/// A checked ordinary CSS function.
	Function(CssFunction),
	/// The fixed structured sRGB color-mix lowering.
	ColorMix {
		/// Color expression receiving `.mix`.
		receiver: Box<CssValue>,
		/// Other color receiving the authored weight.
		other: Box<CssValue>,
		/// Authored percentage weighting `other`.
		amount: Box<CssValue>,
	},
	/// Explicit arithmetic grouping retained inside a calculation.
	Group(Box<CssValue>),
	/// Space-separated value sequence.
	SpaceSequence(Vec<CssValue>),
	/// Comma-separated value list.
	CommaList(Vec<CssValue>),
	/// Slash-separated value pair.
	SlashPair {
		/// Value before the slash.
		left: Box<CssValue>,
		/// Value after the slash.
		right: Box<CssValue>,
	},
	/// The validated explicit opaque function escape, retained as token trees.
	UncheckedFunction(StyleUncheckedFunction),
	/// One explicit outer CSS `calc(...)` boundary.
	Calc(Box<CssValue>),
}

/// One normalized CSS literal.
#[derive(Debug, Clone)]
pub(crate) enum CssLiteral {
	/// An integer or number lexeme with an optional CSS unit.
	Number {
		/// Authored numeric lexeme without a unit.
		source: String,
		/// Canonical CSS unit suffix.
		unit: Option<String>,
	},
	/// An authored hexadecimal color spelling.
	HexColor(String),
	/// A checked keyword or custom identifier.
	Keyword(String),
	/// A decoded quoted string.
	String(String),
}

/// One ordinary checked CSS function call.
#[derive(Debug, Clone)]
pub(crate) struct CssFunction {
	/// Canonical CSS function spelling.
	pub(crate) name: String,
	/// Closed separator used between serialized arguments.
	pub(crate) separator: CssFunctionSeparator,
	/// Structured arguments in authored order.
	pub(crate) arguments: Vec<CssValue>,
}

/// Argument separator for an ordinary checked CSS function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CssFunctionSeparator {
	/// Arguments are separated by `, `.
	Comma,
	/// Arguments are separated by one CSS space.
	Space,
}

/// Lowers one validated style definition into scoped metadata and structured CSS IR.
pub(crate) fn lower_style(
	style: &TypedStyleMacro,
	context: &StyleCompileContext<'_>,
) -> Result<LoweredStyle, StyleDiagnostic> {
	let scope = StyleScope::new(context);
	let classes = style
		.classes
		.iter()
		.map(|class| ScopedClass {
			authored_name: class.authored_name.clone(),
			accessor: class.accessor.clone(),
			css_name: scope.class_name(&class.authored_name),
			span: class.span,
		})
		.collect();
	let variables = style
		.variables
		.iter()
		.map(|variable| ScopedVariable {
			authored_name: variable.declaration.name.value.clone(),
			custom_property_name: scope.variable_name(&variable.css_name),
			runtime_type: variable.runtime_type,
			source_index: variable.source_index,
			default: variable.default.clone(),
			span: variable.declaration.span,
		})
		.collect();
	let variable_defaults = lower_variable_defaults(style, &scope)?;
	let variable_count = variable_defaults.len();
	let mut rules = Vec::new();
	for item in &style.items {
		match item {
			TypedStyleItem::Rule(rule) => {
				rules.extend(lower_rule(rule, None, &scope, variable_count)?);
			}
			TypedStyleItem::Media(media) => {
				if let Some(group) = lower_media(media, None, &scope, variable_count)? {
					rules.push(CssRule::Group(group));
				}
			}
		}
	}
	Ok(LoweredStyle {
		scope,
		classes,
		globals: style.globals.clone(),
		variables,
		css: CssStylesheet {
			rules,
			variable_defaults,
		},
	})
}

fn lower_variable_defaults(
	style: &TypedStyleMacro,
	scope: &StyleScope,
) -> Result<Vec<CssValue>, StyleDiagnostic> {
	style
		.variables
		.iter()
		.map(|variable| lower_value(&variable.default, style.variables.len(), scope, true))
		.collect()
}

fn lower_rule(
	rule: &TypedStyleRule,
	parent_selectors: Option<&[CssSelector]>,
	scope: &StyleScope,
	variable_count: usize,
) -> Result<Vec<CssRule>, StyleDiagnostic> {
	let selectors = lower_selector_list(parent_selectors, &rule.selectors, scope);
	lower_rule_items(&rule.items, Some(&selectors), scope, variable_count)
}

fn lower_media(
	media: &TypedStyleMediaRule,
	parent_selectors: Option<&[CssSelector]>,
	scope: &StyleScope,
	variable_count: usize,
) -> Result<Option<CssGroupingRule>, StyleDiagnostic> {
	let rules = lower_rule_items(&media.items, parent_selectors, scope, variable_count)?;
	Ok((!rules.is_empty()).then(|| CssGroupingRule {
		condition: media.condition.clone(),
		rules,
	}))
}

fn lower_rule_items(
	items: &[TypedStyleRuleItem],
	selectors: Option<&[CssSelector]>,
	scope: &StyleScope,
	variable_count: usize,
) -> Result<Vec<CssRule>, StyleDiagnostic> {
	let mut rules = Vec::new();
	let mut declarations = Vec::new();
	for item in items {
		match item {
			TypedStyleRuleItem::Declaration(declaration) => {
				declarations.push(lower_declaration(declaration, variable_count, scope)?);
			}
			TypedStyleRuleItem::Rule(rule) => {
				flush_declarations(&mut rules, selectors, &mut declarations);
				rules.extend(lower_rule(rule, selectors, scope, variable_count)?);
			}
			TypedStyleRuleItem::Media(media) => {
				flush_declarations(&mut rules, selectors, &mut declarations);
				if let Some(group) = lower_media(media, selectors, scope, variable_count)? {
					rules.push(CssRule::Group(group));
				}
			}
		}
	}
	flush_declarations(&mut rules, selectors, &mut declarations);
	Ok(rules)
}

fn flush_declarations(
	rules: &mut Vec<CssRule>,
	selectors: Option<&[CssSelector]>,
	declarations: &mut Vec<CssDeclaration>,
) {
	if declarations.is_empty() {
		return;
	}
	let Some(selectors) = selectors else {
		declarations.clear();
		return;
	};
	rules.push(CssRule::Style(CssStyleRule {
		selectors: selectors.to_vec(),
		declarations: std::mem::take(declarations),
	}));
}

fn lower_declaration(
	declaration: &TypedStyleDeclaration,
	variable_count: usize,
	scope: &StyleScope,
) -> Result<CssDeclaration, StyleDiagnostic> {
	Ok(CssDeclaration {
		property: declaration.name.value.clone(),
		value: lower_value(&declaration.value, variable_count, scope, true)?,
	})
}

fn lower_selector_list(
	parents: Option<&[CssSelector]>,
	list: &StyleSelectorList,
	scope: &StyleScope,
) -> Vec<CssSelector> {
	match parents {
		Some(parents) => parents
			.iter()
			.flat_map(|parent| {
				list.selectors
					.iter()
					.map(|branch| extend_selector(parent, branch, scope))
			})
			.collect(),
		None => list
			.selectors
			.iter()
			.map(|branch| selector_branch(branch, scope, false))
			.collect(),
	}
}

fn extend_selector(
	parent: &CssSelector,
	branch: &StyleSelector,
	scope: &StyleScope,
) -> CssSelector {
	let mut selector = parent.clone();
	match &branch.kind {
		StyleSelectorKind::SameElement(simple) => {
			let lowered = lower_simple_selector(simple, scope);
			if let Some(segment) = selector.segments.last_mut() {
				segment.simple_selectors.push(lowered);
			} else {
				selector.segments.push(CssSelectorSegment {
					combinator: None,
					simple_selectors: vec![lowered],
				});
			}
		}
		StyleSelectorKind::Relative {
			combinator,
			selector: simple,
		} => selector.segments.push(CssSelectorSegment {
			combinator: Some(*combinator),
			simple_selectors: vec![lower_simple_selector(simple, scope)],
		}),
		StyleSelectorKind::Root(simple) => selector.segments.push(CssSelectorSegment {
			combinator: Some(StyleSelectorCombinator::Descendant),
			simple_selectors: vec![lower_simple_selector(simple, scope)],
		}),
	}
	selector
}

fn selector_branch(branch: &StyleSelector, scope: &StyleScope, relative: bool) -> CssSelector {
	let (combinator, simple) = match &branch.kind {
		StyleSelectorKind::Root(simple) | StyleSelectorKind::SameElement(simple) => (None, simple),
		StyleSelectorKind::Relative {
			combinator,
			selector,
		} => (
			(relative && *combinator != StyleSelectorCombinator::Descendant).then_some(*combinator),
			selector,
		),
	};
	CssSelector {
		segments: vec![CssSelectorSegment {
			combinator,
			simple_selectors: vec![lower_simple_selector(simple, scope)],
		}],
	}
}

fn lower_simple_selector(simple: &StyleSimpleSelector, scope: &StyleScope) -> CssSimpleSelector {
	match simple {
		StyleSimpleSelector::Class(name) => CssSimpleSelector::Class(scope.class_name(&name.value)),
		StyleSimpleSelector::Type(name) => CssSimpleSelector::Type(name.value.clone()),
		StyleSimpleSelector::Id(name) => CssSimpleSelector::Id(name.value.clone()),
		StyleSimpleSelector::Universal { .. } => CssSimpleSelector::Universal,
		StyleSimpleSelector::Attribute(attribute) => {
			CssSimpleSelector::Attribute(CssAttributeSelector {
				name: attribute.name.value.clone(),
				matcher: attribute.matcher,
				value: attribute.value.as_ref().map(|value| match value {
					StyleAttributeValue::Identifier(name) => {
						CssAttributeValue::Identifier(name.value.clone())
					}
					StyleAttributeValue::String { value, .. } => {
						CssAttributeValue::String(value.clone())
					}
				}),
				modifier: attribute.modifier.as_ref().map(|name| name.value.clone()),
			})
		}
		StyleSimpleSelector::Pseudo(pseudo) => {
			CssSimpleSelector::Pseudo(lower_pseudo_selector(pseudo, scope))
		}
	}
}

fn lower_pseudo_selector(pseudo: &StylePseudoSelector, scope: &StyleScope) -> CssPseudoSelector {
	let arguments = pseudo.arguments.as_ref().map(|arguments| {
		if let Some(nth) = &arguments.nth {
			return CssPseudoArguments::Nth {
				formula_tokens: nth.formula_tokens.clone(),
				selectors: arguments.selector_list.as_ref().map(|list| {
					list.selectors
						.iter()
						.map(|branch| selector_branch(branch, scope, true))
						.collect()
				}),
			};
		}
		if let Some(list) = &arguments.selector_list {
			return CssPseudoArguments::SelectorList(
				list.selectors
					.iter()
					.map(|branch| selector_branch(branch, scope, true))
					.collect(),
			);
		}
		CssPseudoArguments::RawTokens(arguments.tokens.clone())
	});
	CssPseudoSelector {
		name: pseudo.name.value.clone(),
		is_element: pseudo.is_element,
		arguments,
	}
}

fn lower_value(
	expression: &TypedValueExpr,
	variable_count: usize,
	scope: &StyleScope,
	calculation_boundary: bool,
) -> Result<CssValue, StyleDiagnostic> {
	let kind = lower_value_kind(expression, variable_count, scope)?;
	let value = CssValue { kind };
	if calculation_boundary && expression.contains_arithmetic {
		Ok(CssValue {
			kind: CssValueKind::Calc(Box::new(value)),
		})
	} else {
		Ok(value)
	}
}

fn lower_value_kind(
	expression: &TypedValueExpr,
	variable_count: usize,
	scope: &StyleScope,
) -> Result<CssValueKind, StyleDiagnostic> {
	match &expression.kind {
		TypedValueExprKind::Literal(literal) => Ok(CssValueKind::Literal(lower_literal(literal))),
		TypedValueExprKind::GlobalReference(reference) => Ok(CssValueKind::GlobalVariable {
			custom_property: format!("--{}", reference.css_name),
		}),
		TypedValueExprKind::VariableReference(reference) => {
			if reference.source_index >= variable_count {
				return Err(StyleDiagnostic::new(
					StyleDiagnosticKind::UndeclaredVariableReference {
						name: reference.name.clone(),
					},
					expression.span,
				));
			}
			Ok(CssValueKind::ComponentVariable {
				custom_property: scope.variable_name(&reference.css_name),
				fallback_index: reference.source_index,
			})
		}
		TypedValueExprKind::Direction(direction) => Ok(CssValueKind::Direction(*direction)),
		TypedValueExprKind::Unary { operator, operand } => Ok(CssValueKind::Unary {
			operator: operator.kind,
			operand: Box::new(lower_value(operand, variable_count, scope, false)?),
		}),
		TypedValueExprKind::Binary {
			left,
			operator,
			right,
		} => Ok(CssValueKind::Binary {
			left: Box::new(lower_value(left, variable_count, scope, false)?),
			operator: operator.kind,
			right: Box::new(lower_value(right, variable_count, scope, false)?),
		}),
		TypedValueExprKind::Function(call) => {
			lower_function(call, expression.span, variable_count, scope)
		}
		TypedValueExprKind::Group(inner) => Ok(CssValueKind::Group(Box::new(lower_value(
			inner,
			variable_count,
			scope,
			false,
		)?))),
		TypedValueExprKind::SpaceSequence(items) => Ok(CssValueKind::SpaceSequence(lower_values(
			items,
			variable_count,
			scope,
		)?)),
		TypedValueExprKind::CommaList(items) => Ok(CssValueKind::CommaList(lower_values(
			items,
			variable_count,
			scope,
		)?)),
		TypedValueExprKind::UncheckedFunction(function) => {
			Ok(CssValueKind::UncheckedFunction(function.clone()))
		}
	}
}

fn lower_values(
	values: &[TypedValueExpr],
	variable_count: usize,
	scope: &StyleScope,
) -> Result<Vec<CssValue>, StyleDiagnostic> {
	values
		.iter()
		.map(|value| lower_value(value, variable_count, scope, true))
		.collect()
}

fn lower_function(
	call: &TypedFunctionCall,
	span: proc_macro2::Span,
	variable_count: usize,
	scope: &StyleScope,
) -> Result<CssValueKind, StyleDiagnostic> {
	let arguments = lower_values(&call.arguments, variable_count, scope)?;
	match call.spec.lowering {
		LoweringStrategy::CommaFunction => Ok(CssValueKind::Function(CssFunction {
			name: call.spec.css_spelling.into(),
			separator: CssFunctionSeparator::Comma,
			arguments,
		})),
		LoweringStrategy::SpaceFunction => Ok(CssValueKind::Function(CssFunction {
			name: call.spec.css_spelling.into(),
			separator: CssFunctionSeparator::Space,
			arguments,
		})),
		LoweringStrategy::ColorMixSrgb => {
			let Some(receiver) = call.receiver.as_deref() else {
				return Err(StyleDiagnostic::new(
					StyleDiagnosticKind::InvalidReceiverMethod {
						receiver: "missing".into(),
						method: "mix".into(),
					},
					span,
				));
			};
			let [other, amount] = arguments.as_slice() else {
				return Err(invalid_lowering_arity(&call.spec, arguments.len(), span));
			};
			Ok(CssValueKind::ColorMix {
				receiver: Box::new(lower_value(receiver, variable_count, scope, true)?),
				other: Box::new(other.clone()),
				amount: Box::new(amount.clone()),
			})
		}
		LoweringStrategy::SpacePair => {
			let [left, right] = arguments.as_slice() else {
				return Err(invalid_lowering_arity(&call.spec, arguments.len(), span));
			};
			Ok(CssValueKind::SpaceSequence(vec![
				left.clone(),
				right.clone(),
			]))
		}
		LoweringStrategy::SlashPair => {
			let [left, right] = arguments.as_slice() else {
				return Err(invalid_lowering_arity(&call.spec, arguments.len(), span));
			};
			Ok(CssValueKind::SlashPair {
				left: Box::new(left.clone()),
				right: Box::new(right.clone()),
			})
		}
	}
}

fn invalid_lowering_arity(
	spec: &crate::FunctionSpec,
	found: usize,
	span: proc_macro2::Span,
) -> StyleDiagnostic {
	StyleDiagnostic::new(
		StyleDiagnosticKind::InvalidFunctionArity {
			function: spec.dsl_path.into(),
			expected: "the validated registry arity".into(),
			found,
		},
		span,
	)
}

fn lower_literal(literal: &StyleValueLiteral) -> CssLiteral {
	match literal {
		StyleValueLiteral::Integer(number) | StyleValueLiteral::Number(number) => {
			CssLiteral::Number {
				source: number.source.clone(),
				unit: number.unit.as_ref().map(|unit| match unit {
					StyleNumericUnit::Named(name) => name.value.clone(),
					StyleNumericUnit::Percentage { .. } => "%".into(),
				}),
			}
		}
		StyleValueLiteral::HexColor(color) => CssLiteral::HexColor(color.source.clone()),
		StyleValueLiteral::Keyword(keyword) => CssLiteral::Keyword(keyword.value.clone()),
		StyleValueLiteral::String(string) => CssLiteral::String(string.value.clone()),
	}
}

#[cfg(test)]
mod tests {
	use std::fmt::Write as _;

	use proc_macro2::TokenTree;
	use rstest::rstest;

	use super::{
		CssFunctionSeparator, CssPseudoArguments, CssRule, CssSimpleSelector, CssValueKind,
		LoweredStyle, lower_style,
	};
	use crate::{
		StyleSelectorCombinator, parser::parse_style, style::StyleCompileContext,
		validator::validate_style,
	};

	fn lower(source: &str) -> LoweredStyle {
		let tokens = source.parse().expect("style test source should tokenize");
		let ast = parse_style(tokens).expect("style test source should parse");
		let typed = validate_style(&ast).expect("style test source should validate");
		lower_style(
			&typed,
			&StyleCompileContext {
				package_name: "poll-app",
				package_version: "0.4.0",
				style_type_name: "PollCardStyles",
			},
		)
		.expect("validated style should lower")
	}

	#[rstest]
	fn same_element_selector_lists_expand_in_stable_parent_major_order() {
		// Arrange
		let source = "
			.card, .panel {
				&:hover, &.featured { color: red; }
			}
		";

		// Act
		let lowered = lower(source);

		// Assert
		let CssRule::Style(rule) = &lowered.css.rules[0] else {
			panic!("expected a flat style rule");
		};
		assert_eq!(rule.selectors.len(), 4);
		let roots = rule
			.selectors
			.iter()
			.map(|selector| match &selector.segments[0].simple_selectors[0] {
				CssSimpleSelector::Class(name) => name.as_str(),
				_ => panic!("expected a scoped root class"),
			})
			.collect::<Vec<_>>();
		assert_eq!(
			roots,
			[
				"card--rs-f69b9cbc74c9",
				"card--rs-f69b9cbc74c9",
				"panel--rs-f69b9cbc74c9",
				"panel--rs-f69b9cbc74c9",
			]
		);
		assert!(matches!(
			rule.selectors[0].segments[0].simple_selectors[1],
			CssSimpleSelector::Pseudo(ref pseudo) if pseudo.name == "hover"
		));
		assert!(matches!(
			rule.selectors[1].segments[0].simple_selectors[1],
			CssSimpleSelector::Class(ref name) if name == "featured--rs-f69b9cbc74c9"
		));
		assert_eq!(
			lowered
				.classes
				.iter()
				.map(|class| class.authored_name.as_str())
				.collect::<Vec<_>>(),
			["card", "panel", "featured"]
		);
	}

	#[rstest]
	fn relative_selectors_scope_classes_and_retain_combinators() {
		// Arrange
		let source = "
			.card {
				.label, > button, + .card, ~ .label { color: red; }
			}
		";

		// Act
		let lowered = lower(source);

		// Assert
		let CssRule::Style(rule) = &lowered.css.rules[0] else {
			panic!("expected a flat style rule");
		};
		assert_eq!(rule.selectors.len(), 4);
		assert_eq!(
			rule.selectors
				.iter()
				.map(|selector| selector.segments[1].combinator)
				.collect::<Vec<_>>(),
			[
				Some(StyleSelectorCombinator::Descendant),
				Some(StyleSelectorCombinator::Child),
				Some(StyleSelectorCombinator::AdjacentSibling),
				Some(StyleSelectorCombinator::GeneralSibling),
			]
		);
		assert!(matches!(
			rule.selectors[0].segments[1].simple_selectors[0],
			CssSimpleSelector::Class(ref name) if name == "label--rs-f69b9cbc74c9"
		));
	}

	#[rstest]
	fn pseudo_argument_branches_remain_structured_and_scope_nested_classes() {
		// Arrange
		let source = "
			.card {
				&:is(:not(.deep), .item) { color: red; }
				&:has(> .child) { color: red; }
				&:nth-child(2n + 1 of .row) { color: red; }
				&:lang(en) { color: red; }
			}
		";

		// Act
		let lowered = lower(source);

		// Assert
		assert_eq!(lowered.css.rules.len(), 4);
		let pseudo = |rule_index: usize| {
			let CssRule::Style(rule) = &lowered.css.rules[rule_index] else {
				panic!("expected a flat style rule");
			};
			let CssSimpleSelector::Pseudo(pseudo) =
				&rule.selectors[0].segments[0].simple_selectors[1]
			else {
				panic!("expected a same-element pseudo selector");
			};
			pseudo
		};

		let Some(CssPseudoArguments::SelectorList(is_branches)) = &pseudo(0).arguments else {
			panic!("expected structured :is selector arguments");
		};
		let CssSimpleSelector::Pseudo(not_pseudo) = &is_branches[0].segments[0].simple_selectors[0]
		else {
			panic!("expected nested :not pseudo selector");
		};
		let Some(CssPseudoArguments::SelectorList(not_branches)) = &not_pseudo.arguments else {
			panic!("expected structured :not selector arguments");
		};
		assert!(matches!(
			not_branches[0].segments[0].simple_selectors[0],
			CssSimpleSelector::Class(ref name) if name == "deep--rs-f69b9cbc74c9"
		));
		assert!(matches!(
			is_branches[1].segments[0].simple_selectors[0],
			CssSimpleSelector::Class(ref name) if name == "item--rs-f69b9cbc74c9"
		));

		let Some(CssPseudoArguments::SelectorList(has_branches)) = &pseudo(1).arguments else {
			panic!("expected structured :has selector arguments");
		};
		assert_eq!(
			has_branches[0].segments[0].combinator,
			Some(StyleSelectorCombinator::Child)
		);
		assert!(matches!(
			has_branches[0].segments[0].simple_selectors[0],
			CssSimpleSelector::Class(ref name) if name == "child--rs-f69b9cbc74c9"
		));

		let Some(CssPseudoArguments::Nth {
			formula_tokens,
			selectors: Some(nth_branches),
		}) = &pseudo(2).arguments
		else {
			panic!("expected structured nth arguments");
		};
		assert_eq!(formula_tokens.len(), 3);
		assert!(matches!(
			nth_branches[0].segments[0].simple_selectors[0],
			CssSimpleSelector::Class(ref name) if name == "row--rs-f69b9cbc74c9"
		));

		let Some(CssPseudoArguments::RawTokens(tokens)) = &pseudo(3).arguments else {
			panic!("expected structured raw pseudo tokens");
		};
		assert!(matches!(
			tokens.as_slice(),
			[TokenTree::Ident(identifier)] if identifier == "en"
		));
	}

	#[rstest]
	fn nested_boundaries_split_declaration_runs_without_reordering() {
		// Arrange
		let source = "
			.card {
				color: red;
				&:hover { color: blue; }
				background-color: white;
				@media (max-width: 640px) { width: 100%; }
				opacity: 1;
			}
		";

		// Act
		let lowered = lower(source);

		// Assert
		assert_eq!(lowered.css.rules.len(), 5);
		let property = |index: usize| match &lowered.css.rules[index] {
			CssRule::Style(rule) => rule.declarations[0].property.as_str(),
			CssRule::Group(_) => "@media",
		};
		assert_eq!(
			(0..5).map(property).collect::<Vec<_>>(),
			["color", "color", "background-color", "@media", "opacity"]
		);
		let CssRule::Group(group) = &lowered.css.rules[3] else {
			panic!("expected a media grouping rule");
		};
		assert_eq!(group.rules.len(), 1);
		let CssRule::Style(media_rule) = &group.rules[0] else {
			panic!("expected a flat rule inside media");
		};
		assert_eq!(media_rule.declarations[0].property, "width");
		assert!(matches!(
			media_rule.selectors[0].segments[0].simple_selectors[0],
			CssSimpleSelector::Class(ref name) if name == "card--rs-f69b9cbc74c9"
		));
	}

	#[rstest]
	fn component_references_embed_recursive_typed_fallbacks() {
		// Arrange
		let source = "
			globals { border: Color; }
			vars {
				base: Length = 1rem;
				doubled: Length = vars.base * 2;
				accent: Color = globals.border;
			}
			.card {
				width: vars.doubled;
				color: vars.accent;
			}
		";

		// Act
		let lowered = lower(source);

		// Assert
		assert_eq!(lowered.scope.suffix, "f69b9cbc74c9");
		assert_eq!(lowered.globals.len(), 1);
		assert_eq!(lowered.globals[0].css_name, "border");
		assert_eq!(lowered.variables.len(), 3);
		assert_eq!(
			lowered
				.variables
				.iter()
				.map(|variable| variable.custom_property_name.as_str())
				.collect::<Vec<_>>(),
			[
				"--rs-f69b9cbc74c9-base",
				"--rs-f69b9cbc74c9-doubled",
				"--rs-f69b9cbc74c9-accent",
			]
		);
		let CssRule::Style(rule) = &lowered.css.rules[0] else {
			panic!("expected a flat style rule");
		};
		let CssValueKind::ComponentVariable {
			custom_property,
			fallback_index,
		} = &rule.declarations[0].value.kind
		else {
			panic!("expected a scoped component variable");
		};
		assert_eq!(custom_property, "--rs-f69b9cbc74c9-doubled");
		assert_eq!(*fallback_index, 1);
		assert_eq!(lowered.css.variable_defaults.len(), 3);
		let CssValueKind::Calc(calculation) = &lowered.css.variable_defaults[1].kind else {
			panic!("expected one calc boundary around arithmetic fallback");
		};
		let CssValueKind::Binary { left, .. } = &calculation.kind else {
			panic!("expected structured fallback arithmetic");
		};
		assert!(matches!(
			left.kind,
			CssValueKind::ComponentVariable {
				ref custom_property,
				fallback_index: 0,
			} if custom_property == "--rs-f69b9cbc74c9-base"
		));
		assert!(matches!(
			rule.declarations[1].value.kind,
			CssValueKind::ComponentVariable {
				fallback_index: 2,
				..
			}
		));
		assert!(matches!(
			lowered.css.variable_defaults[2].kind,
			CssValueKind::GlobalVariable { ref custom_property } if custom_property == "--border"
		));
	}

	#[rstest]
	fn deep_variable_fallbacks_use_a_flat_source_index_arena() {
		// Arrange
		const VARIABLE_COUNT: usize = 16_384;
		let mut source = String::with_capacity(VARIABLE_COUNT * 42);
		source.push_str("vars { v0: Length = 1px;");
		for index in 1..VARIABLE_COUNT {
			write!(source, "v{index}: Length = vars.v{};", index - 1).unwrap();
		}
		write!(
			source,
			"}} .card {{ width: vars.v{}; }}",
			VARIABLE_COUNT - 1
		)
		.unwrap();

		// Act
		let lowered = lower(&source);

		// Assert
		assert_eq!(lowered.css.variable_defaults.len(), VARIABLE_COUNT);
		assert!(matches!(
			lowered.css.variable_defaults[VARIABLE_COUNT - 1].kind,
			CssValueKind::ComponentVariable {
				fallback_index,
				..
			} if fallback_index == VARIABLE_COUNT - 2
		));
		let CssRule::Style(rule) = &lowered.css.rules[0] else {
			panic!("expected a flat style rule");
		};
		assert!(matches!(
			rule.declarations[0].value.kind,
			CssValueKind::ComponentVariable {
				fallback_index,
				..
			} if fallback_index == VARIABLE_COUNT - 1
		));
		drop(lowered);
	}

	#[rstest]
	fn registry_lowering_strategies_remain_structured_in_css_ir() {
		// Arrange
		let source = "
			.card {
				color: red.mix(blue, 20%);
				background-image: linear_gradient(
					Direction::Right,
					[stop(red, 0%), stop(blue, 100%)]
				);
				transform: (translate_x(1rem), rotate(45deg));
				border-radius: slash(1rem, 2rem);
			}
		";

		// Act
		let lowered = lower(source);

		// Assert
		let CssRule::Style(rule) = &lowered.css.rules[0] else {
			panic!("expected a flat style rule");
		};
		assert!(matches!(
			rule.declarations[0].value.kind,
			CssValueKind::ColorMix { .. }
		));
		assert!(matches!(
			rule.declarations[1].value.kind,
			CssValueKind::Function(ref function)
				if function.name == "linear-gradient"
					&& function.separator == CssFunctionSeparator::Comma
		));
		let CssValueKind::SpaceSequence(transform_functions) = &rule.declarations[2].value.kind
		else {
			panic!("expected a structured transform sequence");
		};
		assert!(matches!(
			transform_functions[0].kind,
			CssValueKind::Function(ref function) if function.name == "translateX"
		));
		assert!(matches!(
			rule.declarations[3].value.kind,
			CssValueKind::SlashPair { .. }
		));
	}
}
