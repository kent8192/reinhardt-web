//! Type checking for style values, functions, and property grammars.

use std::collections::{HashMap, HashSet};

use proc_macro2::Span;

use super::dependency::ValidatedBindings;
use crate::style::registry::{
	function_spec, infer_named_keyword_type, property_spec, reserved_function, unit_spec,
};
use crate::{
	ArgumentConstraints, ArityPolicy, CssName, Direction, FunctionResult, FunctionSpec,
	GrammarMember, NumericConstraint, NumericDimension, SemanticType, StyleBinaryOperatorKind,
	StyleDeclaration, StyleDiagnostic, StyleDiagnosticKind, StyleItem, StyleMacro, StyleMediaRule,
	StyleNumericUnit, StyleReferenceNamespace, StyleRule, StyleRuleItem, StyleRuntimeType,
	StyleUnaryOperatorKind, StyleValueExpr, StyleValueExpression, StyleValueLiteral,
	StyleVariableConstraint, TypeConstraint, TypedFunctionCall, TypedGlobalReference,
	TypedStyleDeclaration, TypedStyleGlobal, TypedStyleItem, TypedStyleMacro, TypedStyleMediaRule,
	TypedStyleRule, TypedStyleRuleItem, TypedStyleVariable, TypedValueExpr, TypedValueExprKind,
	TypedVariableReference, ValueGrammar,
};

#[derive(Debug, Clone)]
struct ResolvedSymbol {
	value_type: SemanticType,
	css_name: String,
	source_index: usize,
}

pub(super) fn validate_expressions(
	ast: &StyleMacro,
	bindings: ValidatedBindings,
	classes: Vec<crate::TypedStyleClass>,
) -> Result<TypedStyleMacro, StyleDiagnostic> {
	let global_runtime_types = bindings
		.globals
		.iter()
		.map(|global| resolve_runtime_type(&global.declaration.ty.name, global.declaration.ty.span))
		.collect::<Result<Vec<_>, _>>()?;
	let variable_runtime_types = bindings
		.variables
		.iter()
		.map(|variable| {
			resolve_runtime_type(&variable.declaration.ty.name, variable.declaration.ty.span)
		})
		.collect::<Result<Vec<_>, _>>()?;

	let global_symbols = bindings
		.globals
		.iter()
		.zip(&global_runtime_types)
		.map(|(global, runtime_type)| {
			(
				global.declaration.name.value.clone(),
				ResolvedSymbol {
					value_type: runtime_type.semantic_type(),
					css_name: global.css_name.clone(),
					source_index: global.source_index,
				},
			)
		})
		.collect::<HashMap<_, _>>();
	let variable_symbols = bindings
		.variables
		.iter()
		.zip(&variable_runtime_types)
		.map(|(variable, runtime_type)| {
			(
				variable.declaration.name.value.clone(),
				ResolvedSymbol {
					value_type: runtime_type.semantic_type(),
					css_name: variable.css_name.clone(),
					source_index: variable.source_index,
				},
			)
		})
		.collect::<HashMap<_, _>>();

	let globals = bindings
		.globals
		.into_iter()
		.zip(global_runtime_types)
		.map(|(global, runtime_type)| TypedStyleGlobal {
			declaration: global.declaration,
			value_type: runtime_type.semantic_type(),
			css_name: global.css_name,
			source_index: global.source_index,
		})
		.collect();

	let mut variables = Vec::with_capacity(bindings.variables.len());
	for (variable, runtime_type) in bindings.variables.into_iter().zip(variable_runtime_types) {
		let Some(default) = variable.declaration.default.as_ref() else {
			return Err(StyleDiagnostic::new(
				StyleDiagnosticKind::MissingVariableDefault {
					name: variable.declaration.name.value.clone(),
				},
				variable.declaration.name.span,
			));
		};
		let typed_default = infer_expression(default, &global_symbols, &variable_symbols)?;
		let declared_type = runtime_type.semantic_type();
		if !is_whole_unchecked(&typed_default)
			&& !expression_matches_type(&typed_default, declared_type)
		{
			return Err(StyleDiagnostic::new(
				StyleDiagnosticKind::PropertyValueMismatch {
					property: format!("vars.{}", variable.declaration.name.value),
					expected: semantic_type_label(declared_type).into(),
					found: semantic_type_label(typed_default.value_type).into(),
				},
				typed_default.span,
			)
			.with_related(
				variable.declaration.ty.span,
				format!(
					"component variable `{}` declares this type",
					variable.declaration.name.value
				),
			));
		}
		variables.push(TypedStyleVariable {
			declaration: variable.declaration,
			runtime_type,
			value_type: declared_type,
			default: typed_default,
			runtime_constraint: None,
			css_name: variable.css_name,
			source_index: variable.source_index,
			dependency_indices: variable.dependency_indices,
			evaluation_index: variable.evaluation_index,
		});
	}

	let variable_defaults = variables
		.iter()
		.map(|variable| (variable.source_index, &variable.default))
		.collect::<HashMap<_, _>>();
	let items = ast
		.items
		.iter()
		.map(|item| type_check_item(item, &global_symbols, &variable_symbols, &variable_defaults))
		.collect::<Result<Vec<_>, _>>()?;
	let variable_constraints = collect_component_variable_constraints(&items);
	for variable in &mut variables {
		variable.runtime_constraint = variable_constraints.get(&variable.source_index).copied();
	}

	Ok(TypedStyleMacro {
		globals,
		variables,
		items,
		classes,
		variable_evaluation_order: bindings.evaluation_order,
		span: ast.span,
	})
}

fn collect_component_variable_constraints(
	items: &[TypedStyleItem],
) -> HashMap<usize, StyleVariableConstraint> {
	let mut constraints = HashMap::new();
	for item in items {
		match item {
			TypedStyleItem::Rule(rule) => {
				collect_rule_item_constraints(&rule.items, &mut constraints);
			}
			TypedStyleItem::Media(media) => {
				collect_rule_item_constraints(&media.items, &mut constraints);
			}
		}
	}
	constraints
		.into_iter()
		.filter_map(|(source_index, constraint)| {
			constraint.map(|constraint| (source_index, constraint))
		})
		.collect()
}

fn collect_rule_item_constraints(
	items: &[TypedStyleRuleItem],
	constraints: &mut HashMap<usize, Option<StyleVariableConstraint>>,
) {
	for item in items {
		match item {
			TypedStyleRuleItem::Declaration(declaration) => {
				collect_declaration_constraint(declaration, constraints);
			}
			TypedStyleRuleItem::Rule(rule) => {
				collect_rule_item_constraints(&rule.items, constraints);
			}
			TypedStyleRuleItem::Media(media) => {
				collect_rule_item_constraints(&media.items, constraints);
			}
		}
	}
}

fn collect_declaration_constraint(
	declaration: &TypedStyleDeclaration,
	constraints: &mut HashMap<usize, Option<StyleVariableConstraint>>,
) {
	let Some(spec) = property_spec(declaration.name.as_str()) else {
		return;
	};
	collect_value_constraints(&declaration.value, spec.grammar, constraints);
}

fn collect_value_constraints(
	expression: &TypedValueExpr,
	grammar: &ValueGrammar,
	constraints: &mut HashMap<usize, Option<StyleVariableConstraint>>,
) {
	if let Some(constraint) = variable_constraint_for_grammar(expression, grammar) {
		for source_index in component_variable_references(expression) {
			record_component_variable_constraint(constraints, source_index, constraint);
		}
		return;
	}

	match grammar {
		ValueGrammar::Or(alternatives) => {
			let matching = alternatives
				.iter()
				.filter(|alternative| matches_grammar(expression, alternative))
				.collect::<Vec<_>>();
			if let [alternative] = matching.as_slice() {
				collect_value_constraints(expression, alternative, constraints);
			}
		}
		ValueGrammar::Space {
			item: value_grammar,
			..
		} if matches_grammar(expression, grammar) => {
			for value in sequence_items(expression) {
				collect_value_constraints(value, value_grammar, constraints);
			}
		}
		ValueGrammar::Comma {
			item: value_grammar,
			..
		} if matches_grammar(expression, grammar) => {
			for value in comma_items(expression) {
				collect_value_constraints(value, value_grammar, constraints);
			}
		}
		ValueGrammar::CommaFinal {
			item: leading_grammar,
			final_item,
			..
		} if matches_grammar(expression, grammar) => {
			let items = comma_items(expression);
			let Some((last, leading)) = items.split_last() else {
				return;
			};
			for value in leading {
				collect_value_constraints(value, leading_grammar, constraints);
			}
			collect_value_constraints(last, final_item, constraints);
		}
		ValueGrammar::Slash { left, right }
			if matches_grammar(expression, grammar)
				&& let Some((left_value, right_value)) = slash_pair(expression) =>
		{
			collect_value_constraints(left_value, left, constraints);
			collect_value_constraints(right_value, right, constraints);
		}
		ValueGrammar::SlashList { item, .. } if matches_grammar(expression, grammar) => {
			let mut items = Vec::new();
			flatten_slash(expression, &mut items);
			for item_value in items {
				collect_value_constraints(item_value, item, constraints);
			}
		}
		ValueGrammar::Ordered(members) | ValueGrammar::Unordered { members, .. }
			if matches_grammar(expression, grammar) =>
		{
			for item in sequence_items(expression) {
				let matching = members
					.iter()
					.filter(|member| matches_grammar(item, member.grammar))
					.collect::<Vec<_>>();
				if let [member] = matching.as_slice() {
					collect_value_constraints(item, member.grammar, constraints);
				}
			}
		}
		_ => {}
	}
}

fn record_component_variable_constraint(
	constraints: &mut HashMap<usize, Option<StyleVariableConstraint>>,
	source_index: usize,
	constraint: StyleVariableConstraint,
) {
	let entry = constraints.entry(source_index).or_insert(Some(constraint));
	*entry = (*entry).and_then(|existing| intersect_variable_constraints(existing, constraint));
}

fn component_variable_references(expression: &TypedValueExpr) -> HashSet<usize> {
	let mut references = HashSet::new();
	collect_component_variable_references(expression, &mut references);
	references
}

fn collect_component_variable_references(
	expression: &TypedValueExpr,
	references: &mut HashSet<usize>,
) {
	match &expression.kind {
		TypedValueExprKind::VariableReference(reference) => {
			references.insert(reference.source_index);
		}
		TypedValueExprKind::Unary { operand, .. } | TypedValueExprKind::Group(operand) => {
			collect_component_variable_references(operand, references);
		}
		TypedValueExprKind::Binary { left, right, .. } => {
			collect_component_variable_references(left, references);
			collect_component_variable_references(right, references);
		}
		TypedValueExprKind::Function(call) => {
			if let Some(receiver) = call.receiver.as_deref() {
				collect_component_variable_references(receiver, references);
			}
			for argument in &call.arguments {
				collect_component_variable_references(argument, references);
			}
		}
		TypedValueExprKind::SpaceSequence(items) | TypedValueExprKind::CommaList(items) => {
			for item in items {
				collect_component_variable_references(item, references);
			}
		}
		TypedValueExprKind::Literal(_)
		| TypedValueExprKind::GlobalReference(_)
		| TypedValueExprKind::Direction(_)
		| TypedValueExprKind::UncheckedFunction(_) => {}
	}
}

fn variable_constraint_for_grammar(
	expression: &TypedValueExpr,
	grammar: &ValueGrammar,
) -> Option<StyleVariableConstraint> {
	match grammar {
		ValueGrammar::NonNegative(inner) if matches_grammar(expression, inner) => {
			let inner_constraint = variable_constraint_for_grammar(expression, inner);
			match inner_constraint {
				Some(constraint) => {
					intersect_variable_constraints(StyleVariableConstraint::NonNegative, constraint)
				}
				None => Some(StyleVariableConstraint::NonNegative),
			}
		}
		ValueGrammar::NumericRange {
			grammar,
			minimum,
			maximum,
		} if matches_grammar(expression, grammar) => {
			let constraint = StyleVariableConstraint::NumericRange {
				minimum: *minimum,
				maximum: *maximum,
			};
			match variable_constraint_for_grammar(expression, grammar) {
				Some(inner_constraint) => {
					intersect_variable_constraints(constraint, inner_constraint)
				}
				None => Some(constraint),
			}
		}
		ValueGrammar::Or(alternatives) => common_variable_constraint(
			alternatives
				.iter()
				.filter(|alternative| matches_grammar(expression, alternative))
				.map(|alternative| variable_constraint_for_grammar(expression, alternative)),
		),
		ValueGrammar::Space { item, .. }
		| ValueGrammar::Comma { item, .. }
		| ValueGrammar::SlashList { item, .. }
			if matches_grammar(expression, grammar) =>
		{
			variable_constraint_for_grammar(expression, item)
		}
		ValueGrammar::CommaFinal { final_item, .. } if matches_grammar(expression, grammar) => {
			variable_constraint_for_grammar(expression, final_item)
		}
		ValueGrammar::Primitive(_)
		| ValueGrammar::NonNegative(_)
		| ValueGrammar::NumericRange { .. }
		| ValueGrammar::Keyword(_)
		| ValueGrammar::Identifier
		| ValueGrammar::IdentifierExcept(_)
		| ValueGrammar::FunctionResult(_)
		| ValueGrammar::Slash { .. }
		| ValueGrammar::Ordered(_)
		| ValueGrammar::Unordered { .. }
		| ValueGrammar::Space { .. }
		| ValueGrammar::Comma { .. }
		| ValueGrammar::CommaFinal { .. }
		| ValueGrammar::SlashList { .. } => None,
	}
}

