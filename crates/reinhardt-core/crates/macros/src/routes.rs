//! HTTP method route macros

use crate::injectable_common::{InjectOptions, is_inject_attr, parse_inject_options};
use crate::path_macro;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
	Error, Expr, ExprLit, FnArg, ItemFn, Lit, LitStr, Meta, Pat, PatType, Result, Token, Type,
	parse::Parser, punctuated::Punctuated, spanned::Spanned,
};

/// Options for route macros
#[derive(Clone, Default)]
struct RouteOptions {
	/// Enable DI functionality with `use_inject = true`
	use_inject: bool,
	/// Route name for URL reversal
	name: Option<String>,
}

/// Information about parameter extractors
#[derive(Clone)]
struct ExtractorInfo {
	pat: Box<Pat>,
	ty: Box<Type>,
	extractor_name: String,
}

/// Information about #[inject] parameters
#[derive(Clone)]
struct InjectInfo {
	pat: Box<Pat>,
	ty: Box<Type>,
	options: InjectOptions,
}

/// Validate a route path at compile time
fn validate_route_path(path: &str, span: Span) -> Result<()> {
	path_macro::parse_and_validate(path)
		.map(|_| ())
		.map_err(|e| Error::new(span, format!("Invalid route path: {}", e)))
}

/// Convert snake_case function name to PascalCase + View suffix
fn fn_name_to_view_type(fn_name: &str) -> String {
	let pascal_case: String = fn_name
		.split('_')
		.map(|word| {
			let mut chars = word.chars();
			match chars.next() {
				Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
				None => String::new(),
			}
		})
		.collect();
	format!("{}View", pascal_case)
}

/// Detect whether parameters contain extractors
fn detect_extractors(inputs: &Punctuated<FnArg, Token![,]>) -> Vec<ExtractorInfo> {
	let mut extractors = Vec::new();

	for input in inputs {
		if let FnArg::Typed(pat_type) = input {
			// Detect patterns like Path(id): Path<i64>
			if let Pat::TupleStruct(_) = &*pat_type.pat
				&& let Type::Path(type_path) = &*pat_type.ty
				&& let Some(segment) = type_path.path.segments.last()
			{
				let type_name = segment.ident.to_string();
				if matches!(
					type_name.as_str(),
					"Path" | "Json" | "Query" | "Header" | "Cookie" | "Form" | "Body"
				) {
					extractors.push(ExtractorInfo {
						pat: pat_type.pat.clone(),
						ty: pat_type.ty.clone(),
						extractor_name: type_name,
					});
				}
			}
		}
	}

	extractors
}

/// Detect parameters with #[inject] attribute
fn detect_inject_params(inputs: &Punctuated<FnArg, Token![,]>) -> Vec<InjectInfo> {
	let mut inject_params = Vec::new();

	for input in inputs {
		if let FnArg::Typed(PatType { attrs, pat, ty, .. }) = input {
			let has_inject = attrs.iter().any(is_inject_attr);

			if has_inject {
				let options = parse_inject_options(attrs);
				inject_params.push(InjectInfo {
					pat: pat.clone(),
					ty: ty.clone(),
					options,
				});
			}
		}
	}

	inject_params
}

/// Validate duplication of body-consuming extractors
fn validate_extractors(extractors: &[ExtractorInfo]) -> Result<()> {
	let body_consuming_types = ["Json", "Form", "Body"];
	let body_extractors: Vec<_> = extractors
		.iter()
		.filter(|ext| body_consuming_types.contains(&ext.extractor_name.as_str()))
		.collect();

	if body_extractors.len() > 1 {
		let names: Vec<_> = body_extractors
			.iter()
			.map(|e| e.extractor_name.as_str())
			.collect();
		return Err(Error::new(
			Span::call_site(),
			format!(
				"Cannot use multiple body-consuming extractors: {}. Request body can only be read once.",
				names.join(", ")
			),
		));
	}

	Ok(())
}

