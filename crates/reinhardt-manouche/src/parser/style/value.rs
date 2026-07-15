//! Parser for typed `style!` value expressions.

use proc_macro2::{Delimiter, Span, TokenTree};
use syn::{
	Ident, LitFloat, LitInt, LitStr, Token, bracketed, ext::IdentExt, parenthesized,
	parse::ParseStream, spanned::Spanned,
};

use super::{is_css_decimal_number, unraw_ident};
use crate::core::{
	StyleBinaryExpression, StyleBinaryOperator, StyleBinaryOperatorKind, StyleGroupedValue,
	StyleHexColorLiteral, StyleMethodCall, StyleNumericLiteral, StyleNumericUnit,
	StyleQualifiedReference, StyleRawTokenGroup, StyleReferenceNamespace, StyleStringLiteral,
	StyleUnaryExpression, StyleUnaryOperator, StyleUnaryOperatorKind, StyleUncheckedFunction,
	StyleValueCall, StyleValueCollection, StyleValueExpr, StyleValueExpression, StyleValueLiteral,
	StyleValueName, StyleValuePath,
};

pub(super) fn parse_value_expression(input: ParseStream) -> syn::Result<StyleValueExpression> {
	parse_expression(input, 0)
}

fn parse_expression(
	input: ParseStream,
	minimum_binding_power: u8,
) -> syn::Result<StyleValueExpression> {
	let mut left = parse_prefix_expression(input)?;

	while let Some((left_binding_power, right_binding_power, kind)) = peek_binary_operator(input) {
		if left_binding_power < minimum_binding_power {
			break;
		}

		let operator = parse_binary_operator(input, kind)?;
		if !can_start_expression(input) {
			return Err(syn::Error::new(
				operator.span,
				format!(
					"expected a style value expression after `{}`",
					binary_operator_symbol(operator.kind)
				),
			));
		}
		let right = parse_expression(input, right_binding_power)?;
		let span = joined_span(left.span, right.span);
		left = StyleValueExpression {
			kind: StyleValueExpr::Binary(StyleBinaryExpression {
				left: Box::new(left),
				operator,
				right: Box::new(right),
				span,
			}),
			span,
		};
	}

	Ok(left)
}

fn parse_prefix_expression(input: ParseStream) -> syn::Result<StyleValueExpression> {
	if input.peek(Token![-]) {
		let fork = input.fork();
		fork.parse::<Token![-]>()?;
		let is_qualified_reference = if fork.peek(Ident::peek_any) {
			let namespace = fork.call(Ident::parse_any)?;
			matches!(namespace.to_string().as_str(), "globals" | "vars") && fork.peek(Token![.])
		} else {
			false
		};
		if fork.peek(Ident::peek_any) && !is_qualified_reference {
			let hyphen: Token![-] = input.parse()?;
			let mut name = parse_value_name(input)?;
			name.value.insert(0, '-');
			name.span = joined_span(hyphen.span(), name.span);
			let name = parse_kebab_keyword_name(input, name)?;
			let span = name.span;
			return Ok(StyleValueExpression {
				kind: StyleValueExpr::Literal(StyleValueLiteral::Keyword(name)),
				span,
			});
		}
	}

	if input.peek(Token![+]) || input.peek(Token![-]) {
		let (kind, span) = if input.peek(Token![+]) {
			let token: Token![+] = input.parse()?;
			(StyleUnaryOperatorKind::Plus, token.span())
		} else {
			let token: Token![-] = input.parse()?;
			(StyleUnaryOperatorKind::Minus, token.span())
		};
		let operand = parse_expression(input, 5)?;
		let expression_span = joined_span(span, operand.span);
		return Ok(StyleValueExpression {
			kind: StyleValueExpr::Unary(StyleUnaryExpression {
				operator: StyleUnaryOperator { kind, span },
				operand: Box::new(operand),
				span: expression_span,
			}),
			span: expression_span,
		});
	}

	let primary = parse_primary_expression(input)?;
	parse_method_calls(input, primary)
}

