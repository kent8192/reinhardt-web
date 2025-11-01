//! Endpoint macro for automatic dependency injection
//!
//! Provides FastAPI-style dependency injection via function parameter attributes.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, FnArg, ItemFn, Pat, PatType, Result, Type};

/// Check if an attribute is #[inject]
fn is_inject_attr(attr: &Attribute) -> bool {
	attr.path().is_ident("inject")
}

/// Process a function argument and determine if it needs injection
struct ProcessedArg {
	/// Original pattern (variable name)
	pat: Pat,
	/// Type of the argument
	ty: Type,
	/// Whether this argument should be injected
	inject: bool,
	/// Whether to use cache (default: true)
	use_cache: bool,
}

impl ProcessedArg {
	fn from_fn_arg(arg: &FnArg) -> Option<Self> {
		match arg {
			FnArg::Typed(PatType { attrs, pat, ty, .. }) => {
				let mut inject = false;
				let mut use_cache = true;

				// Check for #[inject] or #[inject(cache = false)]
				for attr in attrs {
					if is_inject_attr(attr) {
						inject = true;

						// Parse arguments like #[inject(cache = false)]
						if let Ok(meta) = attr.parse_args::<syn::Meta>()
							&& let syn::Meta::NameValue(nv) = meta
								&& nv.path.is_ident("cache")
									&& let syn::Expr::Lit(syn::ExprLit {
										lit: syn::Lit::Bool(lit_bool),
										..
									}) = &nv.value
									{
										use_cache = lit_bool.value;
									}
					}
				}

				Some(ProcessedArg {
					pat: (**pat).clone(),
					ty: (**ty).clone(),
					inject,
					use_cache,
				})
			}
			_ => None,
		}
	}
}
/// Implementation of the `endpoint` procedural macro
///
/// This function is used internally by the `#[endpoint]` attribute macro.
/// Users should not call this function directly.
pub fn endpoint_impl(_args: TokenStream, input: ItemFn) -> Result<TokenStream> {
	let ItemFn {
		attrs,
		vis,
		sig,
		block,
	} = input;

	let fn_name = &sig.ident;
	let asyncness = &sig.asyncness;
	let output = &sig.output;
	let generics = &sig.generics;
	let where_clause = &sig.generics.where_clause;

	// Process all function arguments
	let mut processed_args = Vec::new();
	let mut regular_params = Vec::new();
	let mut inject_params = Vec::new();

	for arg in &sig.inputs {
		if let Some(processed) = ProcessedArg::from_fn_arg(arg) {
			if processed.inject {
				inject_params.push(processed);
			} else {
				processed_args.push(processed);
			}
		} else {
			// Keep self parameters as-is
			regular_params.push(arg.clone());
		}
	}

	// Build the new function signature (without #[inject] parameters)
	let mut new_params = regular_params;
	for arg in &processed_args {
		let pat = &arg.pat;
		let ty = &arg.ty;
		new_params.push(syn::parse_quote! { #pat: #ty });
	}

	// Add InjectionContext parameter
	new_params.push(syn::parse_quote! { __di_ctx: &::reinhardt_di::InjectionContext });

	// Generate injection code
	let mut injection_stmts = Vec::new();
	for arg in &inject_params {
		let pat = &arg.pat;
		let ty = &arg.ty;

		let injection_code = if arg.use_cache {
			quote! {
				let #pat = ::reinhardt_di::Depends::<#ty>::new()
					.resolve(__di_ctx)
					.await
					.map_err(|e| {
						eprintln!("Dependency injection failed for {}: {:?}", stringify!(#ty), e);
						e
					})?;
			}
		} else {
			quote! {
				let #pat = ::reinhardt_di::Depends::<#ty>::no_cache()
					.resolve(__di_ctx)
					.await
					.map_err(|e| {
						eprintln!("Dependency injection failed for {}: {:?}", stringify!(#ty), e);
						e
					})?;
			}
		};

		injection_stmts.push(injection_code);
	}

	// Generate the new function
	let expanded = quote! {
		#(#attrs)*
		#vis #asyncness fn #fn_name #generics (#(#new_params),*) #output #where_clause {
			#(#injection_stmts)*

			#block
		}
	};

	Ok(expanded)
}
