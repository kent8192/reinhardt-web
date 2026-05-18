//! action macro implementation

use crate::crate_paths::get_reinhardt_di_crate;
use crate::injectable_common::{
	detect_inject_params, generate_di_context_extraction, generate_injection_calls,
	strip_inject_attrs,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
	Expr, ExprLit, FnArg, ItemFn, Lit, Meta, Result, Token, parse::Parser, punctuated::Punctuated,
};

/// Parsed `#[action(...)]` attribute metadata reused by `#[viewset]`'s
/// impl-form expansion.
///
/// Phase 5 of Issue #4507 extracts this struct so `viewset_macro.rs` can
/// reuse the exact same parsing logic the `#[action]` macro uses when
/// emitting `__url_resolver_meta_action_*` macros.
///
/// Refs Issue #4507.
pub(crate) struct ActionMeta {
	// `methods` is parsed for parity with `action_impl` but is not consumed
	// by the impl-form `#[viewset]` expansion (which only needs the URL
	// metadata). Kept on the struct so future callers do not need to
	// re-parse the attribute. Refs Issue #4507.
	#[allow(dead_code)]
	pub methods: Vec<String>,
	pub detail: bool,
	/// Empty string when `url_path` is absent from the attribute.
	pub url_path: String,
	/// Defaults to the annotated function's identifier when `url_name` is
	/// absent. Always non-empty.
	pub url_name: String,
}

/// Parse the `#[action(...)]` attribute argument list into an `ActionMeta`.
///
/// Defaults `url_name` to `fn_ident` when absent, leaves `url_path` empty
/// when absent, and reuses the same validation as `action_impl` for
/// `methods` / `detail` / `url_path`.
///
/// This extractor is reused by `viewset_macro.rs::parse_action_meta_for_viewset`
/// (Phase 5 of Issue #4507) so the impl-form of `#[viewset]` emits the same
/// names and parameters that `#[action]` would compute.
///
/// Refs Issue #4507.
pub(crate) fn parse_action_args_with_defaults(
	args: TokenStream,
	fn_ident: &syn::Ident,
) -> Result<ActionMeta> {
	let mut methods = Vec::<String>::new();
	let mut detail = false;
	let mut url_path: Option<String> = None;
	let mut url_name: Option<String> = None;
	let mut has_methods = false;
	let mut has_detail = false;

	let meta_list = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;
	for meta in meta_list {
		if let Meta::NameValue(nv) = meta {
			if nv.path.is_ident("methods") {
				has_methods = true;
				if let Expr::Lit(ExprLit {
					lit: Lit::Str(lit), ..
				}) = &nv.value
				{
					let s = lit.value();
					let s = s.trim_matches(|c| c == '[' || c == ']');
					for m in s.split(',') {
						let m = m.trim().trim_matches('"');
						if !m.is_empty() {
							methods.push(m.to_string());
						}
					}
				}
			} else if nv.path.is_ident("detail") {
				has_detail = true;
				if let Expr::Lit(ExprLit {
					lit: Lit::Bool(lit),
					..
				}) = &nv.value
				{
					detail = lit.value;
				}
			} else if nv.path.is_ident("url_path")
				&& let Expr::Lit(ExprLit {
					lit: Lit::Str(lit), ..
				}) = &nv.value
			{
				let p = lit.value();
				if p.contains(' ') {
					return Err(syn::Error::new_spanned(
						lit,
						"url_path cannot contain spaces",
					));
				}
				if !p.starts_with('/') {
					return Err(syn::Error::new_spanned(lit, "url_path must start with '/'"));
				}
				url_path = Some(p);
			} else if nv.path.is_ident("url_name")
				&& let Expr::Lit(ExprLit {
					lit: Lit::Str(lit), ..
				}) = &nv.value
			{
				// `url_name` is later passed to `syn::Ident::new(...)` by
				// the viewset/routes macro emitters to build identifiers
				// like `__for_each_viewset_meta_<url_name>` and the typed
				// `ResolvedUrls::<route>()` accessor. `syn::Ident::new`
				// panics on non-identifier input (e.g. `"highlight-code"`
				// or `"foo bar"`), which surfaces at proc-macro expansion
				// as an obscure rustc message. Validate as a Rust
				// identifier here so the failure becomes a clean
				// compile-time error tied to the attribute's own span.
				let value = lit.value();
				if syn::parse_str::<syn::Ident>(&value).is_err() {
					return Err(syn::Error::new_spanned(
						lit,
						format!(
							"url_name `{value}` is not a valid Rust identifier (use snake_case ASCII letters, digits, and underscores; cannot start with a digit)"
						),
					));
				}
				url_name = Some(value);
			}
		}
	}

	if !has_methods {
		return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"action macro requires 'methods' parameter",
		));
	}
	if !has_detail {
		return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"action macro requires 'detail' parameter",
		));
	}
	if methods.is_empty() {
		methods.push("GET".to_string());
	}

	Ok(ActionMeta {
		methods,
		detail,
		url_path: url_path.unwrap_or_default(),
		url_name: url_name.unwrap_or_else(|| fn_ident.to_string()),
	})
}

