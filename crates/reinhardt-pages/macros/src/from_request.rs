use std::collections::HashSet;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Field, Fields, Result, parse_macro_input};

use crate::crate_paths::get_reinhardt_pages_crate;

pub(crate) fn derive_from_request_impl(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	expand_from_request(input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

fn expand_from_request(input: DeriveInput) -> Result<proc_macro2::TokenStream> {
	let DeriveInput {
		ident,
		generics,
		data,
		..
	} = input;
	let fields = named_fields(&data)?;
	let pages_crate = get_reinhardt_pages_crate();
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	let mut keys = HashSet::new();
	let mut initializers = Vec::new();

	for field in fields {
		let field_ident = field
			.ident
			.as_ref()
			.ok_or_else(|| Error::new_spanned(field, "FromRequest requires named fields"))?;
		let key = field_key(field)?;
		if !keys.insert(key.clone()) {
			return Err(Error::new_spanned(
				field,
				format!("duplicate from_request key `{key}`"),
			));
		}
		let ty = &field.ty;
		initializers.push(quote! {
			#field_ident: <#ty>::extract(ctx, #key)?
		});
	}

	Ok(quote! {
		impl #impl_generics #pages_crate::router::request::FromRequest for #ident #ty_generics
			#where_clause
		{
			fn from_request(
				ctx: &#pages_crate::router::request::RouteContext,
			) -> ::std::result::Result<Self, #pages_crate::router::request::ExtractError> {
				::std::result::Result::Ok(Self {
					#(#initializers,)*
				})
			}
		}
	})
}

fn named_fields(data: &Data) -> Result<&syn::punctuated::Punctuated<Field, syn::Token![,]>> {
	match data {
		Data::Struct(s) => match &s.fields {
			Fields::Named(fields) => Ok(&fields.named),
			_ => Err(Error::new_spanned(
				&s.fields,
				"FromRequest can only be derived for structs with named fields",
			)),
		},
		_ => Err(Error::new(
			proc_macro2::Span::call_site(),
			"FromRequest can only be derived for structs with named fields",
		)),
	}
}

fn field_key(field: &Field) -> Result<String> {
	for attr in &field.attrs {
		if !attr.path().is_ident("from_request") {
			continue;
		}
		let mut name = None;
		attr.parse_nested_meta(|meta| {
			if meta.path.is_ident("name") {
				let value = meta.value()?;
				let lit: syn::LitStr = value.parse()?;
				name = Some(lit.value());
				return Ok(());
			}
			Err(meta.error("expected `name = \"...\"`"))
		})?;
		if let Some(name) = name {
			return Ok(name);
		}
		return Err(Error::new_spanned(
			attr,
			"expected #[from_request(name = \"...\")]",
		));
	}

	field
		.ident
		.as_ref()
		.map(|ident| ident.to_string())
		.ok_or_else(|| Error::new_spanned(field, "FromRequest requires named fields"))
}
