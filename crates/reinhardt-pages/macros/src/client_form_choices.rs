//! `ClientFormChoices` derive implementation.

use std::collections::BTreeSet;

use proc_macro::TokenStream;
use quote::quote;
use syn::meta::ParseNestedMeta;
use syn::{Data, DeriveInput, Fields, LitStr, Token, parse_macro_input};

use crate::crate_paths::get_reinhardt_pages_crate;

/// Derives client-form choice metadata for fieldless enums.
pub(crate) fn derive_client_form_choices_impl(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	match expand_client_form_choices(input) {
		Ok(tokens) => tokens.into(),
		Err(error) => error.to_compile_error().into(),
	}
}

fn expand_client_form_choices(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
	let enum_ident = input.ident;
	let generics = input.generics;
	let attrs = input.attrs;
	let data = input.data;
	if !generics.params.is_empty() {
		return Err(syn::Error::new_spanned(
			generics,
			"ClientFormChoices does not support generic enums",
		));
	}

	let Data::Enum(data_enum) = data else {
		return Err(syn::Error::new_spanned(
			enum_ident,
			"ClientFormChoices can only be derived for enums",
		));
	};

	let pages_crate = get_reinhardt_pages_crate();
	let rename_rule = serde_rename_all(&attrs)?;
	let mut choice_values = Vec::new();
	let mut default_variant = None;
	let mut has_skipped_variant = false;
	let mut seen_serialized_values = BTreeSet::new();

	for variant in data_enum.variants {
		let variant_options = serde_variant_options(&variant.attrs)?;
		let variant_ident = variant.ident.clone();
		if variant_options.skip {
			has_skipped_variant = true;
		}
		if variant_options.skip && variant_options.default {
			return Err(syn::Error::new_spanned(
				variant,
				"ClientFormChoices default variant cannot be skipped by serde",
			));
		}
		if variant_options.default {
			if default_variant.replace(variant_ident.clone()).is_some() {
				return Err(syn::Error::new_spanned(
					variant,
					"ClientFormChoices supports only one default variant",
				));
			}
		}
		if variant_options.skip {
			continue;
		}
		if !matches!(variant.fields, Fields::Unit) {
			return Err(syn::Error::new_spanned(
				variant,
				"ClientFormChoices supports fieldless enum variants only",
			));
		}

		let serialized = variant_options
			.rename
			.unwrap_or_else(|| apply_rename_rule(&variant_ident.to_string(), rename_rule));
		if !seen_serialized_values.insert(serialized.clone()) {
			return Err(syn::Error::new_spanned(
				&variant_ident,
				format!("duplicate ClientFormChoices serialized value `{serialized}`"),
			));
		}
		choice_values.push(quote! {
			#pages_crate::ClientFormChoice {
				value: #enum_ident::#variant_ident,
				serialized_value: #serialized,
				label: #serialized,
			}
		});
	}

	if has_skipped_variant && default_variant.is_none() {
		return Err(syn::Error::new_spanned(
			&enum_ident,
			"ClientFormChoices enums with serde-skipped variants must mark a non-skipped #[default] variant",
		));
	}

	let default_expr = if let Some(default_variant) = default_variant {
		quote! { #enum_ident::#default_variant }
	} else {
		quote! { ::core::default::Default::default() }
	};

	Ok(quote! {
		impl #pages_crate::ClientFormChoiceSource for #enum_ident {
			fn client_form_choices() -> &'static [#pages_crate::ClientFormChoice<Self>] {
				static CHOICES: &[#pages_crate::ClientFormChoice<#enum_ident>] = &[
					#(#choice_values),*
				];
				CHOICES
			}

			fn client_form_default() -> Self {
				#default_expr
			}
		}
	})
}

#[derive(Clone, Copy)]
enum RenameRule {
	Verbatim,
	SnakeCase,
	KebabCase,
	CamelCase,
}

fn serde_rename_all(attrs: &[syn::Attribute]) -> syn::Result<RenameRule> {
	let mut rename_rule = RenameRule::Verbatim;
	for attr in attrs {
		if !attr.path().is_ident("serde") {
			continue;
		}
		attr.parse_nested_meta(|meta| {
			if meta.path.is_ident("rename_all") {
				if meta.input.peek(Token![=]) {
					let value = meta.value()?.parse::<LitStr>()?;
					rename_rule = rename_rule_from_value(&value)?;
				} else {
					let mut serialize_rule = None;
					meta.parse_nested_meta(|rename_meta| {
						if rename_meta.path.is_ident("serialize") {
							let value = rename_meta.value()?.parse::<LitStr>()?;
							serialize_rule = Some(rename_rule_from_value(&value)?);
						} else if rename_meta.path.is_ident("deserialize") {
							let _value = rename_meta.value()?.parse::<LitStr>()?;
						} else {
							return Err(rename_meta.error(
								"unsupported serde rename_all option for ClientFormChoices",
							));
						}
						Ok(())
					})?;
					if let Some(rule) = serialize_rule {
						rename_rule = rule;
					}
				}
			} else if meta.path.is_ident("tag")
				|| meta.path.is_ident("content")
				|| meta.path.is_ident("untagged")
			{
				return Err(meta.error(
					"ClientFormChoices requires externally tagged string enum representation",
				));
			} else {
				consume_ignored_serde_meta(meta)?;
			}
			Ok(())
		})?;
	}
	Ok(rename_rule)
}