fn common_variable_constraint(
	constraints: impl Iterator<Item = Option<StyleVariableConstraint>>,
) -> Option<StyleVariableConstraint> {
	let mut constraints = constraints;
	let mut combined = constraints.next()??;
	for constraint in constraints {
		combined = union_variable_constraints(combined, constraint?)?;
	}
	Some(combined)
}

fn intersect_variable_constraints(
	left: StyleVariableConstraint,
	right: StyleVariableConstraint,
) -> Option<StyleVariableConstraint> {
	match (left, right) {
		(StyleVariableConstraint::NonNegative, StyleVariableConstraint::NonNegative) => {
			Some(StyleVariableConstraint::NonNegative)
		}
		(
			StyleVariableConstraint::NonNegative,
			StyleVariableConstraint::NumericRange { minimum, maximum },
		)
		| (
			StyleVariableConstraint::NumericRange { minimum, maximum },
			StyleVariableConstraint::NonNegative,
		) => {
			let minimum = minimum.max(0);
			(minimum <= maximum)
				.then_some(StyleVariableConstraint::NumericRange { minimum, maximum })
		}
		(
			StyleVariableConstraint::NumericRange {
				minimum: left_minimum,
				maximum: left_maximum,
			},
			StyleVariableConstraint::NumericRange {
				minimum: right_minimum,
				maximum: right_maximum,
			},
		) => {
			let minimum = left_minimum.max(right_minimum);
			let maximum = left_maximum.min(right_maximum);
			(minimum <= maximum)
				.then_some(StyleVariableConstraint::NumericRange { minimum, maximum })
		}
	}
}

fn union_variable_constraints(
	left: StyleVariableConstraint,
	right: StyleVariableConstraint,
) -> Option<StyleVariableConstraint> {
	match (left, right) {
		(StyleVariableConstraint::NonNegative, StyleVariableConstraint::NonNegative) => {
			Some(StyleVariableConstraint::NonNegative)
		}
		(
			StyleVariableConstraint::NonNegative,
			StyleVariableConstraint::NumericRange { minimum, .. },
		)
		| (
			StyleVariableConstraint::NumericRange { minimum, .. },
			StyleVariableConstraint::NonNegative,
		) if minimum >= 0 => Some(StyleVariableConstraint::NonNegative),
		(
			StyleVariableConstraint::NumericRange {
				minimum: left_minimum,
				maximum: left_maximum,
			},
			StyleVariableConstraint::NumericRange {
				minimum: right_minimum,
				maximum: right_maximum,
			},
		) if i32::from(left_maximum) + 1 >= i32::from(right_minimum)
			&& i32::from(right_maximum) + 1 >= i32::from(left_minimum) =>
		{
			Some(StyleVariableConstraint::NumericRange {
				minimum: left_minimum.min(right_minimum),
				maximum: left_maximum.max(right_maximum),
			})
		}
		_ => None,
	}
}

fn resolve_runtime_type(name: &str, span: Span) -> Result<StyleRuntimeType, StyleDiagnostic> {
	StyleRuntimeType::from_dsl_name(name).ok_or_else(|| {
		StyleDiagnostic::new(
			StyleDiagnosticKind::UnknownStyleType {
				name: name.to_owned(),
			},
			span,
		)
	})
}

fn type_check_item(
	item: &StyleItem,
	globals: &HashMap<String, ResolvedSymbol>,
	variables: &HashMap<String, ResolvedSymbol>,
	variable_defaults: &HashMap<usize, &TypedValueExpr>,
) -> Result<TypedStyleItem, StyleDiagnostic> {
	match item {
		StyleItem::Rule(rule) => {
			type_check_rule(rule, globals, variables, variable_defaults).map(TypedStyleItem::Rule)
		}
		StyleItem::Media(media) => type_check_media(media, globals, variables, variable_defaults)
			.map(TypedStyleItem::Media),
	}
}

fn type_check_rule(
	rule: &StyleRule,
	globals: &HashMap<String, ResolvedSymbol>,
	variables: &HashMap<String, ResolvedSymbol>,
	variable_defaults: &HashMap<usize, &TypedValueExpr>,
) -> Result<TypedStyleRule, StyleDiagnostic> {
	Ok(TypedStyleRule {
		selectors: rule.selectors.clone(),
		items: type_check_rule_items(&rule.items, globals, variables, variable_defaults)?,
		span: rule.span,
	})
}

fn type_check_media(
	media: &StyleMediaRule,
	globals: &HashMap<String, ResolvedSymbol>,
	variables: &HashMap<String, ResolvedSymbol>,
	variable_defaults: &HashMap<usize, &TypedValueExpr>,
) -> Result<TypedStyleMediaRule, StyleDiagnostic> {
	Ok(TypedStyleMediaRule {
		condition: media.condition.clone(),
		items: type_check_rule_items(&media.items, globals, variables, variable_defaults)?,
		span: media.span,
	})
}

fn type_check_rule_items(
	items: &[StyleRuleItem],
	globals: &HashMap<String, ResolvedSymbol>,
	variables: &HashMap<String, ResolvedSymbol>,
	variable_defaults: &HashMap<usize, &TypedValueExpr>,
) -> Result<Vec<TypedStyleRuleItem>, StyleDiagnostic> {
	items
		.iter()
		.map(|item| match item {
			StyleRuleItem::Declaration(declaration) => {
				type_check_declaration(declaration, globals, variables, variable_defaults)
					.map(TypedStyleRuleItem::Declaration)
			}
			StyleRuleItem::Rule(rule) => {
				type_check_rule(rule, globals, variables, variable_defaults)
					.map(TypedStyleRuleItem::Rule)
			}
			StyleRuleItem::Media(media) => {
				type_check_media(media, globals, variables, variable_defaults)
					.map(TypedStyleRuleItem::Media)
			}
		})
		.collect()
}

fn type_check_declaration(
	declaration: &StyleDeclaration,
	globals: &HashMap<String, ResolvedSymbol>,
	variables: &HashMap<String, ResolvedSymbol>,
	variable_defaults: &HashMap<usize, &TypedValueExpr>,
) -> Result<TypedStyleDeclaration, StyleDiagnostic> {
	let Some(spec) = property_spec(declaration.name.as_str()) else {
		return Err(StyleDiagnostic::new(
			StyleDiagnosticKind::UnknownProperty {
				name: declaration.name.value.clone(),
			},
			declaration.name.span,
		));
	};
	let value = infer_expression(&declaration.value, globals, variables)?;
	let grammar_matches = matches_property_grammar(
		&value,
		spec.grammar,
		spec.css_wide_keywords,
		variable_defaults,
	) && (spec.name != "box-shadow"
		|| box_shadow_blur_is_not_negative(&value))
		&& property_specific_constraints_match(spec.name, &value)
		&& component_variable_fallback_matches_property_specific_constraints(
			spec.name,
			&value,
			variable_defaults,
		);
	if !is_whole_unchecked(&value) && !grammar_matches {
		return Err(StyleDiagnostic::new(
			StyleDiagnosticKind::PropertyValueMismatch {
				property: spec.name.into(),
				expected: spec.grammar.describe(),
				found: semantic_type_label(value.value_type).into(),
			},
			value.span,
		));
	}
	Ok(TypedStyleDeclaration {
		name: CssName {
			value: spec.name.into(),
			span: declaration.name.span,
		},
		value,
		span: declaration.span,
	})
}

fn box_shadow_blur_is_not_negative(value: &TypedValueExpr) -> bool {
	comma_items(value).into_iter().all(|shadow| {
		let lengths = sequence_items(shadow)
			.into_iter()
			.filter(|item| item.value_type == SemanticType::Length || item.is_contextual_zero())
			.collect::<Vec<_>>();
		!lengths.get(2).is_some_and(|blur| is_negative_literal(blur))
	})
}

fn infer_expression(
	expression: &StyleValueExpression,
	globals: &HashMap<String, ResolvedSymbol>,
	variables: &HashMap<String, ResolvedSymbol>,
) -> Result<TypedValueExpr, StyleDiagnostic> {
	match &expression.kind {
		StyleValueExpr::Literal(literal) => infer_literal(literal, expression.span),
		StyleValueExpr::QualifiedReference(reference) => {
			let symbols = match reference.namespace {
				StyleReferenceNamespace::Globals => globals,
				StyleReferenceNamespace::Variables => variables,
			};
			let Some(symbol) = symbols.get(reference.name.as_str()) else {
				let kind = match reference.namespace {
					StyleReferenceNamespace::Globals => {
						StyleDiagnosticKind::UndeclaredGlobalReference {
							name: reference.name.value.clone(),
						}
					}
					StyleReferenceNamespace::Variables => {
						StyleDiagnosticKind::UndeclaredVariableReference {
							name: reference.name.value.clone(),
						}
					}
				};
				return Err(StyleDiagnostic::new(kind, reference.span));
			};
			let kind = match reference.namespace {
				StyleReferenceNamespace::Globals => {
					TypedValueExprKind::GlobalReference(TypedGlobalReference {
						name: reference.name.value.clone(),
						css_name: symbol.css_name.clone(),
						source_index: symbol.source_index,
					})
				}
				StyleReferenceNamespace::Variables => {
					TypedValueExprKind::VariableReference(TypedVariableReference {
						name: reference.name.value.clone(),
						css_name: symbol.css_name.clone(),
						source_index: symbol.source_index,
						value_type: symbol.value_type,
					})
				}
			};
			Ok(TypedValueExpr {
				value_type: symbol.value_type,
				kind,
				contains_arithmetic: false,
				span: expression.span,
			})
		}
		StyleValueExpr::AssociatedPathValue(path) => {
			let path_name = value_path_name(path);
			let Some(direction) = direction_value(&path_name) else {
				return Err(StyleDiagnostic::new(
					StyleDiagnosticKind::UnknownFunction { name: path_name },
					path.span,
				));
			};
			Ok(TypedValueExpr {
				value_type: SemanticType::Direction,
				kind: TypedValueExprKind::Direction(direction),
				contains_arithmetic: false,
				span: expression.span,
			})
		}
		StyleValueExpr::Unary(unary) => {
			let operand = infer_expression(&unary.operand, globals, variables)?;
			ensure_checked_child(&operand, "a unary expression", unary.operand.span)?;
			if operand.value_type.numeric_dimension().is_none() {
				return Err(invalid_arithmetic(
					match unary.operator.kind {
						StyleUnaryOperatorKind::Plus => "+",
						StyleUnaryOperatorKind::Minus => "-",
					},
					operand.value_type,
					"numeric",
					unary.operator.span,
				));
			}
			let signed_atom = matches!(
				operand.kind,
				TypedValueExprKind::Literal(StyleValueLiteral::Integer(_))
					| TypedValueExprKind::Literal(StyleValueLiteral::Number(_))
			);
			Ok(TypedValueExpr {
				value_type: operand.value_type,
				contains_arithmetic: operand.contains_arithmetic || !signed_atom,
				kind: TypedValueExprKind::Unary {
					operator: unary.operator.clone(),
					operand: Box::new(operand),
				},
				span: expression.span,
			})
		}
		StyleValueExpr::Binary(binary) => {
			let left = infer_expression(&binary.left, globals, variables)?;
			let right = infer_expression(&binary.right, globals, variables)?;
			ensure_checked_child(&left, "arithmetic", binary.left.span)?;
			ensure_checked_child(&right, "arithmetic", binary.right.span)?;
			let value_type =
				infer_binary_type(&left, binary.operator.kind, &right, binary.operator.span)?;
			Ok(TypedValueExpr {
				value_type,
				kind: TypedValueExprKind::Binary {
					left: Box::new(left),
					operator: binary.operator.clone(),
					right: Box::new(right),
				},
				contains_arithmetic: true,
				span: expression.span,
			})
		}
		StyleValueExpr::Call(call) => infer_registered_call(
			&value_path_name(&call.path),
			None,
			&call.arguments,
			call.span,
			globals,
			variables,
		),
		StyleValueExpr::MethodCall(call) => {
			let receiver = infer_expression(&call.receiver, globals, variables)?;
			ensure_checked_child(&receiver, "a receiver method", call.receiver.span)?;
			infer_registered_call(
				&format!(".{}", call.method.value),
				Some(receiver),
				&call.arguments,
				call.span,
				globals,
				variables,
			)
		}
		StyleValueExpr::Group(group) => {
			let inner = infer_expression(&group.expression, globals, variables)?;
			ensure_checked_child(&inner, "a grouped expression", group.span)?;
			Ok(TypedValueExpr {
				value_type: inner.value_type,
				contains_arithmetic: inner.contains_arithmetic,
				kind: TypedValueExprKind::Group(Box::new(inner)),
				span: expression.span,
			})
		}
		StyleValueExpr::SpaceSequence(collection) => {
			let items = infer_collection(
				&collection.items,
				"a space-separated sequence",
				globals,
				variables,
			)?;
			Ok(TypedValueExpr {
				value_type: SemanticType::SpaceSequence,
				kind: TypedValueExprKind::SpaceSequence(items),
				contains_arithmetic: false,
				span: expression.span,
			})
		}
		StyleValueExpr::CommaList(collection) => {
			let items = infer_collection(
				&collection.items,
				"a comma-separated list",
				globals,
				variables,
			)?;
			Ok(TypedValueExpr {
				value_type: SemanticType::CommaList,
				kind: TypedValueExprKind::CommaList(items),
				contains_arithmetic: false,
				span: expression.span,
			})
		}
		StyleValueExpr::UncheckedFunction(function) => Ok(TypedValueExpr {
			value_type: SemanticType::Unchecked,
			kind: TypedValueExprKind::UncheckedFunction(function.clone()),
			contains_arithmetic: false,
			span: expression.span,
		}),
	}
}

