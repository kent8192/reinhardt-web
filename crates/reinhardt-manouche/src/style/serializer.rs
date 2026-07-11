//! Deterministic serialization from structured CSS IR.

use std::{collections::HashSet, fmt::Write as _};

use proc_macro2::{Delimiter, TokenTree};

use super::css_ir::{
	CssAttributeSelector, CssAttributeValue, CssFunctionSeparator, CssLiteral, CssPseudoArguments,
	CssRule, CssSelector, CssSimpleSelector, CssStylesheet, CssValue, CssValueKind,
};
use crate::{
	StyleAttributeMatcher, StyleBinaryOperatorKind, StyleMediaCondition, StyleMediaOperatorKind,
	StyleMediaPunctuationKind, StyleMediaToken, StyleSelectorCombinator, StyleUnaryOperatorKind,
};

/// Serializes one structured stylesheet into deterministic CSS bytes.
pub fn serialize_css(stylesheet: &CssStylesheet) -> String {
	if stylesheet.rules.is_empty() {
		return String::new();
	}

	let mut output = String::new();
	write_rules(
		&mut output,
		&stylesheet.rules,
		&stylesheet.variable_defaults,
		0,
	);
	output.push('\n');
	output
}

const MAX_PRETTY_INDENT: usize = 128;

enum RuleFrame<'a> {
	Rule(&'a CssRule, usize),
	Text(&'static str),
	CloseGroup { indent: usize, has_rules: bool },
}

fn write_rules(
	output: &mut String,
	rules: &[CssRule],
	variable_defaults: &[CssValue],
	indent: usize,
) {
	let mut frames = Vec::new();
	push_rules(&mut frames, rules, indent);
	while let Some(frame) = frames.pop() {
		match frame {
			RuleFrame::Text(text) => output.push_str(text),
			RuleFrame::CloseGroup { indent, has_rules } => {
				if has_rules {
					output.push('\n');
				}
				write_indent(output, indent);
				output.push('}');
			}
			RuleFrame::Rule(rule, indent) => match rule {
				CssRule::Style(rule) => {
					for (index, selector) in rule.selectors.iter().enumerate() {
						if index > 0 {
							output.push_str(",\n");
						}
						write_indent(output, indent);
						write_selector(output, selector);
					}
					output.push_str(" {\n");
					for declaration in &rule.declarations {
						write_indent(output, indent.saturating_add(1));
						write_identifier(output, &declaration.property);
						output.push_str(": ");
						write_value(output, &declaration.value, variable_defaults);
						output.push_str(";\n");
					}
					write_indent(output, indent);
					output.push('}');
				}
				CssRule::Group(group) => {
					write_indent(output, indent);
					output.push_str("@media ");
					write_media_condition(output, &group.condition);
					output.push_str(" {\n");
					frames.push(RuleFrame::CloseGroup {
						indent,
						has_rules: !group.rules.is_empty(),
					});
					push_rules(&mut frames, &group.rules, indent.saturating_add(1));
				}
			},
		}
	}
}

fn push_rules<'a>(frames: &mut Vec<RuleFrame<'a>>, rules: &'a [CssRule], indent: usize) {
	for (index, rule) in rules.iter().enumerate().rev() {
		frames.push(RuleFrame::Rule(rule, indent));
		if index > 0 {
			frames.push(RuleFrame::Text("\n\n"));
		}
	}
}

fn write_indent(output: &mut String, indent: usize) {
	for _ in 0..indent.min(MAX_PRETTY_INDENT) {
		output.push_str("  ");
	}
}

