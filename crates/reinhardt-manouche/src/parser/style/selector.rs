//! Parser for structural style selectors.

use proc_macro2::{Delimiter, Spacing, Span, TokenTree};
use syn::Lit;

use super::unraw_ident;
use crate::core::{
	StyleAttributeMatcher, StyleAttributeSelector, StyleAttributeValue, StyleNthSelectorArguments,
	StylePseudoSelector, StyleSelector, StyleSelectorArguments, StyleSelectorCombinator,
	StyleSelectorKind, StyleSelectorList, StyleSelectorName, StyleSimpleSelector,
};

#[derive(Clone, Copy)]
pub(super) enum SelectorContext {
	TopLevel,
	Nested,
	SelectorFunction { relative: bool },
}

pub(super) fn parse_selector_list(
	tokens: Vec<TokenTree>,
	empty_selector_span: Span,
	context: SelectorContext,
) -> syn::Result<StyleSelectorList> {
	let span = span_of_tokens(&tokens)
		.ok_or_else(|| syn::Error::new(empty_selector_span, "expected at least one selector"))?;
	let mut selectors = Vec::new();
	let mut current = Vec::new();
	let mut last_comma_span = None;

	for token in tokens {
		if matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ',') {
			if current.is_empty() {
				return Err(syn::Error::new(
					token.span(),
					"selector list branches cannot be empty",
				));
			}
			selectors.push(parse_selector_branch(&current, context)?);
			current.clear();
			last_comma_span = Some(token.span());
		} else {
			current.push(token);
		}
	}

	if current.is_empty() {
		return Err(syn::Error::new(
			last_comma_span.unwrap_or(span),
			"expected a selector after `,`",
		));
	}
	selectors.push(parse_selector_branch(&current, context)?);

	Ok(StyleSelectorList { selectors, span })
}

fn parse_selector_branch(
	tokens: &[TokenTree],
	context: SelectorContext,
) -> syn::Result<StyleSelector> {
	let mut index = 0;
	let first_span = tokens[0].span();
	let kind = match context {
		SelectorContext::TopLevel => {
			if punct_at(tokens, index, '&') {
				return Err(syn::Error::new(
					first_span,
					"same-element selectors are only valid inside a style rule",
				));
			}
			if combinator_at(tokens, index).is_some() {
				return Err(syn::Error::new(
					first_span,
					"selector relationships are only valid inside a style rule",
				));
			}
			StyleSelectorKind::Root(parse_simple_selector(tokens, &mut index)?)
		}
		SelectorContext::Nested if punct_at(tokens, index, '&') => {
			index += 1;
			if index == tokens.len() {
				return Err(syn::Error::new(
					first_span,
					"expected a class, attribute, or pseudo selector after `&`",
				));
			}
			let selector = parse_simple_selector(tokens, &mut index)?;
			match selector {
				StyleSimpleSelector::Type(_) => {
					return Err(syn::Error::new(
						selector.span(),
						"same-element type restrictions must use a pseudo-class such as `&:is(button)`",
					));
				}
				StyleSimpleSelector::Id(_) | StyleSimpleSelector::Universal { .. } => {
					return Err(syn::Error::new(
						selector.span(),
						"expected a class, attribute, or pseudo selector after `&`",
					));
				}
				StyleSimpleSelector::Class(_)
				| StyleSimpleSelector::Attribute(_)
				| StyleSimpleSelector::Pseudo(_) => StyleSelectorKind::SameElement(selector),
			}
		}
		SelectorContext::Nested => {
			let combinator = combinator_at(tokens, index);
			if combinator.is_some() {
				index += 1;
			}
			let selector = parse_simple_selector(tokens, &mut index)?;
			StyleSelectorKind::Relative {
				combinator: combinator.unwrap_or(StyleSelectorCombinator::Descendant),
				selector,
			}
		}
		SelectorContext::SelectorFunction { relative } => {
			if punct_at(tokens, index, '&') {
				return Err(syn::Error::new(
					first_span,
					"`&` is not allowed inside selector-list pseudo-functions",
				));
			}
			let combinator = combinator_at(tokens, index);
			if combinator.is_some() && !relative {
				return Err(syn::Error::new(
					first_span,
					"selector relationships inside pseudo-functions are only supported by `:has(...)`",
				));
			}
			if combinator.is_some() {
				index += 1;
			}
			let selector = parse_simple_selector(tokens, &mut index)?;
			if relative {
				StyleSelectorKind::Relative {
					combinator: combinator.unwrap_or(StyleSelectorCombinator::Descendant),
					selector,
				}
			} else {
				StyleSelectorKind::Root(selector)
			}
		}
	};

	if index != tokens.len() {
		return Err(syn::Error::new(
			tokens[index].span(),
			"selector heads must contain exactly one simple selector; express relationships and refinements with nested rules",
		));
	}
	let last_span = match &kind {
		StyleSelectorKind::Root(selector) | StyleSelectorKind::SameElement(selector) => {
			selector.span()
		}
		StyleSelectorKind::Relative { selector, .. } => selector.span(),
	};

	Ok(StyleSelector {
		kind,
		span: joined_span(first_span, last_span),
	})
}

fn parse_simple_selector(
	tokens: &[TokenTree],
	index: &mut usize,
) -> syn::Result<StyleSimpleSelector> {
	let Some(token) = tokens.get(*index) else {
		return Err(syn::Error::new(
			last_span(tokens),
			"expected a simple selector",
		));
	};

	match token {
		TokenTree::Punct(punct) if punct.as_char() == '.' => {
			*index += 1;
			let name = parse_local_class_name(tokens, index)?;
			Ok(StyleSimpleSelector::Class(name))
		}
		TokenTree::Punct(punct) if punct.as_char() == '#' => {
			*index += 1;
			let name = parse_selector_name(tokens, index)?;
			Ok(StyleSimpleSelector::Id(name))
		}
		TokenTree::Punct(punct) if punct.as_char() == '*' => {
			let span = punct.span();
			*index += 1;
			Ok(StyleSimpleSelector::Universal { span })
		}
		TokenTree::Punct(punct) if punct.as_char() == ':' => parse_pseudo_selector(tokens, index),
		TokenTree::Group(group) if group.delimiter() == Delimiter::Bracket => {
			let attribute = parse_attribute_selector(group)?;
			*index += 1;
			Ok(StyleSimpleSelector::Attribute(attribute))
		}
		TokenTree::Ident(_) => Ok(StyleSimpleSelector::Type(parse_selector_name(
			tokens, index,
		)?)),
		_ => Err(syn::Error::new(token.span(), "expected a simple selector")),
	}
}