fn infer_collection(
	expressions: &[StyleValueExpression],
	context: &str,
	globals: &HashMap<String, ResolvedSymbol>,
	variables: &HashMap<String, ResolvedSymbol>,
) -> Result<Vec<TypedValueExpr>, StyleDiagnostic> {
	let mut items = Vec::with_capacity(expressions.len());
	for expression in expressions {
		let item = infer_expression(expression, globals, variables)?;
		ensure_checked_child(&item, context, expression.span)?;
		items.push(item);
	}
	Ok(items)
}

fn infer_literal(
	literal: &StyleValueLiteral,
	span: Span,
) -> Result<TypedValueExpr, StyleDiagnostic> {
	let value_type = match literal {
		StyleValueLiteral::Integer(number) => numeric_literal_type(number, true)?,
		StyleValueLiteral::Number(number) => numeric_literal_type(number, false)?,
		StyleValueLiteral::HexColor(_) => SemanticType::Color,
		StyleValueLiteral::Keyword(keyword) => {
			infer_named_keyword_type(keyword.as_str()).unwrap_or(SemanticType::Keyword)
		}
		StyleValueLiteral::String(_) => SemanticType::QuotedString,
	};
	Ok(TypedValueExpr {
		value_type,
		kind: TypedValueExprKind::Literal(literal.clone()),
		contains_arithmetic: false,
		span,
	})
}

fn numeric_literal_type(
	number: &crate::StyleNumericLiteral,
	is_integer: bool,
) -> Result<SemanticType, StyleDiagnostic> {
	let Some(unit) = &number.unit else {
		return Ok(if is_integer {
			SemanticType::Integer
		} else {
			SemanticType::Number
		});
	};
	let dimension = match unit {
		StyleNumericUnit::Percentage { .. } => NumericDimension::Percentage,
		StyleNumericUnit::Named(name) => {
			let Some(spec) = unit_spec(name.as_str()) else {
				return Err(StyleDiagnostic::new(
					StyleDiagnosticKind::UnknownUnit {
						name: name.value.clone(),
					},
					name.span,
				));
			};
			spec.dimension
		}
	};
	Ok(semantic_type_from_dimension(dimension))
}

fn semantic_type_from_dimension(dimension: NumericDimension) -> SemanticType {
	match dimension {
		NumericDimension::Number => SemanticType::Number,
		NumericDimension::Integer => SemanticType::Integer,
		NumericDimension::Length => SemanticType::Length,
		NumericDimension::LengthPercentage => SemanticType::LengthPercentage,
		NumericDimension::Percentage => SemanticType::Percentage,
		NumericDimension::Angle => SemanticType::Angle,
		NumericDimension::Time => SemanticType::Time,
		NumericDimension::GridFraction => SemanticType::GridFraction,
	}
}

fn infer_binary_type(
	left: &TypedValueExpr,
	operator: StyleBinaryOperatorKind,
	right: &TypedValueExpr,
	span: Span,
) -> Result<SemanticType, StyleDiagnostic> {
	match operator {
		StyleBinaryOperatorKind::Add | StyleBinaryOperatorKind::Subtract => {
			join_numeric(left, right).ok_or_else(|| {
				invalid_arithmetic(
					binary_symbol(operator),
					left.value_type,
					semantic_type_label(right.value_type),
					span,
				)
			})
		}
		StyleBinaryOperatorKind::Multiply => multiply_type(left, right).ok_or_else(|| {
			invalid_arithmetic(
				"*",
				left.value_type,
				semantic_type_label(right.value_type),
				span,
			)
		}),
		StyleBinaryOperatorKind::Divide => {
			if is_literal_zero(right) {
				return Err(invalid_arithmetic(
					"/",
					left.value_type,
					"literal zero",
					span,
				));
			}
			divide_type(left.value_type, right.value_type).ok_or_else(|| {
				invalid_arithmetic(
					"/",
					left.value_type,
					semantic_type_label(right.value_type),
					span,
				)
			})
		}
	}
}

fn join_numeric(left: &TypedValueExpr, right: &TypedValueExpr) -> Option<SemanticType> {
	join_numeric_types(left.value_type, right.value_type)
}

fn join_numeric_types(left: SemanticType, right: SemanticType) -> Option<SemanticType> {
	use SemanticType::{Integer, Length, LengthPercentage, Number, Percentage};
	match (left, right) {
		(Integer, Integer) => Some(Integer),
		(Integer | Number, Integer | Number) => Some(Number),
		(left, right) if left == right && left.numeric_dimension().is_some() => Some(left),
		(Length, Percentage) | (Percentage, Length) => Some(LengthPercentage),
		(LengthPercentage, Length | Percentage | LengthPercentage)
		| (Length | Percentage, LengthPercentage) => Some(LengthPercentage),
		_ => None,
	}
}

fn multiply_type(left: &TypedValueExpr, right: &TypedValueExpr) -> Option<SemanticType> {
	let left_scalar = is_scalar(left.value_type);
	let right_scalar = is_scalar(right.value_type);
	match (left_scalar, right_scalar) {
		(true, true) => join_numeric_types(left.value_type, right.value_type),
		(true, false) if right.value_type.numeric_dimension().is_some() => Some(right.value_type),
		(false, true) if left.value_type.numeric_dimension().is_some() => Some(left.value_type),
		_ => None,
	}
}

fn divide_type(left: SemanticType, right: SemanticType) -> Option<SemanticType> {
	if !is_scalar(right) || left.numeric_dimension().is_none() {
		return None;
	}
	if is_scalar(left) {
		Some(SemanticType::Number)
	} else {
		Some(left)
	}
}

fn is_scalar(value_type: SemanticType) -> bool {
	matches!(value_type, SemanticType::Integer | SemanticType::Number)
}

fn is_literal_zero(expression: &TypedValueExpr) -> bool {
	if expression.is_contextual_zero() {
		return true;
	}
	match &expression.kind {
		TypedValueExprKind::Unary { operand, .. } | TypedValueExprKind::Group(operand) => {
			is_literal_zero(operand)
		}
		_ => false,
	}
}

fn infer_registered_call(
	path: &str,
	receiver: Option<TypedValueExpr>,
	arguments: &[StyleValueExpression],
	span: Span,
	globals: &HashMap<String, ResolvedSymbol>,
	variables: &HashMap<String, ResolvedSymbol>,
) -> Result<TypedValueExpr, StyleDiagnostic> {
	if receiver.is_none()
		&& let Some(reserved) = reserved_function(path)
	{
		return Err(StyleDiagnostic::new(reserved.diagnostic_kind(), span));
	}
	let Some(spec) = function_spec(path).copied() else {
		if let Some(receiver) = receiver {
			return Err(StyleDiagnostic::new(
				StyleDiagnosticKind::InvalidReceiverMethod {
					receiver: semantic_type_label(receiver.value_type).into(),
					method: path.trim_start_matches('.').into(),
				},
				span,
			));
		}
		return Err(StyleDiagnostic::new(
			StyleDiagnosticKind::UnknownFunction { name: path.into() },
			span,
		));
	};
	if !arity_accepts(spec.arity, arguments.len()) {
		return Err(StyleDiagnostic::new(
			StyleDiagnosticKind::InvalidFunctionArity {
				function: path.into(),
				expected: arity_description(spec.arity),
				found: arguments.len(),
			},
			span,
		));
	}
	let mut typed_arguments = Vec::with_capacity(arguments.len());
	for argument in arguments {
		let typed = infer_expression(argument, globals, variables)?;
		ensure_checked_child(&typed, "a checked function argument", argument.span)?;
		typed_arguments.push(typed);
	}
	if let Some(receiver) = &receiver {
		let Some(constraint) = spec.receiver else {
			return Err(StyleDiagnostic::new(
				StyleDiagnosticKind::InvalidReceiverMethod {
					receiver: semantic_type_label(receiver.value_type).into(),
					method: path.trim_start_matches('.').into(),
				},
				span,
			));
		};
		if !matches_constraint(receiver, constraint) {
			return Err(StyleDiagnostic::new(
				StyleDiagnosticKind::InvalidReceiverMethod {
					receiver: semantic_type_label(receiver.value_type).into(),
					method: path.trim_start_matches('.').into(),
				},
				span,
			));
		}
	}
	validate_function_arguments(&spec, &typed_arguments, span)?;
	let value_type = function_result_type(&spec, &typed_arguments, span)?;
	Ok(TypedValueExpr {
		value_type,
		kind: TypedValueExprKind::Function(TypedFunctionCall {
			spec,
			receiver: receiver.map(Box::new),
			arguments: typed_arguments,
		}),
		contains_arithmetic: false,
		span,
	})
}

fn validate_function_arguments(
	spec: &FunctionSpec,
	arguments: &[TypedValueExpr],
	span: Span,
) -> Result<(), StyleDiagnostic> {
	match spec.arguments {
		ArgumentConstraints::Repeated(constraint) => {
			if matches!(
				constraint,
				TypeConstraint::Numeric(NumericConstraint::Joined)
			) {
				joined_argument_type(spec.dsl_path, arguments, span)?;
				return Ok(());
			}
			for (index, argument) in arguments.iter().enumerate() {
				validate_argument(spec.dsl_path, index, argument, constraint)?;
			}
		}
		ArgumentConstraints::Positional(constraints) => {
			if constraints.iter().all(|constraint| {
				matches!(
					constraint,
					TypeConstraint::Numeric(NumericConstraint::Joined)
				)
			}) {
				joined_argument_type(spec.dsl_path, arguments, span)?;
				return Ok(());
			}
			for (index, (argument, constraint)) in arguments.iter().zip(constraints).enumerate() {
				validate_argument(spec.dsl_path, index, argument, *constraint)?;
			}
		}
	}
	Ok(())
}

fn validate_argument(
	function: &str,
	index: usize,
	argument: &TypedValueExpr,
	constraint: TypeConstraint,
) -> Result<(), StyleDiagnostic> {
	if matches_constraint(argument, constraint) {
		return Ok(());
	}
	Err(StyleDiagnostic::new(
		StyleDiagnosticKind::InvalidFunctionArgument {
			function: function.into(),
			index: index + 1,
			expected: constraint_description(constraint),
			found: semantic_type_label(argument.value_type).into(),
		},
		argument.span,
	))
}

