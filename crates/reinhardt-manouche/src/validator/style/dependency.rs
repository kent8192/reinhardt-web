//! Style binding symbol tables, reference validation, and variable dependency analysis.

use std::collections::HashMap;

use proc_macro2::Span;

use super::selector::is_rust_keyword;

use crate::{
	StyleDiagnostic, StyleDiagnosticKind, StyleGlobalDeclaration, StyleItem, StyleMacro,
	StyleReferenceNamespace, StyleRuleItem, StyleValueExpr, StyleValueExpression,
	StyleVariableDeclaration,
};

pub(super) struct ValidatedBindings {
	pub(super) globals: Vec<ValidatedGlobal>,
	pub(super) variables: Vec<ValidatedVariable>,
	pub(super) evaluation_order: Vec<usize>,
}

pub(super) struct ValidatedGlobal {
	pub(super) declaration: StyleGlobalDeclaration,
	pub(super) css_name: String,
	pub(super) source_index: usize,
}

pub(super) struct ValidatedVariable {
	pub(super) declaration: StyleVariableDeclaration,
	pub(super) css_name: String,
	pub(super) source_index: usize,
	pub(super) dependency_indices: Vec<usize>,
	pub(super) evaluation_index: usize,
}

#[derive(Clone, Copy)]
struct DependencyEdge {
	target: usize,
	reference_span: Span,
}

pub(super) fn validate_bindings(ast: &StyleMacro) -> Result<ValidatedBindings, StyleDiagnostic> {
	let mut global_symbols: HashMap<String, usize> = HashMap::new();
	let mut globals = Vec::with_capacity(ast.globals.len());
	for (source_index, declaration) in ast.globals.iter().enumerate() {
		let name = declaration.name.as_str();
		if !is_ordinary_binding_name(name) {
			return Err(StyleDiagnostic::new(
				StyleDiagnosticKind::InvalidGlobalName {
					name: name.to_owned(),
				},
				declaration.name.span,
			));
		}
		if let Some(&first_index) = global_symbols.get(name) {
			return Err(StyleDiagnostic::new(
				StyleDiagnosticKind::DuplicateGlobal {
					name: name.to_owned(),
				},
				declaration.name.span,
			)
			.with_related(
				ast.globals[first_index].name.span,
				format!("global `{name}` was first declared here"),
			));
		}
		global_symbols.insert(name.to_owned(), source_index);
		globals.push(ValidatedGlobal {
			declaration: declaration.clone(),
			css_name: css_binding_name(name),
			source_index,
		});
	}

	let mut variable_symbols: HashMap<String, usize> = HashMap::new();
	for (source_index, declaration) in ast.variables.iter().enumerate() {
		let name = declaration.name.as_str();
		if !is_ordinary_binding_name(name) {
			return Err(StyleDiagnostic::new(
				StyleDiagnosticKind::InvalidVariableName {
					name: name.to_owned(),
				},
				declaration.name.span,
			));
		}
		if let Some(&first_index) = variable_symbols.get(name) {
			return Err(StyleDiagnostic::new(
				StyleDiagnosticKind::DuplicateVariable {
					name: name.to_owned(),
				},
				declaration.name.span,
			)
			.with_related(
				ast.variables[first_index].name.span,
				format!("component variable `{name}` was first declared here"),
			));
		}
		variable_symbols.insert(name.to_owned(), source_index);
	}

	let mut edges = vec![Vec::new(); ast.variables.len()];
	for (source_index, variable) in ast.variables.iter().enumerate() {
		let Some(default) = &variable.default else {
			return Err(StyleDiagnostic::new(
				StyleDiagnosticKind::MissingVariableDefault {
					name: variable.name.as_str().to_owned(),
				},
				variable.name.span,
			));
		};
		validate_expression(
			default,
			Some(source_index),
			&global_symbols,
			&variable_symbols,
			&mut edges,
		)?;
	}
	for item in &ast.items {
		validate_item_references(item, &global_symbols, &variable_symbols, &mut edges)?;
	}
	for dependencies in &mut edges {
		dependencies.sort_by_key(|edge| edge.target);
		dependencies.dedup_by_key(|edge| edge.target);
	}
	let evaluation_order = dependency_first_order(ast, &edges)?;
	let mut evaluation_indices = vec![0; ast.variables.len()];
	for (evaluation_index, &source_index) in evaluation_order.iter().enumerate() {
		evaluation_indices[source_index] = evaluation_index;
	}
	let variables = ast
		.variables
		.iter()
		.enumerate()
		.map(|(source_index, declaration)| ValidatedVariable {
			declaration: declaration.clone(),
			css_name: css_binding_name(declaration.name.as_str()),
			source_index,
			dependency_indices: edges[source_index].iter().map(|edge| edge.target).collect(),
			evaluation_index: evaluation_indices[source_index],
		})
		.collect();

	Ok(ValidatedBindings {
		globals,
		variables,
		evaluation_order,
	})
}

