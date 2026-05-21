//! `#[newtype]` opt-out attribute macro (Issue #4667).
//!
//! Bundles the canonical newtype boilerplate (std derives + Display / From /
//! Into / AsRef / AsMut / Deref / DerefMut / FromStr / serde transparent) into
//! a single attribute. Every emission is enabled by default and can be removed
//! individually via `skip(...)`.
//!
//! Design rationale: see DESIGN_PHILOSOPHY.md §2 (predictable CoC), §3 (opt-out
//! is a right), §5 (API ergonomics), §8 (boilerplate is evil).
//!
//! MVP scope: single-field structs (tuple or named) without generic parameters.
//! Generic newtypes are intentionally out of scope so the macro can emit a
//! correctly-shaped `Deserialize<'de>` lifetime without re-implementing
//! `syn`'s generic-parameter splicing.

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use std::collections::BTreeSet;
use syn::{
	Error, Fields, Ident, ItemStruct, Member, Path, Result, Type, parse::Parser as _, parse2,
	spanned::Spanned,
};

/// Catalogue of every trait the macro can emit. Order is the canonical doc
/// order — kept stable so error messages stay deterministic.
const KNOWN: &[&str] = &[
	"Debug",
	"Clone",
	"Copy",
	"PartialEq",
	"Eq",
	"PartialOrd",
	"Ord",
	"Hash",
	"Default",
	"Display",
	"From",
	"Into",
	"AsRef",
	"AsMut",
	"Deref",
	"DerefMut",
	"FromStr",
	"Serialize",
	"Deserialize",
];

/// std derives — emitted as a single `#[derive(...)]` so the compiler picks up
/// the inner type's existing impls. Everything else is emitted as an explicit
/// `impl` block, which lets us avoid a transitive dependency on `derive_more`.
const STD_DERIVES: &[&str] = &[
	"Debug",
	"Clone",
	"Copy",
	"PartialEq",
	"Eq",
	"PartialOrd",
	"Ord",
	"Hash",
	"Default",
];

struct NewtypeArgs {
	inner: Option<Type>,
	skip: BTreeSet<String>,
	delegate: Vec<Path>,
}

