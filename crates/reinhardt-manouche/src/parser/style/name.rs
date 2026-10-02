//! Parser implementation for canonical CSS names.

use proc_macro2::Span;
use syn::{
	Ident, Token,
	ext::IdentExt,
	parse::{Parse, ParseStream},
	spanned::Spanned,
};

use super::unraw_ident;
use crate::core::CssName;

impl Parse for CssName {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		if input.is_empty() {
			return Err(syn::Error::new(input.span(), "expected a CSS name"));
		}

		if input.peek(Token![-]) {
			let leading_hyphen: Token![-] = input.parse()?;
			let message = if input.peek(Token![-]) {
				"CSS custom property names are not supported"
			} else {
				"vendor-prefixed CSS names are not supported"
			};
			return Err(syn::Error::new(leading_hyphen.span(), message));
		}

		let first = Ident::parse_any(input)?;
		let first_span = first.span();
		let mut last_span = first_span;
		let mut value = canonical_segment(&first)?;

		while input.peek(Token![-]) {
			let hyphen: Token![-] = input.parse()?;
			if input.peek(Token![-]) {
				return Err(syn::Error::new(
					input.span(),
					"CSS names cannot contain consecutive hyphens",
				));
			}
			if !input.peek(Ident::peek_any) {
				return Err(syn::Error::new(
					hyphen.span(),
					"expected a CSS name segment after `-`",
				));
			}

			let segment = Ident::parse_any(input)?;
			value.push('-');
			value.push_str(&canonical_segment(&segment)?);
			last_span = segment.span();
		}

		Ok(Self {
			value,
			span: joined_span(first_span, last_span),
		})
	}
}

fn canonical_segment(ident: &Ident) -> syn::Result<String> {
	let segment = unraw_ident(ident);
	if !segment.bytes().all(|byte| byte.is_ascii_lowercase()) {
		return Err(syn::Error::new(
			ident.span(),
			"CSS names must use kebab-case",
		));
	}
	Ok(segment)
}

fn joined_span(first: Span, last: Span) -> Span {
	first.join(last).unwrap_or(first)
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::CssName;

	#[rstest]
	fn parses_canonical_kebab_name() {
		// Act
		let name = syn::parse_str::<CssName>("border-color").unwrap();

		// Assert
		assert_eq!(name.as_str(), "border-color");
	}

	#[rstest]
	#[case("", "expected a CSS name")]
	#[case("border--color", "CSS names cannot contain consecutive hyphens")]
	#[case("-webkit-color", "vendor-prefixed CSS names are not supported")]
	#[case("--theme-color", "CSS custom property names are not supported")]
	#[case("border_color", "CSS names must use kebab-case")]
	#[case("Border", "CSS names must use kebab-case")]
	#[case("borderColor", "CSS names must use kebab-case")]
	#[case("café", "CSS names must use kebab-case")]
	#[case("color2", "CSS names must use kebab-case")]
	#[case("h1-color", "CSS names must use kebab-case")]
	fn rejects_unsupported_css_names(#[case] input: &str, #[case] expected: &str) {
		// Act
		let error = syn::parse_str::<CssName>(input).unwrap_err();

		// Assert
		assert_eq!(error.to_string(), expected);
	}
}