fn is_ordinary_binding_name(name: &str) -> bool {
	let mut bytes = name.bytes();
	if !bytes.next().is_some_and(|byte| byte.is_ascii_lowercase()) {
		return false;
	}
	let mut previous_underscore = false;
	for byte in bytes {
		if byte == b'_' {
			if previous_underscore {
				return false;
			}
			previous_underscore = true;
		} else if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
			previous_underscore = false;
		} else {
			return false;
		}
	}
	!previous_underscore && !is_rust_keyword(name)
}

fn css_binding_name(name: &str) -> String {
	name.replace('_', "-")
}

fn validate_item_references(
	item: &StyleItem,
	global_symbols: &HashMap<String, usize>,
	variable_symbols: &HashMap<String, usize>,
	edges: &mut [Vec<DependencyEdge>],
) -> Result<(), StyleDiagnostic> {
	match item {
		StyleItem::Rule(rule) => {
			validate_rule_items(&rule.items, global_symbols, variable_symbols, edges)
		}
		StyleItem::Media(media) => {
			validate_rule_items(&media.items, global_symbols, variable_symbols, edges)
		}
	}
}

fn validate_rule_items(
	items: &[StyleRuleItem],
	global_symbols: &HashMap<String, usize>,
	variable_symbols: &HashMap<String, usize>,
	edges: &mut [Vec<DependencyEdge>],
) -> Result<(), StyleDiagnostic> {
	for item in items {
		match item {
			StyleRuleItem::Declaration(declaration) => validate_expression(
				&declaration.value,
				None,
				global_symbols,
				variable_symbols,
				edges,
			)?,
			StyleRuleItem::Rule(rule) => {
				validate_rule_items(&rule.items, global_symbols, variable_symbols, edges)?
			}
			StyleRuleItem::Media(media) => {
				validate_rule_items(&media.items, global_symbols, variable_symbols, edges)?
			}
		}
	}
	Ok(())
}

fn validate_expression(
	expression: &StyleValueExpression,
	current_variable: Option<usize>,
	global_symbols: &HashMap<String, usize>,
	variable_symbols: &HashMap<String, usize>,
	edges: &mut [Vec<DependencyEdge>],
) -> Result<(), StyleDiagnostic> {
	match &expression.kind {
		StyleValueExpr::QualifiedReference(reference) => match reference.namespace {
			StyleReferenceNamespace::Globals => {
				if !global_symbols.contains_key(reference.name.as_str()) {
					return Err(StyleDiagnostic::new(
						StyleDiagnosticKind::UndeclaredGlobalReference {
							name: reference.name.as_str().to_owned(),
						},
						reference.span,
					));
				}
			}
			StyleReferenceNamespace::Variables => {
				let Some(&target) = variable_symbols.get(reference.name.as_str()) else {
					return Err(StyleDiagnostic::new(
						StyleDiagnosticKind::UndeclaredVariableReference {
							name: reference.name.as_str().to_owned(),
						},
						reference.span,
					));
				};
				if let Some(source) = current_variable {
					edges[source].push(DependencyEdge {
						target,
						reference_span: reference.span,
					});
				}
			}
		},
		StyleValueExpr::Unary(unary) => validate_expression(
			&unary.operand,
			current_variable,
			global_symbols,
			variable_symbols,
			edges,
		)?,
		StyleValueExpr::Binary(binary) => {
			validate_expression(
				&binary.left,
				current_variable,
				global_symbols,
				variable_symbols,
				edges,
			)?;
			validate_expression(
				&binary.right,
				current_variable,
				global_symbols,
				variable_symbols,
				edges,
			)?;
		}
		StyleValueExpr::Call(call) => validate_expressions(
			&call.arguments,
			current_variable,
			global_symbols,
			variable_symbols,
			edges,
		)?,
		StyleValueExpr::MethodCall(call) => {
			validate_expression(
				&call.receiver,
				current_variable,
				global_symbols,
				variable_symbols,
				edges,
			)?;
			validate_expressions(
				&call.arguments,
				current_variable,
				global_symbols,
				variable_symbols,
				edges,
			)?;
		}
		StyleValueExpr::Group(group) => validate_expression(
			&group.expression,
			current_variable,
			global_symbols,
			variable_symbols,
			edges,
		)?,
		StyleValueExpr::SpaceSequence(collection) | StyleValueExpr::CommaList(collection) => {
			validate_expressions(
				&collection.items,
				current_variable,
				global_symbols,
				variable_symbols,
				edges,
			)?
		}
		StyleValueExpr::Literal(_)
		| StyleValueExpr::AssociatedPathValue(_)
		| StyleValueExpr::UncheckedFunction(_) => {}
	}
	Ok(())
}