fn consume_ignored_serde_meta(meta: ParseNestedMeta<'_>) -> syn::Result<()> {
	if meta.input.peek(Token![=]) {
		let _value = meta.value()?.parse::<syn::Expr>()?;
	} else if meta.input.peek(syn::token::Paren) {
		meta.parse_nested_meta(consume_ignored_serde_meta)?;
	}
	Ok(())
}

fn rename_rule_from_value(value: &LitStr) -> syn::Result<RenameRule> {
	match value.value().as_str() {
		"snake_case" => Ok(RenameRule::SnakeCase),
		"kebab-case" => Ok(RenameRule::KebabCase),
		"camelCase" => Ok(RenameRule::CamelCase),
		"PascalCase" | "SCREAMING_SNAKE_CASE" | "lowercase" | "UPPERCASE" => Err(syn::Error::new(
			value.span(),
			"ClientFormChoices supports snake_case, kebab-case, and camelCase rename_all rules",
		)),
		_ => Err(syn::Error::new(
			value.span(),
			"unsupported serde rename_all rule for ClientFormChoices",
		)),
	}
}

struct SerdeVariantOptions {
	rename: Option<String>,
	skip: bool,
	default: bool,
}

fn serde_variant_options(attrs: &[syn::Attribute]) -> syn::Result<SerdeVariantOptions> {
	let mut options = SerdeVariantOptions {
		rename: None,
		skip: false,
		default: false,
	};
	for attr in attrs {
		if attr.path().is_ident("default") {
			options.default = true;
			continue;
		}
		if !attr.path().is_ident("serde") {
			continue;
		}
		attr.parse_nested_meta(|meta| {
			if meta.path.is_ident("rename") {
				if meta.input.peek(Token![=]) {
					let value = meta.value()?.parse::<LitStr>()?;
					options.rename = Some(value.value());
				} else {
					let mut serialize_rename = None;
					meta.parse_nested_meta(|rename_meta| {
						if rename_meta.path.is_ident("serialize") {
							let value = rename_meta.value()?.parse::<LitStr>()?;
							serialize_rename = Some(value.value());
						} else if rename_meta.path.is_ident("deserialize") {
							let _value = rename_meta.value()?.parse::<LitStr>()?;
						} else {
							return Err(rename_meta
								.error("unsupported serde rename option for ClientFormChoices"));
						}
						Ok(())
					})?;
					if let Some(value) = serialize_rename {
						options.rename = Some(value);
					}
				}
			} else if meta.path.is_ident("skip")
				|| meta.path.is_ident("skip_serializing")
				|| meta.path.is_ident("skip_deserializing")
			{
				options.skip = true;
			} else if meta.path.is_ident("alias")
				|| meta.path.is_ident("other")
				|| meta.path.is_ident("borrow")
				|| meta.path.is_ident("deserialize_with")
			{
				consume_ignored_serde_variant_option(meta)?;
			} else {
				return Err(meta.error("unsupported serde option for ClientFormChoices variant"));
			}
			Ok(())
		})?;
	}
	Ok(options)
}

fn consume_ignored_serde_variant_option(meta: ParseNestedMeta<'_>) -> syn::Result<()> {
	if meta.input.peek(Token![=]) {
		let _value = meta.value()?.parse::<syn::Expr>()?;
	} else if meta.input.peek(syn::token::Paren) {
		meta.parse_nested_meta(consume_ignored_serde_variant_option)?;
	}
	Ok(())
}

fn apply_rename_rule(name: &str, rename_rule: RenameRule) -> String {
	match rename_rule {
		RenameRule::Verbatim => name.to_string(),
		RenameRule::SnakeCase => serde_snake_case_variant(name),
		RenameRule::KebabCase => serde_snake_case_variant(name).replace('_', "-"),
		RenameRule::CamelCase => serde_camel_case_variant(name),
	}
}

fn serde_snake_case_variant(name: &str) -> String {
	let mut snake = String::new();
	for (index, ch) in name.char_indices() {
		if index > 0 && ch.is_uppercase() {
			snake.push('_');
		}
		snake.push(ch.to_ascii_lowercase());
	}
	snake
}

fn serde_camel_case_variant(name: &str) -> String {
	let Some((_, first)) = name.char_indices().next() else {
		return String::new();
	};
	let next_index = first.len_utf8();
	let mut camel = first.to_ascii_lowercase().to_string();
	camel.push_str(&name[next_index..]);
	camel
}
