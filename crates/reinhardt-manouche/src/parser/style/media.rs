//! Parser for static media conditions.

use proc_macro2::{Delimiter, Spacing, Span, TokenTree};
use syn::Lit;

use super::{is_css_decimal_number, unraw_ident};
use crate::core::{
	StyleMediaCondition, StyleMediaGroup, StyleMediaIdentifier, StyleMediaNumber,
	StyleMediaNumberKind, StyleMediaOperator, StyleMediaOperatorKind, StyleMediaPunctuation,
	StyleMediaPunctuationKind, StyleMediaToken,
};
use crate::{UnitCategory, unit_specs};

pub(super) fn parse_media_condition(
	tokens: Vec<TokenTree>,
	empty_span: Span,
) -> syn::Result<StyleMediaCondition> {
	let span = span_of_tokens(&tokens)
		.ok_or_else(|| syn::Error::new(empty_span, "media conditions cannot be empty"))?;
	let mut tokens = parse_media_tokens(&tokens)?;
	classify_media_operators(&mut tokens);
	validate_media_query_list(&tokens)?;
	Ok(StyleMediaCondition { tokens, span })
}

fn parse_media_tokens(tokens: &[TokenTree]) -> syn::Result<Vec<StyleMediaToken>> {
	let mut parsed = Vec::new();
	let mut index = 0;
	while index < tokens.len() {
		match &tokens[index] {
			TokenTree::Ident(_) => {
				let identifier = parse_identifier(tokens, &mut index)?;
				if matches!(identifier.as_str(), "globals" | "vars") && punct_at(tokens, index, '.')
				{
					return Err(syn::Error::new(
						identifier.span,
						"`globals.*` and `vars.*` references are not allowed in media conditions",
					));
				}
				if is_rust_control_flow_keyword(identifier.as_str()) {
					return Err(syn::Error::new(
						identifier.span,
						"Rust control-flow keywords are not allowed in media conditions",
					));
				}
				parsed.push(StyleMediaToken::Identifier(identifier));
			}
			TokenTree::Literal(literal) => {
				parsed.push(StyleMediaToken::Number(parse_number(literal)?));
				index += 1;
			}
			TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis => {
				if index > 0 && preceding_token_cannot_group(tokens, index - 1) {
					return Err(syn::Error::new(
						group.span(),
						"Rust expression operators are not allowed in media conditions",
					));
				}
				let inner: Vec<_> = group.stream().into_iter().collect();
				if inner.is_empty() {
					return Err(syn::Error::new(
						group.span(),
						"media condition groups cannot be empty",
					));
				}
				parsed.push(StyleMediaToken::Parenthesized(StyleMediaGroup {
					tokens: parse_media_tokens(&inner)?,
					span: group.span(),
				}));
				index += 1;
			}
			TokenTree::Group(group) => {
				return Err(syn::Error::new(
					group.span(),
					"media conditions support only parenthesized groups",
				));
			}
			TokenTree::Punct(punct) if punct.as_char() == '!' => {
				let message = if matches!(tokens.get(index + 1), Some(TokenTree::Group(_))) {
					"macros are not allowed in media conditions"
				} else {
					"Rust expression operators are not allowed in media conditions"
				};
				return Err(syn::Error::new(punct.span(), message));
			}
			TokenTree::Punct(punct) => {
				let (kind, consumed) = parse_punctuation(tokens, index)?;
				parsed.push(StyleMediaToken::Punctuation(StyleMediaPunctuation {
					kind,
					span: joined_span(punct.span(), tokens[index + consumed - 1].span()),
				}));
				index += consumed;
			}
		}
	}
	Ok(parsed)
}

fn parse_identifier(tokens: &[TokenTree], index: &mut usize) -> syn::Result<StyleMediaIdentifier> {
	let TokenTree::Ident(first) = &tokens[*index] else {
		return Err(syn::Error::new(
			tokens[*index].span(),
			"expected a media identifier",
		));
	};
	let first_span = first.span();
	let mut last_span = first_span;
	let mut value = unraw_ident(first);
	*index += 1;

	while punct_at(tokens, *index, '-')
		&& matches!(tokens.get(*index + 1), Some(TokenTree::Ident(_)))
	{
		let hyphen_span = tokens[*index].span();
		let Some(TokenTree::Ident(segment)) = tokens.get(*index + 1) else {
			return Err(syn::Error::new(hyphen_span, "expected a media identifier"));
		};
		value.push('-');
		value.push_str(&unraw_ident(segment));
		last_span = segment.span();
		*index += 2;
	}

	Ok(StyleMediaIdentifier {
		value,
		span: joined_span(first_span, last_span),
	})
}

fn parse_number(literal: &proc_macro2::Literal) -> syn::Result<StyleMediaNumber> {
	let span = literal.span();
	let source = literal.to_string();
	let parsed = syn::parse2::<Lit>(TokenTree::Literal(literal.clone()).into())
		.map_err(|_| syn::Error::new(span, "media conditions accept only numeric literals"))?;
	let (suffix, kind) = match parsed {
		Lit::Int(number) => (number.suffix().to_owned(), StyleMediaNumberKind::Integer),
		Lit::Float(number) => (number.suffix().to_owned(), StyleMediaNumberKind::Float),
		_ => {
			return Err(syn::Error::new(
				span,
				"media conditions accept only numeric literals",
			));
		}
	};
	let value = source.strip_suffix(&suffix).unwrap_or(&source);
	if value.contains('_')
		|| value.starts_with("0x")
		|| value.starts_with("0X")
		|| value.starts_with("0o")
		|| value.starts_with("0O")
		|| value.starts_with("0b")
		|| value.starts_with("0B")
		|| !is_css_decimal_number(value)
	{
		return Err(syn::Error::new(
			span,
			"media numbers must use plain CSS decimal syntax",
		));
	}
	if !suffix.is_empty() && (!is_css_unit_suffix(&suffix) || is_rust_numeric_suffix(&suffix)) {
		return Err(syn::Error::new(
			span,
			"media numeric suffixes must be CSS units",
		));
	}

	Ok(StyleMediaNumber {
		literal: literal.clone(),
		value: value.to_owned(),
		unit: (!suffix.is_empty()).then_some(suffix),
		kind,
		span,
	})
}