fn matches_constraint(expression: &TypedValueExpr, constraint: TypeConstraint) -> bool {
	match constraint {
		TypeConstraint::Exact(expected) => expression_matches_type(expression, expected),
		TypeConstraint::Numeric(NumericConstraint::NumberOrPercentage) => {
			matches!(
				expression.value_type,
				SemanticType::Integer | SemanticType::Number | SemanticType::Percentage
			) || expression.is_contextual_zero()
		}
		TypeConstraint::Numeric(NumericConstraint::PercentageRange { minimum, maximum }) => {
			expression_matches_type(expression, SemanticType::Percentage)
				&& numeric_literal_value(expression)
					.is_none_or(|value| value >= f64::from(minimum) && value <= f64::from(maximum))
		}
		TypeConstraint::Numeric(NumericConstraint::Joined) => {
			expression.value_type.numeric_dimension().is_some()
		}
		TypeConstraint::CommaList { element, min } => {
			let TypedValueExprKind::CommaList(items) = &expression.kind else {
				return false;
			};
			items.len() >= min
				&& items
					.iter()
					.all(|item| expression_matches_type(item, element))
		}
		TypeConstraint::Any => expression.value_type != SemanticType::Unchecked,
	}
}

fn function_result_type(
	spec: &FunctionSpec,
	arguments: &[TypedValueExpr],
	span: Span,
) -> Result<SemanticType, StyleDiagnostic> {
	match spec.result {
		FunctionResult::Exact(value_type) => Ok(value_type),
		FunctionResult::JoinedNumeric => joined_argument_type(spec.dsl_path, arguments, span),
		FunctionResult::SlashPair => Ok(SemanticType::SlashPair),
	}
}

fn joined_argument_type(
	function: &str,
	arguments: &[TypedValueExpr],
	span: Span,
) -> Result<SemanticType, StyleDiagnostic> {
	let mut joined = None;
	let mut zero_fallback = None;
	for (index, argument) in arguments.iter().enumerate() {
		if argument.value_type.numeric_dimension().is_none() {
			return Err(StyleDiagnostic::new(
				StyleDiagnosticKind::InvalidFunctionArgument {
					function: function.into(),
					index: index + 1,
					expected: "a joinable numeric value".into(),
					found: semantic_type_label(argument.value_type).into(),
				},
				argument.span,
			));
		}
		if argument.is_contextual_zero() {
			zero_fallback = Some(match zero_fallback {
				Some(current) => {
					join_numeric_types(current, argument.value_type).unwrap_or(SemanticType::Number)
				}
				None => argument.value_type,
			});
			continue;
		}
		joined = Some(match joined {
			Some(current) => join_numeric_types(current, argument.value_type).ok_or_else(|| {
				StyleDiagnostic::new(
					StyleDiagnosticKind::InvalidFunctionArgument {
						function: function.into(),
						index: index + 1,
						expected: format!(
							"a numeric value compatible with {}",
							semantic_type_label(current)
						),
						found: semantic_type_label(argument.value_type).into(),
					},
					argument.span,
				)
			})?,
			None => argument.value_type,
		});
	}
	joined.or(zero_fallback).ok_or_else(|| {
		StyleDiagnostic::new(
			StyleDiagnosticKind::InvalidFunctionArity {
				function: function.into(),
				expected: "at least one numeric argument".into(),
				found: 0,
			},
			span,
		)
	})
}

fn ensure_checked_child(
	expression: &TypedValueExpr,
	context: &str,
	span: Span,
) -> Result<(), StyleDiagnostic> {
	if expression.value_type == SemanticType::Unchecked {
		return Err(StyleDiagnostic::new(
			StyleDiagnosticKind::InvalidUncheckedPlacement {
				context: context.into(),
			},
			span,
		));
	}
	Ok(())
}

fn is_whole_unchecked(expression: &TypedValueExpr) -> bool {
	matches!(expression.kind, TypedValueExprKind::UncheckedFunction(_))
}

fn expression_matches_type(expression: &TypedValueExpr, expected: SemanticType) -> bool {
	if expression.is_contextual_zero()
		&& expected.numeric_dimension().is_some()
		&& !matches!(expected, SemanticType::Angle | SemanticType::Time)
	{
		return true;
	}
	match expected {
		SemanticType::Number => matches!(
			expression.value_type,
			SemanticType::Number | SemanticType::Integer
		),
		SemanticType::LengthPercentage => matches!(
			expression.value_type,
			SemanticType::Length | SemanticType::Percentage | SemanticType::LengthPercentage
		),
		_ => expression.value_type == expected,
	}
}

fn matches_property_grammar(
	expression: &TypedValueExpr,
	grammar: &ValueGrammar,
	css_wide_keywords: &crate::KeywordDomain,
	variable_defaults: &HashMap<usize, &TypedValueExpr>,
) -> bool {
	if keyword_matches(expression, css_wide_keywords) {
		return true;
	}
	matches_grammar(expression, grammar)
		&& component_variable_fallback_matches_grammar(expression, grammar, variable_defaults)
}

fn component_variable_fallback_matches_grammar(
	expression: &TypedValueExpr,
	grammar: &ValueGrammar,
	variable_defaults: &HashMap<usize, &TypedValueExpr>,
) -> bool {
	resolve_component_variable_defaults(expression, variable_defaults)
		.is_some_and(|resolved| matches_grammar(&resolved, grammar))
}

fn component_variable_fallback_matches_property_specific_constraints(
	property: &str,
	expression: &TypedValueExpr,
	variable_defaults: &HashMap<usize, &TypedValueExpr>,
) -> bool {
	resolve_component_variable_defaults(expression, variable_defaults)
		.is_some_and(|resolved| property_specific_constraints_match(property, &resolved))
}

fn resolve_component_variable_defaults(
	expression: &TypedValueExpr,
	variable_defaults: &HashMap<usize, &TypedValueExpr>,
) -> Option<TypedValueExpr> {
	let mut resolved = expression.clone();
	resolved.kind = match &expression.kind {
		TypedValueExprKind::VariableReference(reference) => {
			return resolve_component_variable_defaults(
				variable_defaults.get(&reference.source_index)?,
				variable_defaults,
			);
		}
		TypedValueExprKind::Unary { operator, operand } => TypedValueExprKind::Unary {
			operator: operator.clone(),
			operand: Box::new(resolve_component_variable_defaults(
				operand,
				variable_defaults,
			)?),
		},
		TypedValueExprKind::Binary {
			left,
			operator,
			right,
		} => TypedValueExprKind::Binary {
			left: Box::new(resolve_component_variable_defaults(
				left,
				variable_defaults,
			)?),
			operator: operator.clone(),
			right: Box::new(resolve_component_variable_defaults(
				right,
				variable_defaults,
			)?),
		},
		TypedValueExprKind::Function(call) => TypedValueExprKind::Function(TypedFunctionCall {
			spec: call.spec,
			receiver: match call.receiver.as_deref() {
				Some(receiver) => Some(Box::new(resolve_component_variable_defaults(
					receiver,
					variable_defaults,
				)?)),
				None => None,
			},
			arguments: call
				.arguments
				.iter()
				.map(|argument| resolve_component_variable_defaults(argument, variable_defaults))
				.collect::<Option<Vec<_>>>()?,
		}),
		TypedValueExprKind::Group(operand) => TypedValueExprKind::Group(Box::new(
			resolve_component_variable_defaults(operand, variable_defaults)?,
		)),
		TypedValueExprKind::SpaceSequence(items) => TypedValueExprKind::SpaceSequence(
			items
				.iter()
				.map(|item| resolve_component_variable_defaults(item, variable_defaults))
				.collect::<Option<Vec<_>>>()?,
		),
		TypedValueExprKind::CommaList(items) => TypedValueExprKind::CommaList(
			items
				.iter()
				.map(|item| resolve_component_variable_defaults(item, variable_defaults))
				.collect::<Option<Vec<_>>>()?,
		),
		_ => expression.kind.clone(),
	};
	Some(resolved)
}

fn matches_grammar(expression: &TypedValueExpr, grammar: &ValueGrammar) -> bool {
	match grammar {
		ValueGrammar::Primitive(value_type) | ValueGrammar::FunctionResult(value_type) => {
			expression_matches_type(expression, *value_type)
		}
		ValueGrammar::NonNegative(grammar) => {
			matches_grammar(expression, grammar)
				&& numeric_literal_value(expression).is_none_or(|value| value >= 0.0)
		}
		ValueGrammar::NumericRange {
			grammar,
			minimum,
			maximum,
		} => {
			matches_grammar(expression, grammar)
				&& numeric_literal_value(expression).is_none_or(|value| {
					value >= f64::from(*minimum) && value <= f64::from(*maximum)
				})
		}
		ValueGrammar::Keyword(domain) => keyword_matches(expression, domain),
		ValueGrammar::Identifier => custom_identifier_matches(expression),
		ValueGrammar::IdentifierExcept(excluded) => {
			custom_identifier_matches(expression)
				&& !excluded
					.iter()
					.any(|value| custom_identifier_equals(expression, value))
		}
		ValueGrammar::Or(alternatives) => alternatives
			.iter()
			.any(|alternative| matches_grammar(expression, alternative)),
		ValueGrammar::Space { min, max, item } => {
			let items = sequence_items(expression);
			items.len() >= *min
				&& max.is_none_or(|maximum| items.len() <= maximum)
				&& items.iter().all(|value| matches_grammar(value, item))
		}
		ValueGrammar::Comma { min, item } => {
			let items = comma_items(expression);
			items.len() >= *min && items.iter().all(|value| matches_grammar(value, item))
		}
		ValueGrammar::CommaFinal {
			min,
			item,
			final_item,
		} => {
			let items = comma_items(expression);
			items.len() >= *min
				&& items[..items.len().saturating_sub(1)]
					.iter()
					.all(|value| matches_grammar(value, item))
				&& items
					.last()
					.is_some_and(|value| matches_grammar(value, final_item))
		}
		ValueGrammar::Slash { left, right } => slash_pair(expression)
			.is_some_and(|(a, b)| matches_grammar(a, left) && matches_grammar(b, right)),
		ValueGrammar::SlashList { min, max, item } => {
			let mut items = Vec::new();
			flatten_slash(expression, &mut items);
			items.len() >= *min
				&& items.len() <= *max
				&& items.iter().all(|value| matches_grammar(value, item))
		}
		ValueGrammar::Ordered(members) => {
			let items = sequence_items(expression);
			matches_ordered(&items, members, 0, 0)
		}
		ValueGrammar::Unordered {
			members,
			min_members,
			..
		} => {
			let items = sequence_items(expression);
			if items.len() < *min_members {
				return false;
			}
			if !transition_time_order_is_valid(&items, members) {
				return false;
			}
			let mut used = vec![false; members.len()];
			matches_unordered(&items, members, *min_members, &mut used, 0)
		}
	}
}

fn keyword_matches(expression: &TypedValueExpr, domain: &crate::KeywordDomain) -> bool {
	let TypedValueExprKind::Literal(StyleValueLiteral::Keyword(keyword)) = &expression.kind else {
		return false;
	};
	domain
		.keywords
		.iter()
		.any(|candidate| keyword.as_str().eq_ignore_ascii_case(candidate))
}

