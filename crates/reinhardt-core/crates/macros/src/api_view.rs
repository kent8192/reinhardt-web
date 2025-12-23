//! api_view macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, ExprLit, ItemFn, Lit, Meta, Result, Token, parse::Parser, punctuated::Punctuated};
/// Implementation of the `api_view` procedural macro
///
/// This function is used internally by the `#[api_view]` attribute macro.
/// Users should not call this function directly.
pub(crate) fn api_view_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
	let mut methods = Vec::new();
	let mut methods_lit = None;

	// Parse method arguments
	let meta_list = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;

	for meta in meta_list {
		match meta {
			Meta::NameValue(nv) if nv.path.is_ident("methods") => {
				if let Expr::Lit(ExprLit {
					lit: Lit::Str(lit), ..
				}) = &nv.value
				{
					methods_lit = Some(lit.clone());
					// Parse methods array
					let methods_str = lit.value();
					let methods_str = methods_str.trim_matches(|c| c == '[' || c == ']');

					for method in methods_str.split(',') {
						let method = method.trim().trim_matches('"');
						if !method.is_empty() {
							methods.push(method.to_string());
						}
					}
				} else {
					return Err(syn::Error::new_spanned(
						&nv.value,
						"methods parameter must be a string literal",
					));
				}
			}
			Meta::Path(path) if path.is_ident("methods") => {
				return Err(syn::Error::new_spanned(
					path,
					"methods parameter requires a value: methods = \"GET,POST\"",
				));
			}
			_ => {}
		}
	}

	// If no methods specified, default to GET
	if methods.is_empty() {
		methods.push("GET".to_string());
	}

	// Validate HTTP methods
	const VALID_METHODS: &[&str] = &["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
	for method in &methods {
		let method_upper = method.to_uppercase();
		if !VALID_METHODS.contains(&method_upper.as_str()) {
			let error_msg = format!(
				"Invalid HTTP method '{}'. Valid methods are: GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS",
				method
			);
			return Err(syn::Error::new_spanned(
				methods_lit.as_ref().unwrap(),
				error_msg,
			));
		}
	}

	let fn_name = &input.sig.ident;
	let fn_block = &input.block;
	let fn_inputs = &input.sig.inputs;
	let fn_output = &input.sig.output;
	let fn_vis = &input.vis;
	let fn_attrs = &input.attrs;
	let asyncness = &input.sig.asyncness;
	let generics = &input.sig.generics;
	let where_clause = &input.sig.generics.where_clause;

	Ok(quote! {
		#(#fn_attrs)*
		#fn_vis #asyncness fn #fn_name #generics (#fn_inputs) #fn_output #where_clause {
			#fn_block
		}
	})
}