pub(crate) fn newtype_impl(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
	let item: ItemStruct = parse2(input)?;
	if !item.generics.params.is_empty() {
		return Err(Error::new(
			item.generics.span(),
			"#[newtype] does not support generic parameters in MVP scope; \
			 see Issue #4667 follow-up",
		));
	}

	let args = parse_args(args)?;
	let (inner_ty, field_access) = resolve_field(&item, args.inner.as_ref())?;
	validate_skip(&args.skip, &item)?;

	let skip = &args.skip;
	let name = &item.ident;

	// 1) std derives that survive `skip(...)`.
	let derives: Vec<_> = STD_DERIVES
		.iter()
		.filter(|d| !skip.contains(**d))
		.map(|d| format_ident!("{}", d))
		.collect();
	let derive_attr = if derives.is_empty() {
		quote!()
	} else {
		quote!( #[derive( #( #derives ),* )] )
	};

	// 2) Hand-written impls. Each helper returns an empty TokenStream when
	// the corresponding trait is skipped, keeping the rest of the expansion
	// flat and readable.
	let display = emit_display(skip, name, &field_access);
	let from = emit_from(skip, name, &inner_ty, &item.fields);
	let into = emit_into(skip, name, &inner_ty, &field_access);
	let as_ref = emit_as_ref(skip, name, &inner_ty, &field_access);
	let as_mut = emit_as_mut(skip, name, &inner_ty, &field_access);
	let deref = emit_deref(skip, name, &inner_ty, &field_access);
	let deref_mut = emit_deref_mut(skip, name, &inner_ty, &field_access);
	let from_str = emit_from_str(skip, name, &inner_ty, &item.fields);
	let (serialize, deserialize) = emit_serde(skip, name, &inner_ty, &item.fields, &field_access);

	// 3) Optional delegations to user traits via the companion
	// `#[delegatable]` macro_rules sidecar (see `delegatable.rs`).
	let delegates = args.delegate.iter().map(|trait_path| {
		let macro_name = match trait_path.segments.last() {
			Some(seg) => format_ident!("__reinhardt_delegate_{}", seg.ident),
			None => format_ident!("__reinhardt_delegate_unknown"),
		};
		let macro_path = {
			let mut p = trait_path.clone();
			if let Some(last) = p.segments.last_mut() {
				last.ident = macro_name;
				last.arguments = syn::PathArguments::None;
			}
			p
		};
		// `syn::Index::to_tokens` emits `0u32`, which the `tt` matcher in the
		// companion macro rejects. Re-emit the literal by hand so we get a
		// bare integer literal that matches `tt`.
		let field_tt = match &field_access {
			Member::Named(id) => id.to_token_stream(),
			Member::Unnamed(idx) => {
				let n = idx.index as usize;
				let lit = proc_macro2::Literal::usize_unsuffixed(n);
				quote!(#lit)
			}
		};
		quote! {
			#macro_path !( #name , #field_tt );
		}
	});

	Ok(quote! {
		#derive_attr
		#item

		#display
		#from
		#into
		#as_ref
		#as_mut
		#deref
		#deref_mut
		#from_str
		#serialize
		#deserialize

		#( #delegates )*
	})
}

// ───────────────────────── argument parsing ─────────────────────────

fn parse_args(args: TokenStream) -> Result<NewtypeArgs> {
	let mut out = NewtypeArgs {
		inner: None,
		skip: BTreeSet::new(),
		delegate: Vec::new(),
	};

	if args.is_empty() {
		return Ok(out);
	}

	let parser = syn::meta::parser(|meta| {
		if meta.path.is_ident("inner") {
			let value = meta.value()?;
			out.inner = Some(value.parse::<Type>()?);
			return Ok(());
		}
		if meta.path.is_ident("skip") {
			meta.parse_nested_meta(|nested| {
				let Some(id) = nested.path.get_ident() else {
					return Err(nested.error("expected a trait name like `Copy`"));
				};
				let name = id.to_string();
				if !KNOWN.iter().any(|k| *k == name) {
					return Err(nested.error(format!(
						"unknown trait `{name}` in `skip(...)`; known names: {}",
						KNOWN.join(", ")
					)));
				}
				out.skip.insert(name);
				Ok(())
			})?;
			return Ok(());
		}
		if meta.path.is_ident("delegate") {
			meta.parse_nested_meta(|nested| {
				out.delegate.push(nested.path.clone());
				Ok(())
			})?;
			return Ok(());
		}
		Err(meta.error("expected `inner = T`, `skip(...)`, or `delegate(...)`"))
	});

	parser.parse2(args)?;
	Ok(out)
}

// ───────────────────────── field resolution ─────────────────────────

fn resolve_field(item: &ItemStruct, explicit: Option<&Type>) -> Result<(Type, Member)> {
	match &item.fields {
		Fields::Unnamed(f) if f.unnamed.len() == 1 => {
			let ty = explicit.cloned().unwrap_or_else(|| f.unnamed[0].ty.clone());
			Ok((
				ty,
				Member::Unnamed(syn::Index {
					index: 0,
					span: item.span(),
				}),
			))
		}
		Fields::Named(f) if f.named.len() == 1 => {
			let only = f.named.first().unwrap();
			let ty = explicit.cloned().unwrap_or_else(|| only.ty.clone());
			Ok((ty, Member::Named(only.ident.clone().unwrap())))
		}
		_ => Err(Error::new(
			item.fields.span(),
			"#[newtype] requires a struct with exactly one field (tuple or named)",
		)),
	}
}

// ───────────────────────── sanity checks ─────────────────────────

fn validate_skip(skip: &BTreeSet<String>, item: &ItemStruct) -> Result<()> {
	// Direction matters: if you skip the weaker bound (PartialEq) you MUST
	// also skip the stronger bound (Eq), because Eq's impl quietly assumes
	// PartialEq is in scope.
	let pairs: &[(&str, &str, &str)] = &[
		(
			"Deref",
			"DerefMut",
			"skipping `Deref` requires also skipping `DerefMut` \
			 (DerefMut depends on Deref)",
		),
		(
			"PartialEq",
			"Eq",
			"skipping `PartialEq` requires also skipping `Eq` \
			 (Eq depends on PartialEq)",
		),
		(
			"PartialOrd",
			"Ord",
			"skipping `PartialOrd` requires also skipping `Ord` \
			 (Ord depends on PartialOrd)",
		),
	];
	for (a, b, msg) in pairs {
		if skip.contains(*a) && !skip.contains(*b) {
			return Err(Error::new(item.ident.span(), *msg));
		}
	}
	Ok(())
}

// ───────────────────────── emitters ─────────────────────────

fn emit_display(skip: &BTreeSet<String>, name: &Ident, field: &Member) -> TokenStream {
	if skip.contains("Display") {
		return TokenStream::new();
	}
	quote! {
		impl ::core::fmt::Display for #name {
			fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
				::core::fmt::Display::fmt(&self.#field, f)
			}
		}
	}
}

fn emit_from(skip: &BTreeSet<String>, name: &Ident, inner: &Type, fields: &Fields) -> TokenStream {
	if skip.contains("From") {
		return TokenStream::new();
	}
	let ctor = ctor_for_value(fields, quote!(v));
	quote! {
		impl ::core::convert::From<#inner> for #name {
			fn from(v: #inner) -> Self { #ctor }
		}
	}
}

fn emit_into(skip: &BTreeSet<String>, name: &Ident, inner: &Type, field: &Member) -> TokenStream {
	if skip.contains("Into") {
		return TokenStream::new();
	}
	quote! {
		impl ::core::convert::From<#name> for #inner {
			fn from(v: #name) -> Self { v.#field }
		}
	}
}

fn emit_as_ref(skip: &BTreeSet<String>, name: &Ident, inner: &Type, field: &Member) -> TokenStream {
	if skip.contains("AsRef") {
		return TokenStream::new();
	}
	quote! {
		impl ::core::convert::AsRef<#inner> for #name {
			fn as_ref(&self) -> &#inner { &self.#field }
		}
	}
}