fn custom_identifier_matches(expression: &TypedValueExpr) -> bool {
	let TypedValueExprKind::Literal(StyleValueLiteral::Keyword(keyword)) = &expression.kind else {
		return false;
	};
	let value = keyword.as_str();
	!is_css_wide_keyword(value)
		&& value.bytes().next().is_some_and(|byte| {
			byte.is_ascii_alphabetic()
				|| byte == b'_'
				|| (byte == b'-'
					&& value.as_bytes().get(1).copied().is_some_and(|next| {
						next.is_ascii_alphabetic() || next == b'_' || next == b'-'
					}))
		}) && value
		.bytes()
		.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn custom_identifier_equals(expression: &TypedValueExpr, expected: &str) -> bool {
	let TypedValueExprKind::Literal(StyleValueLiteral::Keyword(keyword)) = &expression.kind else {
		return false;
	};
	keyword.as_str().eq_ignore_ascii_case(expected)
}

fn is_css_wide_keyword(value: &str) -> bool {
	["inherit", "initial", "unset", "revert", "revert-layer"]
		.iter()
		.any(|keyword| value.eq_ignore_ascii_case(keyword))
}

fn is_negative_literal(expression: &TypedValueExpr) -> bool {
	numeric_literal_value(expression).is_some_and(|value| value < 0.0)
}

fn numeric_literal_value(expression: &TypedValueExpr) -> Option<f64> {
	match &expression.kind {
		TypedValueExprKind::Literal(StyleValueLiteral::Integer(number))
		| TypedValueExprKind::Literal(StyleValueLiteral::Number(number)) => {
			number.source.replace('_', "").parse().ok()
		}
		TypedValueExprKind::Unary { operator, operand } => {
			let value = numeric_literal_value(operand)?;
			match operator.kind {
				StyleUnaryOperatorKind::Minus => Some(-value),
				StyleUnaryOperatorKind::Plus => Some(value),
			}
		}
		TypedValueExprKind::Binary {
			left,
			operator,
			right,
		} => {
			let left = numeric_literal_value(left)?;
			let right = numeric_literal_value(right)?;
			match operator.kind {
				StyleBinaryOperatorKind::Add => Some(left + right),
				StyleBinaryOperatorKind::Subtract => Some(left - right),
				StyleBinaryOperatorKind::Multiply => Some(left * right),
				StyleBinaryOperatorKind::Divide => Some(left / right),
			}
		}
		TypedValueExprKind::Group(operand) => numeric_literal_value(operand),
		_ => None,
	}
}

fn property_specific_constraints_match(property: &str, value: &TypedValueExpr) -> bool {
	match property {
		"font-style" => font_style_oblique_angle_is_valid(value),
		"grid-column" | "grid-row" | "grid-area" => grid_line_numbers_are_nonzero(value),
		"grid-template-areas" => grid_template_areas_are_rectangular(value),
		"background" => background_position_and_size_are_not_split(value),
		"text-decoration" => text_decoration_lines_are_unique(value),
		"box-shadow" => box_shadow_lengths_are_contiguous(value),
		_ => true,
	}
}

fn font_style_oblique_angle_is_valid(value: &TypedValueExpr) -> bool {
	let items = sequence_items(value);
	if items.len() != 2 || !keyword_equals(items[0], "oblique") {
		return true;
	}
	numeric_angle_degrees(items[1]).is_none_or(|degrees| (-90.0..=90.0).contains(&degrees))
}

fn numeric_angle_degrees(expression: &TypedValueExpr) -> Option<f64> {
	match &expression.kind {
		TypedValueExprKind::Literal(StyleValueLiteral::Integer(number))
		| TypedValueExprKind::Literal(StyleValueLiteral::Number(number)) => {
			let value = number.source.replace('_', "").parse::<f64>().ok()?;
			let StyleNumericUnit::Named(unit) = number.unit.as_ref()? else {
				return None;
			};
			match unit.as_str().to_ascii_lowercase().as_str() {
				"deg" => Some(value),
				"grad" => Some(value * 0.9),
				"rad" => Some(value * 180.0 / std::f64::consts::PI),
				"turn" => Some(value * 360.0),
				_ => None,
			}
		}
		TypedValueExprKind::Unary { operator, operand } => {
			numeric_angle_degrees(operand).map(|value| match operator.kind {
				StyleUnaryOperatorKind::Minus => -value,
				StyleUnaryOperatorKind::Plus => value,
			})
		}
		TypedValueExprKind::Group(inner) => numeric_angle_degrees(inner),
		_ => None,
	}
}

fn grid_line_numbers_are_nonzero(expression: &TypedValueExpr) -> bool {
	match &expression.kind {
		TypedValueExprKind::Literal(StyleValueLiteral::Integer(_)) => {
			numeric_literal_value(expression).is_none_or(|value| value != 0.0)
		}
		TypedValueExprKind::Unary { operand, .. } | TypedValueExprKind::Group(operand) => {
			grid_line_numbers_are_nonzero(operand)
		}
		TypedValueExprKind::SpaceSequence(items) | TypedValueExprKind::CommaList(items) => {
			items.iter().all(grid_line_numbers_are_nonzero)
		}
		TypedValueExprKind::Function(call) if call.spec.dsl_path == "slash" => {
			call.arguments.iter().all(grid_line_numbers_are_nonzero)
		}
		_ => true,
	}
}

fn grid_template_areas_are_rectangular(value: &TypedValueExpr) -> bool {
	let rows = sequence_items(value)
		.into_iter()
		.map(grid_template_area_cells)
		.collect::<Option<Vec<_>>>();
	let Some(rows) = rows else {
		return true;
	};
	let Some(width) = rows.first().map(Vec::len) else {
		return true;
	};
	if width == 0 || rows.iter().any(|row| row.len() != width) {
		return false;
	}

	let mut bounds: HashMap<&str, (usize, usize, usize, usize, usize)> = HashMap::new();
	for (row_index, row) in rows.iter().enumerate() {
		for (column_index, cell) in row.iter().enumerate() {
			if !grid_template_area_cell_is_valid(cell) {
				return false;
			}
			if cell.bytes().all(|byte| byte == b'.') {
				continue;
			}
			bounds
				.entry(*cell)
				.and_modify(|(min_row, max_row, min_column, max_column, count)| {
					*min_row = (*min_row).min(row_index);
					*max_row = (*max_row).max(row_index);
					*min_column = (*min_column).min(column_index);
					*max_column = (*max_column).max(column_index);
					*count += 1;
				})
				.or_insert((row_index, row_index, column_index, column_index, 1_usize));
		}
	}

	bounds
		.into_values()
		.all(|(min_row, max_row, min_column, max_column, count)| {
			(max_row - min_row + 1) * (max_column - min_column + 1) == count
		})
}

fn grid_template_area_cells(expression: &TypedValueExpr) -> Option<Vec<&str>> {
	let TypedValueExprKind::Literal(StyleValueLiteral::String(string)) = &expression.kind else {
		return None;
	};
	Some(string.value.split_ascii_whitespace().collect())
}

fn grid_template_area_cell_is_valid(cell: &str) -> bool {
	!cell.is_empty()
		&& (cell.bytes().all(|byte| byte == b'.')
			|| (!is_css_wide_keyword(cell)
				&& cell.bytes().next().is_some_and(|byte| {
					byte.is_ascii_alphabetic()
						|| byte == b'_' || (byte == b'-'
						&& cell.as_bytes().get(1).copied().is_some_and(|next| {
							next.is_ascii_alphabetic() || next == b'_' || next == b'-'
						}))
				}) && cell
				.bytes()
				.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))))
}

fn background_position_and_size_are_not_split(value: &TypedValueExpr) -> bool {
	comma_items(value).into_iter().all(|layer| {
		let items = sequence_items(layer);
		let has_position_size = items.iter().any(|item| {
			slash_pair(item).is_some_and(|(left, _)| is_background_position_component(left))
		});
		!has_position_size
			|| !items
				.iter()
				.filter(|item| slash_pair(item).is_none())
				.any(|item| is_background_position_component(item))
	})
}

fn is_background_position_component(expression: &TypedValueExpr) -> bool {
	keyword_equals(expression, "left")
		|| keyword_equals(expression, "right")
		|| keyword_equals(expression, "top")
		|| keyword_equals(expression, "bottom")
		|| keyword_equals(expression, "center")
		|| matches!(
			expression.value_type,
			SemanticType::Length | SemanticType::LengthPercentage | SemanticType::Percentage
		) || expression.is_contextual_zero()
}

fn text_decoration_lines_are_unique(value: &TypedValueExpr) -> bool {
	let mut lines = HashSet::new();
	sequence_items(value)
		.into_iter()
		.filter_map(keyword_value)
		.filter(|keyword| matches!(*keyword, "underline" | "overline" | "line-through"))
		.all(|keyword| lines.insert(keyword))
}

fn box_shadow_lengths_are_contiguous(value: &TypedValueExpr) -> bool {
	comma_items(value).into_iter().all(|shadow| {
		let indices = sequence_items(shadow)
			.iter()
			.enumerate()
			.filter_map(|(index, item)| {
				(item.value_type == SemanticType::Length || item.is_contextual_zero())
					.then_some(index)
			})
			.collect::<Vec<_>>();
		indices.windows(2).all(|pair| pair[1] == pair[0] + 1)
	})
}

fn keyword_equals(expression: &TypedValueExpr, expected: &str) -> bool {
	keyword_value(expression).is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

fn keyword_value(expression: &TypedValueExpr) -> Option<&str> {
	let TypedValueExprKind::Literal(StyleValueLiteral::Keyword(keyword)) = &expression.kind else {
		return None;
	};
	Some(keyword.as_str())
}

fn transition_time_order_is_valid(
	items: &[&TypedValueExpr],
	members: &[crate::GrammarMember],
) -> bool {
	let Some(duration) = members.iter().find(|member| member.role == "duration") else {
		return true;
	};
	let Some(delay) = members.iter().find(|member| member.role == "delay") else {
		return true;
	};
	let time_values: Vec<_> = items
		.iter()
		.copied()
		.filter(|item| matches_grammar(item, delay.grammar))
		.collect();
	time_values
		.first()
		.is_none_or(|first| matches_grammar(first, duration.grammar))
}

fn sequence_items(expression: &TypedValueExpr) -> Vec<&TypedValueExpr> {
	match &expression.kind {
		TypedValueExprKind::SpaceSequence(items) => items.iter().collect(),
		_ => vec![expression],
	}
}

fn comma_items(expression: &TypedValueExpr) -> Vec<&TypedValueExpr> {
	match &expression.kind {
		TypedValueExprKind::CommaList(items) => items.iter().collect(),
		_ => vec![expression],
	}
}

fn slash_pair(expression: &TypedValueExpr) -> Option<(&TypedValueExpr, &TypedValueExpr)> {
	let TypedValueExprKind::Function(call) = &expression.kind else {
		return None;
	};
	if call.spec.dsl_path != "slash" {
		return None;
	}
	let [left, right] = call.arguments.as_slice() else {
		return None;
	};
	Some((left, right))
}

fn flatten_slash<'a>(expression: &'a TypedValueExpr, output: &mut Vec<&'a TypedValueExpr>) {
	if let Some((left, right)) = slash_pair(expression) {
		flatten_slash(left, output);
		flatten_slash(right, output);
	} else {
		output.push(expression);
	}
}

fn matches_ordered(
	items: &[&TypedValueExpr],
	members: &[GrammarMember],
	item_index: usize,
	member_index: usize,
) -> bool {
	if member_index == members.len() {
		return item_index == items.len();
	}
	let member = &members[member_index];
	if member.optional && matches_ordered(items, members, item_index, member_index + 1) {
		return true;
	}
	matching_prefix_lengths(&items[item_index..], member.grammar)
		.into_iter()
		.any(|consumed| matches_ordered(items, members, item_index + consumed, member_index + 1))
}

fn matches_unordered(
	items: &[&TypedValueExpr],
	members: &[GrammarMember],
	min_members: usize,
	used: &mut [bool],
	item_index: usize,
) -> bool {
	if item_index == items.len() {
		return used.iter().filter(|is_used| **is_used).count() >= min_members
			&& members
				.iter()
				.enumerate()
				.all(|(index, member)| member.optional || used[index]);
	}
	for member_index in 0..members.len() {
		if used[member_index] {
			continue;
		}
		for consumed in matching_prefix_lengths(&items[item_index..], members[member_index].grammar)
		{
			used[member_index] = true;
			if matches_unordered(items, members, min_members, used, item_index + consumed) {
				return true;
			}
			used[member_index] = false;
		}
	}
	false
}

fn matching_prefix_lengths(items: &[&TypedValueExpr], grammar: &ValueGrammar) -> Vec<usize> {
	let Some(first) = items.first() else {
		return Vec::new();
	};
	let mut lengths = Vec::new();
	if matches_grammar(first, grammar) {
		lengths.push(1);
	}
	match grammar {
		ValueGrammar::Or(alternatives) => {
			for alternative in *alternatives {
				lengths.extend(matching_prefix_lengths(items, alternative));
			}
		}
		ValueGrammar::Space { min, max, item } => {
			let maximum = max.unwrap_or(items.len()).min(items.len());
			for count in 1..=maximum {
				if !matches_grammar(items[count - 1], item) {
					break;
				}
				if count >= *min {
					lengths.push(count);
				}
			}
		}
		ValueGrammar::Ordered(members) => {
			for count in 1..=items.len() {
				if matches_ordered(&items[..count], members, 0, 0) {
					lengths.push(count);
				}
			}
		}
		ValueGrammar::Unordered {
			members,
			min_members,
			..
		} => {
			for count in *min_members..=items.len() {
				let mut used = vec![false; members.len()];
				if matches_unordered(&items[..count], members, *min_members, &mut used, 0) {
					lengths.push(count);
				}
			}
		}
		ValueGrammar::Primitive(_)
		| ValueGrammar::NonNegative(_)
		| ValueGrammar::NumericRange { .. }
		| ValueGrammar::Keyword(_)
		| ValueGrammar::Identifier
		| ValueGrammar::IdentifierExcept(_)
		| ValueGrammar::FunctionResult(_)
		| ValueGrammar::Comma { .. }
		| ValueGrammar::CommaFinal { .. }
		| ValueGrammar::Slash { .. }
		| ValueGrammar::SlashList { .. } => {}
	}
	lengths.sort_unstable();
	lengths.dedup();
	lengths
}

