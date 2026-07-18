//! Structural parser for the untyped `style!` macro AST.

mod media;
mod name;
mod selector;
mod value;

use media::parse_media_condition;
use selector::{SelectorContext, parse_selector_list};
use value::parse_value_expression;

use proc_macro2::{Span, TokenStream, TokenTree};
use syn::{
	Ident, Token, braced,
	ext::IdentExt,
	parse::{Parse, ParseStream},
	spanned::Spanned,
};

use crate::core::{
	CssName, StyleBindingName, StyleDeclaration, StyleDslType, StyleGlobalDeclaration, StyleItem,
	StyleMacro, StyleMediaRule, StyleRule, StyleRuleItem, StyleVariableDeclaration,
};

mod keyword {
	syn::custom_keyword!(globals);
	syn::custom_keyword!(media);
	syn::custom_keyword!(vars);
}

/// Parses a `style!` macro body into its untyped AST.
pub fn parse_style(input: TokenStream) -> syn::Result<StyleMacro> {
	syn::parse2(input)
}

impl Parse for StyleMacro {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let span = input.span();
		let mut globals = Vec::new();
		let mut variables = Vec::new();
		let mut items = Vec::new();
		let mut parsed_globals = false;
		let mut parsed_variables = false;

		while !input.is_empty() {
			if input.peek(keyword::globals) {
				let keyword: keyword::globals = input.parse()?;
				if parsed_globals {
					return Err(syn::Error::new(
						keyword.span,
						"only one `globals` block is allowed",
					));
				}
				parsed_globals = true;
				globals = parse_globals_block(input)?;
			} else if input.peek(keyword::vars) {
				let keyword: keyword::vars = input.parse()?;
				if parsed_variables {
					return Err(syn::Error::new(
						keyword.span,
						"only one `vars` block is allowed",
					));
				}
				parsed_variables = true;
				variables = parse_variables_block(input)?;
			} else if input.peek(Token![@]) {
				items.push(StyleItem::Media(parse_media_rule(
					input,
					SelectorContext::TopLevel,
				)?));
			} else {
				items.push(StyleItem::Rule(parse_rule(
					input,
					SelectorContext::TopLevel,
				)?));
			}
		}

		Ok(Self {
			globals,
			variables,
			items,
			span,
		})
	}
}

fn parse_globals_block(input: ParseStream) -> syn::Result<Vec<StyleGlobalDeclaration>> {
	let content;
	braced!(content in input);
	let mut declarations = Vec::new();
	while !content.is_empty() {
		let name = parse_binding_name(&content)?;
		content.parse::<Token![:]>()?;
		let ty = parse_dsl_type(&content)?;
		content.parse::<Token![;]>()?;
		declarations.push(StyleGlobalDeclaration {
			span: joined_span(name.span, ty.span),
			name,
			ty,
		});
	}
	Ok(declarations)
}

fn parse_variables_block(input: ParseStream) -> syn::Result<Vec<StyleVariableDeclaration>> {
	let content;
	braced!(content in input);
	let mut declarations = Vec::new();
	while !content.is_empty() {
		let name = parse_binding_name(&content)?;
		content.parse::<Token![:]>()?;
		let ty = parse_dsl_type(&content)?;
		let default = if content.peek(Token![=]) {
			content.parse::<Token![=]>()?;
			Some(parse_value_expression(&content)?)
		} else {
			None
		};
		content.parse::<Token![;]>()?;
		declarations.push(StyleVariableDeclaration {
			span: joined_span(
				name.span,
				default.as_ref().map_or(ty.span, |value| value.span),
			),
			name,
			ty,
			default,
		});
	}
	Ok(declarations)
}

fn parse_binding_name(input: ParseStream) -> syn::Result<StyleBindingName> {
	let ident = Ident::parse_any(input)?;
	Ok(StyleBindingName {
		value: unraw_ident(&ident),
		span: ident.span(),
	})
}

fn parse_dsl_type(input: ParseStream) -> syn::Result<StyleDslType> {
	let ident = Ident::parse_any(input)?;
	Ok(StyleDslType {
		name: unraw_ident(&ident),
		span: ident.span(),
	})
}

pub(super) fn unraw_ident(ident: &Ident) -> String {
	ident.unraw().to_string()
}