fn parse_pseudo_selector(
	tokens: &[TokenTree],
	index: &mut usize,
) -> syn::Result<StyleSimpleSelector> {
	let colon_span = tokens[*index].span();
	*index += 1;
	let pseudo_element = punct_at(tokens, *index, ':');
	if pseudo_element {
		*index += 1;
	}
	if *index == tokens.len() {
		return Err(syn::Error::new(
			colon_span,
			"expected a pseudo selector name after `:`",
		));
	}
	let name = parse_selector_name(tokens, index)?;
	let selector_context = selector_function_context(name.as_str());
	let nth_function = is_nth_selector_function(name.as_str());
	let arguments = match tokens.get(*index) {
		Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
			*index += 1;
			let tokens = group.stream().into_iter().collect::<Vec<_>>();
			let (selector_list, nth) = if let Some(context) = selector_context {
				(
					Some(parse_selector_list(tokens.clone(), group.span(), context)?),
					None,
				)
			} else if nth_function {
				let (selector_list, nth) =
					parse_nth_selector_arguments(&tokens, group.span(), name.as_str())?;
				(selector_list, Some(nth))
			} else {
				if !name.as_str().eq_ignore_ascii_case("global") {
					reject_unstructured_selector_markers(name.as_str(), &tokens)?;
				}
				(None, None)
			};
			Some(StyleSelectorArguments {
				tokens,
				selector_list,
				nth,
				span: group.span(),
			})
		}
		_ if selector_context.is_some() => {
			return Err(syn::Error::new(
				name.span,
				format!(
					"`:{}` requires a parenthesized selector list",
					name.as_str()
				),
			));
		}
		_ if nth_function => {
			return Err(syn::Error::new(
				name.span,
				format!("`:{}` requires parenthesized nth arguments", name.as_str()),
			));
		}
		_ => None,
	};
	let last_span = arguments
		.as_ref()
		.map_or(name.span, |arguments| arguments.span);

	Ok(StyleSimpleSelector::Pseudo(StylePseudoSelector {
		name,
		is_element: pseudo_element,
		arguments,
		span: joined_span(colon_span, last_span),
	}))
}

fn is_nth_selector_function(name: &str) -> bool {
	name.eq_ignore_ascii_case("nth-child") || name.eq_ignore_ascii_case("nth-last-child")
}

