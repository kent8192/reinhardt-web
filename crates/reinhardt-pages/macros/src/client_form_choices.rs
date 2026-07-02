//! `ClientFormChoices` derive implementation.

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, LitStr, parse_macro_input};

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

	for variant in data_enum.variants {
		if !matches!(variant.fields, Fields::Unit) {
			return Err(syn::Error::new_spanned(
				variant,
				"ClientFormChoices supports fieldless enum variants only",
			));
		}

		let variant_ident = variant.ident;
		let serialized = serde_variant_rename(&variant.attrs)?
			.unwrap_or_else(|| apply_rename_rule(&variant_ident.to_string(), rename_rule));
		choice_values.push(quote! {
			#pages_crate::ClientFormChoice {
				value: #enum_ident::#variant_ident,
				serialized_value: #serialized,
				label: #serialized,
			}
		});
	}

	Ok(quote! {
		impl #pages_crate::ClientFormChoiceSource for #enum_ident {
			fn client_form_choices() -> &'static [#pages_crate::ClientFormChoice<Self>] {
				static CHOICES: &[#pages_crate::ClientFormChoice<#enum_ident>] = &[
					#(#choice_values),*
				];
				CHOICES
			}

			fn client_form_default() -> Self {
				::core::default::Default::default()
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
				let value = meta.value()?.parse::<LitStr>()?;
				rename_rule = match value.value().as_str() {
					"snake_case" => RenameRule::SnakeCase,
					"kebab-case" => RenameRule::KebabCase,
					"camelCase" => RenameRule::CamelCase,
					"PascalCase" | "SCREAMING_SNAKE_CASE" | "lowercase" | "UPPERCASE" => {
						return Err(meta.error(
							"ClientFormChoices supports snake_case, kebab-case, and camelCase rename_all rules",
						));
					}
					_ => {
						return Err(
							meta.error("unsupported serde rename_all rule for ClientFormChoices")
						);
					}
				};
			}
			Ok(())
		})?;
	}
	Ok(rename_rule)
}

fn serde_variant_rename(attrs: &[syn::Attribute]) -> syn::Result<Option<String>> {
	let mut renamed = None;
	for attr in attrs {
		if !attr.path().is_ident("serde") {
			continue;
		}
		attr.parse_nested_meta(|meta| {
			if meta.path.is_ident("rename") {
				let value = meta.value()?.parse::<LitStr>()?;
				renamed = Some(value.value());
			}
			Ok(())
		})?;
	}
	Ok(renamed)
}

fn apply_rename_rule(name: &str, rename_rule: RenameRule) -> String {
	match rename_rule {
		RenameRule::Verbatim => name.to_string(),
		RenameRule::SnakeCase => name.to_case(Case::Snake),
		RenameRule::KebabCase => name.to_case(Case::Kebab),
		RenameRule::CamelCase => name.to_case(Case::Camel),
	}
}