/// Implementation of the `action` procedural macro
///
/// This function is used internally by the `#[action]` attribute macro.
/// Users should not call this function directly.
pub(crate) fn action_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
	let mut methods = Vec::new();
	let mut detail = false;
	let mut _url_path: Option<String> = None;
	let mut _url_name: Option<String> = None;
	let mut use_inject = false;
	let mut methods_lit = None;
	let mut has_methods = false;
	let mut has_detail = false;

	// Parse arguments
	let meta_list = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;

	for meta in meta_list {
		match meta {
			Meta::NameValue(nv) => {
				if nv.path.is_ident("methods") {
					has_methods = true;
					if let Expr::Lit(ExprLit {
						lit: Lit::Str(lit), ..
					}) = &nv.value
					{
						methods_lit = Some(lit.clone());
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
				} else if nv.path.is_ident("detail") {
					has_detail = true;
					if let Expr::Lit(ExprLit {
						lit: Lit::Bool(lit),
						..
					}) = &nv.value
					{
						detail = lit.value;
					} else {
						return Err(syn::Error::new_spanned(
							&nv.value,
							"detail parameter must be a boolean literal (true or false)",
						));
					}
				} else if nv.path.is_ident("url_path") {
					if let Expr::Lit(ExprLit {
						lit: Lit::Str(lit), ..
					}) = &nv.value
					{
						let path = lit.value();
						// Validate url_path format
						if path.contains(' ') {
							return Err(syn::Error::new_spanned(
								lit,
								"url_path cannot contain spaces",
							));
						}
						if !path.starts_with('/') {
							return Err(syn::Error::new_spanned(
								lit,
								"url_path must start with '/'",
							));
						}
						_url_path = Some(path);
					} else {
						return Err(syn::Error::new_spanned(
							&nv.value,
							"url_path parameter must be a string literal",
						));
					}
				} else if nv.path.is_ident("url_name") {
					if let Expr::Lit(ExprLit {
						lit: Lit::Str(lit), ..
					}) = &nv.value
					{
						_url_name = Some(lit.value());
					} else {
						return Err(syn::Error::new_spanned(
							&nv.value,
							"url_name parameter must be a string literal",
						));
					}
				} else if nv.path.is_ident("use_inject") {
					if let Expr::Lit(ExprLit {
						lit: Lit::Bool(lit),
						..
					}) = &nv.value
					{
						use_inject = lit.value;
					} else {
						return Err(syn::Error::new_spanned(
							&nv.value,
							"use_inject parameter must be a boolean literal (true or false)",
						));
					}
				}
			}
			Meta::Path(path) => {
				if path.is_ident("methods") {
					return Err(syn::Error::new_spanned(
						path,
						"methods parameter requires a value: methods = \"GET,POST\"",
					));
				} else if path.is_ident("detail") {
					return Err(syn::Error::new_spanned(
						path,
						"detail parameter requires a value: detail = true or detail = false",
					));
				}
			}
			_ => {}
		}
	}

	// Validate required parameters
	if !has_methods {
		return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"action macro requires 'methods' parameter",
		));
	}

	if !has_detail {
		return Err(syn::Error::new(
			proc_macro2::Span::call_site(),
			"action macro requires 'detail' parameter",
		));
	}

	// Default to GET if no methods specified
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

	// Generate metadata
	let detail_flag = detail;
	let method_list = methods.join(", ");

	// Detect #[inject] parameters
	let inject_params = detect_inject_params(fn_inputs);

	// Validate: error if #[inject] is used when use_inject = false
	if !use_inject && !inject_params.is_empty() {
		return Err(syn::Error::new_spanned(
			&inject_params[0].pat,
			"#[inject] attribute requires use_inject = true option",
		));
	}

	// Generate wrapper function for DI support
	if use_inject && !inject_params.is_empty() {
		let original_fn_name = quote::format_ident!("{}_original", fn_name);

		// Original function (with #[inject] attributes stripped)
		let stripped_inputs = strip_inject_attrs(fn_inputs);
		let stripped_inputs = Punctuated::<FnArg, Token![,]>::from_iter(stripped_inputs);

		// DI context extraction code
		let request_ident = syn::Ident::new("request", proc_macro2::Span::call_site());
		let di_extraction = generate_di_context_extraction(&request_ident);

		// Injection calls code
		let injection_calls = generate_injection_calls(&inject_params);

		// Argument list
		let inject_args: Vec<_> = inject_params.iter().map(|p| &p.pat).collect();
		let regular_args: Vec<_> = stripped_inputs
			.iter()
			.filter_map(|arg| {
				if let FnArg::Typed(pat_type) = arg {
					Some(&pat_type.pat)
				} else {
					None
				}
			})
			.collect();

		let di_crate = get_reinhardt_di_crate();

		Ok(quote! {
			// Original function (renamed, private)
			#asyncness fn #original_fn_name #generics (#stripped_inputs) #fn_output #where_clause {
				#fn_block
			}

			// Wrapper function (with DI support)
			#(#fn_attrs)*
			#[doc = "Custom action with DI support"]
			#[doc = concat!("Methods: ", #method_list)]
			#[doc = concat!("Detail: ", stringify!(#detail_flag))]
			#fn_vis #asyncness fn #fn_name(request: ::Request) #fn_output {
				// DI context extraction
				#di_extraction

				// Execute handler within resolve context scope
				#di_crate::resolve_context::RESOLVE_CTX.scope(__resolve_ctx, async {
					// Dependency resolution
					#(#injection_calls)*

					// Call original function
					#original_fn_name(#(#regular_args,)* #(#inject_args),*).await
				}).await
			}
		})
	} else {
		// Without DI, use conventional approach
		Ok(quote! {
			#(#fn_attrs)*
			#[doc = "Custom action"]
			#[doc = concat!("Methods: ", #method_list)]
			#[doc = concat!("Detail: ", stringify!(#detail_flag))]
			#fn_vis #asyncness fn #fn_name #generics (#fn_inputs) #fn_output #where_clause {
				#fn_block
			}
		})
	}
}