fn parse_nth_selector_arguments(
	tokens: &[TokenTree],
	arguments_span: Span,
	function: &str,
) -> syn::Result<(Option<StyleSelectorList>, StyleNthSelectorArguments)> {
	let separator = tokens.iter().position(
		|token| matches!(token, TokenTree::Ident(ident) if unraw_ident(ident).eq_ignore_ascii_case("of")),
	);
	let (formula_tokens, of_span, selector_list) = if let Some(separator) = separator {
		let formula_tokens = tokens[..separator].to_vec();
		if formula_tokens.is_empty() {
			return Err(syn::Error::new(
				tokens[separator].span(),
				format!("`:{function}(...)` requires a formula before `of`"),
			));
		}
		validate_nth_formula(&formula_tokens, function)?;
		let of_span = tokens[separator].span();
		let selector_list = parse_selector_list(
			tokens[separator + 1..].to_vec(),
			arguments_span,
			SelectorContext::SelectorFunction { relative: false },
		)?;
		(formula_tokens, Some(of_span), Some(selector_list))
	} else {
		if tokens.is_empty() {
			return Err(syn::Error::new(
				arguments_span,
				format!("`:{function}(...)` requires an nth formula"),
			));
		}
		validate_nth_formula(tokens, function)?;
		(tokens.to_vec(), None, None)
	};
	let formula_span = span_of_tokens(&formula_tokens).unwrap_or(arguments_span);
	Ok((
		selector_list,
		StyleNthSelectorArguments {
			formula_tokens,
			formula_span,
			of_span,
		},
	))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NthFormulaTerm {
	Integer,
	N,
}

fn validate_nth_formula(tokens: &[TokenTree], function: &str) -> syn::Result<()> {
	if tokens.len() == 1
		&& matches!(
			&tokens[0],
			TokenTree::Ident(ident)
				if unraw_ident(ident).eq_ignore_ascii_case("odd")
					|| unraw_ident(ident).eq_ignore_ascii_case("even")
		) {
		return Ok(());
	}

	let mut index = 0;
	if nth_sign(tokens.get(index)) {
		index += 1;
	}
	let Some(token) = tokens.get(index) else {
		return Err(invalid_nth_formula(function, last_span(tokens)));
	};
	let Some(term) = nth_formula_term(token) else {
		return Err(invalid_nth_formula(function, token.span()));
	};
	index += 1;

	if term == NthFormulaTerm::Integer {
		return if index == tokens.len() {
			Ok(())
		} else {
			Err(invalid_nth_formula(function, tokens[index].span()))
		};
	}
	if index == tokens.len() {
		return Ok(());
	}
	if !nth_sign(tokens.get(index)) {
		return Err(invalid_nth_formula(function, tokens[index].span()));
	}
	let sign_span = tokens[index].span();
	index += 1;
	let Some(offset) = tokens.get(index) else {
		return Err(invalid_nth_formula(function, sign_span));
	};
	if nth_formula_term(offset) != Some(NthFormulaTerm::Integer) {
		return Err(invalid_nth_formula(function, offset.span()));
	}
	index += 1;
	if index == tokens.len() {
		Ok(())
	} else {
		Err(invalid_nth_formula(function, tokens[index].span()))
	}
}

fn nth_formula_term(token: &TokenTree) -> Option<NthFormulaTerm> {
	match token {
		TokenTree::Ident(ident) if unraw_ident(ident).eq_ignore_ascii_case("n") => {
			Some(NthFormulaTerm::N)
		}
		TokenTree::Literal(literal) => {
			let source = literal.to_string();
			if source.bytes().all(|byte| byte.is_ascii_digit()) {
				Some(NthFormulaTerm::Integer)
			} else {
				let digits = source
					.strip_suffix('n')
					.or_else(|| source.strip_suffix('N'))?;
				(!digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
					.then_some(NthFormulaTerm::N)
			}
		}
		TokenTree::Punct(_) | TokenTree::Group(_) | TokenTree::Ident(_) => None,
	}
}

fn nth_sign(token: Option<&TokenTree>) -> bool {
	matches!(token, Some(TokenTree::Punct(punct)) if matches!(punct.as_char(), '+' | '-'))
}

fn invalid_nth_formula(function: &str, span: Span) -> syn::Error {
	syn::Error::new(
		span,
		format!("`:{function}(...)` contains an invalid An+B formula"),
	)
}

enum RawSelectorMarker {
	Class(Span),
	Global(Span),
}

fn reject_unstructured_selector_markers(function: &str, tokens: &[TokenTree]) -> syn::Result<()> {
	match find_raw_selector_marker(tokens) {
		Some(RawSelectorMarker::Class(span)) => Err(syn::Error::new(
			span,
			format!(
				"pseudo-function `:{function}(...)` contains an unstructured class selector; use `:is`, `:not`, `:where`, `:has`, `:nth-child`, or `:nth-last-child`"
			),
		)),
		Some(RawSelectorMarker::Global(span)) => Err(syn::Error::new(
			span,
			format!(
				"pseudo-function `:{function}(...)` contains unsupported `:global`; use component-local selectors"
			),
		)),
		None => Ok(()),
	}
}

fn find_raw_selector_marker(tokens: &[TokenTree]) -> Option<RawSelectorMarker> {
	for (index, token) in tokens.iter().enumerate() {
		if matches!(token, TokenTree::Punct(punct) if punct.as_char() == ':')
			&& matches!(tokens.get(index + 1), Some(TokenTree::Ident(ident)) if unraw_ident(ident).eq_ignore_ascii_case("global"))
		{
			return Some(RawSelectorMarker::Global(token.span()));
		}
		if matches!(token, TokenTree::Punct(punct) if punct.as_char() == '.')
			&& raw_tokens_start_class(&tokens[index + 1..])
		{
			return Some(RawSelectorMarker::Class(token.span()));
		}
		if let TokenTree::Group(group) = token
			&& let Some(marker) =
				find_raw_selector_marker(&group.stream().into_iter().collect::<Vec<_>>())
		{
			return Some(marker);
		}
	}
	None
}

fn raw_tokens_start_class(tokens: &[TokenTree]) -> bool {
	matches!(tokens.first(), Some(TokenTree::Ident(_)))
		|| (matches!(tokens.first(), Some(TokenTree::Punct(punct)) if punct.as_char() == '-')
			&& matches!(tokens.get(1), Some(TokenTree::Ident(_))))
}

fn selector_function_context(name: &str) -> Option<SelectorContext> {
	if name.eq_ignore_ascii_case("has") {
		Some(SelectorContext::SelectorFunction { relative: true })
	} else if ["is", "not", "where"]
		.iter()
		.any(|known| name.eq_ignore_ascii_case(known))
	{
		Some(SelectorContext::SelectorFunction { relative: false })
	} else {
		None
	}
}

fn parse_attribute_selector(group: &proc_macro2::Group) -> syn::Result<StyleAttributeSelector> {
	let tokens: Vec<_> = group.stream().into_iter().collect();
	if tokens.is_empty() {
		return Err(syn::Error::new(
			group.span(),
			"attribute selectors cannot be empty",
		));
	}
	let mut index = 0;
	let name = parse_selector_name(&tokens, &mut index)?;
	if index == tokens.len() {
		return Ok(StyleAttributeSelector {
			name,
			matcher: None,
			value: None,
			modifier: None,
			span: group.span(),
		});
	}

	let matcher = parse_attribute_matcher(&tokens, &mut index)?;
	let value = parse_attribute_value(&tokens, &mut index)?;
	let modifier = if index < tokens.len() {
		let modifier = parse_selector_name(&tokens, &mut index)?;
		if !["i", "s"]
			.iter()
			.any(|candidate| modifier.as_str().eq_ignore_ascii_case(candidate))
		{
			return Err(syn::Error::new(
				modifier.span,
				"attribute selector modifiers must be `i` or `s`",
			));
		}
		Some(modifier)
	} else {
		None
	};
	if index != tokens.len() {
		return Err(syn::Error::new(
			tokens[index].span(),
			"unexpected token in attribute selector",
		));
	}

	Ok(StyleAttributeSelector {
		name,
		matcher: Some(matcher),
		value: Some(value),
		modifier,
		span: group.span(),
	})
}

fn parse_attribute_matcher(
	tokens: &[TokenTree],
	index: &mut usize,
) -> syn::Result<StyleAttributeMatcher> {
	let Some(TokenTree::Punct(first)) = tokens.get(*index) else {
		return Err(syn::Error::new(
			tokens[*index].span(),
			"expected an attribute selector matcher",
		));
	};
	let matcher = match first.as_char() {
		'=' => {
			*index += 1;
			StyleAttributeMatcher::Equals
		}
		'~' | '|' | '^' | '$' | '*' if punct_at(tokens, *index + 1, '=') => {
			if first.spacing() != Spacing::Joint {
				return Err(syn::Error::new(
					first.span(),
					"attribute selector matcher operators cannot contain whitespace",
				));
			}
			let matcher = match first.as_char() {
				'~' => StyleAttributeMatcher::Includes,
				'|' => StyleAttributeMatcher::DashMatch,
				'^' => StyleAttributeMatcher::Prefix,
				'$' => StyleAttributeMatcher::Suffix,
				'*' => StyleAttributeMatcher::Substring,
				_ => unreachable!(),
			};
			*index += 2;
			matcher
		}
		_ => {
			return Err(syn::Error::new(
				first.span(),
				"expected an attribute selector matcher",
			));
		}
	};
	Ok(matcher)
}

fn parse_attribute_value(
	tokens: &[TokenTree],
	index: &mut usize,
) -> syn::Result<StyleAttributeValue> {
	let Some(token) = tokens.get(*index) else {
		return Err(syn::Error::new(
			last_span(tokens),
			"expected an attribute selector value",
		));
	};
	match token {
		TokenTree::Ident(_) => Ok(StyleAttributeValue::Identifier(parse_selector_name(
			tokens, index,
		)?)),
		TokenTree::Literal(literal) => {
			let span = literal.span();
			let literal = syn::parse2::<Lit>(token.clone().into()).map_err(|_| {
				syn::Error::new(
					span,
					"attribute selector values must be identifiers or strings",
				)
			})?;
			let Lit::Str(value) = literal else {
				return Err(syn::Error::new(
					span,
					"attribute selector values must be identifiers or strings",
				));
			};
			*index += 1;
			Ok(StyleAttributeValue::String {
				value: value.value(),
				span,
			})
		}
		_ => Err(syn::Error::new(
			token.span(),
			"attribute selector values must be identifiers or strings",
		)),
	}
}

fn parse_selector_name(tokens: &[TokenTree], index: &mut usize) -> syn::Result<StyleSelectorName> {
	parse_selector_name_with_numeric_start(tokens, index, false)
}

fn parse_local_class_name(
	tokens: &[TokenTree],
	index: &mut usize,
) -> syn::Result<StyleSelectorName> {
	parse_selector_name_with_numeric_start(tokens, index, true)
}

fn parse_selector_name_with_numeric_start(
	tokens: &[TokenTree],
	index: &mut usize,
	allow_numeric_start: bool,
) -> syn::Result<StyleSelectorName> {
	let leading_hyphen = punct_at(tokens, *index, '-');
	let first_span = if leading_hyphen {
		let span = tokens[*index].span();
		*index += 1;
		span
	} else {
		tokens
			.get(*index)
			.map_or_else(|| last_span(tokens), TokenTree::span)
	};
	let Some(first) = tokens.get(*index) else {
		return Err(syn::Error::new(first_span, "expected a selector name"));
	};
	if !allow_numeric_start && matches!(first, TokenTree::Literal(_)) {
		return Err(syn::Error::new(
			first.span(),
			"selector names cannot start with a number without a CSS escape",
		));
	}
	let Some((first_value, first_segment_span)) = selector_name_segment(first)? else {
		let span = tokens
			.get(*index)
			.map_or_else(|| last_span(tokens), TokenTree::span);
		return Err(syn::Error::new(span, "expected a selector name"));
	};
	let mut last_span = first_segment_span;
	let mut value = if leading_hyphen {
		format!("-{first_value}")
	} else {
		first_value
	};
	*index += 1;

	while punct_at(tokens, *index, '-') {
		let hyphen_span = tokens[*index].span();
		*index += 1;
		if punct_at(tokens, *index, '-') {
			value.push('-');
			continue;
		}
		let Some(segment) = tokens.get(*index) else {
			return Err(syn::Error::new(
				hyphen_span,
				"expected a selector name segment after `-`",
			));
		};
		let Some((segment_value, segment_span)) = selector_name_segment(segment)? else {
			return Err(syn::Error::new(
				hyphen_span,
				"expected a selector name segment after `-`",
			));
		};
		value.push('-');
		value.push_str(&segment_value);
		last_span = segment_span;
		*index += 1;
	}

	Ok(StyleSelectorName {
		value,
		span: joined_span(first_span, last_span),
	})
}

fn selector_name_segment(token: &TokenTree) -> syn::Result<Option<(String, Span)>> {
	match token {
		TokenTree::Ident(ident) => Ok(Some((unraw_ident(ident), ident.span()))),
		TokenTree::Literal(literal) => {
			let Ok(_) = syn::parse2::<syn::LitInt>(token.clone().into()) else {
				return Ok(None);
			};
			Ok(Some((literal.to_string(), literal.span())))
		}
		_ => Ok(None),
	}
}

fn combinator_at(tokens: &[TokenTree], index: usize) -> Option<StyleSelectorCombinator> {
	let TokenTree::Punct(punct) = tokens.get(index)? else {
		return None;
	};
	match punct.as_char() {
		'>' => Some(StyleSelectorCombinator::Child),
		'+' => Some(StyleSelectorCombinator::AdjacentSibling),
		'~' => Some(StyleSelectorCombinator::GeneralSibling),
		_ => None,
	}
}

fn punct_at(tokens: &[TokenTree], index: usize, expected: char) -> bool {
	matches!(tokens.get(index), Some(TokenTree::Punct(punct)) if punct.as_char() == expected)
}

fn span_of_tokens(tokens: &[TokenTree]) -> Option<Span> {
	Some(joined_span(tokens.first()?.span(), tokens.last()?.span()))
}

fn last_span(tokens: &[TokenTree]) -> Span {
	tokens.last().map_or_else(Span::call_site, TokenTree::span)
}

fn joined_span(first: Span, last: Span) -> Span {
	first.join(last).unwrap_or(first)
}

#[cfg(test)]
mod tests {
	use proc_macro2::{Punct, Spacing};
	use quote::quote;
	use rstest::rstest;

	use super::super::parse_style;
	use crate::core::{
		StyleAttributeMatcher, StyleAttributeValue, StyleItem, StyleRule, StyleRuleItem,
		StyleSelectorCombinator, StyleSelectorKind, StyleSimpleSelector,
	};

	fn first_rule(input: proc_macro2::TokenStream) -> StyleRule {
		let style = parse_style(input).unwrap();
		let StyleItem::Rule(rule) = style.items.into_iter().next().unwrap() else {
			panic!("expected a top-level rule");
		};
		rule
	}

	fn nested_rule(rule: &StyleRule, index: usize) -> &StyleRule {
		let StyleRuleItem::Rule(nested) = &rule.items[index] else {
			panic!("expected a nested rule");
		};
		nested
	}

	fn first_nested_attribute_matcher(input: proc_macro2::TokenStream) -> StyleAttributeMatcher {
		let rule = first_rule(input);
		let selector = &nested_rule(&rule, 0).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Attribute(attribute)) =
			&selector.kind
		else {
			panic!("expected a same-element attribute selector");
		};
		attribute.matcher.unwrap()
	}

	fn attribute_rule_with_matcher(first: char, spacing: Spacing) -> proc_macro2::TokenStream {
		let first = Punct::new(first, spacing);
		let equals = Punct::new('=', Spacing::Alone);
		quote! { .card { &[data-state #first #equals open] {} } }
	}

	#[rstest]
	fn parses_top_level_local_class_selector() {
		// Arrange
		let input = quote! { .card {} };

		// Act
		let style = parse_style(input).unwrap();

		// Assert
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
	}

	#[rstest]
	fn defers_top_level_anchor_and_class_name_validation() {
		// Arrange
		let input = quote! {
			button {}
			.123card {}
		};

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		assert_eq!(style.items.len(), 2);
		let StyleItem::Rule(element_rule) = &style.items[0] else {
			panic!("expected a top-level rule");
		};
		let StyleSelectorKind::Root(StyleSimpleSelector::Type(element)) =
			&element_rule.selectors.selectors[0].kind
		else {
			panic!("expected an unvalidated root type selector");
		};
		assert_eq!(element.as_str(), "button");

		let StyleItem::Rule(class_rule) = &style.items[1] else {
			panic!("expected a top-level rule");
		};
		let StyleSelectorKind::Root(StyleSimpleSelector::Class(class)) =
			&class_rule.selectors.selectors[0].kind
		else {
			panic!("expected an unvalidated root class selector");
		};
		assert_eq!(class.as_str(), "123card");
	}

	#[rstest]
	#[case(quote! { .foo-0xff {} }, "foo-0xff")]
	#[case(quote! { .foo-1_000 {} }, "foo-1_000")]
	#[case(quote! { .123card {} }, "123card")]
	fn preserves_numeric_selector_name_segment_spelling(
		#[case] input: proc_macro2::TokenStream,
		#[case] expected: &str,
	) {
		// Arrange
		// Input and expectation are provided by the parameterized case.

		// Act
		let rule = first_rule(input);

		// Assert
		let StyleSelectorKind::Root(StyleSimpleSelector::Class(class)) =
			&rule.selectors.selectors[0].kind
		else {
			panic!("expected a root class selector");
		};
		assert_eq!(class.as_str(), expected);
	}

	#[rstest]
	fn preserves_top_level_selector_list_branch_order() {
		// Arrange
		let input = quote! { .card, .panel {} };

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		let StyleItem::Rule(rule) = &style.items[0] else {
			panic!("expected a top-level rule");
		};
		assert_eq!(rule.selectors.selectors.len(), 2);
		let class_names: Vec<_> = rule
			.selectors
			.selectors
			.iter()
			.map(|selector| match &selector.kind {
				StyleSelectorKind::Root(StyleSimpleSelector::Class(class)) => class.as_str(),
				_ => panic!("expected root class selectors"),
			})
			.collect();
		assert_eq!(class_names, vec!["card", "panel"]);
	}

	#[rstest]
	fn parses_same_element_selector_refinements() {
		// Arrange
		let input = quote! {
			.card {
				&:hover {}
				&[data-state="open"] {}
				&.featured {}
				&:is(button) {}
			}
		};

		// Act
		let rule = first_rule(input);

		// Assert
		assert_eq!(rule.items.len(), 4);

		let hover = &nested_rule(&rule, 0).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Pseudo(pseudo)) = &hover.kind
		else {
			panic!("expected a same-element pseudo selector");
		};
		assert_eq!(pseudo.name.as_str(), "hover");
		assert!(pseudo.arguments.is_none());

		let state = &nested_rule(&rule, 1).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Attribute(attribute)) = &state.kind
		else {
			panic!("expected a same-element attribute selector");
		};
		assert_eq!(attribute.name.as_str(), "data-state");
		assert_eq!(attribute.matcher, Some(StyleAttributeMatcher::Equals));
		let Some(StyleAttributeValue::String { value, .. }) = &attribute.value else {
			panic!("expected a string attribute value");
		};
		assert_eq!(value, "open");

		let featured = &nested_rule(&rule, 2).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Class(class)) = &featured.kind
		else {
			panic!("expected a same-element class selector");
		};
		assert_eq!(class.as_str(), "featured");

		let is_button = &nested_rule(&rule, 3).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Pseudo(pseudo)) = &is_button.kind
		else {
			panic!("expected a same-element pseudo-function selector");
		};
		assert_eq!(pseudo.name.as_str(), "is");
		let arguments = pseudo.arguments.as_ref().unwrap();
		assert_eq!(arguments.tokens.len(), 1);
		let proc_macro2::TokenTree::Ident(argument) = &arguments.tokens[0] else {
			panic!("expected the pseudo-function argument to retain its identifier token");
		};
		assert_eq!(argument, "button");
		let selector_list = arguments.selector_list.as_ref().unwrap();
		assert_eq!(selector_list.selectors.len(), 1);
		let StyleSelectorKind::Root(StyleSimpleSelector::Type(element)) =
			&selector_list.selectors[0].kind
		else {
			panic!("expected the selector-function argument to retain its type selector");
		};
		assert_eq!(element.as_str(), "button");
	}

	#[rstest]
	#[case("is")]
	#[case("not")]
	#[case("where")]
	#[case("has")]
	fn parses_known_selector_list_pseudo_arguments(#[case] function: &str) {
		// Arrange
		let input = format!(".card {{ &:{function}(.child) {{}} }}")
			.parse()
			.unwrap();

		// Act
		let rule = first_rule(input);

		// Assert
		let selector = &nested_rule(&rule, 0).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Pseudo(pseudo)) = &selector.kind
		else {
			panic!("expected a same-element selector-list pseudo");
		};
		assert_eq!(pseudo.name.as_str(), function);
		let selector_list = pseudo
			.arguments
			.as_ref()
			.unwrap()
			.selector_list
			.as_ref()
			.unwrap();
		let class = match &selector_list.selectors[0].kind {
			StyleSelectorKind::Root(StyleSimpleSelector::Class(class)) if function != "has" => {
				class
			}
			StyleSelectorKind::Relative {
				combinator: StyleSelectorCombinator::Descendant,
				selector: StyleSimpleSelector::Class(class),
			} if function == "has" => class,
			_ => panic!("expected the exact structured class selector argument"),
		};
		assert_eq!(class.as_str(), "child");
	}

	#[rstest]
	fn parses_relative_selector_arguments_for_has() {
		// Arrange
		let input = quote! { .card { &:has(> .child) {} } };

		// Act
		let rule = first_rule(input);

		// Assert
		let selector = &nested_rule(&rule, 0).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Pseudo(pseudo)) = &selector.kind
		else {
			panic!("expected a same-element `has` pseudo");
		};
		let selector_list = pseudo
			.arguments
			.as_ref()
			.unwrap()
			.selector_list
			.as_ref()
			.unwrap();
		let StyleSelectorKind::Relative {
			combinator: StyleSelectorCombinator::Child,
			selector: StyleSimpleSelector::Class(class),
		} = &selector_list.selectors[0].kind
		else {
			panic!("expected a direct-child class argument");
		};
		assert_eq!(class.as_str(), "child");
	}

	#[rstest]
	fn retains_lossless_raw_tokens_with_structured_selector_arguments() {
		// Arrange
		let tilde = Punct::new('~', Spacing::Joint);
		let equals = Punct::new('=', Spacing::Alone);
		let input = quote! {
			.card { &:is([data-state #tilde #equals open]) {} }
		};

		// Act
		let rule = first_rule(input);

		// Assert
		let selector = &nested_rule(&rule, 0).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Pseudo(pseudo)) = &selector.kind
		else {
			panic!("expected a same-element `is` pseudo");
		};
		let arguments = pseudo.arguments.as_ref().unwrap();
		assert_eq!(arguments.selector_list.as_ref().unwrap().selectors.len(), 1);
		assert_eq!(arguments.tokens.len(), 1);
		let proc_macro2::TokenTree::Group(attribute) = &arguments.tokens[0] else {
			panic!("expected the raw attribute token group");
		};
		assert_eq!(attribute.delimiter(), proc_macro2::Delimiter::Bracket);
		let tokens = attribute.stream().into_iter().collect::<Vec<_>>();
		assert_eq!(tokens.len(), 6);
		let proc_macro2::TokenTree::Punct(raw_tilde) = &tokens[3] else {
			panic!("expected the raw `~` matcher punctuation");
		};
		let proc_macro2::TokenTree::Punct(raw_equals) = &tokens[4] else {
			panic!("expected the raw `=` matcher punctuation");
		};
		assert_eq!(raw_tilde.as_char(), '~');
		assert_eq!(raw_tilde.spacing(), Spacing::Joint);
		assert_eq!(raw_equals.as_char(), '=');
		assert_eq!(raw_equals.spacing(), Spacing::Alone);
	}

	#[rstest]
	#[case(".card { &:is {} }", "`:is` requires a parenthesized selector list")]
	#[case(".card { &:is() {} }", "expected at least one selector")]
	#[case(".card { &:is(.foo,) {} }", "expected a selector after `,`")]
	#[case(
		".card { &:is(.foo .bar) {} }",
		"selector heads must contain exactly one simple selector; express relationships and refinements with nested rules"
	)]
	#[case(
		".card { &:is(> .foo) {} }",
		"selector relationships inside pseudo-functions are only supported by `:has(...)`"
	)]
	#[case(
		".card { &:has(&.foo) {} }",
		"`&` is not allowed inside selector-list pseudo-functions"
	)]
	#[case(".card { &:is(,.foo) {} }", "selector list branches cannot be empty")]
	#[case(
		".card { &:is(.foo,,.bar) {} }",
		"selector list branches cannot be empty"
	)]
	#[case(".card { &:has(>) {} }", "expected a simple selector")]
	fn rejects_malformed_known_selector_list_pseudo_arguments(
		#[case] input: &str,
		#[case] expected: &str,
	) {
		// Arrange
		let input = input.parse().unwrap();

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(error.to_string(), expected);
	}

	#[rstest]
	#[case("nth-child", "2n", "foo")]
	#[case("NTH-LAST-CHILD", "odd", "last")]
	fn parses_nth_formula_and_selector_list_arguments(
		#[case] function: &str,
		#[case] expected_formula: &str,
		#[case] expected_class: &str,
	) {
		// Arrange
		let input =
			format!(".card {{ &:{function}({expected_formula} OF .{expected_class}) {{}} }}")
				.parse()
				.unwrap();

		// Act
		let rule = first_rule(input);

		// Assert
		let selector = &nested_rule(&rule, 0).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Pseudo(pseudo)) = &selector.kind
		else {
			panic!("expected a same-element nth pseudo-function");
		};
		let arguments = pseudo.arguments.as_ref().unwrap();
		let nth = arguments.nth.as_ref().unwrap();
		assert_eq!(nth.formula_tokens.len(), 1);
		assert_eq!(nth.formula_tokens[0].to_string(), expected_formula);
		assert!(nth.of_span.is_some());
		let selector_list = arguments.selector_list.as_ref().unwrap();
		let StyleSelectorKind::Root(StyleSimpleSelector::Class(class)) =
			&selector_list.selectors[0].kind
		else {
			panic!("expected an absolute nth selector-list class");
		};
		assert_eq!(class.as_str(), expected_class);
		assert_eq!(arguments.tokens.len(), 4);
	}

	#[rstest]
	fn nth_formula_without_of_remains_losslessly_structured() {
		// Arrange
		let input = quote! { .card { &:nth-child(2n + 1) {} } };

		// Act
		let rule = first_rule(input);

		// Assert
		let selector = &nested_rule(&rule, 0).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Pseudo(pseudo)) = &selector.kind
		else {
			panic!("expected a same-element nth pseudo-function");
		};
		let arguments = pseudo.arguments.as_ref().unwrap();
		let nth = arguments.nth.as_ref().unwrap();
		assert_eq!(nth.formula_tokens.len(), 3);
		assert_eq!(nth.formula_tokens[0].to_string(), "2n");
		assert_eq!(nth.formula_tokens[1].to_string(), "+");
		assert_eq!(nth.formula_tokens[2].to_string(), "1");
		assert!(nth.of_span.is_none());
		assert!(arguments.selector_list.is_none());
		assert_eq!(arguments.tokens.len(), 3);
	}

	#[rstest]
	#[case("odd", &["odd"])]
	#[case("EvEn", &["EvEn"])]
	#[case("2n+1", &["2n", "+", "1"])]
	#[case("2n - 1", &["2n", "-", "1"])]
	#[case("n", &["n"])]
	#[case("N", &["N"])]
	#[case("-n+3", &["-", "n", "+", "3"])]
	#[case("+2N - 3", &["+", "2N", "-", "3"])]
	#[case("0", &["0"])]
	#[case("12", &["12"])]
	#[case("7", &["7"])]
	#[case("-7", &["-", "7"])]
	fn accepts_structural_css_nth_formulas(
		#[case] formula: &str,
		#[case] expected_tokens: &[&str],
	) {
		// Arrange
		let input = format!(".card {{ &:nth-child({formula}) {{}} }}")
			.parse()
			.unwrap();

		// Act
		let rule = first_rule(input);

		// Assert
		let selector = &nested_rule(&rule, 0).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Pseudo(pseudo)) = &selector.kind
		else {
			panic!("expected a same-element nth pseudo-function");
		};
		let arguments = pseudo.arguments.as_ref().unwrap();
		let nth = arguments.nth.as_ref().unwrap();
		let actual_tokens = nth
			.formula_tokens
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>();
		assert_eq!(actual_tokens, expected_tokens);
		assert_eq!(arguments.tokens.len(), expected_tokens.len());
		assert!(arguments.selector_list.is_none());
	}

	#[rstest]
	#[case(".card { &:nth-child(2n of) {} }", "expected at least one selector")]
	#[case(
		".card { &:nth-child(of .foo) {} }",
		"`:nth-child(...)` requires a formula before `of`"
	)]
	#[case(
		".card { &:nth-last-child(odd OF , .foo) {} }",
		"selector list branches cannot be empty"
	)]
	#[case(
		".card { &:nth-child(2n of .foo .bar) {} }",
		"selector heads must contain exactly one simple selector; express relationships and refinements with nested rules"
	)]
	fn rejects_malformed_nth_selector_arguments(#[case] input: &str, #[case] expected: &str) {
		// Arrange
		let input = input.parse().unwrap();

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(error.to_string(), expected);
	}

	#[rstest]
	#[case(".card { &:nth-child(.foo) {} }", "nth-child")]
	#[case(".card { &:nth-child(:global(.external)) {} }", "nth-child")]
	#[case(".card { &:nth-child(2n +) {} }", "nth-child")]
	#[case(".card { &:nth-child(2n + of .foo) {} }", "nth-child")]
	#[case(".card { &:nth-child(arbitrary) {} }", "nth-child")]
	#[case(".card { &:nth-child(calc(2n)) {} }", "nth-child")]
	#[case(".card { &:nth-child(formula!(2n)) {} }", "nth-child")]
	#[case(".card { &:nth-child(0xff) {} }", "nth-child")]
	#[case(".card { &:nth-child(0b10) {} }", "nth-child")]
	#[case(".card { &:nth-child(1_000) {} }", "nth-child")]
	#[case(".card { &:nth-child(0x2n) {} }", "nth-child")]
	#[case(".card { &:nth-child(1_0n) {} }", "nth-child")]
	#[case(".card { &:nth-last-child(n 3 of .foo) {} }", "nth-last-child")]
	fn rejects_non_css_nth_formulas(#[case] input: &str, #[case] function: &str) {
		// Arrange
		let input = input.parse().unwrap();

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			format!("`:{function}(...)` contains an invalid An+B formula")
		);
	}

	#[rstest]
	#[case(
		".card { &:custom(.foo) {} }",
		"pseudo-function `:custom(...)` contains an unstructured class selector; use `:is`, `:not`, `:where`, `:has`, `:nth-child`, or `:nth-last-child`"
	)]
	#[case(
		".card { &:custom(.-foo) {} }",
		"pseudo-function `:custom(...)` contains an unstructured class selector; use `:is`, `:not`, `:where`, `:has`, `:nth-child`, or `:nth-last-child`"
	)]
	#[case(
		".card { &:CuStOm(nested(.-Foo)) {} }",
		"pseudo-function `:CuStOm(...)` contains an unstructured class selector; use `:is`, `:not`, `:where`, `:has`, `:nth-child`, or `:nth-last-child`"
	)]
	#[case(
		".card { &:CUSTOM(:GLOBAL(.external)) {} }",
		"pseudo-function `:CUSTOM(...)` contains unsupported `:global`; use component-local selectors"
	)]
	fn rejects_selector_markers_in_unknown_pseudo_arguments(
		#[case] input: &str,
		#[case] expected: &str,
	) {
		// Arrange
		let input = input.parse().unwrap();

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(error.to_string(), expected);
	}

	#[rstest]
	#[case(".card { &:custom(2n + 1, nested(value)) {} }", 6)]
	#[case(".card { &:custom(.5) {} }", 2)]
	#[case(".card { &:custom(.5em) {} }", 2)]
	#[case(".card { &:custom(nested(.5)) {} }", 2)]
	#[case(".card { &:custom(nested(.5em)) {} }", 2)]
	fn non_selector_unknown_pseudo_arguments_remain_losslessly_raw(
		#[case] input: &str,
		#[case] expected_token_count: usize,
	) {
		// Arrange
		let input = input.parse().unwrap();

		// Act
		let rule = first_rule(input);

		// Assert
		let selector = &nested_rule(&rule, 0).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Pseudo(pseudo)) = &selector.kind
		else {
			panic!("expected an unknown raw pseudo-function");
		};
		let arguments = pseudo.arguments.as_ref().unwrap();
		assert_eq!(arguments.tokens.len(), expected_token_count);
		assert!(arguments.selector_list.is_none());
		assert!(arguments.nth.is_none());
	}

	#[rstest]
	#[case('~', StyleAttributeMatcher::Includes)]
	#[case('|', StyleAttributeMatcher::DashMatch)]
	#[case('^', StyleAttributeMatcher::Prefix)]
	#[case('$', StyleAttributeMatcher::Suffix)]
	#[case('*', StyleAttributeMatcher::Substring)]
	fn parses_joint_attribute_matchers(
		#[case] first: char,
		#[case] expected: StyleAttributeMatcher,
	) {
		// Arrange
		let input = attribute_rule_with_matcher(first, Spacing::Joint);

		// Act
		let matcher = first_nested_attribute_matcher(input);

		// Assert
		assert_eq!(matcher, expected);
	}

	#[rstest]
	#[case('~')]
	#[case('|')]
	#[case('^')]
	#[case('$')]
	#[case('*')]
	fn rejects_separated_attribute_matchers(#[case] first: char) {
		// Arrange
		let input = attribute_rule_with_matcher(first, Spacing::Alone);

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"attribute selector matcher operators cannot contain whitespace"
		);
	}

	#[rstest]
	#[case(".card { &[data-state=active foo] {} }")]
	#[case(".card { &[lang|=en insensitive] {} }")]
	fn rejects_unsupported_attribute_selector_modifiers(#[case] source: &str) {
		// Arrange
		let input = source.parse().unwrap();

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"attribute selector modifiers must be `i` or `s`"
		);
	}

	#[rstest]
	#[case("I")]
	#[case("S")]
	fn accepts_case_insensitive_attribute_selector_modifiers(#[case] modifier: &str) {
		// Arrange
		let input = format!(".card {{ &[data-state=open {modifier}] {{}} }}")
			.parse()
			.expect("test tokens should parse");

		// Act
		let rule = first_rule(input);

		// Assert
		let selector = &nested_rule(&rule, 0).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Attribute(attribute)) =
			&selector.kind
		else {
			panic!("expected a same-element attribute selector");
		};
		assert_eq!(
			attribute.modifier.as_ref().map(|value| value.as_str()),
			Some(modifier)
		);
	}

	#[rstest]
	fn parses_pseudo_elements_with_a_double_colon_marker() {
		// Arrange
		let input = quote! { .card { &::before { content: ""; } } };

		// Act
		let rule = first_rule(input);

		// Assert
		let selector = &nested_rule(&rule, 0).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Pseudo(pseudo)) = &selector.kind
		else {
			panic!("expected a same-element pseudo-element selector");
		};
		assert_eq!(pseudo.name.as_str(), "before");
		assert!(pseudo.is_element);
		assert!(pseudo.arguments.is_none());
	}

	#[rstest]
	fn parses_explicit_relationship_combinators_in_source_order() {
		// Arrange
		let input = quote! {
			.card {
				> h5 {}
				+ .card {}
				~ .card {}
			}
		};

		// Act
		let rule = first_rule(input);

		// Assert
		assert_eq!(rule.items.len(), 3);
		let expected = [
			(StyleSelectorCombinator::Child, "h5", false),
			(StyleSelectorCombinator::AdjacentSibling, "card", true),
			(StyleSelectorCombinator::GeneralSibling, "card", true),
		];
		for (index, (expected_combinator, expected_name, expected_class)) in
			expected.into_iter().enumerate()
		{
			let selector = &nested_rule(&rule, index).selectors.selectors[0];
			let StyleSelectorKind::Relative {
				combinator,
				selector,
			} = &selector.kind
			else {
				panic!("expected a relative selector");
			};
			assert_eq!(*combinator, expected_combinator);
			match (selector, expected_class) {
				(StyleSimpleSelector::Class(name), true)
				| (StyleSimpleSelector::Type(name), false) => {
					assert_eq!(name.as_str(), expected_name);
				}
				_ => panic!("expected the exact simple-selector kind"),
			}
		}
	}

	#[rstest]
	fn parses_implicit_descendants_without_whitespace_semantics() {
		// Arrange
		let input = quote! {
			.card {
				.label {}
				button {}
			}
		};

		// Act
		let rule = first_rule(input);

		// Assert
		assert_eq!(rule.items.len(), 2);
		let label = &nested_rule(&rule, 0).selectors.selectors[0];
		let StyleSelectorKind::Relative {
			combinator: StyleSelectorCombinator::Descendant,
			selector: StyleSimpleSelector::Class(class),
		} = &label.kind
		else {
			panic!("expected a descendant class selector");
		};
		assert_eq!(class.as_str(), "label");

		let button = &nested_rule(&rule, 1).selectors.selectors[0];
		let StyleSelectorKind::Relative {
			combinator: StyleSelectorCombinator::Descendant,
			selector: StyleSimpleSelector::Type(element),
		} = &button.kind
		else {
			panic!("expected a descendant type selector");
		};
		assert_eq!(element.as_str(), "button");
	}

	#[rstest]
	#[case("r#type", "type")]
	#[case("r#screen", "screen")]
	fn normalizes_raw_rust_identifiers_in_selectors(#[case] source: &str, #[case] expected: &str) {
		// Arrange
		let input = format!("{source} {{}}").parse().unwrap();

		// Act
		let rule = first_rule(input);

		// Assert
		let StyleSelectorKind::Root(StyleSimpleSelector::Type(name)) =
			&rule.selectors.selectors[0].kind
		else {
			panic!("expected a type selector");
		};
		assert_eq!(name.as_str(), expected);
	}

	#[rstest]
	#[case(".card { #123 {} }")]
	#[case(".card { [123] {} }")]
	#[case(".card { &:123 {} }")]
	fn rejects_unescaped_numeric_selector_name_starts(#[case] source: &str) {
		// Arrange
		let input = source.parse().unwrap();

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"selector names cannot start with a number without a CSS escape"
		);
	}

	#[rstest]
	fn accepts_numeric_segments_after_a_valid_attribute_name_start() {
		// Arrange
		let input = ".card { &[data-123] {} }".parse().unwrap();

		// Act
		let rule = first_rule(input);

		// Assert
		let selector = &nested_rule(&rule, 0).selectors.selectors[0];
		let StyleSelectorKind::SameElement(StyleSimpleSelector::Attribute(attribute)) =
			&selector.kind
		else {
			panic!("expected an attribute selector");
		};
		assert_eq!(attribute.name.as_str(), "data-123");
	}

	#[rstest]
	fn accepts_double_hyphen_class_name_segments() {
		// Arrange
		let input = quote! { .card--active {} };

		// Act
		let rule = first_rule(input);

		// Assert
		let selector = &rule.selectors.selectors[0];
		let StyleSelectorKind::Root(StyleSimpleSelector::Class(name)) = &selector.kind else {
			panic!("expected a class selector");
		};
		assert_eq!(name.as_str(), "card--active");
	}

	#[rstest]
	#[case(quote! { .card.label {} })]
	#[case(quote! { .card .label {} })]
	fn rejects_ambiguous_flat_selector_heads(#[case] input: proc_macro2::TokenStream) {
		// Arrange
		// Input is provided by the parameterized case.

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"selector heads must contain exactly one simple selector; express relationships and refinements with nested rules"
		);
	}

	#[rstest]
	fn rejects_same_element_type_selectors() {
		// Arrange
		let input = quote! { .card { &button {} } };

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"same-element type restrictions must use a pseudo-class such as `&:is(button)`"
		);
	}

	#[rstest]
	fn preserves_detailed_pseudo_name_errors() {
		// Arrange
		let input = quote! { .card { &:foo- {} } };

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"expected a selector name segment after `-`"
		);
	}

	#[rstest]
	#[case(quote! { , .card {} }, "selector list branches cannot be empty")]
	#[case(quote! { .card, {} }, "expected a selector after `,`")]
	fn rejects_empty_selector_list_branches(
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
	fn requires_a_rule_body_after_a_top_level_selector() {
		// Arrange
		let input = quote! { .card };

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(error.to_string(), "expected a style rule body");
	}
}