pub(super) fn is_css_decimal_number(value: &str) -> bool {
	let bytes = value.as_bytes();
	let mut index = 0;
	while matches!(bytes.get(index), Some(byte) if byte.is_ascii_digit()) {
		index += 1;
	}
	if index == 0 {
		return false;
	}
	if bytes.get(index) == Some(&b'.') {
		index += 1;
		let fraction_start = index;
		while matches!(bytes.get(index), Some(byte) if byte.is_ascii_digit()) {
			index += 1;
		}
		if fraction_start == index {
			return false;
		}
	}
	if matches!(bytes.get(index), Some(b'e' | b'E')) {
		index += 1;
		if matches!(bytes.get(index), Some(b'+' | b'-')) {
			index += 1;
		}
		let exponent_start = index;
		while matches!(bytes.get(index), Some(byte) if byte.is_ascii_digit()) {
			index += 1;
		}
		if exponent_start == index {
			return false;
		}
	}
	index == bytes.len()
}

fn parse_rule(input: ParseStream, selector_context: SelectorContext) -> syn::Result<StyleRule> {
	let selector_start = input.span();
	let selector_tokens = collect_until_brace(input, "expected a style rule body")?;
	let selectors = parse_selector_list(selector_tokens, selector_start, selector_context)?;
	let content;
	braced!(content in input);
	let items = parse_rule_items(&content, SelectorContext::Nested)?;
	Ok(StyleRule {
		span: selectors.span,
		selectors,
		items,
	})
}

fn parse_rule_items(
	input: ParseStream,
	selector_context: SelectorContext,
) -> syn::Result<Vec<StyleRuleItem>> {
	let mut items = Vec::new();
	while !input.is_empty() {
		if input.peek(Token![@]) {
			items.push(StyleRuleItem::Media(parse_media_rule(
				input,
				selector_context,
			)?));
		} else if next_item_is_rule(input)? {
			items.push(StyleRuleItem::Rule(parse_rule(input, selector_context)?));
		} else {
			items.push(StyleRuleItem::Declaration(parse_declaration(input)?));
		}
	}
	Ok(items)
}

fn next_item_is_rule(input: ParseStream) -> syn::Result<bool> {
	let fork = input.fork();
	while !fork.is_empty() {
		if fork.peek(syn::token::Brace) {
			return Ok(true);
		}
		if fork.peek(Token![;]) {
			return Ok(false);
		}
		fork.parse::<TokenTree>()?;
	}
	Err(input.error("expected a declaration or nested rule"))
}

fn parse_declaration(input: ParseStream) -> syn::Result<StyleDeclaration> {
	let name = input.parse::<CssName>()?;
	input.parse::<Token![:]>()?;
	let value = parse_value_expression(input)?;
	input.parse::<Token![;]>()?;
	Ok(StyleDeclaration {
		span: joined_span(name.span, value.span),
		name,
		value,
	})
}

fn parse_media_rule(
	input: ParseStream,
	selector_context: SelectorContext,
) -> syn::Result<StyleMediaRule> {
	let at: Token![@] = input.parse()?;
	input.parse::<keyword::media>()?;
	let condition_tokens = collect_until_brace(input, "expected an `@media` body")?;
	let condition = parse_media_condition(condition_tokens, input.span())?;
	let content;
	braced!(content in input);
	let items = parse_rule_items(&content, selector_context)?;
	Ok(StyleMediaRule {
		condition,
		items,
		span: at.span(),
	})
}

fn collect_until_brace(input: ParseStream, message: &str) -> syn::Result<Vec<TokenTree>> {
	let mut tokens = Vec::new();
	while !input.is_empty() && !input.peek(syn::token::Brace) {
		tokens.push(input.parse()?);
	}
	if input.is_empty() {
		return Err(syn::Error::new(input.span(), message));
	}
	Ok(tokens)
}

fn joined_span(first: Span, last: Span) -> Span {
	first.join(last).unwrap_or(first)
}

#[cfg(test)]
mod tests {
	use quote::quote;
	use rstest::rstest;

	use super::parse_style;
	use crate::core::{
		StyleItem, StyleMediaCondition, StyleMediaPunctuationKind, StyleMediaToken,
		StyleNumericUnit, StyleReferenceNamespace, StyleRuleItem, StyleSelectorCombinator,
		StyleSelectorKind, StyleSimpleSelector, StyleValueExpr, StyleValueExpression,
		StyleValueLiteral,
	};

