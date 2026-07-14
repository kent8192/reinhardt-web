//! Attribute macro implementation for `#[dto]`
//!
//! Emits shared `Validate` derives for DTOs used by native/server and wasm/client
//! builds while normalizing legacy native-only `Validate` derives. See the
//! public-facing rustdoc on `crate::dto` in `lib.rs` for the user-facing contract.

use crate::crate_paths::get_reinhardt_crate;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
	Attribute, Data, DeriveInput, Fields, Meta, Path, Result, Token, parse::Parser, parse_quote,
	punctuated::Punctuated,
};

pub(crate) fn dto_impl(args: TokenStream, mut input: DeriveInput) -> Result<TokenStream> {
	if !args.is_empty() {
		return Err(syn::Error::new_spanned(
			args,
			"#[dto] does not accept arguments in this version",
		));
	}

	let reinhardt = get_reinhardt_crate();

	match &input.data {
		Data::Struct(struct_data) => match &struct_data.fields {
			Fields::Named(_) => {}
			Fields::Unnamed(_) | Fields::Unit => {
				return Err(syn::Error::new_spanned(
					&input.ident,
					"#[dto] requires a struct with named fields",
				));
			}
		},
		Data::Enum(_) | Data::Union(_) => {
			return Err(syn::Error::new_spanned(
				&input.ident,
				"#[dto] can only be applied to structs",
			));
		}
	}

	let has_unconditional_validate = has_unconditional_derive(&input.attrs, "Validate")?;
	remove_native_validate_derives(&mut input.attrs, "Validate")?;
	if !has_unconditional_validate {
		let new_attr: Attribute = parse_quote!(#[derive(#reinhardt::Validate)]);
		input.attrs.push(new_attr);
	}

	Ok(quote! {
		#input
	})
}

fn has_unconditional_derive(attrs: &[Attribute], trait_name: &str) -> Result<bool> {
	for attr in attrs {
		if !attr.path().is_ident("derive") {
			continue;
		}
		let Meta::List(list) = &attr.meta else {
			continue;
		};
		let derives =
			Punctuated::<Path, Token![,]>::parse_terminated.parse2(list.tokens.clone())?;
		if derives
			.iter()
			.any(|p| p.segments.last().is_some_and(|seg| seg.ident == trait_name))
		{
			return Ok(true);
		}
	}
	Ok(false)
}

fn remove_native_validate_derives(attrs: &mut Vec<Attribute>, trait_name: &str) -> Result<()> {
	let mut normalized = Vec::with_capacity(attrs.len());

	for attr in attrs.drain(..) {
		if !attr.path().is_ident("cfg_attr") {
			normalized.push(attr);
			continue;
		}
		let Meta::List(list) = &attr.meta else {
			normalized.push(attr);
			continue;
		};
		let nested = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(list.tokens.clone())?;
		let mut iter = nested.into_iter();
		let Some(first) = iter.next() else {
			normalized.push(attr);
			continue;
		};
		if !matches!(&first, Meta::Path(p) if p.is_ident("native")) {
			normalized.push(attr);
			continue;
		}

		let mut rebuilt = Punctuated::<Meta, Token![,]>::new();
		rebuilt.push(first);
		for inner in iter {
			let Meta::List(inner_list) = inner else {
				rebuilt.push(inner);
				continue;
			};
			if !inner_list.path.is_ident("derive") {
				rebuilt.push(Meta::List(inner_list));
				continue;
			}

			let derives = Punctuated::<Path, Token![,]>::parse_terminated
				.parse2(inner_list.tokens.clone())?;
			let mut filtered = Punctuated::<Path, Token![,]>::new();
			for derive in derives {
				if derive
					.segments
					.last()
					.is_some_and(|seg| seg.ident == trait_name)
				{
					continue;
				}
				filtered.push(derive);
			}
			if !filtered.is_empty() {
				let derive_meta: Meta = parse_quote!(derive(#filtered));
				rebuilt.push(derive_meta);
			}
		}

		if rebuilt.len() > 1 {
			let new_attr: Attribute = parse_quote!(#[cfg_attr(#rebuilt)]);
			normalized.push(new_attr);
		}
	}

	*attrs = normalized;
	Ok(())
}