#[cfg(test)]
mod meta_extractor_tests {
	use super::*;
	use quote::quote;

	#[test]
	fn url_name_defaults_to_fn_name_when_absent() {
		// Arrange
		let args = quote! { methods = "POST", detail = true };
		let fn_ident: syn::Ident = syn::parse_quote! { highlight };

		// Act
		let meta = parse_action_args_with_defaults(args, &fn_ident).unwrap();

		// Assert
		assert_eq!(meta.url_name, "highlight");
		assert!(meta.detail);
		assert!(meta.url_path.is_empty());
	}

	#[test]
	fn explicit_url_name_wins_over_fn_name() {
		// Arrange
		let args = quote! { methods = "POST", detail = true, url_name = "highlight_code" };
		let fn_ident: syn::Ident = syn::parse_quote! { highlight };

		// Act
		let meta = parse_action_args_with_defaults(args, &fn_ident).unwrap();

		// Assert
		assert_eq!(meta.url_name, "highlight_code");
	}

	#[test]
	fn url_path_with_placeholders_is_preserved() {
		// Arrange
		let args = quote! {
			methods = "GET",
			detail = true,
			url_name = "child",
			url_path = "/children/{child_id}"
		};
		let fn_ident: syn::Ident = syn::parse_quote! { child };

		// Act
		let meta = parse_action_args_with_defaults(args, &fn_ident).unwrap();

		// Assert
		assert_eq!(meta.url_path, "/children/{child_id}");
	}

	#[test]
	fn missing_methods_errors() {
		// Arrange
		let args = quote! { detail = true };
		let fn_ident: syn::Ident = syn::parse_quote! { x };

		// Act + Assert
		assert!(parse_action_args_with_defaults(args, &fn_ident).is_err());
	}
}