	fn assert_integer_value(
		expression: &StyleValueExpression,
		expected_source: &str,
		expected_unit: Option<&str>,
	) {
		let StyleValueExpr::Literal(StyleValueLiteral::Integer(number)) = &expression.kind else {
			panic!("expected an integer value literal");
		};
		assert_eq!(number.source, expected_source);
		let unit = number.unit.as_ref().map(|unit| match unit {
			StyleNumericUnit::Named(name) => name.as_str(),
			StyleNumericUnit::Percentage { .. } => "%",
		});
		assert_eq!(unit, expected_unit);
		assert_eq!(
			number.contextual_zero,
			expected_source == "0" && expected_unit.is_none()
		);
	}

	fn assert_keyword_value(expression: &StyleValueExpression, expected: &str) {
		let StyleValueExpr::Literal(StyleValueLiteral::Keyword(keyword)) = &expression.kind else {
			panic!("expected a keyword value literal");
		};
		assert_eq!(keyword.as_str(), expected);
	}

	fn assert_reference_value(
		expression: &StyleValueExpression,
		expected_namespace: StyleReferenceNamespace,
		expected_name: &str,
	) {
		let StyleValueExpr::QualifiedReference(reference) = &expression.kind else {
			panic!("expected a qualified style reference");
		};
		assert_eq!(reference.namespace, expected_namespace);
		assert_eq!(reference.name.as_str(), expected_name);
	}

	fn assert_max_width_condition(condition: &StyleMediaCondition) {
		assert_eq!(condition.tokens.len(), 1);
		let StyleMediaToken::Parenthesized(group) = &condition.tokens[0] else {
			panic!("expected a parenthesized media feature");
		};
		assert_eq!(group.tokens.len(), 3);
		let StyleMediaToken::Identifier(feature) = &group.tokens[0] else {
			panic!("expected a media feature name");
		};
		assert_eq!(feature.as_str(), "max-width");
		let StyleMediaToken::Punctuation(punctuation) = &group.tokens[1] else {
			panic!("expected media punctuation");
		};
		assert_eq!(punctuation.kind, StyleMediaPunctuationKind::Colon);
		let StyleMediaToken::Number(number) = &group.tokens[2] else {
			panic!("expected a numeric media value");
		};
		assert_eq!(number.value, "640");
		assert_eq!(number.unit.as_deref(), Some("px"));
	}

	#[rstest]
	fn parses_empty_style_definition() {
		// Arrange
		let input = quote! {};

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		assert_eq!(style.globals.len(), 0);
		assert_eq!(style.variables.len(), 0);
		assert_eq!(style.items.len(), 0);
	}

	#[rstest]
	fn parses_globals_block_in_source_order() {
		// Arrange
		let input = quote! {
			globals {
				border: Color;
				surface_secondary: Color;
			}
		};

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		assert_eq!(style.globals.len(), 2);
		assert_eq!(style.globals[0].name.as_str(), "border");
		assert_eq!(style.globals[0].ty.as_str(), "Color");
		assert_eq!(style.globals[1].name.as_str(), "surface_secondary");
		assert_eq!(style.globals[1].ty.as_str(), "Color");
		assert_eq!(style.variables.len(), 0);
		assert_eq!(style.items.len(), 0);
	}

	#[rstest]
	fn parses_vars_block_with_structured_defaults() {
		// Arrange
		let input = quote! {
			vars {
				padding: Length = 1rem;
				accent: Color = globals.surface_secondary;
			}
		};

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		assert_eq!(style.globals.len(), 0);
		assert_eq!(style.variables.len(), 2);
		assert_eq!(style.variables[0].name.as_str(), "padding");
		assert_eq!(style.variables[0].ty.as_str(), "Length");
		assert_integer_value(
			style.variables[0].default.as_ref().unwrap(),
			"1",
			Some("rem"),
		);
		assert_eq!(style.variables[1].name.as_str(), "accent");
		assert_eq!(style.variables[1].ty.as_str(), "Color");
		assert_reference_value(
			style.variables[1].default.as_ref().unwrap(),
			StyleReferenceNamespace::Globals,
			"surface_secondary",
		);
		assert_eq!(style.items.len(), 0);
	}

