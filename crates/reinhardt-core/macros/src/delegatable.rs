//! `#[delegatable]` companion attribute for `#[newtype(delegate(...))]`
//! (Issue #4667).
//!
//! When applied to a `trait`, this attribute re-emits the trait verbatim and
//! generates a sibling `macro_rules!` helper that knows how to forward every
//! method of the trait to a wrapped inner field. `#[newtype(delegate(T))]`
//! then invokes that helper to produce an `impl T for NewType` block.
//!
//! ## Scope limitation (MVP)
//!
//! The generated `macro_rules!` is **not** marked `#[macro_export]`, so the
//! `#[delegatable] trait` and every `#[newtype(delegate(T))]` that targets it
//! must live in the *same* module-path scope (the macro lookup follows normal
//! path resolution rules from the call site). Cross-crate trait delegation is
//! a deliberate v4 follow-up — see Issue #4667.
//!
//! ## What is supported
//!
//! - Methods that take `&self` or `&mut self` (the standard decorator shape).
//! - Default-implementation methods are **left alone** — the macro only
//!   forwards methods that have no body.
//! - `async fn` declarations are forwarded as `async fn` invocations.
//!
//! ## What is intentionally rejected
//!
//! - Generic traits (`trait Foo<T>`) — the generated `impl Foo for NewType`
//!   would be missing the required type arguments. Picking them is a v4
//!   follow-up.
//! - Required associated types and consts — the generated impl would be
//!   missing those item definitions, so we surface the error at the trait
//!   definition rather than letting the downstream impl fail opaquely.
//! - Methods taking `self` by value (would require knowing `Drop` semantics).
//! - Generic methods (would need turbofish at invocation site).
//! - Macro invocations inside the trait body.
//!
//! All rejection diagnostics are emitted as `syn::Error` from
//! `delegatable_impl` itself, so the message points at the offending
//! item in the trait definition — not at the eventual
//! `#[newtype(delegate(T))]` call site.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Error, FnArg, ItemTrait, Pat, Result, TraitItem, TraitItemFn, parse2, spanned::Spanned};

pub(crate) fn delegatable_impl(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
	if !args.is_empty() {
		return Err(Error::new_spanned(
			args,
			"#[delegatable] does not accept arguments",
		));
	}

	let trait_def: ItemTrait = parse2(input)?;
	let trait_ident = &trait_def.ident;
	let macro_name = format_ident!("__reinhardt_delegate_{}", trait_ident);

	// Reject generic traits — the generated `impl #trait_ident for $NewType`
	// uses the bare trait identifier, so a `trait Foo<T>` would expand to the
	// malformed `impl Foo for NewType`. Picking the right type argument
	// requires it to be supplied at the delegation site, which is a v4
	// follow-up (Issue #4667 Codex stop-time review).
	if !trait_def.generics.params.is_empty() {
		return Err(Error::new(
			trait_def.generics.span(),
			format!(
				"#[delegatable] cannot forward generic trait `{trait_ident}`; \
				 wrap it manually or provide the type arguments at the \
				 newtype site (Issue #4667 v4)"
			),
		));
	}

	// Reject unsupported item shapes at the trait-definition site instead of
	// silently producing an `impl Trait for NewType` that is missing required
	// associated items — that would surface as an opaque downstream error at
	// the `#[newtype(delegate(...))]` call site (Issue #4667 Codex review).
	for item in &trait_def.items {
		let (span, kind): (proc_macro2::Span, &str) = match item {
			TraitItem::Type(ty) if ty.default.is_none() => (ty.ident.span(), "associated type"),
			TraitItem::Const(c) if c.default.is_none() => (c.ident.span(), "associated const"),
			TraitItem::Macro(m) => (m.mac.path.span(), "macro invocation"),
			// Function items are validated per-method by `forward_method`;
			// defaulted assoc types/consts are inherited and need no forward.
			_ => continue,
		};
		return Err(Error::new(
			span,
			format!(
				"#[delegatable] cannot forward traits with required {kind} items; \
				 wrap `{trait_ident}` manually or provide a default value"
			),
		));
	}

	// Forward only the unimplemented methods. Default methods stay as-is on
	// the trait and are inherited by any implementor.
	let forwards = trait_def
		.items
		.iter()
		.filter_map(|item| match item {
			TraitItem::Fn(f) if f.default.is_none() => Some(forward_method(trait_ident, f)),
			_ => None,
		})
		.collect::<Result<Vec<_>>>()?;

	Ok(quote! {
		#trait_def

		#[doc(hidden)]
		macro_rules! #macro_name {
			($__newtype:ident, $__field:tt) => {
				impl #trait_ident for $__newtype {
					#( #forwards )*
				}
			};
		}
	})
}

fn forward_method(trait_ident: &syn::Ident, f: &TraitItemFn) -> Result<TokenStream> {
	let sig = &f.sig;
	let method_name = &sig.ident;

	if !sig.generics.params.is_empty() {
		return Err(Error::new_spanned(
			&sig.generics,
			format!(
				"#[delegatable] cannot forward generic method `{}::{}` (Issue #4667 v4)",
				trait_ident, method_name
			),
		));
	}

	// Collect argument *names* (the things we re-pass) and decide whether the
	// receiver was `&self` / `&mut self`. A value-`self` receiver isn't
	// forwardable, so reject explicitly.
	let mut call_args: Vec<TokenStream> = Vec::new();
	let mut saw_receiver = false;
	for input in &sig.inputs {
		match input {
			FnArg::Receiver(rcv) => {
				saw_receiver = true;
				if rcv.reference.is_none() {
					return Err(Error::new_spanned(
						rcv,
						format!(
							"#[delegatable] cannot forward `{}::{}`: \
							 methods taking `self` by value are not supported",
							trait_ident, method_name
						),
					));
				}
			}
			FnArg::Typed(pt) => match &*pt.pat {
				Pat::Ident(id) => {
					let ident = &id.ident;
					call_args.push(quote!(#ident));
				}
				_ => {
					return Err(Error::new_spanned(
						&pt.pat,
						"#[delegatable] requires plain identifier patterns in method signatures",
					));
				}
			},
		}
	}
	if !saw_receiver {
		return Err(Error::new_spanned(
			sig,
			format!(
				"#[delegatable] cannot forward associated function `{}::{}` (no receiver)",
				trait_ident, method_name
			),
		));
	}

	// `awaitable` toggles `.await` when the trait method is `async fn`.
	let awaitable = if sig.asyncness.is_some() {
		quote!(.await)
	} else {
		quote!()
	};

	// `$__field` is supplied by `#[newtype]` as either `0` (tuple struct) or
	// the named-field ident. The `tt` matcher in the parent macro accepts
	// both forms. Forward via direct field access (not via Deref) so the
	// delegation never silently re-dispatches through another trait.
	Ok(quote! {
		#sig {
			self.$__field.#method_name( #( #call_args ),* ) #awaitable
		}
	})
}