fn parse_punctuation(
	tokens: &[TokenTree],
	index: usize,
) -> syn::Result<(StyleMediaPunctuationKind, usize)> {
	let TokenTree::Punct(punct) = &tokens[index] else {
		return Err(syn::Error::new(
			tokens[index].span(),
			"expected media punctuation",
		));
	};
	let rust_operator_error = || {
		syn::Error::new(
			punct.span(),
			"Rust expression operators are not allowed in media conditions",
		)
	};
	let result = match punct.as_char() {
		':' if punct_at(tokens, index + 1, ':') => return Err(rust_operator_error()),
		':' => (StyleMediaPunctuationKind::Colon, 1),
		'/' => (StyleMediaPunctuationKind::Slash, 1),
		',' => (StyleMediaPunctuationKind::Comma, 1),
		'%' => (StyleMediaPunctuationKind::Percent, 1),
		'+' if !unary_sign_at(tokens, index) => return Err(rust_operator_error()),
		'+' => (StyleMediaPunctuationKind::Plus, 1),
		'-' if !unary_sign_at(tokens, index) => return Err(rust_operator_error()),
		'-' => (StyleMediaPunctuationKind::Minus, 1),
		'<' if punct_at(tokens, index + 1, '<') => return Err(rust_operator_error()),
		'<' if punct_at(tokens, index + 1, '=') => {
			if punct.spacing() != Spacing::Joint {
				return Err(syn::Error::new(
					punct.span(),
					"media comparison operators cannot contain whitespace",
				));
			}
			(StyleMediaPunctuationKind::LessThanOrEqual, 2)
		}
		'<' => (StyleMediaPunctuationKind::LessThan, 1),
		'>' if punct_at(tokens, index + 1, '>') => return Err(rust_operator_error()),
		'>' if punct_at(tokens, index + 1, '=') => {
			if punct.spacing() != Spacing::Joint {
				return Err(syn::Error::new(
					punct.span(),
					"media comparison operators cannot contain whitespace",
				));
			}
			(StyleMediaPunctuationKind::GreaterThanOrEqual, 2)
		}
		'>' => (StyleMediaPunctuationKind::GreaterThan, 1),
		'=' if punct_at(tokens, index + 1, '=') || punct_at(tokens, index + 1, '>') => {
			return Err(rust_operator_error());
		}
		'=' => (StyleMediaPunctuationKind::Equal, 1),
		'*' | '&' | '|' | '?' | '.' | ';' => return Err(rust_operator_error()),
		_ => {
			return Err(syn::Error::new(
				punct.span(),
				"unsupported punctuation in media conditions",
			));
		}
	};
	Ok(result)
}

fn classify_media_operators(tokens: &mut [StyleMediaToken]) {
	classify_media_operators_in_context(tokens, true);
}

fn classify_media_operators_in_context(
	tokens: &mut [StyleMediaToken],
	reserve_query_branch_keywords: bool,
) {
	for token in tokens.iter_mut() {
		if let StyleMediaToken::Parenthesized(group) = token {
			classify_media_operators_in_context(&mut group.tokens, false);
		}
	}

	for index in 0..tokens.len() {
		let Some((kind, span)) = operator_candidate(&tokens[index]) else {
			continue;
		};
		let reserved_query_branch_keyword =
			reserve_query_branch_keywords && query_branch_starts_at(tokens, index);
		if !reserved_query_branch_keyword && !operator_is_grammatical(tokens, index, kind) {
			continue;
		}
		tokens[index] = StyleMediaToken::Operator(StyleMediaOperator { kind, span });
	}
}

fn operator_candidate(token: &StyleMediaToken) -> Option<(StyleMediaOperatorKind, Span)> {
	let StyleMediaToken::Identifier(identifier) = token else {
		return None;
	};
	Some((media_operator(identifier.as_str())?, identifier.span))
}

fn operator_is_grammatical(
	tokens: &[StyleMediaToken],
	index: usize,
	kind: StyleMediaOperatorKind,
) -> bool {
	match kind {
		StyleMediaOperatorKind::Only => {
			query_branch_starts_at(tokens, index)
				&& matches!(tokens.get(index + 1), Some(StyleMediaToken::Identifier(_)))
		}
		StyleMediaOperatorKind::Not => {
			let starts_operand = query_branch_starts_at(tokens, index)
				|| matches!(
					tokens.get(index.wrapping_sub(1)),
					Some(StyleMediaToken::Operator(operator))
						if matches!(
							operator.kind,
							StyleMediaOperatorKind::And | StyleMediaOperatorKind::Or
						)
				);
			starts_operand
				&& (matches!(
					tokens.get(index + 1),
					Some(StyleMediaToken::Parenthesized(_))
				) || query_branch_starts_at(tokens, index)
					&& matches!(tokens.get(index + 1), Some(StyleMediaToken::Identifier(_))))
		}
		StyleMediaOperatorKind::And => {
			matches!(
				tokens.get(index.wrapping_sub(1)),
				Some(StyleMediaToken::Identifier(_) | StyleMediaToken::Parenthesized(_))
			) && next_is_boolean_operand(tokens, index + 1)
		}
		StyleMediaOperatorKind::Or => {
			matches!(
				tokens.get(index.wrapping_sub(1)),
				Some(StyleMediaToken::Parenthesized(_))
			) && next_is_boolean_operand(tokens, index + 1)
		}
	}
}

fn query_branch_starts_at(tokens: &[StyleMediaToken], index: usize) -> bool {
	index == 0
		|| matches!(
			tokens.get(index - 1),
			Some(StyleMediaToken::Punctuation(punctuation))
				if punctuation.kind == StyleMediaPunctuationKind::Comma
		)
}

fn next_is_boolean_operand(tokens: &[StyleMediaToken], index: usize) -> bool {
	if matches!(tokens.get(index), Some(StyleMediaToken::Parenthesized(_))) {
		return true;
	}
	matches!(
		(tokens.get(index), tokens.get(index + 1)),
		(
			Some(StyleMediaToken::Identifier(identifier)),
			Some(StyleMediaToken::Parenthesized(_))
		) if identifier.as_str().eq_ignore_ascii_case("not")
	)
}

fn validate_media_query_list(tokens: &[StyleMediaToken]) -> syn::Result<()> {
	let mut branch_start = 0;
	for index in 0..=tokens.len() {
		let at_separator = index == tokens.len()
			|| matches!(
				tokens.get(index),
				Some(StyleMediaToken::Punctuation(punctuation))
					if punctuation.kind == StyleMediaPunctuationKind::Comma
			);
		if !at_separator {
			continue;
		}
		if branch_start == index {
			let span = tokens
				.get(index)
				.or_else(|| tokens.last())
				.map_or_else(Span::call_site, StyleMediaToken::span);
			return Err(syn::Error::new(
				span,
				"media query list branches cannot be empty",
			));
		}
		validate_media_query(&tokens[branch_start..index])?;
		branch_start = index + 1;
	}
	Ok(())
}