fn validate_expressions(
	expressions: &[StyleValueExpression],
	current_variable: Option<usize>,
	global_symbols: &HashMap<String, usize>,
	variable_symbols: &HashMap<String, usize>,
	edges: &mut [Vec<DependencyEdge>],
) -> Result<(), StyleDiagnostic> {
	for expression in expressions {
		validate_expression(
			expression,
			current_variable,
			global_symbols,
			variable_symbols,
			edges,
		)?;
	}
	Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VisitState {
	Unvisited,
	Visiting,
	Visited,
}

#[derive(Clone, Copy)]
struct VisitFrame {
	source: usize,
	next_edge: usize,
}

fn dependency_first_order(
	ast: &StyleMacro,
	edges: &[Vec<DependencyEdge>],
) -> Result<Vec<usize>, StyleDiagnostic> {
	let mut states = vec![VisitState::Unvisited; ast.variables.len()];
	let mut path = Vec::new();
	let mut frames = Vec::new();
	let mut order = Vec::with_capacity(ast.variables.len());
	for source_index in 0..ast.variables.len() {
		visit_iterative(
			source_index,
			ast,
			edges,
			&mut states,
			&mut path,
			&mut frames,
			&mut order,
		)?;
	}
	Ok(order)
}

fn visit_iterative(
	source: usize,
	ast: &StyleMacro,
	edges: &[Vec<DependencyEdge>],
	states: &mut [VisitState],
	path: &mut Vec<usize>,
	frames: &mut Vec<VisitFrame>,
	order: &mut Vec<usize>,
) -> Result<(), StyleDiagnostic> {
	if states[source] == VisitState::Visited {
		return Ok(());
	}
	states[source] = VisitState::Visiting;
	path.push(source);
	frames.push(VisitFrame {
		source,
		next_edge: 0,
	});
	while let Some(frame) = frames.last_mut() {
		let frame_source = frame.source;
		let Some(edge) = edges[frame_source].get(frame.next_edge).copied() else {
			frames.pop();
			path.pop();
			states[frame_source] = VisitState::Visited;
			order.push(frame_source);
			continue;
		};
		frame.next_edge += 1;
		match states[edge.target] {
			VisitState::Unvisited => {
				states[edge.target] = VisitState::Visiting;
				path.push(edge.target);
				frames.push(VisitFrame {
					source: edge.target,
					next_edge: 0,
				});
			}
			VisitState::Visiting => return Err(cycle_diagnostic(ast, edges, path, edge)),
			VisitState::Visited => {}
		}
	}
	Ok(())
}

fn cycle_diagnostic(
	ast: &StyleMacro,
	edges: &[Vec<DependencyEdge>],
	stack: &[usize],
	closing_edge: DependencyEdge,
) -> StyleDiagnostic {
	let start = stack
		.iter()
		.position(|&index| index == closing_edge.target)
		.unwrap_or(0);
	let cycle_nodes = &stack[start..];
	let mut names = cycle_nodes
		.iter()
		.map(|&index| ast.variables[index].name.as_str().to_owned())
		.collect::<Vec<_>>();
	names.push(ast.variables[closing_edge.target].name.as_str().to_owned());
	let mut diagnostic = StyleDiagnostic::new(
		StyleDiagnosticKind::VariableDependencyCycle { names },
		closing_edge.reference_span,
	);
	for (position, &source) in cycle_nodes.iter().enumerate() {
		let target = cycle_nodes
			.get(position + 1)
			.copied()
			.unwrap_or(closing_edge.target);
		let source_name = ast.variables[source].name.as_str();
		let target_name = ast.variables[target].name.as_str();
		let edge = edges[source]
			.iter()
			.find(|edge| edge.target == target)
			.copied()
			.unwrap_or(closing_edge);
		diagnostic = diagnostic
			.with_related(
				ast.variables[source].span,
				format!("component variable `{source_name}` is declared here"),
			)
			.with_related(
				edge.reference_span,
				format!("`{source_name}` depends on `{target_name}` here"),
			);
	}
	diagnostic
}

#[cfg(test)]
mod tests {
	use std::fmt::Write as _;

	use quote::quote;
	use rstest::rstest;

	use crate::{StyleDiagnosticKind, parser::parse_style, validator::validate_style};

	#[rstest]
	#[case(
		quote! { globals { accent: Color; accent: Color; } .card {} },
		StyleDiagnosticKind::DuplicateGlobal { name: "accent".into() },
		"global `accent` was first declared here"
	)]
	#[case(
		quote! { vars { gap: Length = 1px; gap: Length = 2px; } .card {} },
		StyleDiagnosticKind::DuplicateVariable { name: "gap".into() },
		"component variable `gap` was first declared here"
	)]
	fn duplicate_bindings_report_the_duplicate_and_first_declaration(
		#[case] input: proc_macro2::TokenStream,
		#[case] expected_kind: StyleDiagnosticKind,
		#[case] expected_related: &str,
	) {
		// Arrange
		let ast = parse_style(input).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(diagnostic.kind, expected_kind);
		assert_eq!(diagnostic.related.len(), 1);
		assert_eq!(diagnostic.related[0].reason, expected_related);
	}

	#[rstest]
	#[case(true, "r#type")]
	#[case(true, "Accent")]
	#[case(true, "café")]
	#[case(true, "accent__strong")]
	#[case(true, "accent_")]
	#[case(true, "1accent")]
	#[case(false, "r#match")]
	#[case(false, "Spacing")]
	#[case(false, "間隔")]
	#[case(false, "large__gap")]
	#[case(false, "large_")]
	#[case(false, "2gap")]
	fn rejects_nonordinary_binding_names(#[case] global: bool, #[case] invalid_name: &str) {
		// Arrange
		let mut ast = parse_style(if global {
			quote! { globals { valid: Color; } .card {} }
		} else {
			quote! { vars { valid: Length = 1px; } .card {} }
		})
		.unwrap();
		let expected_kind = if global {
			ast.globals[0].name.value = invalid_name.into();
			StyleDiagnosticKind::InvalidGlobalName {
				name: invalid_name.into(),
			}
		} else {
			ast.variables[0].name.value = invalid_name.into();
			StyleDiagnosticKind::InvalidVariableName {
				name: invalid_name.into(),
			}
		};

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(diagnostic.kind, expected_kind);
		assert_eq!(diagnostic.related.len(), 0);
	}

	#[rstest]
	#[case(true, "type")]
	#[case(true, "async")]
	#[case(false, "match")]
	#[case(false, "gen")]
	fn rejects_rust_keywords_as_binding_names(#[case] global: bool, #[case] keyword: &str) {
		// Arrange
		let source = if global {
			format!("globals {{ {keyword}: Color; }} .card {{}}")
		} else {
			format!("vars {{ {keyword}: Color = red; }} .card {{}}")
		};
		let ast = parse_style(source.parse().unwrap()).unwrap();
		let expected = if global {
			StyleDiagnosticKind::InvalidGlobalName {
				name: keyword.into(),
			}
		} else {
			StyleDiagnosticKind::InvalidVariableName {
				name: keyword.into(),
			}
		};

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(diagnostic.kind, expected);
		assert_eq!(diagnostic.related.len(), 0);
	}

	#[rstest]
	#[case(
		quote! { vars { accent: Color = globals.missing; } .card {} },
		StyleDiagnosticKind::UndeclaredGlobalReference { name: "missing".into() }
	)]
	#[case(
		quote! { vars { accent: Color = vars.missing; } .card {} },
		StyleDiagnosticKind::UndeclaredVariableReference { name: "missing".into() }
	)]
	#[case(
		quote! { .card { color: globals.missing; } },
		StyleDiagnosticKind::UndeclaredGlobalReference { name: "missing".into() }
	)]
	#[case(
		quote! { .card { .label { color: vars.missing; } } },
		StyleDiagnosticKind::UndeclaredVariableReference { name: "missing".into() }
	)]
	#[case(
		quote! { .card { @media (width > 1px) { .label { color: vars.missing; } } } },
		StyleDiagnosticKind::UndeclaredVariableReference { name: "missing".into() }
	)]
	fn rejects_undeclared_references_everywhere(
		#[case] input: proc_macro2::TokenStream,
		#[case] expected_kind: StyleDiagnosticKind,
	) {
		// Arrange
		let ast = parse_style(input).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(diagnostic.kind, expected_kind);
		assert_eq!(diagnostic.related.len(), 0);
	}

	#[rstest]
	fn complete_symbol_tables_allow_forward_references_and_independent_namespaces() {
		// Arrange
		let ast = parse_style(quote! {
			globals { accent: Color; }
			vars {
				foreground: Color = vars.accent;
				accent: Color = globals.accent;
			}
			.card { color: vars.foreground; background-color: globals.accent; }
		})
		.unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(typed.globals.len(), 1);
		assert_eq!(typed.globals[0].declaration.name.as_str(), "accent");
		assert_eq!(typed.globals[0].css_name, "accent");
		assert_eq!(typed.variables.len(), 2);
		assert_eq!(typed.variables[0].declaration.name.as_str(), "foreground");
		assert_eq!(typed.variables[1].declaration.name.as_str(), "accent");
		assert_eq!(typed.variables[1].css_name, "accent");
	}

	#[rstest]
	fn binding_metadata_preserves_source_indices_and_derives_css_names() {
		// Arrange
		let ast = parse_style(quote! {
			globals { base_color: Color; surface_secondary: Color; }
			vars { small_gap: Length = 1px; large_gap: Length = 2px; }
			.card {}
		})
		.unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(
			typed
				.globals
				.iter()
				.map(|binding| (binding.css_name.as_str(), binding.source_index))
				.collect::<Vec<_>>(),
			vec![("base-color", 0), ("surface-secondary", 1)]
		);
		assert_eq!(
			typed
				.variables
				.iter()
				.map(|binding| (binding.css_name.as_str(), binding.source_index))
				.collect::<Vec<_>>(),
			vec![("small-gap", 0), ("large-gap", 1)]
		);
	}

	#[rstest]
	fn rejects_self_dependency_with_a_closed_chain_and_all_contributing_labels() {
		// Arrange
		let ast = parse_style(quote! {
			vars { accent: Color = vars.accent; }
			.card {}
		})
		.unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::VariableDependencyCycle {
				names: vec!["accent".into(), "accent".into()],
			}
		);
		assert_eq!(
			diagnostic
				.related
				.iter()
				.map(|label| label.reason.as_str())
				.collect::<Vec<_>>(),
			vec![
				"component variable `accent` is declared here",
				"`accent` depends on `accent` here",
			]
		);
	}

	#[rstest]
	fn rejects_multi_variable_cycle_with_a_complete_deterministic_chain() {
		// Arrange
		let ast = parse_style(quote! {
			vars {
				a: Length = vars.b;
				b: Length = vars.c;
				c: Length = vars.a;
			}
			.card {}
		})
		.unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::VariableDependencyCycle {
				names: vec!["a".into(), "b".into(), "c".into(), "a".into()],
			}
		);
		assert_eq!(
			diagnostic
				.related
				.iter()
				.map(|label| label.reason.as_str())
				.collect::<Vec<_>>(),
			vec![
				"component variable `a` is declared here",
				"`a` depends on `b` here",
				"component variable `b` is declared here",
				"`b` depends on `c` here",
				"component variable `c` is declared here",
				"`c` depends on `a` here",
			]
		);
	}

	#[rstest]
	fn publishes_dependency_first_order_without_reordering_authored_variables() {
		// Arrange
		let ast = parse_style(quote! {
			vars {
				c: Length = vars.b;
				a: Length = 1px;
				b: Length = vars.a;
			}
			.card {}
		})
		.unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(
			typed
				.variables
				.iter()
				.map(|variable| variable.declaration.name.as_str())
				.collect::<Vec<_>>(),
			vec!["c", "a", "b"]
		);
		assert_eq!(typed.variable_evaluation_order, vec![1, 2, 0]);
		assert_eq!(typed.variables[0].dependency_indices, vec![2]);
		assert_eq!(typed.variables[1].dependency_indices, Vec::<usize>::new());
		assert_eq!(typed.variables[2].dependency_indices, vec![1]);
		assert_eq!(
			typed
				.variables
				.iter()
				.map(|variable| variable.evaluation_index)
				.collect::<Vec<_>>(),
			vec![2, 0, 1]
		);
	}

	#[rstest]
	fn duplicate_variable_references_produce_one_dependency_edge() {
		// Arrange
		let ast = parse_style(quote! {
			vars {
				top: Length = clamp(vars.base, vars.base, vars.base);
				base: Length = 1px;
			}
			.card {}
		})
		.unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(typed.variables[0].dependency_indices, vec![1]);
		assert_eq!(typed.variable_evaluation_order, vec![1, 0]);
	}

	#[rstest]
	fn collects_a_wide_dependency_set_without_quadratic_duplicate_searches() {
		// Arrange
		const DEPENDENCY_COUNT: usize = 4_096;
		let mut source = String::with_capacity(DEPENDENCY_COUNT * 80);
		source.push_str("vars { root: Length = min(");
		for occurrence in 0..DEPENDENCY_COUNT * 2 {
			if occurrence > 0 {
				source.push(',');
			}
			write!(source, "vars.v{}", occurrence % DEPENDENCY_COUNT).unwrap();
		}
		source.push_str("); ");
		for index in 0..DEPENDENCY_COUNT {
			write!(source, "v{index}: Length = 1px;").unwrap();
		}
		source.push_str("} .card {}");
		let ast = parse_style(source.parse().unwrap()).unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(
			typed.variables[0].dependency_indices.len(),
			DEPENDENCY_COUNT
		);
		assert_eq!(typed.variables[0].dependency_indices[0], 1);
		assert_eq!(
			typed.variables[0].dependency_indices[DEPENDENCY_COUNT - 1],
			DEPENDENCY_COUNT
		);
	}

	#[rstest]
	fn validates_a_long_dependency_chain_without_recursive_graph_traversal() {
		// Arrange
		const VARIABLE_COUNT: usize = 16_384;
		let mut source = String::with_capacity(VARIABLE_COUNT * 40);
		source.push_str("vars {");
		for index in 0..VARIABLE_COUNT - 1 {
			write!(source, "v{index}: Length = vars.v{};", index + 1).unwrap();
		}
		write!(source, "v{}: Length = 1px;", VARIABLE_COUNT - 1).unwrap();
		source.push_str("} .card {}");
		let ast = parse_style(source.parse().unwrap()).unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(typed.variable_evaluation_order.len(), VARIABLE_COUNT);
		assert_eq!(typed.variable_evaluation_order[0], VARIABLE_COUNT - 1);
		assert_eq!(typed.variable_evaluation_order[VARIABLE_COUNT - 1], 0);
	}

	#[rstest]
	#[case("-vars.missing")]
	#[case("1px + vars.missing")]
	#[case("clamp(1px, vars.missing, 2px)")]
	#[case("vars.missing.mix(white)")]
	#[case("1px.mix(vars.missing)")]
	#[case("(vars.missing)")]
	#[case("(1px, vars.missing)")]
	#[case("[1px, vars.missing]")]
	fn recursively_validates_references_in_every_composite_value(#[case] default: &str) {
		// Arrange
		let source = format!("vars {{ value: Length = {default}; }} .card {{}}");
		let ast = parse_style(source.parse().unwrap()).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::UndeclaredVariableReference {
				name: "missing".into(),
			}
		);
	}

	#[rstest]
	fn unchecked_function_tokens_are_opaque_to_reference_validation() {
		// Arrange
		let ast = parse_style(quote! {
			vars { paint: Color = unchecked_fn!(paint(vars.missing)); }
			.card {}
		})
		.unwrap();

		// Act
		let typed = validate_style(&ast).unwrap();

		// Assert
		assert_eq!(typed.variables[0].dependency_indices, Vec::<usize>::new());
		assert_eq!(typed.variable_evaluation_order, vec![0]);
	}

	#[rstest]
	fn missing_variable_default_is_rejected_by_validation() {
		// Arrange
		let input = quote! { vars { gap: Length; } .card {} };
		let ast = parse_style(input).unwrap();

		// Act
		let diagnostic = validate_style(&ast).unwrap_err();

		// Assert
		assert_eq!(
			diagnostic.kind,
			StyleDiagnosticKind::MissingVariableDefault { name: "gap".into() }
		);
		assert_eq!(diagnostic.related.len(), 0);
	}
}