enum SelectorFrame<'a> {
	Selector(&'a CssSelector),
	Simple(&'a CssSimpleSelector),
	Combinator(StyleSelectorCombinator, bool),
	RawTokens(&'a [TokenTree]),
	Text(&'static str),
}

fn write_selector(output: &mut String, selector: &CssSelector) {
	let mut frames = vec![SelectorFrame::Selector(selector)];
	while let Some(frame) = frames.pop() {
		match frame {
			SelectorFrame::Text(text) => output.push_str(text),
			SelectorFrame::RawTokens(tokens) => write_raw_tokens(output, tokens),
			SelectorFrame::Combinator(combinator, leading) => {
				write_combinator(output, combinator, leading);
			}
			SelectorFrame::Selector(selector) => push_selector(&mut frames, selector),
			SelectorFrame::Simple(selector) => match selector {
				CssSimpleSelector::Class(name) => {
					output.push('.');
					write_identifier(output, name);
				}
				CssSimpleSelector::Type(name) => write_identifier(output, name),
				CssSimpleSelector::Id(name) => {
					output.push('#');
					write_identifier(output, name);
				}
				CssSimpleSelector::Universal => output.push('*'),
				CssSimpleSelector::Attribute(attribute) => {
					write_attribute_selector(output, attribute);
				}
				CssSimpleSelector::Pseudo(pseudo) => {
					output.push(':');
					write_identifier(output, &pseudo.name);
					let Some(arguments) = &pseudo.arguments else {
						continue;
					};
					output.push('(');
					frames.push(SelectorFrame::Text(")"));
					match arguments {
						CssPseudoArguments::SelectorList(selectors) => {
							push_selector_list(&mut frames, selectors);
						}
						CssPseudoArguments::Nth {
							formula_tokens,
							selectors,
						} => {
							if let Some(selectors) = selectors {
								push_selector_list(&mut frames, selectors);
								frames.push(SelectorFrame::Text(" of "));
							}
							frames.push(SelectorFrame::RawTokens(formula_tokens));
						}
						CssPseudoArguments::RawTokens(tokens) => {
							frames.push(SelectorFrame::RawTokens(tokens));
						}
					}
				}
			},
		}
	}
}

fn push_selector<'a>(frames: &mut Vec<SelectorFrame<'a>>, selector: &'a CssSelector) {
	for (index, segment) in selector.segments.iter().enumerate().rev() {
		for simple in segment.simple_selectors.iter().rev() {
			frames.push(SelectorFrame::Simple(simple));
		}
		if let Some(combinator) = segment.combinator {
			frames.push(SelectorFrame::Combinator(combinator, index == 0));
		}
	}
}

fn push_selector_list<'a>(frames: &mut Vec<SelectorFrame<'a>>, selectors: &'a [CssSelector]) {
	for (index, selector) in selectors.iter().enumerate().rev() {
		frames.push(SelectorFrame::Selector(selector));
		if index > 0 {
			frames.push(SelectorFrame::Text(", "));
		}
	}
}

fn write_combinator(output: &mut String, combinator: StyleSelectorCombinator, leading: bool) {
	match (combinator, leading) {
		(StyleSelectorCombinator::Descendant, true) => {}
		(StyleSelectorCombinator::Descendant, false) => output.push(' '),
		(StyleSelectorCombinator::Child, true) => output.push_str("> "),
		(StyleSelectorCombinator::AdjacentSibling, true) => output.push_str("+ "),
		(StyleSelectorCombinator::GeneralSibling, true) => output.push_str("~ "),
		(StyleSelectorCombinator::Child, false) => output.push_str(" > "),
		(StyleSelectorCombinator::AdjacentSibling, false) => output.push_str(" + "),
		(StyleSelectorCombinator::GeneralSibling, false) => output.push_str(" ~ "),
	}
}

fn write_attribute_selector(output: &mut String, attribute: &CssAttributeSelector) {
	output.push('[');
	write_identifier(output, &attribute.name);
	if let Some(matcher) = attribute.matcher {
		output.push_str(match matcher {
			StyleAttributeMatcher::Equals => "=",
			StyleAttributeMatcher::Includes => "~=",
			StyleAttributeMatcher::DashMatch => "|=",
			StyleAttributeMatcher::Prefix => "^=",
			StyleAttributeMatcher::Suffix => "$=",
			StyleAttributeMatcher::Substring => "*=",
		});
	}
	if let Some(value) = &attribute.value {
		match value {
			CssAttributeValue::Identifier(value) => write_identifier(output, value),
			CssAttributeValue::String(value) => write_string(output, value),
		}
	}
	if let Some(modifier) = &attribute.modifier {
		output.push(' ');
		write_identifier(output, modifier);
	}
	output.push(']');
}

enum ValueFrame<'a> {
	Value(&'a CssValue),
	Text(&'static str),
	EndFallback(usize),
}

fn write_value(output: &mut String, value: &CssValue, variable_defaults: &[CssValue]) {
	let mut active_fallbacks = HashSet::new();
	let mut frames = vec![ValueFrame::Value(value)];
	while let Some(frame) = frames.pop() {
		match frame {
			ValueFrame::Text(text) => output.push_str(text),
			ValueFrame::EndFallback(index) => {
				active_fallbacks.remove(&index);
			}
			ValueFrame::Value(value) => match &value.kind {
				CssValueKind::Literal(literal) => write_literal(output, literal),
				CssValueKind::GlobalVariable { custom_property } => {
					write_function_start(output, "var");
					write_identifier(output, custom_property);
					output.push(')');
				}
				CssValueKind::ComponentVariable {
					custom_property,
					fallback_index,
				} => {
					write_function_start(output, "var");
					write_identifier(output, custom_property);
					if let Some(fallback) = variable_defaults.get(*fallback_index)
						&& active_fallbacks.insert(*fallback_index)
					{
						output.push_str(", ");
						frames.push(ValueFrame::Text(")"));
						frames.push(ValueFrame::EndFallback(*fallback_index));
						frames.push(ValueFrame::Value(fallback));
					} else {
						output.push(')');
					}
				}
				CssValueKind::Direction(direction) => output.push_str(direction.as_css()),
				CssValueKind::Unary { operator, operand } => {
					output.push(match operator {
						StyleUnaryOperatorKind::Plus => '+',
						StyleUnaryOperatorKind::Minus => '-',
					});
					frames.push(ValueFrame::Value(operand));
				}
				CssValueKind::Binary {
					left,
					operator,
					right,
				} => {
					frames.push(ValueFrame::Value(right));
					frames.push(ValueFrame::Text(match operator {
						StyleBinaryOperatorKind::Add => " + ",
						StyleBinaryOperatorKind::Subtract => " - ",
						StyleBinaryOperatorKind::Multiply => " * ",
						StyleBinaryOperatorKind::Divide => " / ",
					}));
					frames.push(ValueFrame::Value(left));
				}
				CssValueKind::Function(function) => {
					write_function_start(output, &function.name);
					frames.push(ValueFrame::Text(")"));
					push_values(
						&mut frames,
						&function.arguments,
						function_separator(function.separator),
					);
				}
				CssValueKind::ColorMix {
					receiver,
					other,
					amount,
				} => {
					write_function_start(output, "color-mix");
					output.push_str("in srgb, ");
					frames.push(ValueFrame::Text(")"));
					frames.push(ValueFrame::Value(amount));
					frames.push(ValueFrame::Text(" "));
					frames.push(ValueFrame::Value(other));
					frames.push(ValueFrame::Text("), "));
					frames.push(ValueFrame::Value(amount));
					frames.push(ValueFrame::Text(" calc(100% - "));
					frames.push(ValueFrame::Value(receiver));
				}
				CssValueKind::Group(inner) => {
					output.push('(');
					frames.push(ValueFrame::Text(")"));
					frames.push(ValueFrame::Value(inner));
				}
				CssValueKind::SpaceSequence(values) => {
					push_values(&mut frames, values, " ");
				}
				CssValueKind::CommaList(values) => {
					push_values(&mut frames, values, ", ");
				}
				CssValueKind::SlashPair { left, right } => {
					frames.push(ValueFrame::Value(right));
					frames.push(ValueFrame::Text(" / "));
					frames.push(ValueFrame::Value(left));
				}
				CssValueKind::UncheckedFunction(function) => {
					write_identifier(output, function.name.as_str());
					write_raw_group(
						output,
						function.arguments.delimiter,
						&function.arguments.tokens,
					);
				}
				CssValueKind::Calc(inner) => {
					write_function_start(output, "calc");
					frames.push(ValueFrame::Text(")"));
					frames.push(ValueFrame::Value(inner));
				}
			},
		}
	}
}

fn write_function_start(output: &mut String, name: &str) {
	write_identifier(output, name);
	output.push('(');
}

const fn function_separator(separator: CssFunctionSeparator) -> &'static str {
	match separator {
		CssFunctionSeparator::Comma => ", ",
		CssFunctionSeparator::Space => " ",
	}
}

fn push_values<'a>(
	frames: &mut Vec<ValueFrame<'a>>,
	values: &'a [CssValue],
	separator: &'static str,
) {
	for (index, value) in values.iter().enumerate().rev() {
		frames.push(ValueFrame::Value(value));
		if index > 0 {
			frames.push(ValueFrame::Text(separator));
		}
	}
}

