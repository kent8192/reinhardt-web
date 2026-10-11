//! Routes attribute macro implementation
//!
//! This module implements the `#[routes]` attribute macro that registers
//! a function as the URL pattern provider for automatic discovery by
//! the framework via `inventory`.
//!
//! # Important: Single Usage Only
//!
//! **Only one function per project can be annotated with `#[routes]`.**
//! If multiple `#[routes]` attributes are used, the linker will fail with a
//! "duplicate symbol" error for `__reinhardt_routes_registration_marker`.
//!
//! To organize routes across multiple files, use the `.mount()` method:
//!
//! ```rust,ignore
//! // Only ONE function in the project should have #[routes]
//! #[routes]
//! pub fn routes() -> UnifiedRouter {
//!     UnifiedRouter::new()
//!         .mount("/api/", api::routes())   // api::routes() is NOT annotated with #[routes]
//!         .mount("/admin/", admin::routes())
//!         .client(|c| c.route("/", home_page))
//! }
//! ```
//!
//! Generated Pages applications normally keep one aggregate per app and merge
//! those values in the single project-level `#[routes]` function. The aggregate
//! can contain HTTP, WebSocket, gRPC, and client routes; `manage runserver`
//! materializes the native parts with one DI context.
//!
//! # Supported Function Signatures
//!
//! The macro supports three function forms:
//!
//! ## 1. Sync function (no `#[inject]`)
//!
//! ```rust,ignore
//! #[routes]
//! pub fn routes() -> UnifiedRouter {
//!     UnifiedRouter::new()
//! }
//! ```
//!
//! ## 2. Async function (no `#[inject]`)
//!
//! ```rust,ignore
//! #[routes]
//! pub async fn routes() -> UnifiedRouter {
//!     UnifiedRouter::new()
//! }
//! ```
//!
//! ## 3. Async function with `#[inject]` parameters
//!
//! ```rust,ignore
//! #[routes]
//! pub async fn routes(#[inject] router: UnifiedRouter) -> UnifiedRouter {
//!     router
//! }
//! ```
//!
//! # Target Behavior
//!
//! Only the synchronous form (1) registers routes on every target: native
//! builds register the server aggregate and `wasm32-unknown-unknown` builds
//! register the client router in `ClientRouterRegistration`. The async forms
//! (2 and 3) are native-only because the annotated function is gated out of
//! wasm builds, so a wasm build would otherwise start with an empty client
//! route table. To keep that failure out of the browser, the macro emits a
//! `compile_error!` for async functions under the wasm cfg. Keep `routes()`
//! synchronous and move native-only async setup behind a native cfg to
//! build for both targets. Client registration from an async function is
//! tracked in issue #6733.
//!
//! # Generated Code
//!
//! The macro preserves the original function and adds:
//! 1. `inventory::submit!(UrlPatternsRegistration)` registration
//! 2. A linker marker symbol to enforce single-usage
//!
//! Native sync functions register a `RouterFactory::NativeSync`; native async
//! functions register `RouterFactory::NativeAsync`, which returns a
//! `Pin<Box<dyn Future>>` wrapping the complete route aggregate.

use crate::crate_paths::{get_reinhardt_crate, get_reinhardt_di_crate};
use crate::injectable_common::generate_inject_resolver_expr;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, PatType, Result, spanned::Spanned};

/// Diagnostic emitted for async `#[routes]` functions under the wasm cfg.
///
/// The async expansions are native-only, so without this error a wasm build
/// would silently register no client routes (Issue #6722).
const ASYNC_WASM_DIAGNOSTIC: &str = "`#[routes]` on an `async fn` cannot register client routes on wasm32-unknown-unknown; client route discovery needs a synchronous function. Keep `routes()` synchronous and move async setup behind a native-only cfg, or track async client registration in issue #6733.";

/// Check if an attribute is `#[inject]`
fn is_inject_attr(attr: &syn::Attribute) -> bool {
	attr.path().is_ident("inject")
}

