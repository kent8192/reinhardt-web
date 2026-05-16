//! Attribute macro implementation for `#[dto]`
//!
//! Absorbs the `cfg_attr(native, ...)` boilerplate required for DTOs shared
//! between native (server) and wasm (client) builds. See the public-facing
//! rustdoc on `crate::dto` in `lib.rs` for the user-facing contract.

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

	let fields = match &mut input.data {
		Data::Struct(s) => match &mut s.fields {
			Fields::Named(f) => Some(&mut f.named),
			Fields::Unnamed(f) => Some(&mut f.unnamed),
			Fields::Unit => None,
		},
		Data::Enum(_) | Data::Union(_) => {
			return Err(syn::Error::new_spanned(
				&input.ident,
				"#[dto] can only be applied to structs",
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

	// Reject unconditional `#[derive(Validate|Schema)]` upfront. Both traits live
	// behind the `native` cfg, so an unconditional derive cannot resolve on wasm
	// builds and would duplicate the macro's `cfg_attr(native, derive(...))` on
	// native builds. The user is expected to either let `#[dto]` emit the derive
	// or write the full `#[cfg_attr(native, derive(...))]` form themselves.
	for trait_name in ["Validate", "Schema"] {
		if let Some(attr) = find_unconditional_derive(&input.attrs, trait_name)? {
			return Err(syn::Error::new_spanned(
				attr,
				format!(
					"#[dto] cannot be combined with unconditional `#[derive({trait_name})]`. \
					 Remove the derive so #[dto] can emit it as `cfg_attr(native, ...)` for you, \
					 or replace it with `#[cfg_attr(native, derive({trait_name}))]`."
				),
			));
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
	// scope at the module level. After the unconditional-derive check above,
	// the struct is guaranteed to carry a `Schema` derive on native — either
	// emitted by this macro (when `needs_schema`) or written by the user as
	// `#[cfg_attr(native, derive(Schema))]`. Emit the import unconditionally
	// so both paths compile. `as _` allows multiple `#[dto]` uses in the same
	// module without a visible-name collision.
	let to_schema_import = quote! {
		#[cfg(native)]
		#[allow(unused_imports)]
		use #reinhardt::rest::openapi::ToSchema as _;
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

/// Returns the first unconditional `#[derive(... TraitName ...)]` attribute on
/// `attrs`, if any. Used to detect derives that would clash with the
/// macro-emitted `cfg_attr(native, derive(...))`.
///
/// Path matching is by the last segment's identifier, mirroring `has_native_derive`,
/// so both `Validate` and `validator::Validate`-style paths are caught.
fn find_unconditional_derive<'a>(
	attrs: &'a [Attribute],
	trait_name: &str,
) -> Result<Option<&'a Attribute>> {
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
			return Ok(Some(attr));
		}
	}
	Ok(None)
}

/// Returns true if `attrs` already contains `#[cfg_attr(native, derive(... TraitName ...))]`.
///
/// Only inspects the `native` cfg branch — unconditional `#[derive(TraitName)]`
/// is handled separately by `find_unconditional_derive` and reported as an error.
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
