//! Attribute macro implementation for `#[shared_schema]`
//!
//! Absorbs the `cfg_attr(native, ...)` boilerplate required for DTOs shared
//! between native (server) and wasm (client) builds. See the public-facing
//! rustdoc on `crate::shared_schema` in `lib.rs` for the user-facing contract.

use crate::crate_paths::get_reinhardt_crate;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
	Attribute, Data, DeriveInput, Fields, Meta, Path, Result, Token, parse::Parser, parse_quote,
	punctuated::Punctuated,
};

pub(crate) fn shared_schema_impl(args: TokenStream, mut input: DeriveInput) -> Result<TokenStream> {
	if !args.is_empty() {
		return Err(syn::Error::new_spanned(
			args,
			"#[shared_schema] does not accept arguments in this version",
		));
	}

	let reinhardt = get_reinhardt_crate();

	let fields = match &mut input.data {
		Data::Struct(s) => match &mut s.fields {
			Fields::Named(f) => Some(&mut f.named),
			Fields::Unnamed(f) => Some(&mut f.unnamed),
			Fields::Unit => None,
		},
		Data::Enum(_) | Data::Union(_) => {
			return Err(syn::Error::new_spanned(
				&input.ident,
				"#[shared_schema] can only be applied to structs",
			));
		}
	};

	if let Some(fields) = fields {
		for field in fields.iter_mut() {
			for attr in field.attrs.iter_mut() {
				if attr.path().is_ident("validate") {
					*attr = wrap_in_cfg_attr_native(attr);
				}
			}
		}
	}

	let needs_validate = !has_native_derive(&input.attrs, "Validate")?;
	let needs_schema = !has_native_derive(&input.attrs, "Schema")?;

	let mut derives: Punctuated<Path, Token![,]> = Punctuated::new();
	if needs_validate {
		derives.push(parse_quote!(#reinhardt::Validate));
	}
	if needs_schema {
		derives.push(parse_quote!(#reinhardt::rest::openapi::Schema));
	}

	if !derives.is_empty() {
		let new_attr: Attribute = parse_quote!(#[cfg_attr(native, derive(#derives))]);
		input.attrs.push(new_attr);
	}

	// The `Schema` derive from `reinhardt-openapi-macros` emits an
	// `inventory::submit!` block that references the schema method as
	// `<StructName>::schema`, which requires the `ToSchema` trait to be in
	// scope at the module level. Bring it in anonymously so we do not
	// introduce a visible name. `as _` allows multiple `#[shared_schema]`
	// uses in the same module without name collisions.
	let to_schema_import = if needs_schema {
		quote! {
			#[cfg(native)]
			#[allow(unused_imports)]
			use #reinhardt::rest::openapi::ToSchema as _;
		}
	} else {
		quote! {}
	};

	Ok(quote! {
		#input
		#to_schema_import
	})
}

fn wrap_in_cfg_attr_native(attr: &Attribute) -> Attribute {
	let meta = &attr.meta;
	parse_quote!(#[cfg_attr(native, #meta)])
}

/// Returns true if `attrs` already contains `#[cfg_attr(native, derive(... TraitName ...))]`.
///
/// Only inspects the `native` cfg branch — `#[derive(TraitName)]` unconditional
/// derives are NOT counted, because they would conflict with the macro-emitted
/// `cfg_attr(native, derive(...))` on native builds. The user is expected to
/// either let the macro emit the derive or write the full `cfg_attr(native, ...)`
/// form themselves.
fn has_native_derive(attrs: &[Attribute], trait_name: &str) -> Result<bool> {
	for attr in attrs {
		if !attr.path().is_ident("cfg_attr") {
			continue;
		}
		let Meta::List(list) = &attr.meta else {
			continue;
		};
		let nested = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(list.tokens.clone())?;
		let mut iter = nested.iter();
		let Some(first) = iter.next() else {
			continue;
		};
		// First arg must be the `native` predicate (bare `native` Path).
		if !matches!(first, Meta::Path(p) if p.is_ident("native")) {
			continue;
		}
		for inner in iter {
			let Meta::List(inner_list) = inner else {
				continue;
			};
			if !inner_list.path.is_ident("derive") {
				continue;
			}
			let derives = Punctuated::<Path, Token![,]>::parse_terminated
				.parse2(inner_list.tokens.clone())?;
			if derives
				.iter()
				.any(|p| p.segments.last().is_some_and(|seg| seg.ident == trait_name))
			{
				return Ok(true);
			}
		}
	}
	Ok(false)
}