/// Implementation of the `#[routes]` attribute macro
///
/// This function generates code that:
/// 1. Preserves the original function definition
/// 2. Adds `inventory::submit!` to register the function with the framework
/// 3. Emits a linker marker to enforce single-usage
/// 4. Emits a wasm-only `compile_error!` for `async` functions, which cannot
///    register client routes
///
/// The `#[routes]` macro no longer accepts arguments. All per-app struct
/// generation, `ResolvedUrls`, `url_prelude`, callback macros, and flags
/// (`standalone`, `server_only`, `no_client_resolvers`, `no_ws_resolvers`,
/// `client_inventory`) have been removed as part of the URL routing
/// infrastructure simplification (Issue #4784).
///
/// # Parameters
///
/// * `args` - Must be empty. Any arguments produce a compile error.
/// * `input` - The function to annotate
///
/// # Returns
///
/// Generated code as a `TokenStream`
///
/// # Errors
///
/// Returns an error if:
/// - The macro receives any arguments
/// - The function has no return type
/// - A sync function uses `#[inject]` parameters
pub(crate) fn routes_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
	// The #[routes] macro no longer accepts arguments.
	if !args.is_empty() {
		return Err(syn::Error::new_spanned(
			args,
			"The #[routes] macro no longer accepts arguments. \
			 The flags `standalone`, `client_inventory`, `server_only`, \
			 `no_client_resolvers`, and `no_ws_resolvers` have been removed \
			 as part of the URL routing simplification (Issue #4784).",
		));
	}

	let reinhardt = get_reinhardt_crate();

	let fn_name = &input.sig.ident;
	let fn_vis = &input.vis;
	let fn_attrs = &input.attrs;
	let fn_block = &input.block;

	// Validate that the function has a return type
	if matches!(input.sig.output, syn::ReturnType::Default) {
		return Err(syn::Error::new_spanned(
			&input.sig,
			"#[routes] function must have a return type (-> UnifiedRouter)",
		));
	}

	let is_async = input.sig.asyncness.is_some();

	// Analyze function parameters for #[inject]
	let mut inject_params = Vec::new();
	let mut has_inject = false;

	for arg in &input.sig.inputs {
		if let FnArg::Typed(PatType { attrs, pat, ty, .. }) = arg
			&& attrs.iter().any(is_inject_attr)
		{
			has_inject = true;
			let resolved_ident = syn::Ident::new(
				&format!("__reinhardt_injected_{}", inject_params.len()),
				pat.span(),
			);
			inject_params.push((pat.clone(), ty.clone(), resolved_ident));
		}
	}

	// Sync + #[inject] is not supported (DI resolution is inherently async)
	if !is_async && has_inject {
		return Err(syn::Error::new_spanned(
			&input.sig,
			"Sync #[routes] functions cannot use #[inject] parameters. \
			 Make the function async to use dependency injection.",
		));
	}

	// Native-only cfg gate for the server-side inventory::submit! block
	// (references ServerRouter and DI types that do not exist on wasm).
	let native_only = quote! {
		#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
	};

	// WASM-only cfg gate for the client-side ClientRouterRegistration submit.
	let wasm_only = quote! {
		#[cfg(all(target_family = "wasm", target_os = "unknown"))]
	};

	let reinhardt_urls = crate::crate_paths::get_reinhardt_urls_crate();

	// Async forms only register native routes. Reject them on wasm at compile
	// time, pointing at the `async` keyword, instead of silently launching the
	// SPA with an empty client route table (Issue #6722).
	let async_wasm_error =
		syn::Error::new_spanned(input.sig.asyncness, ASYNC_WASM_DIAGNOSTIC).to_compile_error();

	let expanded = if !is_async {
		// Case 1: Sync, no #[inject]
		let fn_sig = &input.sig;
		quote! {
			#[allow(private_interfaces)]
			#(#fn_attrs)*
			#fn_vis #fn_sig #fn_block

			#native_only
			#[allow(unsafe_attr_outside_unsafe)]
			const _: () = {
				fn __get_native_routes() -> #reinhardt_urls::routers::NativeRoutes {
					let unified = #fn_name();
					unified.__into_native_routes()
				}

				#reinhardt::inventory::submit! {
					#reinhardt::UrlPatternsRegistration::__macro_new_native(__get_native_routes)
				}
			};

			// Linker marker to enforce single #[routes] usage.
			#native_only
			#[doc(hidden)]
			#[unsafe(no_mangle)]
			#[allow(non_upper_case_globals, dead_code)]
			// non_upper_case_globals: Intentionally lowercase for linker symbol
			// dead_code: Symbol is never directly used, only exists for linker validation
			static __reinhardt_routes_registration_marker: () = ();

			// WASM: submit ClientRouterRegistration so inventory-based
			// route discovery works on wasm32-unknown-unknown (Refs #4853).
			#wasm_only
			#[allow(unsafe_attr_outside_unsafe)]
			const _: () = {
				fn __get_client_router() -> ::std::sync::Arc<#reinhardt_urls::routers::ClientRouter> {
					let unified = #fn_name();
					::std::sync::Arc::new(unified.into_client())
				}

				#reinhardt_urls::inventory::submit! {
					#reinhardt_urls::routers::ClientRouterRegistration::__macro_new(__get_client_router)
				}
			};
		}
	} else if !has_inject {
		// Case 2: Async, no #[inject]
		let fn_sig = &input.sig;
		quote! {
			#native_only
			#[allow(private_interfaces)]
			#(#fn_attrs)*
			#fn_vis #fn_sig #fn_block

			#native_only
			#[allow(unsafe_attr_outside_unsafe)]
			const _: () = {
				fn __get_native_routes() -> ::std::pin::Pin<
					::std::boxed::Box<
						dyn ::std::future::Future<
								Output = ::std::result::Result<
									#reinhardt_urls::routers::NativeRoutes,
									::std::boxed::Box<dyn ::std::error::Error + Send + Sync>,
								>,
							> + Send,
					>,
				> {
					::std::boxed::Box::pin(async {
						let unified = #fn_name().await;
						::std::result::Result::Ok(unified.__into_native_routes())
					})
				}

				#reinhardt::inventory::submit! {
					#reinhardt::UrlPatternsRegistration::__macro_new_native_async(__get_native_routes)
				}
			};

			#native_only
			#[doc(hidden)]
			#[unsafe(no_mangle)]
			#[allow(non_upper_case_globals, dead_code)]
			static __reinhardt_routes_registration_marker: () = ();

			// WASM: async functions cannot register client routes.
			#wasm_only
			#async_wasm_error
		}
	} else {
		// Case 3: Async, with #[inject]
		let di_crate = get_reinhardt_di_crate();

		// Generate dependency resolution code
		let inject_resolutions: Vec<_> = inject_params
			.iter()
			.map(|(_, ty, resolved_ident)| {
				let resolve_expr =
					generate_inject_resolver_expr(&di_crate, ty, quote! { &*__ctx }, true);
				quote! {
					let #resolved_ident: #ty = #resolve_expr
						.map_err(|e| -> ::std::boxed::Box<dyn ::std::error::Error + Send + Sync> {
							::std::boxed::Box::new(e)
						})?;
				}
			})
			.collect();

		// Generate parameter names for the call
		let inject_param_names: Vec<_> = inject_params
			.iter()
			.map(|(_, _, resolved_ident)| resolved_ident)
			.collect();

		// Strip #[inject] from original function params
		let fn_return = &input.sig.output;
		let fn_generics = &input.sig.generics;
		let stripped_params: Vec<_> = input
			.sig
			.inputs
			.iter()
			.map(|arg| {
				if let FnArg::Typed(pat_type) = arg {
					let attrs: Vec<_> = pat_type
						.attrs
						.iter()
						.filter(|a| !is_inject_attr(a))
						.collect();
					let pat = &pat_type.pat;
					let ty = &pat_type.ty;
					quote! { #(#attrs)* #pat: #ty }
				} else {
					quote! { #arg }
				}
			})
			.collect();

		quote! {
			#native_only
			#[allow(private_interfaces)]
			#(#fn_attrs)*
			#fn_vis async fn #fn_name #fn_generics(#(#stripped_params),*) #fn_return #fn_block

			#native_only
			#[allow(unsafe_attr_outside_unsafe)]
			const _: () = {
				fn __get_native_routes() -> ::std::pin::Pin<
					::std::boxed::Box<
						dyn ::std::future::Future<
								Output = ::std::result::Result<
									#reinhardt_urls::routers::NativeRoutes,
									::std::boxed::Box<dyn ::std::error::Error + Send + Sync>,
								>,
							> + Send,
					>,
				> {
					::std::boxed::Box::pin(async {
						// Create DI context for resolving #[inject] parameters
						let __scope = ::std::sync::Arc::new(
							#di_crate::SingletonScope::new()
						);
						let __ctx = ::std::sync::Arc::new(
							#di_crate::InjectionContext::builder(__scope).build()
						);

						// Resolve #[inject] dependencies
						#(#inject_resolutions)*

						let unified = #fn_name(#(#inject_param_names),*).await;
						unified
							.__into_native_routes()
							.__with_di_context(__ctx)
							.map_err(|error| -> ::std::boxed::Box<dyn ::std::error::Error + Send + Sync> {
								::std::boxed::Box::new(error)
							})
					})
				}

				#reinhardt::inventory::submit! {
					#reinhardt::UrlPatternsRegistration::__macro_new_native_async(__get_native_routes)
				}
			};

			#native_only
			#[doc(hidden)]
			#[unsafe(no_mangle)]
			#[allow(non_upper_case_globals, dead_code)]
			static __reinhardt_routes_registration_marker: () = ();

			// WASM: async functions cannot register client routes.
			#wasm_only
			#async_wasm_error
		}
	};

	Ok(expanded)
}