	#[rstest]
	fn parses_nested_value_expression_structure() {
		// Arrange
		let input = quote! {
			vars {
				expression: Value = calc((a + b), path::value);
			}
		};

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		let StyleValueExpr::Call(call) = &style.variables[0].default.as_ref().unwrap().kind else {
			panic!("expected a function call");
		};
		assert_eq!(call.path.segments.len(), 1);
		assert_eq!(call.path.segments[0].as_str(), "calc");
		assert_eq!(call.arguments.len(), 2);
		let StyleValueExpr::Group(group) = &call.arguments[0].kind else {
			panic!("expected a grouped first argument");
		};
		assert!(matches!(group.expression.kind, StyleValueExpr::Binary(_)));
		let StyleValueExpr::AssociatedPathValue(path) = &call.arguments[1].kind else {
			panic!("expected an associated path second argument");
		};
		assert_eq!(path.segments.len(), 2);
		assert_eq!(path.segments[0].as_str(), "path");
		assert_eq!(path.segments[1].as_str(), "value");
	}

	#[rstest]
	fn retains_duplicate_source_names_for_validation() {
		// Arrange
		let input = quote! {
			globals {
				tone: Color;
				tone: Length;
			}
			vars {
				gap: Length = 1rem;
				gap: Length = 2rem;
			}
		};

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		assert_eq!(style.globals.len(), 2);
		assert_eq!(style.globals[0].name.as_str(), "tone");
		assert_eq!(style.globals[0].ty.as_str(), "Color");
		assert_eq!(style.globals[1].name.as_str(), "tone");
		assert_eq!(style.globals[1].ty.as_str(), "Length");
		assert_eq!(style.variables.len(), 2);
		assert_eq!(style.variables[0].name.as_str(), "gap");
		assert_integer_value(
			style.variables[0].default.as_ref().unwrap(),
			"1",
			Some("rem"),
		);
		assert_eq!(style.variables[1].name.as_str(), "gap");
		assert_integer_value(
			style.variables[1].default.as_ref().unwrap(),
			"2",
			Some("rem"),
		);
		assert_eq!(style.items.len(), 0);
	}

	#[rstest]
	fn parses_one_top_level_class_rule() {
		// Arrange
		let input = quote! {
			.card {
				border-color: red;
			}
		};

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		assert_eq!(style.globals.len(), 0);
		assert_eq!(style.variables.len(), 0);
		assert_eq!(style.items.len(), 1);
		let StyleItem::Rule(rule) = &style.items[0] else {
			panic!("expected a top-level rule");
		};
		assert_eq!(rule.selectors.selectors.len(), 1);
		let StyleSelectorKind::Root(StyleSimpleSelector::Class(class)) =
			&rule.selectors.selectors[0].kind
		else {
			panic!("expected a root class selector");
		};
		assert_eq!(class.as_str(), "card");
		assert_eq!(rule.items.len(), 1);
		let StyleRuleItem::Declaration(declaration) = &rule.items[0] else {
			panic!("expected a declaration");
		};
		assert_eq!(declaration.name.as_str(), "border-color");
		assert_keyword_value(&declaration.value, "red");
	}

	#[rstest]
	fn preserves_interleaved_top_level_source_order() {
		// Arrange
		let input = quote! {
			.first {}
			@media (max-width: 640px) {
				.compact {}
			}
			.last {}
		};

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		assert_eq!(style.globals.len(), 0);
		assert_eq!(style.variables.len(), 0);
		assert_eq!(style.items.len(), 3);
		let StyleItem::Rule(first) = &style.items[0] else {
			panic!("expected the first item to be a rule");
		};
		assert_eq!(first.selectors.selectors.len(), 1);
		let StyleSelectorKind::Root(StyleSimpleSelector::Class(first_class)) =
			&first.selectors.selectors[0].kind
		else {
			panic!("expected a root class selector");
		};
		assert_eq!(first_class.as_str(), "first");
		assert_eq!(first.items.len(), 0);

		let StyleItem::Media(media) = &style.items[1] else {
			panic!("expected the second item to be a media rule");
		};
		assert_max_width_condition(&media.condition);
		assert_eq!(media.items.len(), 1);
		let StyleRuleItem::Rule(compact) = &media.items[0] else {
			panic!("expected a rule inside the media block");
		};
		assert_eq!(compact.selectors.selectors.len(), 1);
		let StyleSelectorKind::Root(StyleSimpleSelector::Class(compact_class)) =
			&compact.selectors.selectors[0].kind
		else {
			panic!("expected a root class selector");
		};
		assert_eq!(compact_class.as_str(), "compact");
		assert_eq!(compact.items.len(), 0);

		let StyleItem::Rule(last) = &style.items[2] else {
			panic!("expected the third item to be a rule");
		};
		assert_eq!(last.selectors.selectors.len(), 1);
		let StyleSelectorKind::Root(StyleSimpleSelector::Class(last_class)) =
			&last.selectors.selectors[0].kind
		else {
			panic!("expected a root class selector");
		};
		assert_eq!(last_class.as_str(), "last");
		assert_eq!(last.items.len(), 0);
	}