fn validate_media_query(tokens: &[StyleMediaToken]) -> syn::Result<()> {
	match tokens {
		[StyleMediaToken::Identifier(_), ..] => validate_media_type_query(tokens, 0),
		[
			StyleMediaToken::Operator(modifier),
			StyleMediaToken::Identifier(_),
			..,
		] if matches!(
			modifier.kind,
			StyleMediaOperatorKind::Only | StyleMediaOperatorKind::Not
		) =>
		{
			validate_media_type_query(tokens, 1)
		}
		_ => validate_boolean_condition(tokens),
	}
}

fn validate_media_type_query(
	tokens: &[StyleMediaToken],
	media_type_index: usize,
) -> syn::Result<()> {
	let tail_index = media_type_index + 1;
	let Some(token) = tokens.get(tail_index) else {
		return Ok(());
	};
	let StyleMediaToken::Operator(operator) = token else {
		return Err(invalid_boolean_order(token));
	};
	if operator.kind != StyleMediaOperatorKind::And {
		return Err(invalid_boolean_order(token));
	}
	validate_media_condition_without_or(&tokens[tail_index + 1..])
}

fn validate_media_condition_without_or(tokens: &[StyleMediaToken]) -> syn::Result<()> {
	if tokens.is_empty() {
		return Err(syn::Error::new(
			Span::call_site(),
			"invalid boolean operator order in media condition",
		));
	}

	let starts_with_not = matches!(
		tokens.first(),
		Some(StyleMediaToken::Operator(operator))
			if operator.kind == StyleMediaOperatorKind::Not
	);
	let mut index = 0;
	validate_boolean_operand(tokens, &mut index)?;
	if starts_with_not {
		return if index == tokens.len() {
			Ok(())
		} else {
			Err(invalid_boolean_order(&tokens[index]))
		};
	}

	while index < tokens.len() {
		let Some(StyleMediaToken::Operator(operator)) = tokens.get(index) else {
			return Err(invalid_boolean_order(&tokens[index]));
		};
		if operator.kind != StyleMediaOperatorKind::And {
			return Err(invalid_boolean_order(&tokens[index]));
		}
		index += 1;
		let Some(StyleMediaToken::Parenthesized(group)) = tokens.get(index) else {
			let token = tokens.get(index).unwrap_or(&tokens[index - 1]);
			return Err(invalid_boolean_order(token));
		};
		validate_media_group(group)?;
		index += 1;
	}
	Ok(())
}

fn validate_boolean_condition(tokens: &[StyleMediaToken]) -> syn::Result<()> {
	if tokens.is_empty() {
		return Err(syn::Error::new(
			Span::call_site(),
			"invalid boolean operator order in media condition",
		));
	}
	let mut index = 0;
	let mut connective = None;
	validate_boolean_operand(tokens, &mut index)?;
	while index < tokens.len() {
		let Some(StyleMediaToken::Operator(operator)) = tokens.get(index) else {
			return Err(invalid_boolean_order(&tokens[index]));
		};
		if !matches!(
			operator.kind,
			StyleMediaOperatorKind::And | StyleMediaOperatorKind::Or
		) {
			return Err(invalid_boolean_order(&tokens[index]));
		}
		if let Some(expected) = connective {
			if operator.kind != expected {
				return Err(syn::Error::new(
					operator.span,
					"mixed `and` and `or` media conditions must be parenthesized",
				));
			}
		} else {
			connective = Some(operator.kind);
		}
		index += 1;
		validate_boolean_operand(tokens, &mut index)?;
	}
	Ok(())
}

fn validate_boolean_operand(tokens: &[StyleMediaToken], index: &mut usize) -> syn::Result<()> {
	if matches!(
		tokens.get(*index),
		Some(StyleMediaToken::Operator(operator))
			if operator.kind == StyleMediaOperatorKind::Not
	) {
		*index += 1;
	}
	let Some(StyleMediaToken::Parenthesized(group)) = tokens.get(*index) else {
		let span = tokens
			.get(*index)
			.or_else(|| tokens.last())
			.map_or_else(Span::call_site, StyleMediaToken::span);
		return Err(syn::Error::new(
			span,
			"invalid boolean operator order in media condition",
		));
	};
	validate_media_group(group)?;
	*index += 1;
	Ok(())
}

fn validate_media_group(group: &StyleMediaGroup) -> syn::Result<()> {
	if group.tokens.iter().any(|token| {
		matches!(
			token,
			StyleMediaToken::Operator(_) | StyleMediaToken::Parenthesized(_)
		)
	}) {
		validate_boolean_condition(&group.tokens)
	} else {
		validate_media_feature(&group.tokens)
	}
}

fn validate_media_feature(tokens: &[StyleMediaToken]) -> syn::Result<()> {
	let comparisons: Vec<_> = tokens
		.iter()
		.enumerate()
		.filter_map(|(index, token)| comparison_kind(token).map(|kind| (index, kind)))
		.collect();
	if !comparisons.is_empty() {
		return validate_media_range(tokens, &comparisons);
	}

	let colon_indexes: Vec<_> = tokens
		.iter()
		.enumerate()
		.filter_map(|(index, token)| {
			matches!(
				token,
				StyleMediaToken::Punctuation(punctuation)
					if punctuation.kind == StyleMediaPunctuationKind::Colon
			)
			.then_some(index)
		})
		.collect();
	match colon_indexes.as_slice() {
		[] if matches!(tokens, [StyleMediaToken::Identifier(_)]) => Ok(()),
		[1] => match tokens.first() {
			Some(StyleMediaToken::Identifier(feature))
				if valid_colon_feature_value(feature.as_str(), &tokens[2..]) =>
			{
				Ok(())
			}
			_ => Err(syn::Error::new(
				tokens
					.first()
					.map_or_else(Span::call_site, StyleMediaToken::span),
				"invalid media feature expression",
			)),
		},
		_ => Err(syn::Error::new(
			tokens
				.first()
				.map_or_else(Span::call_site, StyleMediaToken::span),
			"invalid media feature expression",
		)),
	}
}