/// Generate wrapper function with both extractors and inject params
fn generate_wrapper_with_both(
	original_fn: &ItemFn,
	extractors: &[ExtractorInfo],
	inject_params: &[InjectInfo],
) -> (TokenStream, TokenStream) {
	let fn_name = &original_fn.sig.ident;
	let original_fn_name = quote::format_ident!("{}_original", fn_name);
	let fn_attrs: Vec<_> = original_fn
		.attrs
		.iter()
		.filter(|attr| !attr.path().is_ident("inject"))
		.collect();
	let output = &original_fn.sig.output;
	let fn_block = &original_fn.block;
	let asyncness = &original_fn.sig.asyncness;

	// Build original function parameters (without #[inject] attributes)
	let original_inputs: Vec<_> = original_fn
		.sig
		.inputs
		.iter()
		.map(|arg| {
			if let FnArg::Typed(pat_type) = arg {
				let mut pat_type = pat_type.clone();
				pat_type.attrs.retain(|attr| !is_inject_attr(attr));
				FnArg::Typed(pat_type)
			} else {
				arg.clone()
			}
		})
		.collect();

	// Generate DI context extraction
	let di_context_extraction = if !inject_params.is_empty() {
		quote! {
			let __di_ctx = req.get_di_context::<::std::sync::Arc<::reinhardt::reinhardt_di::InjectionContext>>()
				.ok_or_else(|| ::reinhardt::reinhardt_core::exception::Error::Internal(
					"DI context not set. Ensure the router is configured with .with_di_context()".to_string()
				))?;
		}
	} else {
		quote! {}
	};

	// Generate injection calls
	let injection_calls: Vec<_> = inject_params
		.iter()
		.map(|param| {
			let pat = &param.pat;
			let ty = &param.ty;
			let use_cache = param.options.use_cache;

			if use_cache {
				quote! {
					let #pat: #ty = ::reinhardt::reinhardt_di::Injected::<#ty>::resolve(&__di_ctx)
						.await
						.map_err(|e| ::reinhardt::reinhardt_core::exception::Error::Internal(
							format!("Dependency injection failed for {}: {:?}", stringify!(#ty), e)
						))?
						.into_inner();
				}
			} else {
				quote! {
					let #pat: #ty = ::reinhardt::reinhardt_di::Injected::<#ty>::resolve_uncached(&__di_ctx)
						.await
						.map_err(|e| ::reinhardt::reinhardt_core::exception::Error::Internal(
							format!("Dependency injection failed for {}: {:?}", stringify!(#ty), e)
						))?
						.into_inner();
				}
			}
		})
		.collect();

	// Generate extractor calls
	let extractor_calls: Vec<_> = extractors
		.iter()
		.map(|ext| {
			let pat = &ext.pat;
			let ty = &ext.ty;
			quote! {
				let #pat = <#ty as ::reinhardt::reinhardt_params::FromRequest>::from_request(&req, &ctx)
					.await
					.map_err(|e| ::reinhardt::reinhardt_core::exception::Error::Validation(
						format!("Parameter extraction failed: {:?}", e)
					))?;
			}
		})
		.collect();

	// Build call arguments (extractors first, then inject params)
	let extractor_args: Vec<_> = extractors.iter().map(|ext| &ext.pat).collect();
	let inject_args: Vec<_> = inject_params.iter().map(|param| &param.pat).collect();

	// Generate code
	(
		quote! {
			// Original function (renamed, private)
			#(#fn_attrs)*
			#asyncness fn #original_fn_name(#(#original_inputs),*) #output {
				#fn_block
			}
		},
		quote! {
			// Build ParamContext for extractors
			let ctx = ::reinhardt::reinhardt_params::ParamContext::with_path_params(req.path_params.clone());

			// Extract DI context (if needed)
			#di_context_extraction

			// Resolve injected dependencies
			#(#injection_calls)*

			// Extract request parameters
			#(#extractor_calls)*

			// Call the original function
			#original_fn_name(#(#extractor_args,)* #(#inject_args),*).await
		},
	)
}

/// Generate View type and factory function
fn generate_view_type(
	input: &ItemFn,
	method: &str,
	path: &str,
	route_name: &str,
	extractors: &[ExtractorInfo],
	inject_params: &[InjectInfo],
) -> Result<TokenStream> {
	let fn_name = &input.sig.ident;
	let fn_vis = &input.vis;
	let fn_attrs: Vec<_> = input
		.attrs
		.iter()
		.filter(|attr| !attr.path().is_ident("inject"))
		.collect();
	let output = &input.sig.output;
	let asyncness = &input.sig.asyncness;

	let view_type_name =
		syn::Ident::new(&fn_name_to_view_type(&fn_name.to_string()), fn_name.span());
	let method_ident = syn::Ident::new(method, Span::call_site());

	// Generate wrapper parts
	let (original_fn, wrapper_body) = generate_wrapper_with_both(input, extractors, inject_params);

	let route_doc = format!("Route: {} {}", method, path);

	Ok(quote! {
		#original_fn

		/// View type for route registration
		#[doc = #route_doc]
		#fn_vis struct #view_type_name;

		impl ::reinhardt::reinhardt_core::endpoint::EndpointInfo for #view_type_name {
			fn path() -> &'static str {
				#path
			}

			fn method() -> ::hyper::Method {
				::hyper::Method::#method_ident
			}

			fn name() -> &'static str {
				#route_name
			}
		}

		#[::reinhardt::async_trait::async_trait]
		impl ::reinhardt::reinhardt_core::Handler for #view_type_name {
			async fn handle(&self, req: ::reinhardt::Request) -> ::reinhardt::reinhardt_http::Result<::reinhardt::Response> {
				#view_type_name::#fn_name(req).await
			}
		}

		impl #view_type_name {
			/// Handler function for this view
			#(#fn_attrs)*
			#fn_vis #asyncness fn #fn_name(req: ::reinhardt::Request) #output {
				#wrapper_body
			}
		}

		/// Factory function for endpoint registration
		///
		/// Returns the View type for use with `UnifiedRouter::endpoint()`
		#fn_vis fn #fn_name() -> #view_type_name {
			#view_type_name
		}
	})
}