fn direction_value(path: &str) -> Option<Direction> {
	match path {
		"Direction::Top" => Some(Direction::Top),
		"Direction::TopRight" => Some(Direction::TopRight),
		"Direction::Right" => Some(Direction::Right),
		"Direction::BottomRight" => Some(Direction::BottomRight),
		"Direction::Bottom" => Some(Direction::Bottom),
		"Direction::BottomLeft" => Some(Direction::BottomLeft),
		"Direction::Left" => Some(Direction::Left),
		"Direction::TopLeft" => Some(Direction::TopLeft),
		_ => None,
	}
}

fn value_path_name(path: &crate::StyleValuePath) -> String {
	path.segments
		.iter()
		.map(|segment| segment.as_str())
		.collect::<Vec<_>>()
		.join("::")
}

fn arity_accepts(policy: ArityPolicy, count: usize) -> bool {
	match policy {
		ArityPolicy::Exact(expected) => count == expected,
		ArityPolicy::AtLeast(minimum) => count >= minimum,
	}
}

fn arity_description(policy: ArityPolicy) -> String {
	match policy {
		ArityPolicy::Exact(count) => format!("exactly {count}"),
		ArityPolicy::AtLeast(count) => format!("at least {count}"),
	}
}

fn constraint_description(constraint: TypeConstraint) -> String {
	match constraint {
		TypeConstraint::Exact(value_type) => semantic_type_label(value_type).into(),
		TypeConstraint::Numeric(NumericConstraint::NumberOrPercentage) => {
			"Number or Percentage".into()
		}
		TypeConstraint::Numeric(NumericConstraint::PercentageRange { minimum, maximum }) => {
			format!("Percentage between {minimum} and {maximum}")
		}
		TypeConstraint::Numeric(NumericConstraint::Joined) => "a joinable numeric value".into(),
		TypeConstraint::CommaList { element, min } => format!(
			"a comma list of at least {min} {} values",
			semantic_type_label(element)
		),
		TypeConstraint::Any => "a checked value".into(),
	}
}

fn invalid_arithmetic(
	operation: &str,
	left: SemanticType,
	right: &str,
	span: Span,
) -> StyleDiagnostic {
	StyleDiagnostic::new(
		StyleDiagnosticKind::InvalidArithmeticDimensions {
			operation: operation.into(),
			left: semantic_type_label(left).into(),
			right: right.into(),
		},
		span,
	)
}

fn binary_symbol(operator: StyleBinaryOperatorKind) -> &'static str {
	match operator {
		StyleBinaryOperatorKind::Add => "+",
		StyleBinaryOperatorKind::Subtract => "-",
		StyleBinaryOperatorKind::Multiply => "*",
		StyleBinaryOperatorKind::Divide => "/",
	}
}

fn semantic_type_label(value_type: SemanticType) -> &'static str {
	match value_type {
		SemanticType::Color => "Color",
		SemanticType::Length => "Length",
		SemanticType::LengthPercentage => "LengthPercentage",
		SemanticType::Percentage => "Percentage",
		SemanticType::Angle => "Angle",
		SemanticType::Time => "Time",
		SemanticType::Number => "Number",
		SemanticType::Integer => "Integer",
		SemanticType::GridFraction => "GridFraction",
		SemanticType::QuotedString => "QuotedString",
		SemanticType::CustomIdentifier => "CustomIdentifier",
		SemanticType::Keyword => "Keyword",
		SemanticType::Direction => "Direction",
		SemanticType::GradientStop => "GradientStop",
		SemanticType::Image => "Image",
		SemanticType::TransformFunction => "TransformFunction",
		SemanticType::SpaceSequence => "SpaceSequence",
		SemanticType::CommaList => "CommaList",
		SemanticType::SlashPair => "SlashPair",
		SemanticType::Unchecked => "Unchecked",
	}
}

#[cfg(test)]
mod tests {
	use quote::quote;
	use rstest::rstest;

	use super::{expression_matches_type, infer_expression, matches_grammar};
	use crate::{
		GrammarMember, SemanticType, StyleDiagnosticKind, TypedStyleItem, TypedStyleRuleItem,
		TypedValueExpr, TypedValueExprKind, ValueGrammar, parser::parse_style,
		validator::validate_style,
	};

	static TEST_COLOR_GRAMMAR: ValueGrammar = ValueGrammar::Primitive(SemanticType::Color);
	static TEST_NUMBER_GRAMMAR: ValueGrammar = ValueGrammar::Primitive(SemanticType::Number);
	static TEST_COLOR_PAIR_GRAMMAR: ValueGrammar = ValueGrammar::Space {
		min: 2,
		max: Some(2),
		item: &TEST_COLOR_GRAMMAR,
	};
	static TEST_UNORDERED_MEMBERS: &[GrammarMember] = &[
		GrammarMember {
			role: "colors",
			grammar: &TEST_COLOR_PAIR_GRAMMAR,
			optional: true,
		},
		GrammarMember {
			role: "number",
			grammar: &TEST_NUMBER_GRAMMAR,
			optional: true,
		},
	];
	static TEST_TWO_ROLE_GRAMMAR: ValueGrammar = ValueGrammar::Unordered {
		members: TEST_UNORDERED_MEMBERS,
		min_members: 2,
		preserve_source_order: false,
	};

	fn validated(input: proc_macro2::TokenStream) -> crate::TypedStyleMacro {
		let ast = parse_style(input).expect("test style should parse");
		validate_style(&ast).expect("test style should pass semantic validation")
	}

	fn validated_text(input: &str) -> crate::TypedStyleMacro {
		validated(input.parse().expect("test tokens should parse"))
	}

	fn first_declaration_value(input: &str) -> TypedValueExpr {
		let typed = validated_text(input);
		let TypedStyleItem::Rule(rule) = &typed.items[0] else {
			panic!("expected a typed style rule");
		};
		let TypedStyleRuleItem::Declaration(declaration) = &rule.items[0] else {
			panic!("expected a typed declaration");
		};
		declaration.value.clone()
	}

	fn first_variable_default(input: &str) -> TypedValueExpr {
		validated_text(input).variables[0].default.clone()
	}

	fn diagnostic_kind_text(input: &str) -> StyleDiagnosticKind {
		diagnostic_kind(input.parse().expect("test tokens should parse"))
	}

	fn diagnostic_kind(input: proc_macro2::TokenStream) -> StyleDiagnosticKind {
		let ast = parse_style(input).expect("test style should parse");
		validate_style(&ast)
			.expect_err("test style should fail semantic validation")
			.kind
	}

	#[rstest]
	fn rejects_cross_dimension_addition() {
		// Arrange and Act
		let kind = diagnostic_kind(quote! { .card { width: 1deg + 1px; } });

		// Assert
		assert_eq!(
			kind,
			StyleDiagnosticKind::InvalidArithmeticDimensions {
				operation: "+".into(),
				left: "Angle".into(),
				right: "Length".into(),
			}
		);
	}

	#[rstest]
	fn rejects_color_arithmetic() {
		// Arrange
		let input = ".card { color: #fff + #000; }"
			.parse()
			.expect("test tokens should parse");

		// Act
		let kind = diagnostic_kind(input);

		// Assert
		assert_eq!(
			kind,
			StyleDiagnosticKind::InvalidArithmeticDimensions {
				operation: "+".into(),
				left: "Color".into(),
				right: "Color".into(),
			}
		);
	}

	#[rstest]
	fn rejects_dimension_by_dimension_multiplication() {
		// Arrange and Act
		let kind = diagnostic_kind(quote! { .card { width: 2px * 3px; } });

		// Assert
		assert_eq!(
			kind,
			StyleDiagnosticKind::InvalidArithmeticDimensions {
				operation: "*".into(),
				left: "Length".into(),
				right: "Length".into(),
			}
		);
	}

	#[rstest]
	fn rejects_dimension_by_dimension_division() {
		// Arrange and Act
		let kind = diagnostic_kind(quote! { .card { width: 2px / 3px; } });

		// Assert
		assert_eq!(
			kind,
			StyleDiagnosticKind::InvalidArithmeticDimensions {
				operation: "/".into(),
				left: "Length".into(),
				right: "Length".into(),
			}
		);
	}

	#[rstest]
	fn rejects_literal_division_by_zero() {
		// Arrange and Act
		let kind = diagnostic_kind(quote! { .card { width: 2px / 0; } });

		// Assert
		assert_eq!(
			kind,
			StyleDiagnosticKind::InvalidArithmeticDimensions {
				operation: "/".into(),
				left: "Length".into(),
				right: "literal zero".into(),
			}
		);
	}

	#[rstest]
	fn rejects_direct_calc_call() {
		// Arrange and Act
		let kind = diagnostic_kind(quote! { .card { width: calc(100% - 1rem); } });

		// Assert
		assert_eq!(kind, StyleDiagnosticKind::DirectCalcCall);
	}

	#[rstest]
	fn rejects_direct_var_call() {
		// Arrange and Act
		let kind = diagnostic_kind(quote! { .card { color: var(accent); } });

		// Assert
		assert_eq!(kind, StyleDiagnosticKind::DirectVarCall);
	}

	#[rstest]
	fn rejects_unknown_function_with_stable_kind() {
		// Arrange and Act
		let kind = diagnostic_kind(quote! { .card { background: paint(worklet); } });

		// Assert
		assert_eq!(
			kind,
			StyleDiagnosticKind::UnknownFunction {
				name: "paint".into(),
			}
		);
	}

	#[rstest]
	fn rejects_unregistered_property() {
		// Arrange
		let property = "colour";
		let input: proc_macro2::TokenStream = ".card { colour: red; }"
			.parse()
			.expect("test tokens should parse");

		// Act
		let kind = diagnostic_kind(input);

		// Assert
		assert_eq!(
			kind,
			StyleDiagnosticKind::UnknownProperty {
				name: property.into(),
			}
		);
	}

	#[rstest]
	#[case("--accent", "CSS custom property names are not supported")]
	#[case("-webkit-mask", "vendor-prefixed CSS names are not supported")]
	fn rejects_non_registry_property_surfaces_during_parsing(
		#[case] property: &str,
		#[case] expected: &str,
	) {
		// Arrange
		let input: proc_macro2::TokenStream = format!(".card {{ {property}: red; }}")
			.parse()
			.expect("test tokens should parse");

		// Act
		let error = parse_style(input).expect_err("property should be rejected");

		// Assert
		assert_eq!(error.to_string(), expected);
	}

	#[rstest]
	#[case("Integer", "1 + 2", SemanticType::Integer)]
	#[case("Integer", "3 - 2", SemanticType::Integer)]
	#[case("Integer", "2 * 3", SemanticType::Integer)]
	#[case("Number", "1 / 2", SemanticType::Number)]
	#[case("Number", "1 + 2.5", SemanticType::Number)]
	#[case("Number", "2.5 + 1", SemanticType::Number)]
	#[case("Number", "2.5 - 1", SemanticType::Number)]
	#[case("Number", "2 * 1.5", SemanticType::Number)]
	#[case("Length", "1px + 2px", SemanticType::Length)]
	#[case("Length", "3rem - 1rem", SemanticType::Length)]
	#[case("Percentage", "10% + 20%", SemanticType::Percentage)]
	#[case("Angle", "1deg + 2deg", SemanticType::Angle)]
	#[case("Time", "1s - 200ms", SemanticType::Time)]
	#[case("LengthPercentage", "1px + 2%", SemanticType::LengthPercentage)]
	#[case("LengthPercentage", "10% + 2px", SemanticType::LengthPercentage)]
	#[case("LengthPercentage", "1rem + 2% + 3px", SemanticType::LengthPercentage)]
	#[case("LengthPercentage", "(1px + 2%) + 3%", SemanticType::LengthPercentage)]
	#[case("LengthPercentage", "3% + (1px + 2%)", SemanticType::LengthPercentage)]
	#[case("LengthPercentage", "(1px + 2%) + 3px", SemanticType::LengthPercentage)]
	#[case("LengthPercentage", "3px + (1px + 2%)", SemanticType::LengthPercentage)]
	#[case("Length", "2px * 3", SemanticType::Length)]
	#[case("Length", "3 * 2px", SemanticType::Length)]
	#[case("Length", "6px / 3", SemanticType::Length)]
	#[case("Percentage", "30% / 2", SemanticType::Percentage)]
	#[case("Angle", "2 * 45deg", SemanticType::Angle)]
	#[case("Time", "3s / 2", SemanticType::Time)]
	fn implements_complete_numeric_arithmetic_lattice(
		#[case] declared_type: &str,
		#[case] expression: &str,
		#[case] expected: SemanticType,
	) {
		// Arrange
		let source = format!("vars {{ value: {declared_type} = {expression}; }} .card {{}}");

		// Act
		let value = first_variable_default(&source);

		// Assert
		assert_eq!(value.value_type, expected);
		assert!(value.contains_arithmetic);
	}