fn validate_media_range(
	tokens: &[StyleMediaToken],
	comparisons: &[(usize, StyleMediaPunctuationKind)],
) -> syn::Result<()> {
	match *comparisons {
		[(comparison_index, _)] => {
			let left = &tokens[..comparison_index];
			let right = &tokens[comparison_index + 1..];
			let left_is_feature = is_feature_identifier(left);
			let right_is_feature = is_feature_identifier(right);
			if left_is_feature == right_is_feature {
				return Err(syn::Error::new(
					tokens[comparison_index].span(),
					"media range expressions must contain a feature identifier",
				));
			}
			let (feature, value) = if let Some(feature) = media_feature_identifier(left) {
				(feature, right)
			} else if let Some(feature) = media_feature_identifier(right) {
				(feature, left)
			} else {
				return Err(syn::Error::new(
					tokens[comparison_index].span(),
					"media range expressions must contain a feature identifier",
				));
			};
			if !valid_numeric_range_value(feature, value) {
				return Err(syn::Error::new(
					value
						.first()
						.map_or_else(|| tokens[comparison_index].span(), StyleMediaToken::span),
					"invalid media range value",
				));
			}
			Ok(())
		}
		[(first_index, first_kind), (second_index, second_kind)] => {
			let left = &tokens[..first_index];
			let feature = &tokens[first_index + 1..second_index];
			let right = &tokens[second_index + 1..];
			let Some(feature) = media_feature_identifier(feature) else {
				return Err(syn::Error::new(
					tokens[first_index].span(),
					"media range expressions must contain a feature identifier",
				));
			};
			if !valid_numeric_range_value(feature, left)
				|| !valid_numeric_range_value(feature, right)
			{
				return Err(syn::Error::new(
					tokens[first_index].span(),
					"invalid media range value",
				));
			}
			if range_direction(first_kind).is_none()
				|| range_direction(first_kind) != range_direction(second_kind)
			{
				return Err(syn::Error::new(
					tokens[second_index].span(),
					"two-sided media ranges must use comparisons in the same direction",
				));
			}
			Ok(())
		}
		_ => Err(syn::Error::new(
			tokens
				.first()
				.map_or_else(Span::call_site, StyleMediaToken::span),
			"invalid media feature expression",
		)),
	}
}

fn valid_colon_feature_value(feature: &str, tokens: &[StyleMediaToken]) -> bool {
	match tokens {
		[StyleMediaToken::Identifier(value)] => {
			valid_discrete_media_feature_value(feature, value.as_str())
		}
		_ => !is_discrete_media_feature(feature) && valid_numeric_feature_value(feature, tokens),
	}
}

fn is_discrete_media_feature(feature: &str) -> bool {
	discrete_media_feature_values(feature).is_some()
}

fn valid_discrete_media_feature_value(feature: &str, value: &str) -> bool {
	discrete_media_feature_values(feature).is_some_and(|values| {
		values
			.iter()
			.any(|candidate| value.eq_ignore_ascii_case(candidate))
	})
}

fn discrete_media_feature_values(feature: &str) -> Option<&'static [&'static str]> {
	const DISCRETE_MEDIA_FEATURE_VALUES: &[(&str, &[&str])] = &[
		("any-hover", &["none", "hover"]),
		("any-pointer", &["none", "coarse", "fine"]),
		("color-gamut", &["srgb", "p3", "rec2020"]),
		("device-posture", &["continuous", "folded"]),
		(
			"display-mode",
			&[
				"fullscreen",
				"standalone",
				"minimal-ui",
				"browser",
				"picture-in-picture",
				"window-controls-overlay",
			],
		),
		("dynamic-range", &["standard", "high"]),
		("forced-colors", &["none", "active"]),
		("hover", &["none", "hover"]),
		("inverted-colors", &["none", "inverted"]),
		("light-level", &["dim", "normal", "washed"]),
		("nav-controls", &["none", "back"]),
		("overflow-block", &["none", "scroll", "paged"]),
		("overflow-inline", &["none", "scroll"]),
		("orientation", &["portrait", "landscape"]),
		("pointer", &["none", "coarse", "fine"]),
		("prefers-color-scheme", &["light", "dark"]),
		(
			"prefers-contrast",
			&["no-preference", "less", "more", "custom"],
		),
		("prefers-reduced-data", &["no-preference", "reduce"]),
		("prefers-reduced-motion", &["no-preference", "reduce"]),
		("prefers-reduced-transparency", &["no-preference", "reduce"]),
		("scan", &["interlace", "progressive"]),
		("scripting", &["none", "initial-only", "enabled"]),
		("update", &["none", "slow", "fast"]),
		("video-color-gamut", &["srgb", "p3", "rec2020"]),
		("video-dynamic-range", &["standard", "high"]),
	];
	DISCRETE_MEDIA_FEATURE_VALUES
		.iter()
		.find_map(|(candidate, values)| feature.eq_ignore_ascii_case(candidate).then_some(*values))
}

fn valid_numeric_feature_value(feature: &str, tokens: &[StyleMediaToken]) -> bool {
	valid_numeric_media_value(feature, tokens, false)
}

fn valid_numeric_range_value(feature: &str, tokens: &[StyleMediaToken]) -> bool {
	valid_numeric_media_value(feature, tokens, true)
}

fn valid_numeric_media_value(
	feature: &str,
	tokens: &[StyleMediaToken],
	allow_negative_dimension_values: bool,
) -> bool {
	match tokens {
		[StyleMediaToken::Number(number)] => valid_media_numeric_unit(feature, number),
		[
			StyleMediaToken::Punctuation(sign),
			StyleMediaToken::Number(number),
		] => {
			is_numeric_sign(sign.kind)
				&& valid_media_numeric_unit(feature, number)
				&& (allow_negative_dimension_values
					|| sign.kind != StyleMediaPunctuationKind::Minus
					|| !requires_dimension_unit(feature)
					|| is_zero_decimal(&number.value))
		}
		[
			StyleMediaToken::Number(number),
			StyleMediaToken::Punctuation(percent),
		] => number.unit.is_none() && percent.kind == StyleMediaPunctuationKind::Percent,
		[
			StyleMediaToken::Punctuation(sign),
			StyleMediaToken::Number(number),
			StyleMediaToken::Punctuation(percent),
		] => {
			is_numeric_sign(sign.kind)
				&& number.unit.is_none()
				&& percent.kind == StyleMediaPunctuationKind::Percent
		}
		[
			StyleMediaToken::Number(numerator),
			StyleMediaToken::Punctuation(slash),
			StyleMediaToken::Number(denominator),
		] => {
			numerator.unit.is_none()
				&& denominator.unit.is_none()
				&& slash.kind == StyleMediaPunctuationKind::Slash
				&& !is_zero_decimal(&denominator.value)
		}
		_ => false,
	}
}

fn valid_media_numeric_unit(feature: &str, number: &StyleMediaNumber) -> bool {
	let Some(unit) = number.unit.as_deref() else {
		return !requires_dimension_unit(feature) || is_zero_decimal(&number.value);
	};

	match normalized_dimension_feature(feature) {
		Some("width" | "height" | "device-width" | "device-height") => is_length_unit(unit),
		Some("resolution") => is_resolution_unit(unit),
		_ => is_length_unit(unit) || is_resolution_unit(unit),
	}
}

fn requires_dimension_unit(feature: &str) -> bool {
	matches!(
		normalized_dimension_feature(feature),
		Some("width" | "height" | "device-width" | "device-height" | "resolution")
	)
}

