//! Attribute macro implementation for `#[app_config(...)]`

use crate::crate_paths::get_reinhardt_crate;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, ItemStruct, Result};

pub(crate) fn app_config_attribute_impl(
	args: TokenStream,
	mut input: ItemStruct,
) -> Result<TokenStream> {
	let reinhardt = get_reinhardt_crate();

	// Check if #[derive(AppConfig)] already exists - this is an error
	let has_derive_app_config = input.attrs.iter().any(|attr| {
		if attr.path().is_ident("derive")
			&& let syn::Meta::List(meta_list) = &attr.meta
		{
			return meta_list.tokens.to_string().contains("AppConfig");
		}
		false
	});

	if has_derive_app_config {
		return Err(syn::Error::new_spanned(
			&input.ident,
			"#[derive(AppConfig)] must not be used with #[app_config(...)]. \
			 The #[app_config(...)] attribute automatically derives AppConfig.",
		));
	}

	// Create #[app_config_internal(...)] helper attribute with the arguments
	// Using app_config_internal to avoid name collision with the attribute macro
	let config_attr: Attribute = if args.is_empty() {
		syn::parse_quote! { #[app_config_internal] }
	} else {
		syn::parse_quote! { #[app_config_internal(#args)] }
	};

	// Build derive attribute with AppConfig derive macro
	// Use reinhardt::macros::AppConfig for hierarchical imports
	let app_config_path = quote!(#reinhardt::macros::AppConfig);

	// Find existing derive attribute to merge with, or create a new one
	let existing_derive_idx = input.attrs.iter().position(|attr| {
		attr.path().is_ident("derive") && matches!(&attr.meta, syn::Meta::List(_))
	});

	if let Some(idx) = existing_derive_idx {
		// Merge AppConfig into the existing derive attribute
		if let syn::Meta::List(ref meta_list) = input.attrs[idx].meta {
			let existing_tokens = &meta_list.tokens;
			let new_derive_attr: Attribute =
				syn::parse_quote! { #[derive(#app_config_path, #existing_tokens)] };
			input.attrs[idx] = new_derive_attr;
		}
	} else {
		// No existing derive attribute, create a new one
		let derive_attr: Attribute = syn::parse_quote! { #[derive(#app_config_path)] };
		// Insert at the beginning to ensure AppConfig is processed first
		input.attrs.insert(0, derive_attr);
	}

	// Add the helper attribute AFTER the derive
	// Position depends on whether we merged into existing derive or created new one
	let config_insert_pos = if let Some(idx) = existing_derive_idx {
		// Merged into existing derive, insert after it
		idx + 1
	} else {
		// Created new derive at position 0, insert at position 1
		1
	};
	input.attrs.insert(config_insert_pos, config_attr);

	Ok(quote! { #input })
}