#[cfg(test)]
mod tests {
	use super::*;
	use quote::ToTokens;
	use rstest::rstest;
	use syn::{Item, parse_quote};

	fn expand(input: ItemFn) -> TokenStream {
		routes_impl(TokenStream::new(), input).expect("routes expansion must succeed")
	}

	fn expansion(input: ItemFn) -> String {
		expand(input).to_string()
	}

	/// Messages of the top-level `compile_error!` items that are gated on the
	/// wasm cfg, so the assertions cannot be satisfied by an unconditional
	/// error or by a message that only appears inside another item.
	fn wasm_gated_compile_errors(generated: TokenStream) -> Vec<String> {
		let wasm_cfg: syn::Attribute =
			parse_quote!(#[cfg(all(target_family = "wasm", target_os = "unknown"))]);
		let file: syn::File = syn::parse2(generated).expect("expansion must parse as items");

		file.items
			.into_iter()
			.filter_map(|item| match item {
				Item::Macro(item_macro)
					if item_macro
						.mac
						.path
						.segments
						.last()
						.is_some_and(|segment| segment.ident == "compile_error")
						&& item_macro.attrs.iter().any(|attr| {
							attr.to_token_stream().to_string()
								== wasm_cfg.to_token_stream().to_string()
						}) =>
				{
					let message: syn::LitStr = syn::parse2(item_macro.mac.tokens)
						.expect("compile_error! must carry a string literal");
					Some(message.value())
				}
				_ => None,
			})
			.collect()
	}

	#[test]
	fn native_expansions_extract_the_complete_aggregate() {
		let sync = expansion(parse_quote! {
			fn routes() -> UnifiedRouter { UnifiedRouter::new() }
		});
		let asynchronous = expansion(parse_quote! {
			async fn routes() -> UnifiedRouter { UnifiedRouter::new() }
		});
		let di_aware = expansion(parse_quote! {
			async fn routes(#[inject] value: Arc<Value>) -> UnifiedRouter {
				let _ = value;
				UnifiedRouter::new()
			}
		});

		for generated in [&sync, &asynchronous, &di_aware] {
			assert_eq!(generated.matches("__into_native_routes").count(), 1);
		}
		assert_eq!(sync.matches("SingletonScope :: new").count(), 0);
		assert_eq!(asynchronous.matches("SingletonScope :: new").count(), 0);
		assert_eq!(di_aware.matches("SingletonScope :: new").count(), 1);
		assert_eq!(di_aware.matches("__with_di_context").count(), 1);
	}

	#[test]
	fn wasm_sync_expansion_keeps_client_inventory_extraction() {
		let generated = expansion(parse_quote! {
			fn routes() -> UnifiedRouter { UnifiedRouter::new() }
		});

		assert_eq!(generated.matches("ClientRouterRegistration").count(), 1);
		assert_eq!(generated.matches("into_client").count(), 1);
	}

	#[test]
	fn sync_expansion_has_no_compile_error() {
		// Arrange
		let input: ItemFn = parse_quote! {
			fn routes() -> UnifiedRouter { UnifiedRouter::new() }
		};

		// Act
		let generated = expand(input);

		// Assert
		assert_eq!(generated.to_string().matches("compile_error").count(), 0);
		assert_eq!(wasm_gated_compile_errors(generated), Vec::<String>::new());
	}

	#[rstest]
	#[case::async_without_inject(parse_quote! {
		pub async fn routes() -> UnifiedRouter { UnifiedRouter::new() }
	})]
	#[case::async_with_inject(parse_quote! {
		pub async fn routes(#[inject] value: Arc<Value>) -> UnifiedRouter {
			let _ = value;
			UnifiedRouter::new()
		}
	})]
	fn async_expansion_rejects_wasm_with_diagnostic(#[case] input: ItemFn) {
		// Arrange
		// (the annotated function is supplied by the case)

		// Act
		let generated = expand(input);
		let rendered = generated.to_string();
		let errors = wasm_gated_compile_errors(generated);

		// Assert
		assert_eq!(errors, vec![ASYNC_WASM_DIAGNOSTIC.to_string()]);
		assert_eq!(rendered.matches("compile_error").count(), 1);
		assert_eq!(rendered.matches("ClientRouterRegistration").count(), 0);
	}

	#[test]
	fn async_diagnostic_names_the_cause_and_the_remedy() {
		// Arrange
		let message = ASYNC_WASM_DIAGNOSTIC;

		// Act
		let names_cause =
			message.contains("`async fn`") && message.contains("wasm32-unknown-unknown");
		let names_remedy = message.contains("synchronous");

		// Assert
		assert!(
			names_cause,
			"diagnostic must name the async cause: {message}"
		);
		assert!(names_remedy, "diagnostic must name the remedy: {message}");
		assert_eq!(message.lines().count(), 1);
	}
}