fn normalized_dimension_feature(feature: &str) -> Option<&'static str> {
	let feature = feature.to_ascii_lowercase();
	let feature = feature
		.strip_prefix("min-")
		.or_else(|| feature.strip_prefix("max-"))
		.unwrap_or(&feature);
	match feature {
		"width" => Some("width"),
		"height" => Some("height"),
		"device-width" => Some("device-width"),
		"device-height" => Some("device-height"),
		"resolution" => Some("resolution"),
		_ => None,
	}
}

fn is_length_unit(unit: &str) -> bool {
	unit_specs().iter().any(|spec| {
		spec.name.eq_ignore_ascii_case(unit)
			&& matches!(
				spec.category,
				UnitCategory::AbsoluteLength
					| UnitCategory::FontRelativeLength
					| UnitCategory::ViewportLength
					| UnitCategory::ContainerLength
			)
	})
}

fn is_resolution_unit(unit: &str) -> bool {
	["dpi", "dpcm", "dppx"]
		.iter()
		.any(|candidate| unit.eq_ignore_ascii_case(candidate))
}

fn is_zero_decimal(value: &str) -> bool {
	let mantissa = value
		.split_once(['e', 'E'])
		.map_or(value, |(mantissa, _)| mantissa);
	mantissa
		.bytes()
		.filter(u8::is_ascii_digit)
		.all(|digit| digit == b'0')
}

fn is_feature_identifier(tokens: &[StyleMediaToken]) -> bool {
	media_feature_identifier(tokens).is_some()
}

fn media_feature_identifier(tokens: &[StyleMediaToken]) -> Option<&str> {
	match tokens {
		[StyleMediaToken::Identifier(feature)] => Some(feature.as_str()),
		_ => None,
	}
}

fn is_numeric_sign(kind: StyleMediaPunctuationKind) -> bool {
	matches!(
		kind,
		StyleMediaPunctuationKind::Plus | StyleMediaPunctuationKind::Minus
	)
}

fn comparison_kind(token: &StyleMediaToken) -> Option<StyleMediaPunctuationKind> {
	let StyleMediaToken::Punctuation(punctuation) = token else {
		return None;
	};
	matches!(
		punctuation.kind,
		StyleMediaPunctuationKind::LessThan
			| StyleMediaPunctuationKind::LessThanOrEqual
			| StyleMediaPunctuationKind::GreaterThan
			| StyleMediaPunctuationKind::GreaterThanOrEqual
			| StyleMediaPunctuationKind::Equal
	)
	.then_some(punctuation.kind)
}

fn range_direction(kind: StyleMediaPunctuationKind) -> Option<bool> {
	match kind {
		StyleMediaPunctuationKind::LessThan | StyleMediaPunctuationKind::LessThanOrEqual => {
			Some(true)
		}
		StyleMediaPunctuationKind::GreaterThan | StyleMediaPunctuationKind::GreaterThanOrEqual => {
			Some(false)
		}
		_ => None,
	}
}

fn invalid_boolean_order(token: &StyleMediaToken) -> syn::Error {
	syn::Error::new(
		token.span(),
		"invalid boolean operator order in media condition",
	)
}

fn is_rust_numeric_suffix(suffix: &str) -> bool {
	matches!(
		suffix,
		"u8" | "u16"
			| "u32" | "u64"
			| "u128" | "usize"
			| "i8" | "i16"
			| "i32" | "i64"
			| "i128" | "isize"
			| "f16" | "f32"
			| "f64" | "f128"
	)
}