	#[rstest]
	#[case("min(1px, 2%, 3rem)")]
	#[case("max(1px, 2%, 3rem)")]
	#[case("clamp(1px, 2%, 3rem)")]
	#[case("min(0, 2%, 3rem)")]
	fn numeric_functions_join_length_and_percentage(#[case] expression: &str) {
		// Arrange
		let source = format!("vars {{ value: LengthPercentage = {expression}; }} .card {{}}");

		// Act
		let value = first_variable_default(&source);

		// Assert
		assert_eq!(value.value_type, SemanticType::LengthPercentage);
		assert!(!value.contains_arithmetic);
	}

	#[rstest]
	fn grid_fraction_uses_the_same_dimension_arithmetic_rules() {
		// Arrange
		let source = ".card { grid-template-columns: 1fr + 2fr * 3; }";

		// Act
		let value = first_declaration_value(source);

		// Assert
		assert_eq!(value.value_type, SemanticType::GridFraction);
		assert!(value.contains_arithmetic);
	}

	#[rstest]
	fn signed_atoms_and_compound_negation_have_distinct_calc_boundaries() {
		// Arrange
		let atom = "vars { value: Length = -1px; } .card {}";
		let compound = "vars { value: Length = -(1px + 2px); } .card {}";

		// Act
		let atom = first_variable_default(atom);
		let compound = first_variable_default(compound);

		// Assert
		assert!(!atom.contains_arithmetic);
		assert!(compound.contains_arithmetic);
	}

	#[rstest]
	#[case(SemanticType::Length)]
	#[case(SemanticType::LengthPercentage)]
	#[case(SemanticType::Percentage)]
	#[case(SemanticType::Number)]
	#[case(SemanticType::Integer)]
	#[case(SemanticType::GridFraction)]
	fn unitless_zero_is_contextually_accepted_for_non_angle_non_time_numeric_dimensions(
		#[case] expected: SemanticType,
	) {
		// Arrange
		let ast = parse_style(quote! { .card { opacity: 0; } }).unwrap();
		let crate::StyleItem::Rule(rule) = &ast.items[0] else {
			panic!("expected a style rule");
		};
		let crate::StyleRuleItem::Declaration(declaration) = &rule.items[0] else {
			panic!("expected a declaration");
		};

		// Act
		let zero = infer_expression(
			&declaration.value,
			&std::collections::HashMap::new(),
			&std::collections::HashMap::new(),
		)
		.unwrap();

		// Assert
		assert!(expression_matches_type(&zero, expected));
	}

	#[rstest]
	fn contextual_zero_flows_through_variables_properties_and_functions() {
		// Arrange
		let source = "
			vars {
				color: Color = transparent;
				length: Length = 0;
				length_percentage: LengthPercentage = 0;
				percentage: Percentage = 0;
				time: Time = 0s;
				number: Number = 0;
				integer: Integer = 0;
			}
			.card {
				outline-offset: 0;
				width: 0;
				opacity: 0;
				z-index: 0;
				transition-duration: 0s;
				grid-template-columns: 0;
			}
		";

		// Act
		let typed = validated_text(source);

		// Assert
		assert_eq!(typed.variables.len(), 7);
		assert_eq!(typed.items.len(), 1);
	}