fn write_literal(output: &mut String, literal: &CssLiteral) {
	match literal {
		CssLiteral::Number { source, unit } => write_number(output, source, unit.as_deref()),
		CssLiteral::HexColor(value) => output.push_str(value),
		CssLiteral::Keyword(value) => write_keyword(output, value),
		CssLiteral::String(value) => write_string(output, value),
	}
}

fn write_identifier(output: &mut String, identifier: &str) {
	output.push_str(identifier);
}

fn write_number(output: &mut String, source: &str, unit: Option<&str>) {
	output.extend(source.chars().filter(|character| *character != '_'));
	if let Some(unit) = unit {
		output.push_str(unit);
	}
}

fn write_keyword(output: &mut String, keyword: &str) {
	output.push_str(keyword);
}

fn write_string(output: &mut String, value: &str) {
	output.push('"');
	for character in value.chars() {
		match character {
			'"' => output.push_str("\\\""),
			'\\' => output.push_str("\\\\"),
			'\0' => output.push('\u{fffd}'),
			'\n' => output.push_str("\\a "),
			'\r' => output.push_str("\\d "),
			'\u{c}' => output.push_str("\\c "),
			character if character.is_control() => {
				write!(output, "\\{:x} ", character as u32)
					.expect("writing a CSS escape to String cannot fail");
			}
			character => output.push(character),
		}
	}
	output.push('"');
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RawBoundary {
	None,
	Identifier,
	Literal,
	Close,
	Percent,
	Other,
}

impl RawBoundary {
	const fn ends_value(self) -> bool {
		matches!(
			self,
			Self::Identifier | Self::Literal | Self::Close | Self::Percent
		)
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RawContext {
	General,
	Calc,
}

enum RawFrame {
	Sequence {
		tokens: Vec<TokenTree>,
		index: usize,
		boundary: RawBoundary,
		context: RawContext,
	},
	CloseGroup(&'static str),
}

fn write_raw_tokens(output: &mut String, tokens: &[TokenTree]) {
	let mut frames = vec![RawFrame::Sequence {
		tokens: tokens.to_vec(),
		index: 0,
		boundary: RawBoundary::None,
		context: RawContext::General,
	}];
	while let Some(frame) = frames.pop() {
		match frame {
			RawFrame::CloseGroup(closing) => {
				trim_spaces(output);
				output.push_str(closing);
			}
			RawFrame::Sequence {
				tokens,
				index,
				boundary,
				context,
			} => {
				let Some(token) = tokens.get(index).cloned() else {
					continue;
				};
				match token {
					TokenTree::Ident(identifier) => {
						write_raw_word(output, &raw_identifier_text(&identifier), boundary);
						frames.push(RawFrame::Sequence {
							tokens,
							index: index + 1,
							boundary: RawBoundary::Identifier,
							context,
						});
					}
					TokenTree::Literal(literal) => {
						write_raw_word(output, &literal.to_string(), boundary);
						frames.push(RawFrame::Sequence {
							tokens,
							index: index + 1,
							boundary: RawBoundary::Literal,
							context,
						});
					}
					TokenTree::Punct(punctuation) => {
						let next_boundary = write_raw_punctuation(
							output,
							punctuation.as_char(),
							boundary,
							context,
							tokens.get(index + 1),
						);
						frames.push(RawFrame::Sequence {
							tokens,
							index: index + 1,
							boundary: next_boundary,
							context,
						});
					}
					TokenTree::Group(group) => {
						let nested_context = raw_group_context(
							context,
							index.checked_sub(1).and_then(|index| tokens.get(index)),
						);
						let (opening, closing) = delimiter_pair(group.delimiter());
						output.push_str(opening);
						frames.push(RawFrame::Sequence {
							tokens,
							index: index + 1,
							boundary: RawBoundary::Close,
							context,
						});
						frames.push(RawFrame::CloseGroup(closing));
						frames.push(RawFrame::Sequence {
							tokens: group.stream().into_iter().collect(),
							index: 0,
							boundary: RawBoundary::None,
							context: nested_context,
						});
					}
				}
			}
		}
	}
}

fn write_raw_word(output: &mut String, word: &str, boundary: RawBoundary) {
	if boundary.ends_value() && !output.ends_with(char::is_whitespace) {
		output.push(' ');
	}
	output.push_str(word);
}

fn write_raw_punctuation(
	output: &mut String,
	punctuation: char,
	boundary: RawBoundary,
	context: RawContext,
	next: Option<&TokenTree>,
) -> RawBoundary {
	if punctuation == ',' {
		trim_spaces(output);
		output.push_str(", ");
		return RawBoundary::Other;
	}
	if matches!(punctuation, '+' | '-') && raw_sign_is_binary(punctuation, boundary, context, next)
	{
		ensure_space(output);
		output.push(punctuation);
		output.push(' ');
		return RawBoundary::Other;
	}
	if matches!(punctuation, '+' | '-') {
		output.push(punctuation);
		return RawBoundary::Other;
	}
	trim_spaces(output);
	output.push(punctuation);
	if punctuation == '%' {
		RawBoundary::Percent
	} else {
		RawBoundary::Other
	}
}

fn raw_sign_is_binary(
	punctuation: char,
	boundary: RawBoundary,
	context: RawContext,
	next: Option<&TokenTree>,
) -> bool {
	if context != RawContext::Calc || !boundary.ends_value() || !raw_token_starts_value(next) {
		return false;
	}
	!(punctuation == '-'
		&& boundary == RawBoundary::Identifier
		&& matches!(next, Some(TokenTree::Ident(_) | TokenTree::Group(_))))
}

fn raw_token_starts_value(token: Option<&TokenTree>) -> bool {
	matches!(
		token,
		Some(TokenTree::Ident(_) | TokenTree::Literal(_) | TokenTree::Group(_))
	)
}

fn raw_group_context(context: RawContext, previous: Option<&TokenTree>) -> RawContext {
	if context == RawContext::Calc
		|| matches!(previous, Some(TokenTree::Ident(identifier)) if raw_identifier_text(identifier).eq_ignore_ascii_case("calc"))
	{
		RawContext::Calc
	} else {
		RawContext::General
	}
}

fn raw_identifier_text(identifier: &proc_macro2::Ident) -> String {
	let rendered = identifier.to_string();
	rendered.strip_prefix("r#").unwrap_or(&rendered).to_owned()
}

fn write_raw_group(output: &mut String, delimiter: Delimiter, tokens: &[TokenTree]) {
	let (opening, closing) = delimiter_pair(delimiter);
	output.push_str(opening);
	write_raw_tokens(output, tokens);
	trim_spaces(output);
	output.push_str(closing);
}

fn delimiter_pair(delimiter: Delimiter) -> (&'static str, &'static str) {
	match delimiter {
		Delimiter::Parenthesis => ("(", ")"),
		Delimiter::Brace => ("{", "}"),
		Delimiter::Bracket => ("[", "]"),
		Delimiter::None => ("", ""),
	}
}

fn trim_spaces(output: &mut String) {
	while output.ends_with(char::is_whitespace) {
		output.pop();
	}
}

fn write_media_condition(output: &mut String, condition: &StyleMediaCondition) {
	write_media_tokens(output, &condition.tokens);
	trim_spaces(output);
}

enum MediaFrame<'a> {
	Sequence(&'a [StyleMediaToken], usize),
	CloseGroup,
}

fn write_media_tokens(output: &mut String, tokens: &[StyleMediaToken]) {
	let mut frames = vec![MediaFrame::Sequence(tokens, 0)];
	while let Some(frame) = frames.pop() {
		match frame {
			MediaFrame::CloseGroup => {
				trim_spaces(output);
				output.push(')');
			}
			MediaFrame::Sequence(tokens, index) => {
				let Some(token) = tokens.get(index) else {
					continue;
				};
				frames.push(MediaFrame::Sequence(tokens, index + 1));
				match token {
					StyleMediaToken::Identifier(identifier) => {
						write_media_word(output, identifier.as_str());
					}
					StyleMediaToken::Operator(operator) => {
						ensure_space(output);
						output.push_str(match operator.kind {
							StyleMediaOperatorKind::And => "and",
							StyleMediaOperatorKind::Or => "or",
							StyleMediaOperatorKind::Not => "not",
							StyleMediaOperatorKind::Only => "only",
						});
						output.push(' ');
					}
					StyleMediaToken::Number(number) => {
						if output.chars().next_back().is_some_and(|character| {
							character.is_ascii_alphanumeric() || character == ')'
						}) {
							output.push(' ');
						}
						write_number(output, &number.value, number.unit.as_deref());
					}
					StyleMediaToken::Punctuation(punctuation) => {
						write_media_punctuation(output, punctuation.kind);
					}
					StyleMediaToken::Parenthesized(group) => {
						if output.chars().next_back().is_some_and(|character| {
							character.is_ascii_alphanumeric() || matches!(character, '%' | ')')
						}) {
							output.push(' ');
						}
						output.push('(');
						frames.push(MediaFrame::CloseGroup);
						frames.push(MediaFrame::Sequence(&group.tokens, 0));
					}
				}
			}
		}
	}
}

fn write_media_word(output: &mut String, word: &str) {
	if output.chars().next_back().is_some_and(|character| {
		character.is_ascii_alphanumeric() || matches!(character, '%' | ')')
	}) {
		output.push(' ');
	}
	output.push_str(word);
}

fn write_media_punctuation(output: &mut String, punctuation: StyleMediaPunctuationKind) {
	match punctuation {
		StyleMediaPunctuationKind::Colon => {
			trim_spaces(output);
			output.push_str(": ");
		}
		StyleMediaPunctuationKind::Slash => {
			ensure_space(output);
			output.push_str("/ ");
		}
		StyleMediaPunctuationKind::Comma => {
			trim_spaces(output);
			output.push_str(", ");
		}
		StyleMediaPunctuationKind::Percent => {
			trim_spaces(output);
			output.push('%');
		}
		StyleMediaPunctuationKind::LessThan => write_media_comparison(output, "<"),
		StyleMediaPunctuationKind::LessThanOrEqual => write_media_comparison(output, "<="),
		StyleMediaPunctuationKind::GreaterThan => write_media_comparison(output, ">"),
		StyleMediaPunctuationKind::GreaterThanOrEqual => write_media_comparison(output, ">="),
		StyleMediaPunctuationKind::Equal => write_media_comparison(output, "="),
		StyleMediaPunctuationKind::Plus => output.push('+'),
		StyleMediaPunctuationKind::Minus => output.push('-'),
	}
}

fn write_media_comparison(output: &mut String, operator: &str) {
	ensure_space(output);
	output.push_str(operator);
	output.push(' ');
}

fn ensure_space(output: &mut String) {
	if !output.is_empty() && !output.ends_with(char::is_whitespace) {
		output.push(' ');
	}
}

#[cfg(test)]
mod tests {
	use std::{fmt::Write as _, thread};

	use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream, TokenTree};
	use rstest::rstest;

	use super::serialize_css;
	use crate::style::css_ir::{
		CssDeclaration, CssGroupingRule, CssLiteral, CssPseudoArguments, CssPseudoSelector,
		CssRule, CssSelector, CssSelectorSegment, CssSimpleSelector, CssStyleRule, CssStylesheet,
		CssValue, CssValueKind,
	};
	use crate::{
		StyleCompileContext, StyleMediaCondition, StyleMediaGroup, StyleMediaIdentifier,
		StyleMediaToken, compile_style,
	};

	fn compile(source: &str) -> CssStylesheet {
		let input = source.parse().expect("style test source should tokenize");
		compile_style(
			input,
			&StyleCompileContext {
				package_name: "poll-app",
				package_version: "0.4.0",
				style_type_name: "PollCardStyles",
			},
		)
		.expect("style test source should compile")
		.css
	}

	fn length_value(kind: CssValueKind) -> CssValue {
		CssValue { kind }
	}

	fn stylesheet_with_width(value: CssValue, variable_defaults: Vec<CssValue>) -> CssStylesheet {
		CssStylesheet {
			rules: vec![CssRule::Style(CssStyleRule {
				selectors: vec![CssSelector {
					segments: vec![CssSelectorSegment {
						combinator: None,
						simple_selectors: vec![CssSimpleSelector::Class("card".into())],
					}],
				}],
				declarations: vec![CssDeclaration {
					property: "width".into(),
					value,
				}],
			})],
			variable_defaults,
		}
	}

	fn one_px() -> CssValue {
		length_value(CssValueKind::Literal(CssLiteral::Number {
			source: "1".into(),
			unit: Some("px".into()),
		}))
	}

	fn run_on_standard_thread(task: impl FnOnce() + Send + 'static) {
		thread::Builder::new()
			.stack_size(2 * 1024 * 1024)
			.spawn(task)
			.expect("the stack-safety test thread should spawn")
			.join()
			.expect("serialization should finish on a standard 2 MiB thread");
	}

	fn screen_condition() -> StyleMediaCondition {
		StyleMediaCondition {
			tokens: vec![StyleMediaToken::Identifier(StyleMediaIdentifier {
				value: "screen".into(),
				span: Span::call_site(),
			})],
			span: Span::call_site(),
		}
	}

	fn drop_stylesheet_iteratively(mut stylesheet: CssStylesheet) {
		let mut rules = std::mem::take(&mut stylesheet.rules);
		while let Some(rule) = rules.pop() {
			match rule {
				CssRule::Group(mut group) => {
					rules.append(&mut group.rules);
					drop_media_tokens_iteratively(std::mem::take(&mut group.condition.tokens));
				}
				CssRule::Style(mut rule) => {
					drop_selectors_iteratively(std::mem::take(&mut rule.selectors));
				}
			}
		}
	}

	fn drop_selectors_iteratively(mut selectors: Vec<CssSelector>) {
		while let Some(mut selector) = selectors.pop() {
			for mut segment in std::mem::take(&mut selector.segments) {
				for simple in std::mem::take(&mut segment.simple_selectors) {
					let CssSimpleSelector::Pseudo(mut pseudo) = simple else {
						continue;
					};
					match pseudo.arguments.take() {
						Some(CssPseudoArguments::SelectorList(mut nested)) => {
							selectors.append(&mut nested);
						}
						Some(CssPseudoArguments::Nth {
							selectors: Some(mut nested),
							..
						}) => selectors.append(&mut nested),
						Some(CssPseudoArguments::Nth {
							selectors: None, ..
						})
						| Some(CssPseudoArguments::RawTokens(_))
						| None => {}
					}
				}
			}
		}
	}

	fn drop_media_tokens_iteratively(mut tokens: Vec<StyleMediaToken>) {
		while let Some(token) = tokens.pop() {
			if let StyleMediaToken::Parenthesized(mut group) = token {
				tokens.append(&mut group.tokens);
			}
		}
	}

	#[rstest]
	fn empty_stylesheet_serializes_to_zero_bytes() {
		// Arrange
		let stylesheet = CssStylesheet::default();

		// Act
		let css = serialize_css(&stylesheet);

		// Assert
		assert_eq!(css, "");
	}

	#[rstest]
	fn checked_values_serialize_with_canonical_functions_and_one_calc_boundary() {
		// Arrange
		let stylesheet = compile(
			"
			vars { gutter: Length = 1rem; }
			.card {
				color: Color::rgb(10, 20%, 30);
				background-color: Color::hsl(180deg, 50%, 25%);
				border-color: Color::oklch(50%, 0.2, 30deg);
				outline-color: red.mix(blue, 20%);
				width: 100% - vars.gutter * 2;
				transform: (translate_x(1rem), rotate(45deg));
				border-radius: slash(1rem, 2rem);
			}
			",
		);

		// Act
		let css = serialize_css(&stylesheet);

		// Assert
		assert_eq!(
			css,
			concat!(
				".card--rs-c6b395a1e8e9 {\n",
				"  color: rgb(10 20% 30);\n",
				"  background-color: hsl(180deg 50% 25%);\n",
				"  border-color: oklch(50% 0.2 30deg);\n",
				"  outline-color: color-mix(in srgb, red calc(100% - 20%), blue 20%);\n",
				"  width: calc(100% - var(--rs-c6b395a1e8e9-gutter, 1rem) * 2);\n",
				"  transform: translateX(1rem) rotate(45deg);\n",
				"  border-radius: 1rem / 2rem;\n",
				"}\n",
			)
		);
		assert_eq!(css.matches("calc(").count(), 2);
		assert!(css.ends_with('\n'));
		assert!(!css.ends_with("\n\n"));
	}

	#[rstest]
	fn unchecked_function_renders_only_its_validated_raw_token_tree() {
		// Arrange
		let stylesheet = compile(
			r#"
			.card {
				background: unchecked_fn!(paint(namespace::worklet, [red, blue], { tone: "dark" }));
			}
			"#,
		);

		// Act
		let css = serialize_css(&stylesheet);

		// Assert
		assert_eq!(
			css,
			concat!(
				".card--rs-c6b395a1e8e9 {\n",
				"  background: paint(namespace::worklet, [red, blue], {tone:\"dark\"});\n",
				"}\n",
			)
		);
	}

	#[rstest]
	fn unchecked_calc_binary_signs_keep_required_css_whitespace() {
		// Arrange
		let stylesheet = compile(
			"
			.card {
				background: unchecked_fn!(paint(
					calc(100% - 1px),
					calc(50% + 2px),
					custom-ident,
					-1px
				));
			}
			",
		);

		// Act
		let css = serialize_css(&stylesheet);

		// Assert
		assert_eq!(
			css,
			concat!(
				".card--rs-c6b395a1e8e9 {\n",
				"  background: paint(calc(100% - 1px), calc(50% + 2px), custom-ident, -1px);\n",
				"}\n",
			)
		);
	}

	#[rstest]
	fn media_queries_use_stable_spacing_for_ranges_lists_and_signed_values() {
		// Arrange
		let stylesheet = compile(
			"
			@media screen and (400px < width <= 1200px), print and (width: +1px) {
				.card { color: red; }
			}
			",
		);

		// Act
		let css = serialize_css(&stylesheet);

		// Assert
		assert_eq!(
			css,
			concat!(
				"@media screen and (400px < width <= 1200px), print and (width: +1px) {\n",
				"  .card--rs-c6b395a1e8e9 {\n",
				"    color: red;\n",
				"  }\n",
				"}\n",
			)
		);
	}

	#[rstest]
	fn variable_fallback_arena_serializes_a_deep_chain_iteratively() {
		// Arrange
		const VARIABLE_COUNT: usize = 16_384;
		let mut defaults = Vec::with_capacity(VARIABLE_COUNT);
		defaults.push(length_value(CssValueKind::Literal(CssLiteral::Number {
			source: "1".into(),
			unit: Some("px".into()),
		})));
		for index in 1..VARIABLE_COUNT {
			defaults.push(length_value(CssValueKind::ComponentVariable {
				custom_property: format!("--v{}", index - 1),
				fallback_index: index - 1,
			}));
		}
		let stylesheet = stylesheet_with_width(
			length_value(CssValueKind::ComponentVariable {
				custom_property: format!("--v{}", VARIABLE_COUNT - 1),
				fallback_index: VARIABLE_COUNT - 1,
			}),
			defaults,
		);
		let mut expected = String::from(".card {\n  width: ");
		for index in (0..VARIABLE_COUNT).rev() {
			write!(expected, "var(--v{index}, ").unwrap();
		}
		expected.push_str("1px");
		for _ in 0..VARIABLE_COUNT {
			expected.push(')');
		}
		expected.push_str(";\n}\n");

		// Act
		let css = serialize_css(&stylesheet);

		// Assert
		assert_eq!(css, expected);
	}

	#[rstest]
	fn invalid_and_cyclic_fallback_indices_terminate_with_a_valid_var_reference() {
		// Arrange
		let invalid = stylesheet_with_width(
			length_value(CssValueKind::ComponentVariable {
				custom_property: "--broken".into(),
				fallback_index: 99,
			}),
			Vec::new(),
		);
		let cyclic = stylesheet_with_width(
			length_value(CssValueKind::ComponentVariable {
				custom_property: "--root".into(),
				fallback_index: 0,
			}),
			vec![
				length_value(CssValueKind::ComponentVariable {
					custom_property: "--a".into(),
					fallback_index: 1,
				}),
				length_value(CssValueKind::ComponentVariable {
					custom_property: "--b".into(),
					fallback_index: 0,
				}),
			],
		);

		// Act
		let invalid_css = serialize_css(&invalid);
		let cyclic_css = serialize_css(&cyclic);

		// Assert
		assert_eq!(invalid_css, ".card {\n  width: var(--broken);\n}\n");
		assert_eq!(
			cyclic_css,
			".card {\n  width: var(--root, var(--a, var(--b)));\n}\n"
		);
	}

	#[rstest]
	fn deeply_nested_rule_groups_serialize_without_using_the_call_stack() {
		run_on_standard_thread(|| {
			// Arrange
			const DEPTH: usize = 16_384;
			let mut leaf = stylesheet_with_width(one_px(), Vec::new())
				.rules
				.pop()
				.unwrap();
			for _ in 0..DEPTH {
				leaf = CssRule::Group(CssGroupingRule {
					condition: screen_condition(),
					rules: vec![leaf],
				});
			}
			let stylesheet = CssStylesheet {
				rules: vec![leaf],
				variable_defaults: Vec::new(),
			};

			// Act
			let css = serialize_css(&stylesheet);
			drop_stylesheet_iteratively(stylesheet);

			// Assert
			assert_eq!(css.matches("@media screen").count(), DEPTH);
			assert!(css.contains(".card"));
		});
	}

	#[rstest]
	fn deeply_nested_pseudo_selector_lists_serialize_without_using_the_call_stack() {
		run_on_standard_thread(|| {
			// Arrange
			const DEPTH: usize = 16_384;
			let mut selector = CssSelector {
				segments: vec![CssSelectorSegment {
					combinator: None,
					simple_selectors: vec![CssSimpleSelector::Class("leaf".into())],
				}],
			};
			for _ in 0..DEPTH {
				selector = CssSelector {
					segments: vec![CssSelectorSegment {
						combinator: None,
						simple_selectors: vec![CssSimpleSelector::Pseudo(CssPseudoSelector {
							name: "is".into(),
							arguments: Some(CssPseudoArguments::SelectorList(vec![selector])),
						})],
					}],
				};
			}
			let stylesheet = CssStylesheet {
				rules: vec![CssRule::Style(CssStyleRule {
					selectors: vec![selector],
					declarations: vec![CssDeclaration {
						property: "width".into(),
						value: one_px(),
					}],
				})],
				variable_defaults: Vec::new(),
			};

			// Act
			let css = serialize_css(&stylesheet);
			drop_stylesheet_iteratively(stylesheet);

			// Assert
			assert_eq!(css.matches(":is(").count(), DEPTH);
			assert!(css.contains(".leaf"));
		});
	}

	#[rstest]
	fn deeply_nested_raw_token_groups_serialize_without_using_the_call_stack() {
		run_on_standard_thread(|| {
			// Arrange
			const DEPTH: usize = 16_384;
			let mut token = TokenTree::Ident(Ident::new("en", Span::call_site()));
			let mut anchors = Vec::with_capacity(DEPTH);
			for _ in 0..DEPTH {
				anchors.push(token.clone());
				token =
					TokenTree::Group(Group::new(Delimiter::Parenthesis, TokenStream::from(token)));
			}
			let stylesheet = CssStylesheet {
				rules: vec![CssRule::Style(CssStyleRule {
					selectors: vec![CssSelector {
						segments: vec![CssSelectorSegment {
							combinator: None,
							simple_selectors: vec![CssSimpleSelector::Pseudo(CssPseudoSelector {
								name: "lang".into(),
								arguments: Some(CssPseudoArguments::RawTokens(vec![token])),
							})],
						}],
					}],
					declarations: vec![CssDeclaration {
						property: "width".into(),
						value: one_px(),
					}],
				})],
				variable_defaults: Vec::new(),
			};

			// Act
			let css = serialize_css(&stylesheet);
			drop_stylesheet_iteratively(stylesheet);
			while let Some(anchor) = anchors.pop() {
				drop(anchor);
			}

			// Assert
			assert_eq!(css.matches('(').count(), DEPTH + 1);
			assert!(css.contains("en"));
		});
	}

	#[rstest]
	fn deeply_nested_media_groups_serialize_without_using_the_call_stack() {
		run_on_standard_thread(|| {
			// Arrange
			const DEPTH: usize = 16_384;
			let mut token = StyleMediaToken::Identifier(StyleMediaIdentifier {
				value: "color".into(),
				span: Span::call_site(),
			});
			for _ in 0..DEPTH {
				token = StyleMediaToken::Parenthesized(StyleMediaGroup {
					tokens: vec![token],
					span: Span::call_site(),
				});
			}
			let leaf = stylesheet_with_width(one_px(), Vec::new())
				.rules
				.pop()
				.unwrap();
			let stylesheet = CssStylesheet {
				rules: vec![CssRule::Group(CssGroupingRule {
					condition: StyleMediaCondition {
						tokens: vec![token],
						span: Span::call_site(),
					},
					rules: vec![leaf],
				})],
				variable_defaults: Vec::new(),
			};

			// Act
			let css = serialize_css(&stylesheet);
			drop_stylesheet_iteratively(stylesheet);

			// Assert
			assert_eq!(css.matches('(').count(), DEPTH);
			assert!(css.contains("color"));
		});
	}
}
