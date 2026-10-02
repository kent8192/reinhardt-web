//! Parser for the independent named ClientForm view mode.

use crate::core::client_form_view::{
	ClientFormViewClause, ClientFormViewMacro, FieldCustomization, PresentationProperty,
};
use proc_macro2::{Span, TokenStream, TokenTree};
use std::collections::BTreeMap;
use syn::{
	Expr, Ident, Lit, Token, braced,
	ext::IdentExt,
	parse::{Parse, ParseStream},
};

/// Detects the explicit first-property discriminator without parsing standalone forms.
pub fn is_client_form_view(input: &TokenStream) -> bool {
	let mut tokens = input.clone().into_iter();
	matches!(tokens.next(), Some(TokenTree::Ident(key)) if key == "client_form")
		&& matches!(tokens.next(), Some(TokenTree::Punct(colon)) if colon.as_char() == ':')
}

/// Parses a complete named ClientForm view.
pub fn parse_client_form_view(input: TokenStream) -> syn::Result<ClientFormViewMacro> {
	syn::parse2(input)
}

impl Parse for ClientFormViewMacro {
	fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
		let first = input.call(Ident::parse_any)?;
		if first != "client_form" {
			return Err(syn::Error::new(
				first.span(),
				"`client_form` must be the first property",
			));
		}
		input.parse::<Token![:]>()?;
		let companion = input.parse()?;
		let mut seen = BTreeMap::from([(String::from("client_form"), first.span())]);
		let mut clauses = Vec::new();
		comma_or_end(input)?;
		while !input.is_empty() {
			let key = input.call(Ident::parse_any)?;
			unique(&mut seen, &key, "property")?;
			input.parse::<Token![:]>()?;
			let clause = match key.to_string().as_str() {
				"mutation" => ClientFormViewClause::Mutation(input.parse()?),
				"id" => {
					let value: Expr = input.parse()?;
					if let Expr::Lit(literal) = &value
						&& let Lit::Str(id) = &literal.lit
						&& (id.value().is_empty()
							|| id.value().chars().any(|ch| ch.is_ascii_whitespace()))
					{
						return Err(syn::Error::new(
							id.span(),
							"`id` must be nonempty and contain no ASCII whitespace",
						));
					}
					ClientFormViewClause::Id(value)
				}
				"styling" => ClientFormViewClause::Styling(properties(
					input,
					"styling",
					&[
						"class",
						"field_class",
						"input_class",
						"label_class",
						"help_class",
						"error_class",
						"summary_class",
					],
				)?),
				"customize" => ClientFormViewClause::Customize(customizations(input)?),
				"submit" => ClientFormViewClause::Submit(properties(
					input,
					"submit",
					&["label", "pending_label", "class"],
				)?),
				"summary" => {
					ClientFormViewClause::Summary(properties(input, "summary", &["label"])?)
				}
				_ => {
					return Err(syn::Error::new(
						key.span(),
						format!("`{key}` is not allowed in a named ClientForm view"),
					));
				}
			};
			clauses.push(clause);
			comma_or_end(input)?;
		}
		if !seen.contains_key("mutation") {
			return Err(syn::Error::new(first.span(), "missing `mutation` property"));
		}
		Ok(Self { companion, clauses })
	}
}

fn comma_or_end(input: ParseStream<'_>) -> syn::Result<()> {
	if !input.is_empty() {
		input.parse::<Token![,]>()?;
	}
	Ok(())
}

fn unique(seen: &mut BTreeMap<String, Span>, key: &Ident, kind: &str) -> syn::Result<()> {
	let name = key.to_string().trim_start_matches("r#").to_owned();
	if seen.insert(name.clone(), key.span()).is_some() {
		return Err(syn::Error::new(
			key.span(),
			format!("duplicate `{name}` {kind}"),
		));
	}
	Ok(())
}

fn properties(
	input: ParseStream<'_>,
	context: &str,
	allowed: &[&str],
) -> syn::Result<Vec<PresentationProperty>> {
	let content;
	braced!(content in input);
	let mut properties = Vec::new();
	let mut seen = BTreeMap::new();
	while !content.is_empty() {
		let name = content.call(Ident::parse_any)?;
		unique(&mut seen, &name, "property")?;
		if !allowed.contains(&name.to_string().as_str()) {
			return Err(syn::Error::new(
				name.span(),
				format!("unknown {context} presentation property `{name}`"),
			));
		}
		content.parse::<Token![:]>()?;
		let value: Expr = content.parse()?;
		if name == "widget" {
			let supported = [
				"TextInput",
				"Textarea",
				"EmailInput",
				"UrlInput",
				"PasswordInput",
				"NumberInput",
				"CheckboxInput",
				"Select",
			];
			let valid = matches!(&value, Expr::Path(path)
                if path.qself.is_none() && path.path.leading_colon.is_none()
                && path.path.get_ident().is_some_and(|widget| supported.contains(&widget.to_string().as_str())));
			if !valid {
				return Err(syn::Error::new_spanned(
					value,
					"expected a supported widget identifier",
				));
			}
		}
		properties.push(PresentationProperty { name, value });
		comma_or_end(&content)?;
	}
	Ok(properties)
}

fn customizations(input: ParseStream<'_>) -> syn::Result<Vec<FieldCustomization>> {
	let content;
	braced!(content in input);
	let mut fields = Vec::new();
	let mut seen = BTreeMap::new();
	while !content.is_empty() {
		let field = content.call(Ident::parse_any)?;
		unique(&mut seen, &field, "field customization")?;
		content.parse::<Token![:]>()?;
		let properties = properties(
			&content,
			"field",
			&[
				"widget",
				"label",
				"aria_label",
				"aria_describedby",
				"help_text",
				"placeholder",
				"autocomplete",
				"class",
				"wrapper_class",
				"label_class",
				"help_class",
				"error_class",
				"empty_label",
				"true_label",
				"false_label",
			],
		)?;
		fields.push(FieldCustomization { field, properties });
		comma_or_end(&content)?;
	}
	Ok(fields)
}