	#[rstest]
	#[case("vars { angle: Angle = 0; } .card {}", "vars.angle")]
	#[case(".card { font-style: (oblique, 0); }", "font-style")]
	fn rejects_unitless_zero_in_angle_property_slots(#[case] source: &str, #[case] property: &str) {
		// Arrange and Act
		let kind = diagnostic_kind_text(source);

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { property: actual, .. }
				if actual == property
		));
	}

	#[rstest]
	fn rejects_unitless_zero_in_rotate() {
		// Arrange and Act
		let kind = diagnostic_kind_text(".card { transform: rotate(0); }");

		// Assert
		assert_eq!(
			kind,
			StyleDiagnosticKind::InvalidFunctionArgument {
				function: "rotate".into(),
				index: 1,
				expected: "Angle".into(),
				found: "Integer".into(),
			}
		);
	}

	#[rstest]
	#[case("0")]
	#[case("0.0")]
	#[case("(0)")]
	#[case("-0")]
	#[case("+0")]
	fn signed_and_grouped_literal_zero_remains_contextual(#[case] expression: &str) {
		// Arrange
		let source = format!("vars {{ value: Length = {expression}; }} .card {{}}");

		// Act
		let value = first_variable_default(&source);

		// Assert
		assert!(value.is_contextual_zero());
	}

	#[rstest]
	#[case("Length")]
	#[case("LengthPercentage")]
	#[case("Percentage")]
	#[case("Angle")]
	#[case("Time")]
	fn nonzero_unitless_number_does_not_satisfy_a_dimension(#[case] expected: &str) {
		// Arrange
		let source = format!("vars {{ value: {expected} = 1; }} .card {{}}");

		// Act
		let kind = diagnostic_kind_text(&source);

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { .. }
		));
	}

	#[rstest]
	fn references_retain_resolved_type_and_source_index() {
		// Arrange
		let source = "
			globals { external: Color; }
			vars {
				base: Length = 1rem;
				alias: Length = vars.base;
				accent: Color = globals.external;
			}
			.card {}
		";

		// Act
		let typed = validated_text(source);

		// Assert
		let TypedValueExprKind::VariableReference(reference) = &typed.variables[1].default.kind
		else {
			panic!("expected a typed variable reference");
		};
		assert_eq!(reference.source_index, 0);
		assert_eq!(reference.value_type, SemanticType::Length);
		assert_eq!(reference.css_name, "base");
		let TypedValueExprKind::GlobalReference(reference) = &typed.variables[2].default.kind
		else {
			panic!("expected a typed global reference");
		};
		assert_eq!(reference.source_index, 0);
		assert_eq!(typed.variables[2].value_type, SemanticType::Color);
		assert_eq!(reference.css_name, "external");
	}

	#[rstest]
	#[case(".card { color: Color::rgb(1, 2); }")]
	#[case(".card { width: min(1px); }")]
	#[case(".card { transform: clamp(1px, 2px); }")]
	fn rejects_invalid_function_arity(#[case] source: &str) {
		// Arrange and Act
		let kind = diagnostic_kind_text(source);

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::InvalidFunctionArity { .. }
		));
	}

	#[rstest]
	#[case("Color::rgb(1, 2%, 3)")]
	#[case("Color::rgb(1%, 2, 3%)")]
	#[case("Color::rgb(1%, 2%, 3)")]
	fn rgb_accepts_each_number_or_percentage_channel_mix(#[case] expression: &str) {
		// Arrange
		let source = format!(".card {{ color: {expression}; }}");

		// Act
		let value = first_declaration_value(&source);

		// Assert
		assert_eq!(value.value_type, SemanticType::Color);
	}

	#[rstest]
	fn rejects_invalid_rgb_channel_type() {
		// Arrange and Act
		let kind = diagnostic_kind(quote! { .card { color: Color::rgb(1px, 2, 3); } });

		// Assert
		assert_eq!(
			kind,
			StyleDiagnosticKind::InvalidFunctionArgument {
				function: "Color::rgb".into(),
				index: 1,
				expected: "Number or Percentage".into(),
				found: "Length".into(),
			}
		);
	}

	#[rstest]
	fn rejects_mix_on_a_non_color_receiver() {
		// Arrange and Act
		let kind = diagnostic_kind_text(".card { color: 1px.mix(red, 20%); }");

		// Assert
		assert_eq!(
			kind,
			StyleDiagnosticKind::InvalidReceiverMethod {
				receiver: "Length".into(),
				method: "mix".into(),
			}
		);
	}

	#[rstest]
	fn rejects_a_gradient_with_fewer_than_two_stops() {
		// Arrange and Act
		let kind = diagnostic_kind_text(
			".card { background-image: linear_gradient(Direction::Right, [stop(red, 0%)]); }",
		);

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::InvalidFunctionArgument {
				function,
				index: 2,
				..
			} if function == "linear_gradient"
		));
	}

	#[rstest]
	fn rejects_a_value_with_the_wrong_property_result_type() {
		// Arrange
		let input = ".card { width: #fff; }"
			.parse()
			.expect("test tokens should parse");

		// Act
		let kind = diagnostic_kind(input);

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { property, found, .. }
				if property == "width" && found == "Color"
		));
	}

	#[rstest]
	fn rejects_a_component_variable_with_a_negative_fallback_in_a_nonnegative_slot() {
		// Arrange and Act
		let kind =
			diagnostic_kind_text("vars { gap: Length = -1px; } .card { padding: vars.gap; }");

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { property, .. } if property == "padding"
		));
	}

	#[rstest]
	fn rejects_a_component_variable_with_a_negative_fallback_inside_a_sequence() {
		// Arrange and Act
		let kind = diagnostic_kind_text(
			"vars { gap: Length = -1px; } .card { padding: (vars.gap, 1px); }",
		);

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { property, .. } if property == "padding"
		));
	}

	#[rstest]
	#[case("(vars.gap, 1px)")]
	#[case("min(vars.gap, 1px)")]
	fn nested_component_variable_references_inherit_nonnegative_constraints(#[case] value: &str) {
		// Arrange
		let source = format!("vars {{ gap: Length = 1px; }} .card {{ padding: {value}; }}");

		// Act
		let typed = validated_text(&source);

		// Assert
		assert_eq!(
			typed.variables[0].runtime_constraint,
			Some(crate::StyleVariableConstraint::NonNegative)
		);
	}

	#[rstest]
	#[case(
		"vars { line: Integer = 0; } .card { grid-column: vars.line; }",
		"grid-column"
	)]
	#[case(
		"vars { slant: Angle = 91deg; } .card { font-style: (oblique, vars.slant); }",
		"font-style"
	)]
	fn rejects_component_variable_fallbacks_that_violate_property_specific_constraints(
		#[case] source: &str,
		#[case] property: &str,
	) {
		// Arrange and Act
		let kind = diagnostic_kind_text(source);

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { property: actual, .. }
				if actual == property
		));
	}

	#[rstest]
	#[case(".card { width: 1px + 0; }")]
	#[case(".card { width: 0 + 1px; }")]
	#[case(".card { width: 1px - 0; }")]
	#[case(".card { transform: rotate(0deg + 0); }")]
	fn rejects_contextual_zero_as_a_math_operand(#[case] source: &str) {
		// Arrange and Act
		let kind = diagnostic_kind_text(source);

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::InvalidArithmeticDimensions { .. }
		));
	}

	#[rstest]
	fn whole_unchecked_values_use_the_surrounding_contract() {
		// Arrange
		let source = "
			vars { accent: Color = unchecked_fn!(paint(accent)); }
			.card { background: unchecked_fn!(paint(surface)); }
		";

		// Act
		let typed = validated_text(source);

		// Assert
		assert_eq!(typed.variables[0].value_type, SemanticType::Color);
		assert_eq!(
			typed.variables[0].default.value_type,
			SemanticType::Unchecked
		);
	}

	#[rstest]
	#[case("vars { value: Length = unchecked_fn!(paint(x)) + 1px; } .card {}")]
	#[case("vars { value: Color = unchecked_fn!(paint(x)).mix(red, 20%); } .card {}")]
	#[case(".card { border: (unchecked_fn!(paint(x)), solid); }")]
	#[case(".card { background-image: [unchecked_fn!(paint(x))]; }")]
	#[case(".card { color: Color::rgb(unchecked_fn!(paint(x)), 0, 0); }")]
	#[case(".card { grid-column: slash(unchecked_fn!(paint(x)), 1); }")]
	fn rejects_unchecked_inside_checked_composites(#[case] source: &str) {
		// Arrange and Act
		let kind = diagnostic_kind_text(source);

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::InvalidUncheckedPlacement { .. }
		));
	}

	#[rstest]
	fn rejects_an_unknown_declared_style_type() {
		// Arrange and Act
		let kind = diagnostic_kind(quote! {
			vars { value: Distance = 1px; }
			.card {}
		});

		// Assert
		assert_eq!(
			kind,
			StyleDiagnosticKind::UnknownStyleType {
				name: "Distance".into(),
			}
		);
	}

	#[rstest]
	fn rejects_an_unknown_numeric_unit() {
		// Arrange and Act
		let kind = diagnostic_kind(quote! {
			vars { value: Length = 1furlong; }
			.card {}
		});

		// Assert
		assert_eq!(
			kind,
			StyleDiagnosticKind::UnknownUnit {
				name: "furlong".into(),
			}
		);
	}

	#[rstest]
	fn checked_functions_remain_unwrapped_while_arguments_track_arithmetic() {
		// Arrange
		let source = ".card { transform: translate_x(100% - 1rem); }";

		// Act
		let value = first_declaration_value(source);

		// Assert
		assert!(!value.contains_arithmetic);
		let TypedValueExprKind::Function(call) = value.kind else {
			panic!("expected a typed function call");
		};
		assert!(call.arguments[0].contains_arithmetic);
	}

	#[rstest]
	fn validates_representative_structural_property_grammars() {
		// Arrange
		let source = "
			.card {
				display: grid;
				width: min(10rem, 50%);
				border: (1px, solid, red);
				font-family: [\"Inter\", system-ui];
				transform: (translate_x(1rem), rotate(45deg));
				grid-column: slash(1, 2);
				background-image: linear_gradient(
					Direction::Right,
					[stop(red, 0%), stop(blue, 100%)]
				);
			}
		";

		// Act
		let typed = validated_text(source);

		// Assert
		let TypedStyleItem::Rule(rule) = &typed.items[0] else {
			panic!("expected a typed style rule");
		};
		assert_eq!(rule.items.len(), 7);
	}

	#[rstest]
	fn property_grammar_keywords_match_ascii_case_insensitively() {
		// Arrange
		let source = ".card { display: Flex; position: Sticky; color: Inherit; }";

		// Act
		let typed = validated_text(source);

		// Assert
		let TypedStyleItem::Rule(rule) = &typed.items[0] else {
			panic!("expected a style rule");
		};
		assert_eq!(rule.items.len(), 3);
	}

	#[rstest]
	#[case(".card { width: 1PX; }")]
	#[case(".card { transform: rotate(90DEG); }")]
	#[case(".card { transition-duration: 1MS; }")]
	fn numeric_css_units_match_ascii_case_insensitively(#[case] source: &str) {
		// Act
		let typed = validated_text(source);

		// Assert
		assert_eq!(typed.items.len(), 1);
	}

	#[rstest]
	fn unordered_grammars_accept_optional_shorthand_members() {
		// Arrange
		let sources = [
			".card { flex-flow: row; }",
			".card { flex-flow: wrap; }",
			".card { grid-auto-flow: dense; }",
		];

		// Act and Assert
		for source in sources {
			let typed = validated_text(source);
			assert_eq!(typed.items.len(), 1, "source should validate: {source}");
		}
	}

	#[rstest]
	fn reviewed_shorthand_grammars_accept_valid_css_orderings() {
		// Arrange
		let sources = [
			".card { flex: 10rem; }",
			".card { flex: 30%; }",
			".card { flex: min-content; }",
			".card { text-decoration: (solid, red, 2px); }",
			".card { box-shadow: (red, 0, 0, 4px); }",
			".card { box-shadow: (1px, 0, 2px, red); }",
			".card { box-shadow: (0, 0, 4px, inset); }",
			".card { box-shadow: none; }",
			".card { box-shadow: [(0, 0, 4px, red), (1px, 1px, blue)]; }",
			".card { outline: auto; }",
			".card { outline-style: auto; }",
			".card { background: [linear_gradient(Direction::Right, [stop(red, 0%), stop(blue, 100%)]), red]; }",
			".card { background: none; }",
		];

		// Act and Assert
		for source in sources {
			let tokens = source.parse().expect("test tokens should parse");
			let ast = parse_style(tokens).unwrap_or_else(|error| {
				panic!("source should parse: {source}: {error}");
			});
			validate_style(&ast).unwrap_or_else(|error| {
				panic!("source should validate: {source}: {error}");
			});
		}
	}

	#[rstest]
	fn reviewed_registry_grammars_accept_supported_css_values() {
		// Arrange
		let sources = [
			".card { grid-template-columns: 1fr; }",
			".card { grid-column: (span, card-start); }",
			".card { grid-row: (span, 2); }",
			".card { grid-column: (2, sidebar); }",
			".card { grid-column: (span, 2, sidebar); }",
			".card { flex-basis: content; }",
			".card { flex: content; }",
			".card { max-width: max-content; }",
			".card { max-height: min-content; }",
			".card { grid-template-areas: (\"a a\", \"a a\"); }",
			".card { background-position: (left, 10px, top, 20%); }",
			".card { font-style: (oblique, 90deg); }",
			".card { font-style: (oblique, 100grad); }",
			".card { touch-action: (pan-left, pan-up, pinch-zoom); }",
			".card { color: red.mix(blue, 100%); }",
			".card { color: Color::oklch(60%, 40%, 30deg); }",
			".card { color: currentColor; }",
			".card { transition-property: none; }",
			".card { text-decoration: (underline, overline, wavy, auto); }",
			".card { text-decoration: (underline, from-font); }",
			".card { text-overflow: (clip, ellipsis); }",
		];

		// Act and Assert
		for source in sources {
			let typed = validated_text(source);
			assert_eq!(typed.items.len(), 1, "source should validate: {source}");
		}
	}

	#[rstest]
	fn reviewed_registry_grammars_reject_invalid_css_values() {
		// Arrange
		let sources = [
			".card { grid-template-columns: -1fr; }",
			".card { grid-column: span; }",
			".card { grid-column: slash(1, span); }",
			".card { grid-column: (span, auto); }",
			".card { grid-row: (span, span); }",
			".card { grid-template-areas: (\"a a\", \"a .\"); }",
			".card { grid-template-areas: (\"a\", \"a a\"); }",
			".card { grid-template-areas: (\"nav.main nav\", \"nav.main nav\"); }",
			".card { transition-duration: 0; }",
			".card { transition: (opacity, 0, ease); }",
			".card { background-position: (left, right); }",
			".card { font-style: (oblique, 91deg); }",
			".card { font-style: (oblique, -91deg); }",
			".card { font-style: (oblique, 0.5turn); }",
			".card { grid-column: 0; }",
			".card { grid-row: 0; }",
			".card { grid-area: 0; }",
			".card { background-position: (1px, 2px, 3px); }",
			".card { background: (left, slash(right, cover)); }",
			".card { background: (0, slash(right, cover)); }",
			".card { touch-action: (pan-left, pan-right); }",
			".card { color: red.mix(blue, 150%); }",
			".card { transition-property: [none, opacity]; }",
			".card { text-decoration: hidden; }",
			".card { text-decoration: (underline, thin); }",
			".card { text-decoration: (underline, underline); }",
			".card { box-shadow: (1px, red, 2px); }",
			".card { outline: hidden; }",
			".card { outline-style: hidden; }",
		];

		// Act and Assert
		for source in sources {
			let kind = diagnostic_kind_text(source);
			assert!(
				matches!(kind, StyleDiagnosticKind::PropertyValueMismatch { .. })
					|| matches!(kind, StyleDiagnosticKind::InvalidFunctionArgument { .. }),
				"source should be rejected: {source}: {kind:?}"
			);
		}
	}

	#[rstest]
	#[case("transition", "(opacity, -1s)")]
	#[case("transform-origin", "(10px, 20px, 30%)")]
	#[case("transform-origin", "(1px, 2px, 3px, 4px)")]
	#[case("transform-origin", "(left, right)")]
	#[case("padding", "0px - 1px")]
	#[case("transition-duration", "0s - 1s")]
	fn rejects_invalid_values_previously_accepted_by_broad_grammars(
		#[case] property: &str,
		#[case] value: &str,
	) {
		// Arrange
		let source = format!(".card {{ {property}: {value}; }}");

		// Act
		let kind = diagnostic_kind_text(&source);

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { property: actual, .. }
				if actual == property
		));
	}

	#[rstest]
	fn background_color_is_restricted_to_the_final_layer() {
		// Arrange
		let source = ".card { background: [red, linear_gradient(Direction::Right, [stop(red, 0%), stop(blue, 100%)])]; }";

		// Act
		let kind = diagnostic_kind_text(source);

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { property, .. }
				if property == "background"
		));
	}

	#[rstest]
	fn structural_members_consume_variable_length_prefixes() {
		// Arrange
		let source = "
			.card {
				transform-origin: (left, top);
				background: (red, left, top);
			}
		";

		// Act
		let typed = validated_text(source);

		// Assert
		let TypedStyleItem::Rule(rule) = &typed.items[0] else {
			panic!("expected a typed style rule");
		};
		assert_eq!(rule.items.len(), 2);
	}

	#[rstest]
	fn unordered_members_may_consume_more_items_than_the_member_count() {
		// Arrange
		let source = "
			.card {
				background: (
					red,
					linear_gradient(
						Direction::Right,
						[stop(red, 0%), stop(blue, 100%)]
					),
					left,
					top,
					no-repeat,
					repeat
				);
			}
		";

		// Act
		let typed = validated_text(source);

		// Assert
		let TypedStyleItem::Rule(rule) = &typed.items[0] else {
			panic!("expected a typed style rule");
		};
		assert_eq!(rule.items.len(), 1);
	}

	#[rstest]
	fn unordered_grammar_does_not_reuse_one_member_role() {
		// Arrange and Act
		let kind = diagnostic_kind_text(".card { background: (red, blue); }");

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { property, .. }
				if property == "background"
		));
	}

	#[rstest]
	fn unordered_minimum_counts_consumed_roles_not_tokens() {
		// Arrange
		let value = first_declaration_value(".card { border-color: (red, blue); }");

		// Act
		let matches = matches_grammar(&value, &TEST_TWO_ROLE_GRAMMAR);

		// Assert
		assert!(!matches);
	}

	#[rstest]
	fn css_wide_keywords_are_not_custom_identifiers_in_any_ascii_case() {
		// Arrange and Act
		let kind = diagnostic_kind_text(".card { font-family: [system-ui, Inherit]; }");

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { property, .. }
				if property == "font-family"
		));
	}

	#[rstest]
	#[case("padding", "-1px")]
	#[case("padding-left", "-0.5rem")]
	#[case("flex-grow", "-1")]
	#[case("flex-shrink", "-1")]
	#[case("flex", "(-1, 0, auto)")]
	#[case("border-width", "-1px")]
	#[case("border", "(-1px, solid, red)")]
	#[case("outline-width", "-2px")]
	#[case("border-radius", "-1px")]
	#[case("border-top-left-radius", "-10%")]
	fn rejects_negative_values_for_nonnegative_property_grammars(
		#[case] property: &str,
		#[case] value: &str,
	) {
		// Arrange
		let source = format!(".card {{ {property}: {value}; }}");

		// Act
		let kind = diagnostic_kind_text(&source);

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { property: actual, .. }
				if actual == property
		));
	}

	#[rstest]
	fn accepts_unordered_font_members_before_the_size() {
		// Arrange
		let source = ".card { font: (bold, italic, 16px, serif); }";

		// Act
		let typed = validated_text(source);

		// Assert
		assert_eq!(typed.items.len(), 1);
	}

	#[rstest]
	fn rejects_font_family_before_the_size() {
		let kind = diagnostic_kind_text(".card { font: (Arial, 16px); }");

		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { property, .. } if property == "font"
		));
	}

	#[rstest]
	fn accepts_multi_token_font_family_names() {
		let typed = validated_text(".card { font-family: [(Times, New, Roman), sans-serif]; }");

		assert_eq!(typed.items.len(), 1);
	}

	#[rstest]
	fn rejects_negative_box_shadow_blur_radius() {
		let kind = diagnostic_kind_text(".card { box-shadow: (0, 0, -4px, red); }");

		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { property, .. } if property == "box-shadow"
		));
	}

	#[rstest]
	fn accepts_hyphen_leading_custom_identifiers() {
		// Arrange
		let source = ".card { font-family: [-apple-system, sans-serif]; }";

		// Act
		let typed = validated_text(source);

		// Assert
		assert_eq!(typed.items.len(), 1);
	}

	#[rstest]
	#[case("gap", "-1px")]
	#[case("row-gap", "-10%")]
	#[case("width", "-1px")]
	#[case("flex", "-1px")]
	#[case("grid-template-columns", "sidebar")]
	#[case("font-size", "-1rem")]
	#[case("line-height", "-1.2")]
	#[case("background-repeat", "(repeat-x, no-repeat)")]
	#[case("background-size", "min-content")]
	#[case("font-weight", "0")]
	#[case("font-weight", "2000")]
	#[case("font-weight", "2001")]
	#[case("padding", "-11px")]
	#[case("transition-duration", "-1s")]
	fn rejects_invalid_css_value_constraints(#[case] property: &str, #[case] value: &str) {
		// Arrange
		let source = format!(".card {{ {property}: {value}; }}");

		// Act
		let kind = diagnostic_kind_text(&source);

		// Assert
		assert!(matches!(
			kind,
			StyleDiagnosticKind::PropertyValueMismatch { property: actual, .. }
				if actual == property
		));
	}
}