	#[rstest]
	fn preserves_nested_rule_item_source_order() {
		// Arrange
		let input = quote! {
			.card {
				color: red;
				&:hover {
					color: blue;
				}
				@media (max-width: 640px) {
					padding: 1rem;
					.label {}
				}
				background: white;
			}
		};

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		assert_eq!(style.items.len(), 1);
		let StyleItem::Rule(rule) = &style.items[0] else {
			panic!("expected a top-level rule");
		};
		assert_eq!(rule.items.len(), 4);

		let StyleRuleItem::Declaration(color) = &rule.items[0] else {
			panic!("expected the first rule item to be a declaration");
		};
		assert_eq!(color.name.as_str(), "color");
		assert_keyword_value(&color.value, "red");

		let StyleRuleItem::Rule(hover) = &rule.items[1] else {
			panic!("expected the second rule item to be a nested rule");
		};
		assert_eq!(hover.selectors.selectors.len(), 1);
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Pseudo(pseudo)) =
			&hover.selectors.selectors[0].kind
		else {
			panic!("expected a same-element pseudo selector");
		};
		assert_eq!(pseudo.name.as_str(), "hover");
		assert_eq!(hover.items.len(), 1);

		let StyleRuleItem::Media(media) = &rule.items[2] else {
			panic!("expected the third rule item to be a media rule");
		};
		assert_max_width_condition(&media.condition);
		assert_eq!(media.items.len(), 2);
		let StyleRuleItem::Declaration(padding) = &media.items[0] else {
			panic!("expected the first media item to be a declaration");
		};
		assert_eq!(padding.name.as_str(), "padding");
		assert_integer_value(&padding.value, "1", Some("rem"));
		let StyleRuleItem::Rule(label) = &media.items[1] else {
			panic!("expected the second media item to be a rule");
		};
		assert_eq!(label.selectors.selectors.len(), 1);
		let StyleSelectorKind::Relative {
			combinator: StyleSelectorCombinator::Descendant,
			selector: StyleSimpleSelector::Class(label_class),
		} = &label.selectors.selectors[0].kind
		else {
			panic!("expected a descendant class selector");
		};
		assert_eq!(label_class.as_str(), "label");
		assert_eq!(label.items.len(), 0);

		let StyleRuleItem::Declaration(background) = &rule.items[3] else {
			panic!("expected the fourth rule item to be a declaration");
		};
		assert_eq!(background.name.as_str(), "background");
		assert_keyword_value(&background.value, "white");
	}

	#[rstest]
	#[case(
		quote! { globals {} globals {} },
		"only one `globals` block is allowed"
	)]
	#[case(quote! { vars {} vars {} }, "only one `vars` block is allowed")]
	fn rejects_duplicate_definition_blocks(
		#[case] input: proc_macro2::TokenStream,
		#[case] expected: &str,
	) {
		// Arrange
		// Input and expectation are provided by the parameterized case.

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(error.to_string(), expected);
	}

	#[rstest]
	fn retains_a_missing_default_for_semantic_validation() {
		// Arrange
		let input = quote! { vars { padding: Length; } };

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		assert_eq!(style.variables.len(), 1);
		assert_eq!(style.variables[0].name.as_str(), "padding");
		assert!(style.variables[0].default.is_none());
	}

	#[rstest]
	#[case(quote! { globals { border: ; } })]
	#[case(quote! { vars { padding: = 1rem; } })]
	fn requires_a_dsl_type_for_each_definition(#[case] input: proc_macro2::TokenStream) {
		// Arrange
		// Input is provided by the parameterized case.

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(error.to_string(), "expected ident");
	}
}