fn route_impl(method: &str, args: TokenStream, input: ItemFn) -> Result<TokenStream> {
	let mut path: Option<(String, Span)> = None;
	let mut options = RouteOptions::default();

	// Handle the common case: #[get("/users/{id}")]
	// Try to parse as a single string literal first
	if let Ok(lit) = syn::parse2::<LitStr>(args.clone()) {
		let path_str = lit.value();
		validate_route_path(&path_str, lit.span())?;
		path = Some((path_str, lit.span()));
	} else {
		// Parse path and options: #[get("/path", use_inject = true)]
		let parser = Punctuated::<Expr, Token![,]>::parse_terminated;
		if let Ok(exprs) = parser.parse2(args.clone()) {
			for (i, expr) in exprs.iter().enumerate() {
				match expr {
					// First argument: path string literal
					Expr::Lit(ExprLit {
						lit: Lit::Str(lit), ..
					}) if i == 0 => {
						let path_str = lit.value();
						validate_route_path(&path_str, lit.span())?;
						path = Some((path_str, lit.span()));
					}
					// use_inject = true/false or name = "xxx"
					Expr::Assign(assign) => {
						if let Expr::Path(path_expr) = &*assign.left {
							if path_expr.path.is_ident("use_inject") {
								if let Expr::Lit(ExprLit {
									lit: Lit::Bool(bool_lit),
									..
								}) = &*assign.right
								{
									options.use_inject = bool_lit.value;
								} else {
									return Err(Error::new_spanned(
										&assign.right,
										"use_inject must be a boolean (true or false)",
									));
								}
							} else if path_expr.path.is_ident("name") {
								if let Expr::Lit(ExprLit {
									lit: Lit::Str(str_lit),
									..
								}) = &*assign.right
								{
									options.name = Some(str_lit.value());
								} else {
									return Err(Error::new_spanned(
										&assign.right,
										"name must be a string literal",
									));
								}
							}
						}
					}
					_ => {}
				}
			}
		} else {
			// Fallback: try parsing as Meta for backwards compatibility
			let meta_list = Punctuated::<Meta, Token![,]>::parse_terminated.parse2(args)?;

			for meta in meta_list {
				match meta {
					Meta::Path(p) => {
						if let Some(ident) = p.get_ident() {
							let path_str = ident.to_string();
							validate_route_path(&path_str, p.span())?;
							path = Some((path_str, p.span()));
						}
					}
					Meta::NameValue(nv) if nv.path.is_ident("path") => {
						if let Expr::Lit(ExprLit {
							lit: Lit::Str(lit), ..
						}) = &nv.value
						{
							let path_str = lit.value();
							validate_route_path(&path_str, lit.span())?;
							path = Some((path_str, lit.span()));
						}
					}
					_ => {}
				}
			}
		}
	}

	// Detect extractors
	let extractors = detect_extractors(&input.sig.inputs);

	// Detect inject params (always detect for error checking)
	let all_inject_params = detect_inject_params(&input.sig.inputs);

	// Error if use_inject = false and #[inject] parameters exist
	if !options.use_inject && !all_inject_params.is_empty() {
		let first_inject = &all_inject_params[0];
		return Err(Error::new_spanned(
			&first_inject.pat,
			"#[inject] attribute requires use_inject = true option. \
			 Usage: #[get(\"/path\", use_inject = true)]",
		));
	}

	// Use inject params only when use_inject = true
	let inject_params = if options.use_inject {
		all_inject_params
	} else {
		Vec::new()
	};

	// Validate extractors
	if !extractors.is_empty() {
		validate_extractors(&extractors)?;
	}

	// If we have extractors or inject params, generate View type
	if !extractors.is_empty() || !inject_params.is_empty() {
		let path_str = path
			.as_ref()
			.map(|(p, _)| p.clone())
			.unwrap_or_else(|| "/".to_string());
		let route_name = options
			.name
			.clone()
			.unwrap_or_else(|| input.sig.ident.to_string());

		return generate_view_type(
			&input,
			method,
			&path_str,
			&route_name,
			&extractors,
			&inject_params,
		);
	}

	// Simple case: no extractors, no inject - generate View type with EndpointInfo + Handler
	let fn_name = &input.sig.ident;
	let fn_block = &input.block;
	let fn_inputs = &input.sig.inputs;
	let fn_output = &input.sig.output;
	let fn_vis = &input.vis;
	let fn_attrs = &input.attrs;
	let asyncness = &input.sig.asyncness;
	let generics = &input.sig.generics;
	let where_clause = &input.sig.generics.where_clause;

	let path_str = path
		.as_ref()
		.map(|(p, _)| p.clone())
		.unwrap_or_else(|| "/".to_string());
	let route_name = options.name.clone().unwrap_or_else(|| fn_name.to_string());
	let view_type_name =
		syn::Ident::new(&fn_name_to_view_type(&fn_name.to_string()), fn_name.span());
	let method_ident = syn::Ident::new(method, Span::call_site());
	let original_fn_name = quote::format_ident!("{}_original", fn_name);

	let route_doc = format!("Route: {} {}", method, path_str);

	// Determine if the original function takes a Request parameter
	let has_request_param = !fn_inputs.is_empty();

	// Wrapper function signature and body based on whether original takes request
	let (wrapper_sig, wrapper_body) = if has_request_param {
		(
			quote! { req: ::reinhardt::Request },
			quote! { #original_fn_name(req).await },
		)
	} else {
		(
			quote! { _req: ::reinhardt::Request },
			quote! { #original_fn_name().await },
		)
	};

	Ok(quote! {
		// Original function (renamed, private)
		#(#fn_attrs)*
		#asyncness fn #original_fn_name #generics (#fn_inputs) #fn_output #where_clause {
			#fn_block
		}

		/// View type for route registration
		#[doc = #route_doc]
		#fn_vis struct #view_type_name;

		impl ::reinhardt::reinhardt_core::endpoint::EndpointInfo for #view_type_name {
			fn path() -> &'static str {
				#path_str
			}

			fn method() -> ::hyper::Method {
				::hyper::Method::#method_ident
			}

			fn name() -> &'static str {
				#route_name
			}
		}

		#[::reinhardt::async_trait::async_trait]
		impl ::reinhardt::reinhardt_core::Handler for #view_type_name {
			async fn handle(&self, req: ::reinhardt::Request) -> ::reinhardt::reinhardt_http::Result<::reinhardt::Response> {
				#view_type_name::#fn_name(req).await
			}
		}

		impl #view_type_name {
			/// Handler function for this view
			#(#fn_attrs)*
			#fn_vis #asyncness fn #fn_name(#wrapper_sig) #fn_output {
				#wrapper_body
			}
		}

		/// Factory function for endpoint registration
		///
		/// Returns the View type for use with `UnifiedRouter::endpoint()`
		#fn_vis fn #fn_name() -> #view_type_name {
			#view_type_name
		}
	})
}

/// Implementation of GET route macro
pub fn get_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
	route_impl("GET", args, input)
}

/// Implementation of POST route macro
pub fn post_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
	route_impl("POST", args, input)
}

/// Implementation of PUT route macro
pub fn put_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
	route_impl("PUT", args, input)
}

/// Implementation of PATCH route macro
pub fn patch_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
	route_impl("PATCH", args, input)
}

/// Implementation of DELETE route macro
pub fn delete_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
	route_impl("DELETE", args, input)
}