fn parse_primary_expression(input: ParseStream) -> syn::Result<StyleValueExpression> {
	if input.peek(syn::token::Paren) {
		return parse_parenthesized_expression(input);
	}
	if input.peek(syn::token::Bracket) {
		return parse_comma_list(input);
	}
	if input.peek(Token![#]) {
		return parse_hex_color(input);
	}
	if input.peek(LitStr) {
		return parse_string_literal(input);
	}
	if input.peek(LitFloat) {
		return parse_float_literal(input);
	}
	if input.peek(LitInt) {
		return parse_integer_literal(input);
	}
	if input.peek(Ident::peek_any) {
		return parse_identifier_expression(input);
	}

	Err(input.error("expected a style value expression"))
}

fn parse_parenthesized_expression(input: ParseStream) -> syn::Result<StyleValueExpression> {
	let content;
	let parentheses = parenthesized!(content in input);
	let span = parentheses.span.join();
	if content.is_empty() {
		return Err(syn::Error::new(
			span,
			"style value sequences cannot be empty",
		));
	}

	let first = parse_expression(&content, 0)?;
	if content.is_empty() {
		return Ok(StyleValueExpression {
			kind: StyleValueExpr::Group(StyleGroupedValue {
				expression: Box::new(first),
				span,
			}),
			span,
		});
	}

	let mut items = vec![first];
	let mut comma_spans = Vec::new();
	while !content.is_empty() {
		let comma: Token![,] = content.parse()?;
		comma_spans.push(comma.span());
		if content.is_empty() {
			break;
		}
		items.push(parse_expression(&content, 0)?);
	}

	if items.len() == 1 {
		return Err(syn::Error::new(
			comma_spans.first().copied().unwrap_or(span),
			"one-item parenthesized style values cannot have a trailing comma",
		));
	}

	Ok(StyleValueExpression {
		kind: StyleValueExpr::SpaceSequence(StyleValueCollection {
			items,
			comma_spans,
			span,
		}),
		span,
	})
}

fn parse_comma_list(input: ParseStream) -> syn::Result<StyleValueExpression> {
	let content;
	let brackets = bracketed!(content in input);
	let span = brackets.span.join();
	if content.is_empty() {
		return Err(syn::Error::new(
			span,
			"style value comma lists cannot be empty",
		));
	}

	let mut items = Vec::new();
	let mut comma_spans = Vec::new();
	loop {
		items.push(parse_expression(&content, 0)?);
		if content.is_empty() {
			break;
		}
		let comma: Token![,] = content.parse()?;
		comma_spans.push(comma.span());
		if content.is_empty() {
			break;
		}
	}

	Ok(StyleValueExpression {
		kind: StyleValueExpr::CommaList(StyleValueCollection {
			items,
			comma_spans,
			span,
		}),
		span,
	})
}

fn parse_hex_color(input: ParseStream) -> syn::Result<StyleValueExpression> {
	let hash: Token![#] = input.parse()?;
	let digits_token: TokenTree = input
		.parse()
		.map_err(|_| syn::Error::new(hash.span(), "expected hexadecimal digits after `#`"))?;
	let (digits, digits_span) = match digits_token {
		TokenTree::Ident(ident) => (ident.to_string(), ident.span()),
		TokenTree::Literal(literal) => (literal.to_string(), literal.span()),
		other => {
			return Err(syn::Error::new(
				other.span(),
				"expected hexadecimal digits after `#`",
			));
		}
	};
	if !matches!(digits.len(), 3 | 4 | 6 | 8)
		|| !digits.bytes().all(|digit| digit.is_ascii_hexdigit())
	{
		return Err(syn::Error::new(
			digits_span,
			"hex colors require exactly 3, 4, 6, or 8 hexadecimal digits",
		));
	}
	let span = joined_span(hash.span(), digits_span);
	let source = format!("#{digits}");
	Ok(StyleValueExpression {
		kind: StyleValueExpr::Literal(StyleValueLiteral::HexColor(StyleHexColorLiteral {
			source,
			digits,
			span,
		})),
		span,
	})
}

fn parse_string_literal(input: ParseStream) -> syn::Result<StyleValueExpression> {
	let literal: LitStr = input.parse()?;
	let span = literal.span();
	Ok(StyleValueExpression {
		kind: StyleValueExpr::Literal(StyleValueLiteral::String(StyleStringLiteral {
			source: literal.token().to_string(),
			value: literal.value(),
			span,
		})),
		span,
	})
}

fn parse_integer_literal(input: ParseStream) -> syn::Result<StyleValueExpression> {
	let literal: LitInt = input.parse()?;
	let is_zero = numeric_mantissa_is_zero(literal.base10_digits());
	let suffix = literal.suffix().to_owned();
	parse_numeric_literal(
		input,
		literal.to_string(),
		suffix,
		literal.span(),
		is_zero,
		StyleValueLiteral::Integer,
	)
}

fn parse_float_literal(input: ParseStream) -> syn::Result<StyleValueExpression> {
	let literal: LitFloat = input.parse()?;
	let is_zero = numeric_mantissa_is_zero(literal.base10_digits());
	let suffix = literal.suffix().to_owned();
	parse_numeric_literal(
		input,
		literal.to_string(),
		suffix,
		literal.span(),
		is_zero,
		StyleValueLiteral::Number,
	)
}

fn numeric_mantissa_is_zero(base10_digits: &str) -> bool {
	let mantissa = base10_digits
		.split_once(['e', 'E'])
		.map_or(base10_digits, |(mantissa, _)| mantissa);
	let mut has_digit = false;
	for character in mantissa.chars() {
		match character {
			'0' => has_digit = true,
			'1'..='9' => return false,
			'.' | '_' => {}
			_ => return false,
		}
	}
	has_digit
}

fn parse_numeric_literal(
	input: ParseStream,
	rendered: String,
	suffix: String,
	literal_span: Span,
	is_zero: bool,
	constructor: fn(StyleNumericLiteral) -> StyleValueLiteral,
) -> syn::Result<StyleValueExpression> {
	let source = rendered
		.strip_suffix(&suffix)
		.unwrap_or(&rendered)
		.to_owned();
	let decimal_source = source.replace('_', "");
	if !is_css_decimal_number(&decimal_source) {
		return Err(syn::Error::new(
			literal_span,
			"style numbers must use plain CSS decimal syntax",
		));
	}
	let mut span = literal_span;
	let unit = if input.peek(Token![%]) {
		if !suffix.is_empty() {
			return Err(syn::Error::new(
				literal_span,
				"a numeric literal cannot have both a suffix unit and `%`",
			));
		}
		let percentage: Token![%] = input.parse()?;
		span = joined_span(literal_span, percentage.span());
		Some(StyleNumericUnit::Percentage {
			span: percentage.span(),
		})
	} else if suffix.is_empty() {
		None
	} else {
		Some(StyleNumericUnit::Named(StyleValueName {
			value: suffix,
			span: literal_span,
		}))
	};
	let contextual_zero = is_zero && unit.is_none();
	Ok(StyleValueExpression {
		kind: StyleValueExpr::Literal(constructor(StyleNumericLiteral {
			source,
			unit,
			contextual_zero,
			span,
		})),
		span,
	})
}

fn parse_identifier_expression(input: ParseStream) -> syn::Result<StyleValueExpression> {
	let first = parse_value_name(input)?;
	if input.peek(Token![!]) {
		return parse_macro_expression(input, first);
	}

	if matches!(first.as_str(), "globals" | "vars") && input.peek(Token![.]) {
		let dot: Token![.] = input.parse()?;
		let name = parse_value_name(input)?;
		let namespace = if first.as_str() == "globals" {
			StyleReferenceNamespace::Globals
		} else {
			StyleReferenceNamespace::Variables
		};
		let span = joined_span(first.span, name.span);
		let reference = StyleQualifiedReference {
			namespace,
			namespace_span: first.span,
			dot_span: dot.span(),
			name,
			span,
		};
		let expression = StyleValueExpression {
			kind: StyleValueExpr::QualifiedReference(reference),
			span,
		};
		return Ok(expression);
	}

	if input.peek(Token![::]) {
		let path = parse_value_path(input, first)?;
		if input.peek(syn::token::Paren) {
			return parse_call(input, path);
		}
		let span = path.span;
		return Ok(StyleValueExpression {
			kind: StyleValueExpr::AssociatedPathValue(path),
			span,
		});
	}

	let name = parse_kebab_keyword_name(input, first)?;
	let span = name.span;
	if input.peek(syn::token::Paren) {
		return parse_call(
			input,
			StyleValuePath {
				segments: vec![name],
				separator_spans: Vec::new(),
				span,
			},
		);
	}
	Ok(StyleValueExpression {
		kind: StyleValueExpr::Literal(StyleValueLiteral::Keyword(name)),
		span,
	})
}

fn parse_kebab_keyword_name(
	input: ParseStream,
	mut name: StyleValueName,
) -> syn::Result<StyleValueName> {
	while input.peek(Token![-]) {
		let fork = input.fork();
		fork.parse::<Token![-]>()?;
		if !fork.peek(Ident::peek_any) {
			break;
		}

		input.parse::<Token![-]>()?;
		let segment = parse_value_name(input)?;
		name.value.push('-');
		name.value.push_str(segment.as_str());
		name.span = joined_span(name.span, segment.span);
	}
	Ok(name)
}

fn parse_value_path(input: ParseStream, first: StyleValueName) -> syn::Result<StyleValuePath> {
	let first_span = first.span;
	let mut last_span = first.span;
	let mut segments = vec![first];
	let mut separator_spans = Vec::new();
	while input.peek(Token![::]) {
		let separator: Token![::] = input.parse()?;
		separator_spans.push(separator.span());
		let segment = parse_value_name(input)?;
		last_span = segment.span;
		segments.push(segment);
	}
	Ok(StyleValuePath {
		segments,
		separator_spans,
		span: joined_span(first_span, last_span),
	})
}

fn parse_call(input: ParseStream, path: StyleValuePath) -> syn::Result<StyleValueExpression> {
	let (arguments, arguments_span) = parse_call_arguments(input)?;
	let span = joined_span(path.span, arguments_span);
	Ok(StyleValueExpression {
		kind: StyleValueExpr::Call(StyleValueCall {
			path,
			arguments,
			arguments_span,
			span,
		}),
		span,
	})
}

fn parse_method_calls(
	input: ParseStream,
	mut receiver: StyleValueExpression,
) -> syn::Result<StyleValueExpression> {
	while input.peek(Token![.]) {
		let dot: Token![.] = input.parse()?;
		let method = parse_value_name(input)?;
		if !input.peek(syn::token::Paren) {
			return Err(syn::Error::new(
				method.span,
				"style value member access must be a method call",
			));
		}
		let (arguments, arguments_span) = parse_call_arguments(input)?;
		let span = joined_span(receiver.span, arguments_span);
		receiver = StyleValueExpression {
			kind: StyleValueExpr::MethodCall(StyleMethodCall {
				receiver: Box::new(receiver),
				method,
				arguments,
				dot_span: dot.span(),
				arguments_span,
				span,
			}),
			span,
		};
	}
	Ok(receiver)
}

fn parse_call_arguments(input: ParseStream) -> syn::Result<(Vec<StyleValueExpression>, Span)> {
	let content;
	let parentheses = parenthesized!(content in input);
	let span = parentheses.span.join();
	let mut arguments = Vec::new();
	while !content.is_empty() {
		arguments.push(parse_expression(&content, 0)?);
		if content.is_empty() {
			break;
		}
		content.parse::<Token![,]>()?;
	}
	Ok((arguments, span))
}

fn parse_macro_expression(
	input: ParseStream,
	macro_name: StyleValueName,
) -> syn::Result<StyleValueExpression> {
	let bang: Token![!] = input.parse()?;
	if macro_name.as_str() != "unchecked_fn" {
		return Err(syn::Error::new(
			macro_name.span,
			"only `unchecked_fn!` is allowed in style values",
		));
	}

	let content;
	let outer_parentheses = parenthesized!(content in input);
	if content.is_empty() {
		return Err(syn::Error::new(
			outer_parentheses.span.join(),
			"`unchecked_fn!` requires exactly one function call",
		));
	}
	let function_name = parse_value_name(&content).map_err(|_| {
		syn::Error::new(
			content.span(),
			"`unchecked_fn!` requires one plain function name and argument group",
		)
	})?;
	if !content.peek(syn::token::Paren) {
		return Err(syn::Error::new(
			content.span(),
			"`unchecked_fn!` requires one balanced function call",
		));
	}
	let arguments_content;
	let arguments_parentheses = parenthesized!(arguments_content in content);
	let mut tokens = Vec::new();
	while !arguments_content.is_empty() {
		tokens.push(arguments_content.parse()?);
	}
	if !content.is_empty() {
		return Err(syn::Error::new(
			content.span(),
			"`unchecked_fn!` accepts exactly one function call with no trailing tokens",
		));
	}

	let arguments_span = arguments_parentheses.span.join();
	let span = joined_span(macro_name.span, outer_parentheses.span.join());
	Ok(StyleValueExpression {
		kind: StyleValueExpr::UncheckedFunction(StyleUncheckedFunction {
			name: function_name,
			arguments: StyleRawTokenGroup {
				delimiter: Delimiter::Parenthesis,
				tokens,
				span: arguments_span,
			},
			macro_span: macro_name.span,
			bang_span: bang.span(),
			span,
		}),
		span,
	})
}

fn parse_value_name(input: ParseStream) -> syn::Result<StyleValueName> {
	let ident = Ident::parse_any(input)?;
	Ok(StyleValueName {
		value: unraw_ident(&ident),
		span: ident.span(),
	})
}

fn peek_binary_operator(input: ParseStream) -> Option<(u8, u8, StyleBinaryOperatorKind)> {
	if input.peek(Token![+]) {
		Some((1, 2, StyleBinaryOperatorKind::Add))
	} else if input.peek(Token![-]) {
		Some((1, 2, StyleBinaryOperatorKind::Subtract))
	} else if input.peek(Token![*]) {
		Some((3, 4, StyleBinaryOperatorKind::Multiply))
	} else if input.peek(Token![/]) {
		Some((3, 4, StyleBinaryOperatorKind::Divide))
	} else {
		None
	}
}

fn can_start_expression(input: ParseStream) -> bool {
	input.peek(Token![+])
		|| input.peek(Token![-])
		|| input.peek(syn::token::Paren)
		|| input.peek(syn::token::Bracket)
		|| input.peek(Token![#])
		|| input.peek(LitStr)
		|| input.peek(LitFloat)
		|| input.peek(LitInt)
		|| input.peek(Ident::peek_any)
}

fn binary_operator_symbol(kind: StyleBinaryOperatorKind) -> &'static str {
	match kind {
		StyleBinaryOperatorKind::Add => "+",
		StyleBinaryOperatorKind::Subtract => "-",
		StyleBinaryOperatorKind::Multiply => "*",
		StyleBinaryOperatorKind::Divide => "/",
	}
}

fn parse_binary_operator(
	input: ParseStream,
	kind: StyleBinaryOperatorKind,
) -> syn::Result<StyleBinaryOperator> {
	let span = match kind {
		StyleBinaryOperatorKind::Add => input.parse::<Token![+]>()?.span(),
		StyleBinaryOperatorKind::Subtract => input.parse::<Token![-]>()?.span(),
		StyleBinaryOperatorKind::Multiply => input.parse::<Token![*]>()?.span(),
		StyleBinaryOperatorKind::Divide => input.parse::<Token![/]>()?.span(),
	};
	Ok(StyleBinaryOperator { kind, span })
}

fn joined_span(first: Span, last: Span) -> Span {
	first.join(last).unwrap_or(first)
}

#[cfg(test)]
mod tests {
	use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
	use rstest::rstest;

	use crate::core::{
		StyleBinaryOperatorKind, StyleItem, StyleNumericUnit, StyleReferenceNamespace,
		StyleRuleItem, StyleUnaryOperatorKind, StyleValueExpr, StyleValueExpression,
		StyleValueLiteral,
	};
	use crate::parser::parse_style;
	use crate::style::registry::named_color_domain;

	#[derive(Debug, PartialEq, Eq)]
	enum ExprShape {
		Integer {
			source: String,
			unit: Option<String>,
			contextual_zero: bool,
		},
		Number {
			source: String,
			unit: Option<String>,
			contextual_zero: bool,
		},
		HexColor(String),
		Keyword(String),
		String {
			source: String,
			value: String,
		},
		QualifiedReference {
			namespace: StyleReferenceNamespace,
			name: String,
		},
		AssociatedPath(Vec<String>),
		Unary {
			operator: StyleUnaryOperatorKind,
			operand: Box<ExprShape>,
		},
		Binary {
			operator: StyleBinaryOperatorKind,
			left: Box<ExprShape>,
			right: Box<ExprShape>,
		},
		Call {
			path: Vec<String>,
			arguments: Vec<ExprShape>,
		},
		MethodCall {
			receiver: Box<ExprShape>,
			method: String,
			arguments: Vec<ExprShape>,
		},
		Group(Box<ExprShape>),
		SpaceSequence(Vec<ExprShape>),
		CommaList(Vec<ExprShape>),
		UncheckedFunction {
			name: String,
			arguments: RawGroupShape,
		},
	}

	#[derive(Debug, PartialEq, Eq)]
	struct RawGroupShape {
		delimiter: Delimiter,
		tokens: Vec<RawTokenShape>,
	}

	#[derive(Debug, PartialEq, Eq)]
	enum RawTokenShape {
		Group(RawGroupShape),
		Ident(String),
		Punct(char, Spacing),
		Literal(String),
	}

	fn parse_value(source: &str) -> StyleValueExpression {
		parse_value_result(source).unwrap()
	}

	fn parse_value_result(source: &str) -> syn::Result<StyleValueExpression> {
		let input: TokenStream = format!(".sample {{ value: {source}; }}").parse().unwrap();
		let style = parse_style(input)?;
		let StyleItem::Rule(rule) = &style.items[0] else {
			panic!("expected a style rule");
		};
		let StyleRuleItem::Declaration(declaration) = &rule.items[0] else {
			panic!("expected a style declaration");
		};
		Ok(declaration.value.clone())
	}

	fn expression_shape(expression: &StyleValueExpression) -> ExprShape {
		match &expression.kind {
			StyleValueExpr::Literal(StyleValueLiteral::Integer(number)) => ExprShape::Integer {
				source: number.source.clone(),
				unit: numeric_unit_name(number.unit.as_ref()),
				contextual_zero: number.contextual_zero,
			},
			StyleValueExpr::Literal(StyleValueLiteral::Number(number)) => ExprShape::Number {
				source: number.source.clone(),
				unit: numeric_unit_name(number.unit.as_ref()),
				contextual_zero: number.contextual_zero,
			},
			StyleValueExpr::Literal(StyleValueLiteral::HexColor(color)) => {
				ExprShape::HexColor(color.source.clone())
			}
			StyleValueExpr::Literal(StyleValueLiteral::Keyword(keyword)) => {
				ExprShape::Keyword(keyword.value.clone())
			}
			StyleValueExpr::Literal(StyleValueLiteral::String(string)) => ExprShape::String {
				source: string.source.clone(),
				value: string.value.clone(),
			},
			StyleValueExpr::QualifiedReference(reference) => ExprShape::QualifiedReference {
				namespace: reference.namespace,
				name: reference.name.value.clone(),
			},
			StyleValueExpr::AssociatedPathValue(path) => ExprShape::AssociatedPath(
				path.segments
					.iter()
					.map(|segment| segment.value.clone())
					.collect(),
			),
			StyleValueExpr::Unary(unary) => ExprShape::Unary {
				operator: unary.operator.kind,
				operand: Box::new(expression_shape(&unary.operand)),
			},
			StyleValueExpr::Binary(binary) => ExprShape::Binary {
				operator: binary.operator.kind,
				left: Box::new(expression_shape(&binary.left)),
				right: Box::new(expression_shape(&binary.right)),
			},
			StyleValueExpr::Call(call) => ExprShape::Call {
				path: call
					.path
					.segments
					.iter()
					.map(|segment| segment.value.clone())
					.collect(),
				arguments: call.arguments.iter().map(expression_shape).collect(),
			},
			StyleValueExpr::MethodCall(call) => ExprShape::MethodCall {
				receiver: Box::new(expression_shape(&call.receiver)),
				method: call.method.value.clone(),
				arguments: call.arguments.iter().map(expression_shape).collect(),
			},
			StyleValueExpr::Group(group) => {
				ExprShape::Group(Box::new(expression_shape(&group.expression)))
			}
			StyleValueExpr::SpaceSequence(sequence) => {
				ExprShape::SpaceSequence(sequence.items.iter().map(expression_shape).collect())
			}
			StyleValueExpr::CommaList(list) => {
				ExprShape::CommaList(list.items.iter().map(expression_shape).collect())
			}
			StyleValueExpr::UncheckedFunction(function) => ExprShape::UncheckedFunction {
				name: function.name.value.clone(),
				arguments: raw_group_shape(
					function.arguments.delimiter,
					&function.arguments.tokens,
				),
			},
		}
	}

	fn numeric_unit_name(unit: Option<&StyleNumericUnit>) -> Option<String> {
		unit.map(|unit| match unit {
			StyleNumericUnit::Named(name) => name.value.clone(),
			StyleNumericUnit::Percentage { .. } => "%".to_owned(),
		})
	}

	fn raw_group_shape(delimiter: Delimiter, tokens: &[TokenTree]) -> RawGroupShape {
		RawGroupShape {
			delimiter,
			tokens: tokens.iter().map(raw_token_shape).collect(),
		}
	}

	fn raw_token_shape(token: &TokenTree) -> RawTokenShape {
		match token {
			TokenTree::Group(group) => RawTokenShape::Group(raw_group_shape(
				group.delimiter(),
				&group.stream().into_iter().collect::<Vec<_>>(),
			)),
			TokenTree::Ident(ident) => RawTokenShape::Ident(ident.to_string()),
			TokenTree::Punct(punct) => RawTokenShape::Punct(punct.as_char(), punct.spacing()),
			TokenTree::Literal(literal) => RawTokenShape::Literal(literal.to_string()),
		}
	}

	fn integer(source: &str, unit: Option<&str>) -> ExprShape {
		ExprShape::Integer {
			source: source.to_owned(),
			unit: unit.map(str::to_owned),
			contextual_zero: source == "0" && unit.is_none(),
		}
	}

	fn keyword(value: &str) -> ExprShape {
		ExprShape::Keyword(value.to_owned())
	}

	fn reference(namespace: StyleReferenceNamespace, name: &str) -> ExprShape {
		ExprShape::QualifiedReference {
			namespace,
			name: name.to_owned(),
		}
	}

	#[rstest]
	#[case("1rem", integer("1", Some("rem")))]
	#[case("15%", integer("15", Some("%")))]
	#[case("#ff00aa", ExprShape::HexColor("#ff00aa".to_owned()))]
	#[case(
		"globals.surface_secondary",
		reference(StyleReferenceNamespace::Globals, "surface_secondary")
	)]
	#[case("vars.gutter", reference(StyleReferenceNamespace::Variables, "gutter"))]
	#[case(
		"100% - vars.gutter * 2",
		ExprShape::Binary {
			operator: StyleBinaryOperatorKind::Subtract,
			left: Box::new(integer("100", Some("%"))),
			right: Box::new(ExprShape::Binary {
				operator: StyleBinaryOperatorKind::Multiply,
				left: Box::new(reference(StyleReferenceNamespace::Variables, "gutter")),
				right: Box::new(integer("2", None)),
			}),
		}
	)]
	#[case(
		"clamp(240px, vars.height, 80vh)",
		ExprShape::Call {
			path: vec!["clamp".to_owned()],
			arguments: vec![
				integer("240", Some("px")),
				reference(StyleReferenceNamespace::Variables, "height"),
				integer("80", Some("vh")),
			],
		}
	)]
	#[case(
		"vars.accent.mix(white, 15%)",
		ExprShape::MethodCall {
			receiver: Box::new(reference(StyleReferenceNamespace::Variables, "accent")),
			method: "mix".to_owned(),
			arguments: vec![keyword("white"), integer("15", Some("%"))],
		}
	)]
	#[case(
		"(1px, solid, globals.border)",
		ExprShape::SpaceSequence(vec![
			integer("1", Some("px")),
			keyword("solid"),
			reference(StyleReferenceNamespace::Globals, "border"),
		])
	)]
	#[case(
		"[stop(red, 0%), stop(black, 100%)]",
		ExprShape::CommaList(vec![
			ExprShape::Call {
				path: vec!["stop".to_owned()],
				arguments: vec![keyword("red"), integer("0", Some("%"))],
			},
			ExprShape::Call {
				path: vec!["stop".to_owned()],
				arguments: vec![keyword("black"), integer("100", Some("%"))],
			},
		])
	)]
	#[case(
		"Color::rgb(20%, 30%, 40%)",
		ExprShape::Call {
			path: vec!["Color".to_owned(), "rgb".to_owned()],
			arguments: vec![
				integer("20", Some("%")),
				integer("30", Some("%")),
				integer("40", Some("%")),
			],
		}
	)]
	#[case(
		"Direction::Right",
		ExprShape::AssociatedPath(vec!["Direction".to_owned(), "Right".to_owned()])
	)]
	#[case(
		"unchecked_fn!(paint(my_worklet))",
		ExprShape::UncheckedFunction {
			name: "paint".to_owned(),
			arguments: RawGroupShape {
				delimiter: Delimiter::Parenthesis,
				tokens: vec![RawTokenShape::Ident("my_worklet".to_owned())],
			},
		}
	)]
	fn parses_representative_value_expression(#[case] source: &str, #[case] expected: ExprShape) {
		// Arrange
		// Source and expected AST are provided by the parameterized case.

		// Act
		let expression = parse_value(source);

		// Assert
		assert_eq!(expression_shape(&expression), expected);
	}

	#[rstest]
	fn applies_binary_operator_precedence_and_left_associativity() {
		// Arrange
		let source = "1px + 2px * 3 - 4px / 2";

		// Act
		let expression = parse_value(source);

		// Assert
		assert_eq!(
			expression_shape(&expression),
			ExprShape::Binary {
				operator: StyleBinaryOperatorKind::Subtract,
				left: Box::new(ExprShape::Binary {
					operator: StyleBinaryOperatorKind::Add,
					left: Box::new(integer("1", Some("px"))),
					right: Box::new(ExprShape::Binary {
						operator: StyleBinaryOperatorKind::Multiply,
						left: Box::new(integer("2", Some("px"))),
						right: Box::new(integer("3", None)),
					}),
				}),
				right: Box::new(ExprShape::Binary {
					operator: StyleBinaryOperatorKind::Divide,
					left: Box::new(integer("4", Some("px"))),
					right: Box::new(integer("2", None)),
				}),
			},
		);
	}

	#[rstest]
	fn binds_unary_operators_to_the_next_value() {
		// Arrange
		let source = "-1px + +(2px * 3)";

		// Act
		let expression = parse_value(source);

		// Assert
		assert_eq!(
			expression_shape(&expression),
			ExprShape::Binary {
				operator: StyleBinaryOperatorKind::Add,
				left: Box::new(ExprShape::Unary {
					operator: StyleUnaryOperatorKind::Minus,
					operand: Box::new(integer("1", Some("px"))),
				}),
				right: Box::new(ExprShape::Unary {
					operator: StyleUnaryOperatorKind::Plus,
					operand: Box::new(ExprShape::Group(Box::new(ExprShape::Binary {
						operator: StyleBinaryOperatorKind::Multiply,
						left: Box::new(integer("2", Some("px"))),
						right: Box::new(integer("3", None)),
					}))),
				}),
			},
		);
	}

	#[rstest]
	fn distinguishes_group_space_sequence_and_comma_list_nodes() {
		// Arrange
		let group_source = "(1px + 2px)";
		let sequence_source = "(1px, solid)";
		let list_source = "[red, blue]";

		// Act
		let group = parse_value(group_source);
		let sequence = parse_value(sequence_source);
		let list = parse_value(list_source);

		// Assert
		assert_eq!(
			expression_shape(&group),
			ExprShape::Group(Box::new(ExprShape::Binary {
				operator: StyleBinaryOperatorKind::Add,
				left: Box::new(integer("1", Some("px"))),
				right: Box::new(integer("2", Some("px"))),
			}))
		);
		assert_eq!(
			expression_shape(&sequence),
			ExprShape::SpaceSequence(vec![integer("1", Some("px")), keyword("solid")])
		);
		assert_eq!(
			expression_shape(&list),
			ExprShape::CommaList(vec![keyword("red"), keyword("blue")])
		);
	}

	#[rstest]
	#[case("#abc", "#abc", "abc")]
	#[case("#ABCD", "#ABCD", "ABCD")]
	#[case("#12aBcD", "#12aBcD", "12aBcD")]
	#[case("#1234aBcD", "#1234aBcD", "1234aBcD")]
	fn preserves_valid_hex_color_spelling(
		#[case] source: &str,
		#[case] expected_source: &str,
		#[case] expected_digits: &str,
	) {
		// Arrange
		// Source and expected spellings are provided by the parameterized case.

		// Act
		let expression = parse_value(source);

		// Assert
		let StyleValueExpr::Literal(StyleValueLiteral::HexColor(color)) = expression.kind else {
			panic!("expected a hexadecimal color literal");
		};
		assert_eq!(color.source, expected_source);
		assert_eq!(color.digits, expected_digits);
	}

	#[rstest]
	#[case("#a")]
	#[case("#ab")]
	#[case("#abcde")]
	#[case("#abcdefg")]
	#[case("#123456789")]
	#[case("#ggg")]
	fn rejects_invalid_hex_color_digits(#[case] source: &str) {
		// Arrange
		// Source is provided by the parameterized case.

		// Act
		let error = parse_value_result(source).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"hex colors require exactly 3, 4, 6, or 8 hexadecimal digits"
		);
	}

	#[rstest]
	#[case("42", integer("42", None))]
	#[case(
		"1_000.50",
		ExprShape::Number {
			source: "1_000.50".to_owned(),
			unit: None,
			contextual_zero: false,
		}
	)]
	#[case("0", integer("0", None))]
	#[case(
		"0.0",
		ExprShape::Number {
			source: "0.0".to_owned(),
			unit: None,
			contextual_zero: true,
		}
	)]
	#[case(
		"0e-999",
		ExprShape::Number {
			source: "0e-999".to_owned(),
			unit: None,
			contextual_zero: true,
		}
	)]
	#[case(
		"1e-999",
		ExprShape::Number {
			source: "1e-999".to_owned(),
			unit: None,
			contextual_zero: false,
		}
	)]
	#[case(
		"1e-324",
		ExprShape::Number {
			source: "1e-324".to_owned(),
			unit: None,
			contextual_zero: false,
		}
	)]
	#[case("0px", integer("0", Some("px")))]
	fn retains_unitless_number_kind_and_contextual_zero(
		#[case] source: &str,
		#[case] expected: ExprShape,
	) {
		// Arrange
		// Source and expected AST are provided by the parameterized case.

		// Act
		let expression = parse_value(source);

		// Assert
		assert_eq!(expression_shape(&expression), expected);
	}

	#[rstest]
	#[case("\"Open Sans\"", "\"Open Sans\"", "Open Sans")]
	#[case(
		"r#\"quoted \\\"family\\\"\"#",
		"r#\"quoted \\\"family\\\"\"#",
		"quoted \\\"family\\\""
	)]
	fn parses_quoted_css_strings_as_string_literals(
		#[case] source: &str,
		#[case] expected_source: &str,
		#[case] expected_value: &str,
	) {
		// Arrange
		// Source and expected string forms are provided by the parameterized case.

		// Act
		let expression = parse_value(source);

		// Assert
		assert_eq!(
			expression_shape(&expression),
			ExprShape::String {
				source: expected_source.to_owned(),
				value: expected_value.to_owned(),
			}
		);
	}

	#[rstest]
	#[case("px")]
	#[case("cm")]
	#[case("mm")]
	#[case("q")]
	#[case("in")]
	#[case("pc")]
	#[case("pt")]
	#[case("em")]
	#[case("rem")]
	#[case("ex")]
	#[case("rex")]
	#[case("cap")]
	#[case("rcap")]
	#[case("ch")]
	#[case("rch")]
	#[case("ic")]
	#[case("ric")]
	#[case("lh")]
	#[case("rlh")]
	#[case("vw")]
	#[case("vh")]
	#[case("vi")]
	#[case("vb")]
	#[case("vmin")]
	#[case("vmax")]
	#[case("svw")]
	#[case("svh")]
	#[case("svi")]
	#[case("svb")]
	#[case("svmin")]
	#[case("svmax")]
	#[case("lvw")]
	#[case("lvh")]
	#[case("lvi")]
	#[case("lvb")]
	#[case("lvmin")]
	#[case("lvmax")]
	#[case("dvw")]
	#[case("dvh")]
	#[case("dvi")]
	#[case("dvb")]
	#[case("dvmin")]
	#[case("dvmax")]
	#[case("cqw")]
	#[case("cqh")]
	#[case("cqi")]
	#[case("cqb")]
	#[case("cqmin")]
	#[case("cqmax")]
	#[case("fr")]
	#[case("deg")]
	#[case("grad")]
	#[case("rad")]
	#[case("turn")]
	#[case("ms")]
	#[case("s")]
	#[case("%")]
	fn parses_every_mvp_numeric_unit(#[case] unit: &str) {
		// Arrange
		let source = if unit == "%" {
			"17%".to_owned()
		} else {
			format!("17{unit}")
		};

		// Act
		let expression = parse_value(&source);

		// Assert
		assert_eq!(expression_shape(&expression), integer("17", Some(unit)));
	}

	#[rstest]
	fn parses_every_css_color_keyword_as_an_identifier_literal() {
		// Arrange
		let color_keywords = named_color_domain().keywords;
		let expected_count = 150;

		// Act
		let parsed: Vec<_> = color_keywords
			.iter()
			.map(|keyword| expression_shape(&parse_value(keyword)))
			.collect();

		// Assert
		assert_eq!(color_keywords.len(), expected_count);
		assert_eq!(parsed.len(), expected_count);
		for (actual, expected) in parsed.into_iter().zip(color_keywords) {
			assert_eq!(actual, keyword(expected));
		}
	}

	#[rstest]
	#[case("inline-flex", "inline-flex")]
	#[case("inline - flex", "inline-flex")]
	#[case("ease-in-out", "ease-in-out")]
	#[case("ease - in - out", "ease-in-out")]
	fn parses_bare_identifier_hyphens_as_one_keyword_literal(
		#[case] source: &str,
		#[case] expected: &str,
	) {
		// Arrange
		// Rust token streams do not retain whitespace around a standalone `-` token.

		// Act
		let expression = parse_value(source);

		// Assert
		assert_eq!(expression_shape(&expression), keyword(expected));
	}

	#[rstest]
	#[case(
		"vars.a - vars.b",
		ExprShape::Binary {
			operator: StyleBinaryOperatorKind::Subtract,
			left: Box::new(reference(StyleReferenceNamespace::Variables, "a")),
			right: Box::new(reference(StyleReferenceNamespace::Variables, "b")),
		}
	)]
	#[case(
		"1 - vars.x",
		ExprShape::Binary {
			operator: StyleBinaryOperatorKind::Subtract,
			left: Box::new(integer("1", None)),
			right: Box::new(reference(StyleReferenceNamespace::Variables, "x")),
		}
	)]
	fn keeps_typed_subtraction_as_a_binary_expression(
		#[case] source: &str,
		#[case] expected: ExprShape,
	) {
		// Arrange
		// Source and expected AST are provided by the parameterized case.

		// Act
		let expression = parse_value(source);

		// Assert
		assert_eq!(expression_shape(&expression), expected);
	}

	#[rstest]
	fn keeps_unchecked_function_argument_tokens_losslessly_structured() {
		// Arrange
		let source = "unchecked_fn!(paint(namespace::worklet, [red, blue], { tone: \"dark\" }))";

		// Act
		let expression = parse_value(source);

		// Assert
		assert_eq!(
			expression_shape(&expression),
			ExprShape::UncheckedFunction {
				name: "paint".to_owned(),
				arguments: RawGroupShape {
					delimiter: Delimiter::Parenthesis,
					tokens: vec![
						RawTokenShape::Ident("namespace".to_owned()),
						RawTokenShape::Punct(':', Spacing::Joint),
						RawTokenShape::Punct(':', Spacing::Alone),
						RawTokenShape::Ident("worklet".to_owned()),
						RawTokenShape::Punct(',', Spacing::Alone),
						RawTokenShape::Group(RawGroupShape {
							delimiter: Delimiter::Bracket,
							tokens: vec![
								RawTokenShape::Ident("red".to_owned()),
								RawTokenShape::Punct(',', Spacing::Alone),
								RawTokenShape::Ident("blue".to_owned()),
							],
						}),
						RawTokenShape::Punct(',', Spacing::Alone),
						RawTokenShape::Group(RawGroupShape {
							delimiter: Delimiter::Brace,
							tokens: vec![
								RawTokenShape::Ident("tone".to_owned()),
								RawTokenShape::Punct(':', Spacing::Alone),
								RawTokenShape::Literal("\"dark\"".to_owned()),
							],
						}),
					],
				},
			}
		);
	}

	#[rstest]
	#[case("0xff")]
	#[case("0b10")]
	#[case("0o10")]
	fn rejects_rust_base_prefixed_integer_literals(#[case] source: &str) {
		// Arrange
		// Source is provided by the parameterized case.

		// Act
		let error = parse_value_result(source).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"style numbers must use plain CSS decimal syntax"
		);
	}

	#[rstest]
	#[case("r#static", keyword("static"))]
	#[case(
		"r#type::r#match",
		ExprShape::AssociatedPath(vec!["type".to_owned(), "match".to_owned()])
	)]
	fn normalizes_raw_rust_identifiers_to_css_text(
		#[case] source: &str,
		#[case] expected: ExprShape,
	) {
		// Arrange
		// Source and expected AST are provided by the parameterized case.

		// Act
		let expression = parse_value(source);

		// Assert
		assert_eq!(expression_shape(&expression), expected);
	}

	#[rstest]
	#[case("()", "style value sequences cannot be empty")]
	#[case("[]", "style value comma lists cannot be empty")]
	#[case(
		"paint!(my_worklet)",
		"only `unchecked_fn!` is allowed in style values"
	)]
	#[case(
		"unchecked_fn!((paint(my_worklet)))",
		"`unchecked_fn!` requires one plain function name and argument group"
	)]
	#[case(
		"unchecked_fn!(paint)",
		"`unchecked_fn!` requires one balanced function call"
	)]
	#[case(
		"unchecked_fn!(paint(first) paint(second))",
		"`unchecked_fn!` accepts exactly one function call with no trailing tokens"
	)]
	#[case(
		"unchecked_fn!(paint(my_worklet) trailing)",
		"`unchecked_fn!` accepts exactly one function call with no trailing tokens"
	)]
	#[case(
		"unchecked_fn!(paint(nested(value)) trailing)",
		"`unchecked_fn!` accepts exactly one function call with no trailing tokens"
	)]
	#[case("1rem%", "a numeric literal cannot have both a suffix unit and `%`")]
	#[case("1px +", "expected a style value expression after `+`")]
	#[case(
		"(value,)",
		"one-item parenthesized style values cannot have a trailing comma"
	)]
	fn rejects_invalid_value_expression_shapes(#[case] source: &str, #[case] expected: &str) {
		// Arrange
		// Source and expected diagnostic are provided by the parameterized case.

		// Act
		let error = parse_value_result(source).unwrap_err();

		// Assert
		assert_eq!(error.to_string(), expected);
	}
}