fn emit_as_mut(skip: &BTreeSet<String>, name: &Ident, inner: &Type, field: &Member) -> TokenStream {
	if skip.contains("AsMut") {
		return TokenStream::new();
	}
	quote! {
		impl ::core::convert::AsMut<#inner> for #name {
			fn as_mut(&mut self) -> &mut #inner { &mut self.#field }
		}
	}
}

fn emit_deref(skip: &BTreeSet<String>, name: &Ident, inner: &Type, field: &Member) -> TokenStream {
	if skip.contains("Deref") {
		return TokenStream::new();
	}
	quote! {
		impl ::core::ops::Deref for #name {
			type Target = #inner;
			fn deref(&self) -> &#inner { &self.#field }
		}
	}
}

fn emit_deref_mut(
	skip: &BTreeSet<String>,
	name: &Ident,
	inner: &Type,
	field: &Member,
) -> TokenStream {
	if skip.contains("DerefMut") {
		return TokenStream::new();
	}
	quote! {
		impl ::core::ops::DerefMut for #name {
			fn deref_mut(&mut self) -> &mut #inner { &mut self.#field }
		}
	}
}

fn emit_from_str(
	skip: &BTreeSet<String>,
	name: &Ident,
	inner: &Type,
	fields: &Fields,
) -> TokenStream {
	if skip.contains("FromStr") {
		return TokenStream::new();
	}
	let ctor = ctor_for_value(fields, quote!(__v));
	quote! {
		impl ::core::str::FromStr for #name
		where #inner: ::core::str::FromStr
		{
			type Err = <#inner as ::core::str::FromStr>::Err;
			fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
				<#inner as ::core::str::FromStr>::from_str(s).map(|__v| #ctor)
			}
		}
	}
}

fn emit_serde(
	skip: &BTreeSet<String>,
	name: &Ident,
	inner: &Type,
	fields: &Fields,
	field: &Member,
) -> (TokenStream, TokenStream) {
	let ser = if skip.contains("Serialize") {
		TokenStream::new()
	} else {
		quote! {
			impl ::serde::Serialize for #name
			where #inner: ::serde::Serialize
			{
				fn serialize<__S>(&self, serializer: __S) -> ::core::result::Result<__S::Ok, __S::Error>
				where __S: ::serde::Serializer
				{
					::serde::Serialize::serialize(&self.#field, serializer)
				}
			}
		}
	};
	let de = if skip.contains("Deserialize") {
		TokenStream::new()
	} else {
		let ctor = ctor_for_value(fields, quote!(__v));
		quote! {
			impl<'__de> ::serde::Deserialize<'__de> for #name
			where #inner: ::serde::Deserialize<'__de>
			{
				fn deserialize<__D>(deserializer: __D) -> ::core::result::Result<Self, __D::Error>
				where __D: ::serde::Deserializer<'__de>
				{
					<#inner as ::serde::Deserialize<'__de>>::deserialize(deserializer).map(|__v| #ctor)
				}
			}
		}
	};
	(ser, de)
}

// ───────────────────────── helpers ─────────────────────────

fn ctor_for_value(fields: &Fields, value: TokenStream) -> TokenStream {
	match fields {
		Fields::Unnamed(_) => quote!(Self(#value)),
		Fields::Named(f) => {
			let id = f.named.first().and_then(|x| x.ident.clone()).unwrap();
			quote!(Self { #id: #value })
		}
		Fields::Unit => quote!(Self),
	}
}