fn is_css_unit_suffix(suffix: &str) -> bool {
	let mut bytes = suffix.bytes();
	let Some(first) = bytes.next() else {
		return false;
	};
	first.is_ascii_alphabetic() && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_rust_control_flow_keyword(value: &str) -> bool {
	matches!(
		value,
		"return"
			| "break" | "continue"
			| "if" | "else"
			| "match" | "loop"
			| "while" | "for"
			| "let" | "async"
			| "await" | "move"
			| "yield"
	)
}

fn media_operator(value: &str) -> Option<StyleMediaOperatorKind> {
	if value.eq_ignore_ascii_case("and") {
		Some(StyleMediaOperatorKind::And)
	} else if value.eq_ignore_ascii_case("or") {
		Some(StyleMediaOperatorKind::Or)
	} else if value.eq_ignore_ascii_case("not") {
		Some(StyleMediaOperatorKind::Not)
	} else if value.eq_ignore_ascii_case("only") {
		Some(StyleMediaOperatorKind::Only)
	} else {
		None
	}
}

fn preceding_token_cannot_group(tokens: &[TokenTree], index: usize) -> bool {
	match &tokens[index] {
		TokenTree::Ident(ident) => media_operator(&unraw_ident(ident)).is_none(),
		TokenTree::Literal(_) | TokenTree::Group(_) => true,
		TokenTree::Punct(_) => false,
	}
}

fn unary_sign_at(tokens: &[TokenTree], index: usize) -> bool {
	if !matches!(tokens.get(index + 1), Some(TokenTree::Literal(_))) {
		return false;
	}
	if index == 0 {
		return true;
	}
	matches!(
		tokens.get(index - 1),
		Some(TokenTree::Punct(punct))
			if matches!(punct.as_char(), ':' | '<' | '>' | '=' | '/' | ',')
	)
}

fn punct_at(tokens: &[TokenTree], index: usize, expected: char) -> bool {
	matches!(tokens.get(index), Some(TokenTree::Punct(punct)) if punct.as_char() == expected)
}

fn span_of_tokens(tokens: &[TokenTree]) -> Option<Span> {
	Some(joined_span(tokens.first()?.span(), tokens.last()?.span()))
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
		StyleItem, StyleMediaOperatorKind, StyleMediaPunctuationKind, StyleMediaRule,
		StyleMediaToken, StyleRuleItem, StyleSelectorCombinator, StyleSelectorKind,
		StyleSimpleSelector,
	};

	fn first_media(input: proc_macro2::TokenStream) -> StyleMediaRule {
		let style = parse_style(input).unwrap();
		let StyleItem::Media(media) = style.items.into_iter().next().unwrap() else {
			panic!("expected a top-level media rule");
		};
		media
	}

	fn media_rule_with_comparison(first: char, spacing: Spacing) -> proc_macro2::TokenStream {
		let first = Punct::new(first, spacing);
		let equals = Punct::new('=', Spacing::Alone);
		quote! { @media (width #first #equals 640px) {} }
	}

	fn collect_media_operators(
		tokens: &[StyleMediaToken],
		operators: &mut Vec<StyleMediaOperatorKind>,
	) {
		for token in tokens {
			match token {
				StyleMediaToken::Operator(operator) => operators.push(operator.kind),
				StyleMediaToken::Parenthesized(group) => {
					collect_media_operators(&group.tokens, operators);
				}
				_ => {}
			}
		}
	}

	#[rstest]
	fn parses_nested_media_condition_and_preserves_body_shape() {
		// Arrange
		let input = quote! {
			.card {
				@media (max-width: 640px) {
					padding: 1rem;
					.label {}
				}
			}
		};

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		let StyleItem::Rule(rule) = &style.items[0] else {
			panic!("expected a top-level rule");
		};
		let StyleRuleItem::Media(media) = &rule.items[0] else {
			panic!("expected a nested media rule");
		};
		assert_eq!(media.condition.tokens.len(), 1);
		let StyleMediaToken::Parenthesized(group) = &media.condition.tokens[0] else {
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

		assert_eq!(media.items.len(), 2);
		let StyleRuleItem::Declaration(declaration) = &media.items[0] else {
			panic!("expected a declaration before the nested rule");
		};
		assert_eq!(declaration.name.as_str(), "padding");
		let StyleRuleItem::Rule(label) = &media.items[1] else {
			panic!("expected a nested selector inside the media rule");
		};
		let StyleSelectorKind::Relative {
			combinator: StyleSelectorCombinator::Descendant,
			selector: StyleSimpleSelector::Class(class),
		} = &label.selectors.selectors[0].kind
		else {
			panic!("expected a descendant class selector");
		};
		assert_eq!(class.as_str(), "label");
	}

	#[rstest]
	#[case("and-color")]
	#[case("or-color")]
	#[case("not-color")]
	#[case("only-color")]
	fn parses_operator_prefixed_kebab_feature_identifiers(#[case] expected: &str) {
		// Arrange
		let feature: proc_macro2::TokenStream = expected.parse().unwrap();
		let input = quote! { @media (#feature) {} };

		// Act
		let media = first_media(input);

		// Assert
		let StyleMediaToken::Parenthesized(group) = &media.condition.tokens[0] else {
			panic!("expected a parenthesized media feature");
		};
		assert_eq!(group.tokens.len(), 1);
		let StyleMediaToken::Identifier(identifier) = &group.tokens[0] else {
			panic!("expected a media feature identifier");
		};
		assert_eq!(identifier.as_str(), expected);
	}

	#[rstest]
	fn parses_top_level_media_with_boolean_operator_and_local_class_rule() {
		// Arrange
		let input = quote! {
			@media screen and (min-width: 48rem) {
				.card {}
			}
		};

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		let StyleItem::Media(media) = &style.items[0] else {
			panic!("expected a top-level media rule");
		};
		assert_eq!(media.condition.tokens.len(), 3);
		let StyleMediaToken::Identifier(media_type) = &media.condition.tokens[0] else {
			panic!("expected a media type");
		};
		assert_eq!(media_type.as_str(), "screen");
		let StyleMediaToken::Operator(operator) = &media.condition.tokens[1] else {
			panic!("expected a boolean media operator");
		};
		assert_eq!(operator.kind, StyleMediaOperatorKind::And);
		let StyleMediaToken::Parenthesized(feature) = &media.condition.tokens[2] else {
			panic!("expected a parenthesized media feature");
		};
		assert_eq!(feature.tokens.len(), 3);
		let StyleMediaToken::Number(number) = &feature.tokens[2] else {
			panic!("expected a numeric media value");
		};
		assert_eq!(number.value, "48");
		assert_eq!(number.unit.as_deref(), Some("rem"));

		assert_eq!(media.items.len(), 1);
		let StyleRuleItem::Rule(card) = &media.items[0] else {
			panic!("expected a local class rule inside the media rule");
		};
		let StyleSelectorKind::Root(StyleSimpleSelector::Class(class)) =
			&card.selectors.selectors[0].kind
		else {
			panic!("expected a root class selector");
		};
		assert_eq!(class.as_str(), "card");
	}

	#[rstest]
	#[case(
		quote! { @media screen and not (color) {} },
		vec![StyleMediaOperatorKind::And, StyleMediaOperatorKind::Not]
	)]
	#[case(
		quote! { @media only screen and not (color) {} },
		vec![
			StyleMediaOperatorKind::Only,
			StyleMediaOperatorKind::And,
			StyleMediaOperatorKind::Not,
		]
	)]
	#[case(
		quote! { @media not screen and not (color) {} },
		vec![
			StyleMediaOperatorKind::Not,
			StyleMediaOperatorKind::And,
			StyleMediaOperatorKind::Not,
		]
	)]
	fn parses_media_type_with_negated_condition_tail(
		#[case] input: proc_macro2::TokenStream,
		#[case] expected: Vec<StyleMediaOperatorKind>,
	) {
		// Arrange
		// Input and expectation are provided by the parameterized case.

		// Act
		let media = first_media(input);
		let mut operators = Vec::new();
		collect_media_operators(&media.condition.tokens, &mut operators);

		// Assert
		assert_eq!(operators, expected);
	}

	#[rstest]
	fn preserves_all_boolean_media_operators_in_source_order() {
		// Arrange
		let input = quote! {
			@media only screen and ((color) or (not (monochrome))) {}
		};

		// Act
		let style = parse_style(input).unwrap();

		// Assert
		let StyleItem::Media(media) = &style.items[0] else {
			panic!("expected a top-level media rule");
		};
		let mut operators = Vec::new();
		collect_media_operators(&media.condition.tokens, &mut operators);
		assert_eq!(
			operators,
			vec![
				StyleMediaOperatorKind::Only,
				StyleMediaOperatorKind::And,
				StyleMediaOperatorKind::Or,
				StyleMediaOperatorKind::Not,
			]
		);
	}

	#[rstest]
	fn parses_ascii_case_insensitive_media_operators() {
		// Arrange
		let input = quote! {
			@media NOT screen AND NOT (color) {}
		};

		// Act
		let media = first_media(input);
		let mut operators = Vec::new();
		collect_media_operators(&media.condition.tokens, &mut operators);

		// Assert
		assert_eq!(
			operators,
			vec![
				StyleMediaOperatorKind::Not,
				StyleMediaOperatorKind::And,
				StyleMediaOperatorKind::Not,
			]
		);
	}

	#[rstest]
	fn rejects_same_level_mixed_boolean_connectives() {
		// Arrange
		let input = quote! {
			@media ((color) and (hover) or (width > 1px)) {}
		};

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"mixed `and` and `or` media conditions must be parenthesized"
		);
	}

	#[rstest]
	#[case(quote! { @media not {} })]
	#[case(quote! { @media only {} })]
	#[case(quote! { @media and {} })]
	#[case(quote! { @media or {} })]
	fn rejects_standalone_media_query_keywords(#[case] input: proc_macro2::TokenStream) {
		// Arrange
		// Input is provided by the parameterized case.

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"invalid boolean operator order in media condition"
		);
	}

	#[rstest]
	fn parses_parenthesized_mixed_boolean_connectives() {
		// Arrange
		let input = quote! {
			@media (((color) and (hover)) or (width > 1px)) {}
		};

		// Act
		let media = first_media(input);

		// Assert
		let StyleMediaToken::Parenthesized(outer) = &media.condition.tokens[0] else {
			panic!("expected an outer boolean condition");
		};
		assert_eq!(outer.tokens.len(), 3);
		let StyleMediaToken::Operator(or) = &outer.tokens[1] else {
			panic!("expected an outer `or` connective");
		};
		assert_eq!(or.kind, StyleMediaOperatorKind::Or);
		let StyleMediaToken::Parenthesized(left) = &outer.tokens[0] else {
			panic!("expected a parenthesized left condition");
		};
		let StyleMediaToken::Operator(and) = &left.tokens[1] else {
			panic!("expected an inner `and` connective");
		};
		assert_eq!(and.kind, StyleMediaOperatorKind::And);
	}

	#[rstest]
	fn parses_media_query_lists_in_source_order() {
		// Arrange
		let input = quote! {
			@media screen and (min-width: 40rem), print and (color) {}
		};

		// Act
		let media = first_media(input);

		// Assert
		assert_eq!(media.condition.tokens.len(), 7);
		let StyleMediaToken::Identifier(screen) = &media.condition.tokens[0] else {
			panic!("expected the first media type");
		};
		assert_eq!(screen.as_str(), "screen");
		let StyleMediaToken::Punctuation(comma) = &media.condition.tokens[3] else {
			panic!("expected a media-query list separator");
		};
		assert_eq!(comma.kind, StyleMediaPunctuationKind::Comma);
		let StyleMediaToken::Identifier(print) = &media.condition.tokens[4] else {
			panic!("expected the second media type");
		};
		assert_eq!(print.as_str(), "print");
	}

	#[rstest]
	fn parses_one_and_two_sided_media_ranges() {
		// Arrange
		let input = quote! {
			@media (width < 640px), (400px < width <= 1000px), (2dppx >= resolution) {}
		};

		// Act
		let media = first_media(input);

		// Assert
		assert_eq!(media.condition.tokens.len(), 5);
		let StyleMediaToken::Parenthesized(one_sided) = &media.condition.tokens[0] else {
			panic!("expected a one-sided range");
		};
		assert_eq!(one_sided.tokens.len(), 3);
		let StyleMediaToken::Identifier(width) = &one_sided.tokens[0] else {
			panic!("expected a media feature identifier");
		};
		assert_eq!(width.as_str(), "width");
		let StyleMediaToken::Punctuation(less_than) = &one_sided.tokens[1] else {
			panic!("expected a range comparison");
		};
		assert_eq!(less_than.kind, StyleMediaPunctuationKind::LessThan);

		let StyleMediaToken::Parenthesized(two_sided) = &media.condition.tokens[2] else {
			panic!("expected a two-sided range");
		};
		assert_eq!(two_sided.tokens.len(), 5);
		let StyleMediaToken::Punctuation(inclusive) = &two_sided.tokens[3] else {
			panic!("expected an inclusive range comparison");
		};
		assert_eq!(inclusive.kind, StyleMediaPunctuationKind::LessThanOrEqual);

		let StyleMediaToken::Parenthesized(reversed) = &media.condition.tokens[4] else {
			panic!("expected a reversed one-sided range");
		};
		let StyleMediaToken::Identifier(resolution) = &reversed.tokens[2] else {
			panic!("expected a reversed media feature identifier");
		};
		assert_eq!(resolution.as_str(), "resolution");
	}

	#[rstest]
	fn parses_ratio_media_feature_values() {
		// Arrange
		let input = quote! { @media (aspect-ratio: 16/9) {} };

		// Act
		let media = first_media(input);

		// Assert
		let StyleMediaToken::Parenthesized(feature) = &media.condition.tokens[0] else {
			panic!("expected a parenthesized media feature");
		};
		assert_eq!(feature.tokens.len(), 5);
		let StyleMediaToken::Punctuation(slash) = &feature.tokens[3] else {
			panic!("expected an aspect-ratio separator");
		};
		assert_eq!(slash.kind, StyleMediaPunctuationKind::Slash);
	}

	#[rstest]
	#[case(quote! { @media (orientation: landscape) {} })]
	#[case(quote! { @media (prefers-color-scheme: dark) {} })]
	fn accepts_identifier_values_for_discrete_media_features(
		#[case] input: proc_macro2::TokenStream,
	) {
		// Act
		let media = first_media(input);

		// Assert
		assert_eq!(media.condition.tokens.len(), 1);
	}

	#[rstest]
	#[case(quote! { @media (orientation: sideways) {} })]
	#[case(quote! { @media (hover: banana) {} })]
	#[case(quote! { @media (orientation: 1px) {} })]
	fn rejects_invalid_values_for_discrete_media_features(#[case] input: proc_macro2::TokenStream) {
		// Act
		let error = parse_style(input).expect_err("discrete media features require defined values");

		// Assert
		assert_eq!(error.to_string(), "invalid media feature expression");
	}

	#[rstest]
	#[case(
		quote! { @media (width: +1px) {} },
		2,
		StyleMediaPunctuationKind::Plus
	)]
	#[case(
		quote! { @media (-1px < width) {} },
		0,
		StyleMediaPunctuationKind::Minus
	)]
	#[case(
		quote! { @media (width >= +1px) {} },
		2,
		StyleMediaPunctuationKind::Plus
	)]
	fn parses_signed_numeric_media_values(
		#[case] input: proc_macro2::TokenStream,
		#[case] sign_index: usize,
		#[case] expected: StyleMediaPunctuationKind,
	) {
		// Arrange
		// Input is provided by the parameterized case.

		// Act
		let media = first_media(input);

		// Assert
		assert_eq!(media.condition.tokens.len(), 1);
		let StyleMediaToken::Parenthesized(feature) = &media.condition.tokens[0] else {
			panic!("expected a parenthesized media feature");
		};
		let StyleMediaToken::Punctuation(sign) = &feature.tokens[sign_index] else {
			panic!("expected a numeric sign");
		};
		assert_eq!(sign.kind, expected);
	}

	#[rstest]
	fn rejects_decimal_points_without_a_fraction_digit() {
		// Arrange
		let input = quote! { @media (width: 1.) {} };

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"media numbers must use plain CSS decimal syntax"
		);
	}

	#[rstest]
	#[case('<', StyleMediaPunctuationKind::LessThanOrEqual)]
	#[case('>', StyleMediaPunctuationKind::GreaterThanOrEqual)]
	fn parses_joint_inclusive_range_comparisons(
		#[case] first: char,
		#[case] expected: StyleMediaPunctuationKind,
	) {
		// Arrange
		let input = media_rule_with_comparison(first, Spacing::Joint);

		// Act
		let media = first_media(input);

		// Assert
		let StyleMediaToken::Parenthesized(feature) = &media.condition.tokens[0] else {
			panic!("expected a parenthesized range feature");
		};
		let StyleMediaToken::Punctuation(comparison) = &feature.tokens[1] else {
			panic!("expected an inclusive comparison");
		};
		assert_eq!(comparison.kind, expected);
	}

	#[rstest]
	#[case('<')]
	#[case('>')]
	fn rejects_separated_inclusive_range_comparisons(#[case] first: char) {
		// Arrange
		let input = media_rule_with_comparison(first, Spacing::Alone);

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"media comparison operators cannot contain whitespace"
		);
	}

	#[rstest]
	#[case(
		quote! { @media (1 < 2) {} },
		"media range expressions must contain a feature identifier"
	)]
	#[case(
		quote! { @media (400px < width > 1000px) {} },
		"two-sided media ranges must use comparisons in the same direction"
	)]
	#[case(
		quote! { @media screen and or (color) {} },
		"invalid boolean operator order in media condition"
	)]
	fn rejects_malformed_static_media_grammar(
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
	#[case(quote! { @media (aspect-ratio: 16/0) {} })]
	#[case(quote! { @media (aspect-ratio: 16/0.0) {} })]
	fn rejects_media_ratios_with_zero_denominators(#[case] input: proc_macro2::TokenStream) {
		// Arrange
		// Input is provided by the parameterized case.

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(error.to_string(), "invalid media feature expression");
	}

	#[rstest]
	#[case(quote! { @media (width: 10foo) {} })]
	#[case(quote! { @media (width: 1fr) {} })]
	#[case(quote! { @media (width: 10deg) {} })]
	fn rejects_non_media_numeric_units(#[case] input: proc_macro2::TokenStream) {
		// Arrange
		// Input is provided by the parameterized case.

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(error.to_string(), "invalid media feature expression");
	}

	#[rstest]
	#[case(quote! { @media (width: 640) {} })]
	#[case(quote! { @media (resolution: 2) {} })]
	#[case(quote! { @media (width: -640) {} })]
	#[case(quote! { @media (resolution: -2) {} })]
	fn rejects_unitless_nonzero_values_for_dimensioned_media_features(
		#[case] input: proc_macro2::TokenStream,
	) {
		// Arrange
		// Input is provided by the parameterized case.

		// Act
		let error = parse_style(input).expect_err("dimensioned media features require units");

		// Assert
		assert_eq!(error.to_string(), "invalid media feature expression");
	}

	#[rstest]
	#[case(quote! { @media (width: -1px) {} })]
	#[case(quote! { @media (resolution: -2dppx) {} })]
	#[case(quote! { @media (min-height: -1rem) {} })]
	fn rejects_negative_values_for_nonnegative_dimensioned_media_features(
		#[case] input: proc_macro2::TokenStream,
	) {
		// Act
		let error = parse_style(input).expect_err("dimensioned media features cannot be negative");

		// Assert
		assert_eq!(error.to_string(), "invalid media feature expression");
	}

	#[rstest]
	#[case(quote! { @media (WIDTH: 640PX) {} })]
	#[case(quote! { @media (RESOLUTION: 2DPI) {} })]
	fn accepts_case_insensitive_dimensioned_media_features(
		#[case] input: proc_macro2::TokenStream,
	) {
		// Act
		let parsed = parse_style(input);

		// Assert
		assert!(parsed.is_ok());
	}

	#[rstest]
	#[case(quote! { @media (width: auto) {} })]
	#[case(quote! { @media (resolution: screen) {} })]
	fn rejects_identifier_values_for_numeric_media_features(
		#[case] input: proc_macro2::TokenStream,
	) {
		// Act
		let error = parse_style(input).expect_err("numeric media features must reject identifiers");

		// Assert
		assert_eq!(error.to_string(), "invalid media feature expression");
	}

	#[rstest]
	#[case(
		quote! { @media (width < 0x280) {} },
		"media numbers must use plain CSS decimal syntax"
	)]
	#[case(
		quote! { @media (width < 1_000px) {} },
		"media numbers must use plain CSS decimal syntax"
	)]
	#[case(
		quote! { @media (width < 640usize) {} },
		"media numeric suffixes must be CSS units"
	)]
	#[case(
		quote! { @media (width < 640u32) {} },
		"media numeric suffixes must be CSS units"
	)]
	fn rejects_rust_numeric_literal_forms(
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
	#[case(quote! { @media return {} })]
	#[case(quote! { @media break {} })]
	#[case(quote! { @media continue {} })]
	fn rejects_rust_control_flow_in_media_conditions(#[case] input: proc_macro2::TokenStream) {
		// Arrange
		// Input is provided by the parameterized case.

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"Rust control-flow keywords are not allowed in media conditions"
		);
	}

	#[rstest]
	#[case("globals")]
	#[case("vars")]
	fn rejects_style_namespaces_in_media_conditions(#[case] namespace: &str) {
		// Arrange
		let namespace: proc_macro2::TokenStream = namespace.parse().unwrap();
		let input = quote! { @media (max-width: #namespace.breakpoint) {} };

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"`globals.*` and `vars.*` references are not allowed in media conditions"
		);
	}

	#[rstest]
	fn rejects_macros_in_media_conditions() {
		// Arrange
		let input = quote! { @media cfg!(feature = "compact") {} };

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"macros are not allowed in media conditions"
		);
	}

	#[rstest]
	fn normalizes_raw_rust_identifiers_in_media_conditions() {
		// Arrange
		let input = quote! { @media r#screen and (color) {} };

		// Act
		let media = first_media(input);

		// Assert
		let StyleMediaToken::Identifier(media_type) = &media.condition.tokens[0] else {
			panic!("expected a media type identifier");
		};
		assert_eq!(media_type.as_str(), "screen");
	}

	#[rstest]
	#[case(quote! { @media (width + 1px) {} })]
	#[case(quote! { @media (width - 1px) {} })]
	#[case(quote! { @media (width && compact) {} })]
	#[case(quote! { @media (width == 640px) {} })]
	#[case(quote! { @media breakpoint(640px) {} })]
	fn rejects_rust_operators_in_media_conditions(#[case] input: proc_macro2::TokenStream) {
		// Arrange
		// Input is provided by the parameterized case.

		// Act
		let error = parse_style(input).unwrap_err();

		// Assert
		assert_eq!(
			error.to_string(),
			"Rust expression operators are not allowed in media conditions"
		);
	}
}
