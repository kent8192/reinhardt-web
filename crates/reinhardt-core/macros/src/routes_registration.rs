//! Routes attribute macro implementation
//!
//! This module implements the `#[routes]` attribute macro that allows
//! functions to be registered as URL pattern providers for automatic
//! discovery by the framework.
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
//! # Generated Code
//!
//! The macro preserves the original function and adds `inventory::submit!`
//! registration code. The generated code is feature-independent to avoid
//! feature context mismatches between the library and downstream crates.
//!
//! For sync functions, a sync `RouterFactory::Sync` is registered.
//! For async functions, an async `RouterFactory::Async` is registered,
//! which returns a `Pin<Box<dyn Future>>` wrapping the async call.
//!
//! # Migration from rc.18 (breaking change in rc.19)
//!
//! The WebSocket resolver location moved from
//! `crate::apps::<app>::ws_urls::ws_url_resolvers` to
//! `crate::apps::<app>::urls::ws_urls::ws_url_resolvers`. To migrate:
//!
//! ```bash
//! mkdir -p src/apps/<app>/urls
//! git mv src/apps/<app>/ws_urls.rs src/apps/<app>/urls/ws_urls.rs
//! ```
//!
//! Then declare the submodule in `urls.rs`:
//!
//! ```rust,ignore
//! #[cfg(server)]
//! pub mod ws_urls;
//! ```
//!
//! See <https://github.com/kent8192/reinhardt-web/issues/3914>.

use crate::crate_paths::{get_reinhardt_crate, get_reinhardt_di_crate};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, Pat, PatType, Result};

/// Maximum number of URL parameters supported in typed resolver methods.
const MAX_URL_PARAMS: usize = 5;

/// Generate the `macro_rules!` arms for a per-app URL resolver callback macro.
///
/// Produces arms for 0 through `MAX_URL_PARAMS` parameters. Each arm generates
/// an `impl` block on `struct_ident` with a typed method that calls
/// `resolve_call` with the appropriate parameter list.
///
/// # Parameters
///
/// * `struct_ident` — the resolver struct (e.g., `PollsUrls`)
/// * `resolve_call` — the method call expression for URL resolution
/// * `name_prefix` — the route name prefix expression (e.g.,
///   `stringify!($app_label)` for server, or a string literal for client)
/// * `use_clause` — optional trait import (e.g., `use reinhardt::UrlResolver as _`)
fn gen_resolver_callback_arms(
	struct_ident: &proc_macro2::Ident,
	resolve_call: &TokenStream,
	name_prefix: &TokenStream,
	use_clause: &TokenStream,
) -> TokenStream {
	let arms: Vec<TokenStream> = (0..=MAX_URL_PARAMS)
		.map(|n| {
			let param_matchers: Vec<TokenStream> = (1..=n)
				.map(|i| {
					let p = format_ident!("p{}", i);
					quote! { $#p:literal }
				})
				.collect();

			let fn_params: Vec<TokenStream> = (1..=n)
				.map(|i| {
					let p = format_ident!("p{}", i);
					quote! { #p: &str }
				})
				.collect();

			let pairs: Vec<TokenStream> = (1..=n)
				.map(|i| {
					let p = format_ident!("p{}", i);
					quote! { ($#p, #p) }
				})
				.collect();

			quote! {
				($app_label:ident, $method:ident, $route_name:literal, #(#param_matchers),*) => {
					impl #struct_ident<'_> {
						pub fn $method(&self #(, #fn_params)*) -> String {
							#use_clause
							self.resolver.#resolve_call(
								concat!(#name_prefix, ":", $route_name),
								&[#(#pairs),*],
							)
						}
					}
				};
			}
		})
		.collect();

	quote! { #(#arms)* }
}

/// Check if an attribute is `#[inject]`
fn is_inject_attr(attr: &syn::Attribute) -> bool {
	attr.path().is_ident("inject")
}

/// Extract the inner type `T` from `Depends<T>`.
///
/// Returns `Some(T)` if the type is `Depends<T>`, `None` otherwise.
/// A sibling copy lives in `crates/reinhardt-pages/macros/src/server_fn.rs`;
/// the two proc-macro crates cannot share code directly, so keep both copies
/// in sync.
pub(crate) fn extract_depends_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
	if let syn::Type::Path(type_path) = ty {
		let last_segment = type_path.path.segments.last()?;
		if last_segment.ident == "Depends"
			&& let syn::PathArguments::AngleBracketed(args) = &last_segment.arguments
			&& args.args.len() == 1
			&& let syn::GenericArgument::Type(inner) = args.args.first()?
		{
			return Some(inner);
		}
	}
	None
}

/// Implementation of the `#[routes]` attribute macro
///
/// This function generates code that:
/// 1. Preserves the original function definition
/// 2. Adds `inventory::submit!` to register the function with the framework
///
/// The macro generates feature-independent code that only registers the
/// server router. The client router is handled within the library's own
/// feature-gated code via `with_client_router()`, avoiding the problem
/// where `#[cfg(feature = "client-router")]` in macro output would be
/// evaluated in the downstream crate's feature context.
///
/// # Supported cases
///
/// | Case | Detection | Generated wrapper |
/// |------|-----------|-------------------|
/// | Sync, no `#[inject]` | `!async && no inject` | `RouterFactory::Sync` (unchanged) |
/// | Async, no `#[inject]` | `async && no inject` | `RouterFactory::Async` wrapper |
/// | Async, with `#[inject]` | `async && has inject` | `RouterFactory::Async` with DI context |
///
/// Sync + `#[inject]` produces a compile error.
///
/// # Parameters
///
/// * `args` - Attribute arguments. Accepts a comma-separated list of flags:
///   - `standalone`: skip URL resolver generation (for projects that don't use
///     `installed_apps!`)
///   - `client_inventory`: opt into the WASM cross-target `ClientRouter`
///     `inventory::submit!` registration
///   - `server_only`: shorthand for `no_client_resolvers, no_ws_resolvers`;
///     for REST-only apps that don't have `client_url_resolvers` or
///     `ws_url_resolvers` modules per app (Issue #4509)
///   - `no_client_resolvers`: skip per-app client resolver lookups; the
///     `ClientUrls` gateway and `<app>ClientUrls` structs are not emitted
///   - `no_ws_resolvers`: skip per-app WebSocket resolver lookups; the
///     `WsUrls` gateway and `<app>WsUrls` structs are not emitted, and the
///     stub `WebSocketUrlResolver` impl is suppressed
///
///   `client_inventory` is mutually exclusive with the `no_*` flags
///   (`client_inventory` exists precisely to register the client surface
///   the suppression flags disable).
/// * `input` - The function to annotate
///
/// # Returns
///
/// Generated code as a `TokenStream`
///
/// # Errors
///
/// Returns an error if the function signature is invalid (e.g., missing return type,
/// sync function with `#[inject]` parameters)
pub(crate) fn routes_impl(args: TokenStream, input: ItemFn) -> Result<TokenStream> {
	// Parse comma-separated arguments: `standalone`, `client_inventory`,
	// `server_only`, `no_client_resolvers`, `no_ws_resolvers`.
	//
	// - `standalone` (existing): skip per-app URL-resolver generation and the
	//   `url_prelude` module for projects that do not use `installed_apps!`.
	// - `client_inventory` (#4453): opt into the cross-target macro
	//   expansion. Drops the `native_only` gate from the user function and
	//   linker marker, and emits a WASM-only `ClientRouterRegistration`
	//   `inventory::submit!` block consumed by
	//   `ClientLauncher::register_routes_from_inventory()`. The user body
	//   MUST compile on `wasm32-unknown-unknown` (closure-style
	//   `.server(|s| ...).client(|c| ...)` is the recommended shape;
	//   `UnifiedRouter::new().mount(server_router())` does NOT compile on
	//   wasm and should stay with plain `#[routes]` or
	//   `#[routes(standalone)]` without `client_inventory`).
	// - `server_only` (#4509): shorthand for `no_client_resolvers,
	//   no_ws_resolvers`. Lets REST-only apps use `#[routes]` (rather than
	//   `#[routes(standalone)]`) without supplying per-app
	//   `client_url_resolvers` / `ws_url_resolvers` modules. The
	//   `ResolvedUrls::<app>()` server-side accessor is preserved.
	// - `no_client_resolvers` (#4509): skip the client-side per-app
	//   resolver lookups (`__for_each_client_url_resolver!`) and the
	//   `ClientUrls` / `<app>ClientUrls` gateway. Apps that do not use
	//   `#[url_patterns(..., mode = client | unified)]` do not need
	//   `client_url_resolvers` modules.
	// - `no_ws_resolvers` (#4509): skip the WebSocket per-app resolver
	//   lookups (`__for_each_ws_url_resolver!`), the `WsUrls` /
	//   `<app>WsUrls` gateway, and the stub `WebSocketUrlResolver` impl.
	//   Apps that do not use `#[url_patterns(..., mode = ws)]` do not
	//   need `ws_urls::ws_url_resolvers` modules.
	//
	// `client_inventory` cannot combine with any `no_*` (or `server_only`)
	// flag, since `client_inventory` registers the very surface those
	// flags suppress.
	//
	// Plain `#[routes]` (no arguments) keeps the pre-#4453 native-only
	// behavior verbatim, so existing consumers see no regression.
	let mut standalone = false;
	let mut client_inventory = false;
	// Track the `Ident` token positions of the flags we accept so downstream
	// diagnostics (e.g. the mutual-exclusion check below) can point at the
	// exact offending argument instead of the function signature.
	let mut client_inventory_ident: Option<syn::Ident> = None;
	let mut server_only_seen = false;
	let mut server_only_ident: Option<syn::Ident> = None;
	let mut no_client_resolvers_seen = false;
	let mut no_client_resolvers_ident: Option<syn::Ident> = None;
	let mut no_ws_resolvers_seen = false;
	let mut no_ws_resolvers_ident: Option<syn::Ident> = None;
	if !args.is_empty() {
		let parser = syn::punctuated::Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated;
		let parsed = syn::parse::Parser::parse2(parser, args).map_err(|e| {
			syn::Error::new(
				e.span(),
				"invalid arguments for #[routes]; expected comma-separated flags from \
				 `standalone`, `client_inventory`, `server_only`, \
				 `no_client_resolvers`, `no_ws_resolvers`",
			)
		})?;
		for ident in parsed {
			if ident == "standalone" {
				if standalone {
					return Err(syn::Error::new_spanned(
						ident,
						"`standalone` specified twice",
					));
				}
				standalone = true;
			} else if ident == "client_inventory" {
				if client_inventory {
					return Err(syn::Error::new_spanned(
						ident,
						"`client_inventory` specified twice",
					));
				}
				client_inventory = true;
				client_inventory_ident = Some(ident);
			} else if ident == "server_only" {
				if server_only_seen {
					return Err(syn::Error::new_spanned(
						ident,
						"`server_only` specified twice",
					));
				}
				server_only_seen = true;
				server_only_ident = Some(ident);
			} else if ident == "no_client_resolvers" {
				if no_client_resolvers_seen {
					return Err(syn::Error::new_spanned(
						ident,
						"`no_client_resolvers` specified twice",
					));
				}
				no_client_resolvers_seen = true;
				no_client_resolvers_ident = Some(ident);
			} else if ident == "no_ws_resolvers" {
				if no_ws_resolvers_seen {
					return Err(syn::Error::new_spanned(
						ident,
						"`no_ws_resolvers` specified twice",
					));
				}
				no_ws_resolvers_seen = true;
				no_ws_resolvers_ident = Some(ident);
			} else {
				return Err(syn::Error::new_spanned(
					ident,
					"unknown argument for #[routes]; expected `standalone`, \
					 `client_inventory`, `server_only`, `no_client_resolvers`, \
					 `no_ws_resolvers`, or no arguments",
				));
			}
		}
	}

	// `server_only` is shorthand: it sets both suppression flags. Idempotent
	// with explicit `no_*` flags (no error if both forms appear).
	let no_client_resolvers = server_only_seen || no_client_resolvers_seen;
	let no_ws_resolvers = server_only_seen || no_ws_resolvers_seen;

	// `client_inventory` registers the WASM `ClientRouter` surface that the
	// `no_*` flags suppress. Combining them is contradictory — fail at parse
	// time with an actionable message rather than emitting unreachable code.
	if client_inventory && (no_client_resolvers || no_ws_resolvers) {
		// Span the diagnostic on the offending `client_inventory` argument
		// (preferred) or the first conflicting suppression flag, so the
		// user is taken straight to one of the contradictory tokens. Fall
		// back to `&input.sig` only when the tokens have been moved out of
		// scope (which the bookkeeping above prevents in practice).
		let err_msg = "`#[routes(client_inventory)]` cannot be combined with `server_only`, \
			 `no_client_resolvers`, or `no_ws_resolvers` — `client_inventory` \
			 registers the WASM `ClientRouter` surface that the suppression flags \
			 disable. Drop one of the flags.";
		if let Some(ident) = client_inventory_ident {
			return Err(syn::Error::new_spanned(ident, err_msg));
		}
		if let Some(ident) = server_only_ident {
			return Err(syn::Error::new_spanned(ident, err_msg));
		}
		if let Some(ident) = no_client_resolvers_ident {
			return Err(syn::Error::new_spanned(ident, err_msg));
		}
		if let Some(ident) = no_ws_resolvers_ident {
			return Err(syn::Error::new_spanned(ident, err_msg));
		}
		return Err(syn::Error::new_spanned(&input.sig, err_msg));
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

	// `client_inventory` is only meaningful for sync `#[routes]` because
	// `inventory::submit!` requires a `const`-constructible registration
	// whose factory is `fn() -> Arc<ClientRouter>` (sync). Driving an async
	// `routes()` body from a sync factory would require a per-target
	// executor stub on `wasm32-unknown-unknown`, which is out of scope for
	// #4453. Async client inventory may be added in a follow-up.
	//
	// Reject the combination at compile time (fail early per DP-4) instead
	// of silently dropping the flag and surprising the user with an empty
	// inventory at `launch()` time. Refs Codex review finding #2 on PR
	// #4477.
	if is_async && client_inventory {
		return Err(syn::Error::new_spanned(
			&input.sig,
			"`#[routes(client_inventory)]` is not supported on async `routes()` \
			 functions. The WASM `ClientRouterRegistration::submit!` factory must \
			 be a sync `fn() -> Arc<ClientRouter>`, but the annotated function is \
			 async. Either make `routes()` sync, or drop `client_inventory` and \
			 keep the async server-only behavior. Refs #4453.",
		));
	}

	// Analyze function parameters for #[inject]
	let mut inject_params = Vec::new();
	let mut has_inject = false;

	for arg in &input.sig.inputs {
		if let FnArg::Typed(PatType { attrs, pat, ty, .. }) = arg
			&& attrs.iter().any(is_inject_attr)
		{
			has_inject = true;
			inject_params.push((pat.clone(), ty.clone()));
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

	// Wasm-not gate applied to native-only emitted items: the
	// `inventory::submit!` block that registers a `ServerRouter`, the
	// matching async submit block, and the DI-aware async server factory.
	// These reference native-only types (`ServerRouter`, DI scopes) that
	// do not exist on `wasm32-unknown-unknown`.
	//
	// Native-only cfg gate. Used unconditionally for the server-side
	// `inventory::submit!` block (which references `ServerRouter` and DI
	// types), and conditionally for the user's `routes()` function body
	// and the linker marker via `user_fn_and_marker_gate` below.
	//
	// The user-function gating is controlled by the `client_inventory`
	// flag, NOT by which target the macro is expanding on. With
	// `client_inventory`, the gate is dropped so the body compiles on
	// both targets; without it, the body remains native-only, preserving
	// the pre-#4453 behavior for legacy `mount(..)`-style bodies. Refs
	// #4175, #4453.
	let native_only = quote! {
		#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
	};

	// WASM-only emission gate for the `ClientRouterRegistration`
	// `inventory::submit!` block. The block is emitted only when
	// `client_inventory` is set (controlled by `wasm_client_submit_block`
	// below); when emitted, its factory calls back into the user's
	// `routes()` function, which by virtue of the same `client_inventory`
	// flag has had its `native_only` gate dropped. Refs #4453.
	let wasm_only = quote! {
		#[cfg(all(target_family = "wasm", target_os = "unknown"))]
	};

	// Gate applied to the user's `routes()` function body and the linker
	// marker. Behavior controlled by the explicit `client_inventory` flag,
	// NOT by `standalone`:
	//
	// - Without `client_inventory` (default): gate to native-only. This
	//   preserves the pre-#4453 behavior verbatim and prevents legacy
	//   `mount(..)`-style bodies — which reference native-only types — from
	//   needing to compile on `wasm32-unknown-unknown` (#4175 protection,
	//   Codex adversarial review feedback).
	// - With `client_inventory`: drop the gate. The user opts into a
	//   cross-target body and must write it accordingly (closure-style
	//   `.server(|s| ...).client(|c| ...)` is the recommended shape).
	let user_fn_and_marker_gate = if client_inventory {
		quote! {}
	} else {
		quote! { #native_only }
	};

	// WASM client `inventory::submit!` block. Emitted only when the user
	// opts in via `#[routes(client_inventory)]`. The default `#[routes]`
	// (and `#[routes(standalone)]` without `client_inventory`) emit no
	// WASM-side inventory submission, preserving pre-#4453 behavior for
	// every existing consumer.
	//
	// `standalone` is orthogonal: it controls URL-resolver generation in
	// `#url_prelude_code` below and has no effect on this block.
	let wasm_client_submit_block = if client_inventory {
		quote! {
			#wasm_only
			#[allow(unsafe_attr_outside_unsafe)]
			const _: () = {
				fn __get_client_router() -> ::std::sync::Arc<#reinhardt::ClientRouter> {
					::std::sync::Arc::new(#fn_name().into_client())
				}
				#reinhardt::inventory::submit! {
					#reinhardt::ClientRouterRegistration::__macro_new(__get_client_router)
				}
			};
		}
	} else {
		quote! {}
	};

	let expanded = if !is_async {
		// Case 1: Sync, no #[inject] — existing behavior unchanged
		let fn_sig = &input.sig;
		quote! {
			// User's `routes()` function.
			//
			// Cross-target emission is gated on the `client_inventory` flag
			// (carried by `user_fn_and_marker_gate`), NOT on `standalone`:
			//
			// - With `client_inventory`: gate is empty, so the function is
			//   emitted on both targets. On native it is consumed by the
			//   server `inventory::submit!` below; on WASM by the parallel
			//   `ClientRouterRegistration` submit. The body MUST compile
			//   cross-target — closure-style
			//   `UnifiedRouter::new().server(|s| ...).client(|c| ...)` is
			//   the recommended shape; the WASM `UnifiedRouter` variant
			//   treats `.server(...)` as a closure-discarding stub.
			// - Without `client_inventory` (default, including bare
			//   `#[routes]` and `#[routes(standalone)]`): gate is
			//   `#[cfg(not(wasm))]`, so the function and the linker marker
			//   are native-only. Legacy `UnifiedRouter::new().mount(..)`
			//   bodies that reference native-only `ServerRouter` continue
			//   to compile unchanged on WASM consumers. Refs #4175, #4453,
			//   and Codex adversarial review feedback (regression
			//   protection for existing consumers).
			//
			// private_interfaces: The macro forces `pub` visibility, but users
			// legitimately use `pub(crate)` newtype wrappers for DI parameters
			// (see #3498, #3468 DI pseudo orphan rule).
			#user_fn_and_marker_gate
			#[allow(private_interfaces)]
			#(#fn_attrs)*
			#fn_vis #fn_sig #fn_block

			// Allow unsafe attributes used by inventory::submit! (#[link_section])
			// Required for Rust 2024 edition compatibility
			#native_only
			#[allow(unsafe_attr_outside_unsafe)]
			const _: () = {
				// Server router extraction function
				fn __get_server_router() -> ::std::sync::Arc<#reinhardt::ServerRouter> {
					let unified = #fn_name();
					::std::sync::Arc::new(unified.into_server())
				}

				// Register with inventory using feature-independent internal constructor
				#reinhardt::inventory::submit! {
					#reinhardt::UrlPatternsRegistration::__macro_new(__get_server_router)
				}
			};

			// WASM-only parallel registration: submit a ClientRouter factory
			// derived from the same `routes()` function. The factory calls
			// `routes()` and converts the returned `UnifiedRouter` into a
			// `ClientRouter` via `into_client()`. Refs #4453.
			//
			// Suppressed for `#[routes(standalone)]` so legacy standalone
			// bodies using native-only `mount(..)` continue to compile on
			// WASM (the user's function itself remains `native_only`-gated
			// in the standalone path).
			#wasm_client_submit_block

			// Linker marker to enforce single #[routes] usage.
			//
			// For plain `#[routes]`: cross-target so the duplicate-symbol
			// guard applies on WASM SPA builds too. For `#[routes(standalone)]`:
			// native-only (mirrors the user function gate).
			#user_fn_and_marker_gate
			#[doc(hidden)]
			#[unsafe(no_mangle)]
			#[allow(non_upper_case_globals, dead_code)]
			// non_upper_case_globals: Intentionally lowercase for linker symbol
			// dead_code: Symbol is never directly used, only exists for linker validation
			static __reinhardt_routes_registration_marker: () = ();
		}
	} else if !has_inject {
		// Case 2: Async, no #[inject]
		//
		// No WASM client `inventory::submit!` is emitted for async
		// `#[routes]` because `inventory::submit!` requires a `const`-
		// constructible registration and the inventory entry is a
		// `fn() -> Arc<ClientRouter>` (sync). Driving an async `routes()`
		// from a sync factory would require a per-target executor stub on
		// `wasm32-unknown-unknown`, which is out of scope for #4453.
		// Async `#[routes]` is server-oriented today; client-side WASM SPAs
		// use the sync arm.
		let fn_sig = &input.sig;
		quote! {
			// User function gating mirrors the sync arm: controlled by the
			// `client_inventory` flag, not by `standalone`. Since
			// `client_inventory` is rejected on async `#[routes]` at parse
			// time, this arm always sees `client_inventory == false`, so
			// `user_fn_and_marker_gate` resolves to `#[cfg(not(wasm))]`
			// and the user function + linker marker remain native-only.
			#user_fn_and_marker_gate
			#[allow(private_interfaces)]
			#(#fn_attrs)*
			#fn_vis #fn_sig #fn_block

			#native_only
			#[allow(unsafe_attr_outside_unsafe)]
			const _: () = {
				fn __get_server_router() -> ::std::pin::Pin<
					::std::boxed::Box<
						dyn ::std::future::Future<
								Output = ::std::result::Result<
									::std::sync::Arc<#reinhardt::ServerRouter>,
									::std::boxed::Box<dyn ::std::error::Error + Send + Sync>,
								>,
							> + Send,
					>,
				> {
					::std::boxed::Box::pin(async {
						let unified = #fn_name().await;
						::std::result::Result::Ok(::std::sync::Arc::new(unified.into_server()))
					})
				}

				#reinhardt::inventory::submit! {
					#reinhardt::UrlPatternsRegistration::__macro_new_async(__get_server_router)
				}
			};

			// Linker marker mirrors the user-function gate (see sync arm).
			#user_fn_and_marker_gate
			#[doc(hidden)]
			#[unsafe(no_mangle)]
			#[allow(non_upper_case_globals, dead_code)]
			static __reinhardt_routes_registration_marker: () = ();
		}
	} else {
		// Case 3: Async, with #[inject]
		let di_crate = get_reinhardt_di_crate();

		// Generate dependency resolution code
		let inject_resolutions: Vec<_> = inject_params
			.iter()
			.map(|(pat, ty)| {
				if let Some(inner_ty) = extract_depends_inner_type(ty) {
					// Parameter is Depends<T>: resolve via registry only.
					// Factory-produced types (via #[injectable_factory]) do not implement
					// Injectable, so resolve_from_registry() is used to avoid requiring the bound.
					quote! {
						let #pat: #ty = #di_crate::Depends::<#inner_ty>::resolve_from_registry(&*__ctx, true).await
							.map_err(|e| -> ::std::boxed::Box<dyn ::std::error::Error + Send + Sync> {
								::std::boxed::Box::new(e)
							})?;
					}
				} else {
					// Parameter is T: resolve T, unwrap Arc<T> via clone
					quote! {
						let #pat: #ty = {
							let __arc = __ctx.resolve::<#ty>().await
								.map_err(|e| -> ::std::boxed::Box<dyn ::std::error::Error + Send + Sync> {
									::std::boxed::Box::new(e)
								})?;
							(*__arc).clone()
						};
					}
				}
			})
			.collect();

		// Generate parameter names for the call
		let inject_param_names: Vec<_> = inject_params
			.iter()
			.map(|(pat, _)| {
				if let Pat::Ident(pat_ident) = pat.as_ref() {
					let ident = &pat_ident.ident;
					quote! { #ident }
				} else {
					quote! { #pat }
				}
			})
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

		// Case 3: Async, with #[inject]
		//
		// The async server factory references DI types (`SingletonScope`,
		// `InjectionContext`) that are native-only, so the factory and its
		// `inventory::submit!` block remain gated on `#native_only`.
		//
		// The user's function definition is gated by
		// `user_fn_and_marker_gate`, mirroring the sync arm. Because
		// `client_inventory` is rejected on async `#[routes]` at parse
		// time, this arm always falls into the legacy native-only branch
		// (gate = `#[cfg(not(wasm))]`). No WASM client `inventory::submit!`
		// is emitted either: the inventory factory would have to drive an
		// async function synchronously, which is out of scope for #4453.
		// Async `#[routes]` is server-oriented; client-side WASM SPAs use
		// the sync arm with `#[routes(client_inventory)]`.
		//
		// Because async `#[routes]` bodies may freely reference native-
		// only `#[inject]` types, the native-only gating here is what
		// keeps the surrounding WASM module compilable. Refs #4175, #4453.
		quote! {
			#user_fn_and_marker_gate
			#[allow(private_interfaces)]
			#(#fn_attrs)*
			#fn_vis async fn #fn_name #fn_generics(#(#stripped_params),*) #fn_return #fn_block

			#native_only
			#[allow(unsafe_attr_outside_unsafe)]
			const _: () = {
				fn __get_server_router() -> ::std::pin::Pin<
					::std::boxed::Box<
						dyn ::std::future::Future<
								Output = ::std::result::Result<
									::std::sync::Arc<#reinhardt::ServerRouter>,
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
						::std::result::Result::Ok(::std::sync::Arc::new(unified.into_server()))
					})
				}

				#reinhardt::inventory::submit! {
					#reinhardt::UrlPatternsRegistration::__macro_new_async(__get_server_router)
				}
			};

			// Linker marker mirrors the user-function gate (see sync arm).
			#user_fn_and_marker_gate
			#[doc(hidden)]
			#[unsafe(no_mangle)]
			#[allow(non_upper_case_globals, dead_code)]
			static __reinhardt_routes_registration_marker: () = ();
		}
	};

	// Generate namespaced resolvers + url_prelude only when not in standalone mode.
	// Standalone mode skips these for projects that don't use
	// `installed_apps!`. Fixes #3542.
	//
	// Per-app URL resolver struct generation (Issue #3526).
	// Reads installed app labels from the state file written by `installed_apps!`
	// and generates directly:
	//   1. Per-app struct XxxUrls<'a> with route methods
	//   2. Accessor methods on ResolvedUrls
	//   3. url_prelude module with re-exports
	//
	// This replaces the __reinhardt_for_each_app callback pattern that triggers
	// macro_expanded_macro_exports_accessed_by_absolute_paths on Rust 1.94+.
	// Fixes #3639.
	let url_prelude_code = if standalone {
		// Standalone mode: projects that don't use installed_apps!.
		// Note: wasm targets are no longer skipped here. Server / ws resolver
		// blocks inside the generated tokens are individually gated with
		// `#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]`,
		// while the client-side `__client_router_gate` block emits
		// unconditionally so that `urls.client().<app>().<route>()` typed
		// accessors compile on `wasm32-unknown-unknown`. Fixes #4119.
		//
		// The `#expanded` token stream above (the user's `routes()` function
		// body, the `inventory::submit!` registration, and the linker marker)
		// is wasm-gated as a whole via `#native_only` because the
		// user-written body is allowed to reference any native-only items
		// (admin / middleware / Redis sessions / `#[inject]` / server fns)
		// the consuming crate uses. Gating it out on wasm lets the
		// surrounding module compile cleanly so that
		// `__url_resolver_support::ResolvedUrls` is reachable from wasm SPA
		// consumers. Fixes #4175.
		quote! {}
	} else {
		// Soft-fallback when the installed-apps state file is missing
		// (Issue #4189). Hard-erroring here breaks wasm SPA consumers
		// where the scaffold's `mod apps` (containing `installed_apps!`)
		// is gated `#[cfg(server)]`, so the file is never written for
		// wasm builds. Cargo does not expose `CARGO_CFG_TARGET_FAMILY`
		// to proc-macro processes (only to build scripts), so the macro
		// cannot detect the consumer's target at expansion time. Falling
		// back to an empty label list joins the existing
		// `app_idents.is_empty()` branch below, which emits only the
		// minimal `url_prelude { pub use super::ResolvedUrls; }` block —
		// exactly the surface wasm SPA consumers need.
		// `read_installed_apps()` returns `Ok(empty)` when the state file is
		// absent (the expected wasm soft-fallback) and `Err` for all other IO
		// failures, so genuine misconfigurations still surface as macro errors
		// rather than being silently swallowed.
		let app_labels = crate::macro_state::read_installed_apps().map_err(|e| {
			syn::Error::new(
				proc_macro2::Span::call_site(),
				format!("Failed to read installed apps state: {e}"),
			)
		})?;

		let app_idents: Vec<proc_macro2::Ident> = app_labels
			.iter()
			.filter(|s| !s.is_empty())
			.map(|s| {
				syn::parse_str::<syn::Ident>(s).map_err(|_| {
					syn::Error::new(
						proc_macro2::Span::call_site(),
						format!(
							"Invalid installed app label `{s}`: expected a valid Rust identifier"
						),
					)
				})
			})
			.collect::<Result<Vec<_>>>()?;

		if app_idents.is_empty() {
			quote! {
				#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
				/// Prelude module re-exporting URL resolver types.
				pub mod url_prelude {
					pub use super::ResolvedUrls;
				}
			}
		} else {
			// Generate per-app resolver structs and methods.
			// The callback macro __gen_<app>_method creates impl blocks
			// for each route discovered via __for_each_url_resolver.
			let per_app_code: Vec<_> = app_idents
				.iter()
				.map(|app| {
					let urls_struct_name =
						crate::pascal_case::to_pascal_case_with_suffix(&app.to_string(), "Urls");
					let urls_struct =
						proc_macro2::Ident::new(&urls_struct_name, proc_macro2::Span::call_site());
					let gen_method_macro = proc_macro2::Ident::new(
						&format!("__gen_{}_method", app),
						proc_macro2::Span::call_site(),
					);

					let server_callback_arms = gen_resolver_callback_arms(
						&urls_struct,
						&quote! { resolve_url },
						&quote! { stringify!($app_label) },
						&quote! { use #reinhardt::UrlResolver as _; },
					);

					quote! {
						/// Per-app URL resolver.
						///
						/// Access via `ResolvedUrls::#app()`.
						pub struct #urls_struct<'a> {
							resolver: &'a ResolvedUrls,
						}

						// Callback macro for __for_each_url_resolver to generate methods.
						// Each arm imports UrlResolver trait to bring resolve_url() into
						// scope. (Issue #3669)
						// Arms generated by gen_resolver_callback_arms() for 0..=5 params.
						macro_rules! #gen_method_macro {
							#server_callback_arms
						}

						// Invoke __for_each_url_resolver to populate methods.
						// Pass the absolute path to `url_resolvers` as `$base`
						// so that metadata macros resolve correctly at the call site.
						crate::apps::#app::urls::url_resolvers::__for_each_url_resolver!(
							#gen_method_macro, #app,
							crate::apps::#app::urls::url_resolvers
						);
					}
				})
				.collect();

			// Generate per-app client URL resolver structs.
			// When #[url_patterns(InstalledApp::<variant>, mode = client)] is used,
			// typed methods are generated via __for_each_client_url_resolver
			// (same pattern as server-side). A fallback resolve() method is
			// always available for runtime string-based resolution.
			//
			// Issue #4509: when `no_client_resolvers` (or `server_only`) is set,
			// the per-app client resolver lookups would target nonexistent
			// `crate::apps::<app>::urls::client_url_resolvers` modules — skip
			// the entire construction so REST-only apps need not stub them.
			let per_app_client_code: Vec<_> = if no_client_resolvers {
				Vec::new()
			} else {
				app_idents
					.iter()
					.map(|app| {
						let client_urls_struct_name =
							crate::pascal_case::to_pascal_case_with_suffix(
								&app.to_string(),
								"ClientUrls",
							);
						let client_urls_struct = proc_macro2::Ident::new(
							&client_urls_struct_name,
							proc_macro2::Span::call_site(),
						);
						let gen_client_method_macro = proc_macro2::Ident::new(
							&format!("__gen_{}_client_method", app),
							proc_macro2::Span::call_site(),
						);
						let app_str = app.to_string();

						let client_callback_arms = gen_resolver_callback_arms(
							&client_urls_struct,
							&quote! { resolve_client_url },
							&quote! { #app_str },
							&quote! { use #reinhardt::ClientUrlResolver as _; },
						);

						quote! {
							/// Per-app client URL resolver.
							///
							/// Access via `ResolvedUrls::#accessor_method()`.
							pub struct #client_urls_struct<'a> {
								resolver: &'a ResolvedUrls,
							}

							impl #client_urls_struct<'_> {
								/// Resolve a client-side URL by route name and parameters.
								///
								/// Fallback for routes not covered by typed methods.
								/// The route name is automatically prefixed with the app label.
								pub fn resolve(&self, route_name: &str, params: &[(&str, &str)]) -> String {
									// Bring `ClientUrlResolver` into scope so that
									// `resolve_client_url` is callable on `&ResolvedUrls`.
									use #reinhardt::ClientUrlResolver as _;
									let full_name = ::std::format!("{}:{}", #app_str, route_name);
									self.resolver.resolve_client_url(&full_name, params)
								}
							}

							// Callback macro for __for_each_client_url_resolver to generate
							// typed methods (same pattern as server-side __gen_<app>_method).
							// Arms generated by gen_resolver_callback_arms() for 0..=5 params.
							macro_rules! #gen_client_method_macro {
								#client_callback_arms
							}

							// Invoke __for_each_client_url_resolver to populate typed methods.
							// This is a no-op if the app has no client_url_resolvers module
							// (i.e., does not use #[url_patterns(..., mode = client)] or
							// #[url_patterns(..., mode = unified)]).
							crate::apps::#app::urls::client_url_resolvers::__for_each_client_url_resolver!(
								#gen_client_method_macro, #app,
								crate::apps::#app::urls::client_url_resolvers
							);
						}
					})
					.collect()
			};

			// Generate url_prelude re-exports.
			//
			// Issue #4509: when `no_client_resolvers` is set the
			// `__namespaced_client_resolvers` module is not emitted, so the
			// per-app `<App>ClientUrls` re-export must be skipped — otherwise
			// it would dangle on the missing module path. The
			// `<App>Urls` (server-side) re-export and the deprecated trait
			// glob are always emitted.
			let prelude_exports: Vec<_> = app_idents
				.iter()
				.map(|app| {
					let urls_struct_name =
						crate::pascal_case::to_pascal_case_with_suffix(&app.to_string(), "Urls");
					let urls_struct =
						proc_macro2::Ident::new(&urls_struct_name, proc_macro2::Span::call_site());
					let client_export = if no_client_resolvers {
						quote! {}
					} else {
						let client_urls_struct_name =
							crate::pascal_case::to_pascal_case_with_suffix(
								&app.to_string(),
								"ClientUrls",
							);
						let client_urls_struct = proc_macro2::Ident::new(
							&client_urls_struct_name,
							proc_macro2::Span::call_site(),
						);
						quote! {
							#[cfg(feature = "client-router")]
							pub use super::super::__namespaced_client_resolvers::#client_urls_struct;
						}
					};
					quote! {
						pub use super::#urls_struct;
						#client_export
						// Deprecated flat trait re-exports (backward compatibility)
						#[allow(deprecated)]
						pub use crate::apps::#app::urls::url_resolvers::*;
					}
				})
				.collect();

			// Generate per-app WS resolver structs (parallel to per_app_code for HTTP).
			//
			// Issue #4509: when `no_ws_resolvers` (or `server_only`) is set, the
			// per-app WS resolver lookups would target nonexistent
			// `crate::apps::<app>::urls::ws_urls::ws_url_resolvers` modules — skip
			// the entire construction so REST-only apps need not stub them.
			let per_app_ws_code: Vec<_> = if no_ws_resolvers {
				Vec::new()
			} else {
				app_idents
					.iter()
					.map(|app| {
						let ws_urls_struct_name = crate::pascal_case::to_pascal_case_with_suffix(
							&app.to_string(),
							"WsUrls",
						);
						let ws_urls_struct = proc_macro2::Ident::new(
							&ws_urls_struct_name,
							proc_macro2::Span::call_site(),
						);
						let gen_ws_method_macro = proc_macro2::Ident::new(
							&format!("__gen_{}_ws_method", app),
							proc_macro2::Span::call_site(),
						);

						// Parallel to server_callback_arms but uses resolve_ws_url + WebSocketUrlResolver
						let ws_callback_arms = gen_resolver_callback_arms(
							&ws_urls_struct,
							&quote! { resolve_ws_url },
							&quote! { stringify!($app_label) },
							&quote! { use #reinhardt::WebSocketUrlResolver as _; },
						);

						quote! {
							/// Per-app WebSocket URL resolver.
							///
							/// Access via `WsUrls::#app()`.
							pub struct #ws_urls_struct<'a> {
								resolver: &'a ResolvedUrls,
							}

							macro_rules! #gen_ws_method_macro {
								#ws_callback_arms
							}

							// Invoke __for_each_ws_url_resolver to populate methods.
							// This is a no-op if the app has no urls/ws_urls.rs module.
							// #3914: ws resolver was hoisted under `urls/` in rc.19 (breaking change).
							crate::apps::#app::urls::ws_urls::ws_url_resolvers::__for_each_ws_url_resolver!(
								#gen_ws_method_macro, #app,
								crate::apps::#app::urls::ws_urls::ws_url_resolvers
							);
						}
					})
					.collect()
			};

			// ServerUrls gateway: urls.server().<app>().<handler>()
			// Delegates to the existing XxxUrls structs (no new struct needed).
			let server_app_accessors: Vec<_> = app_idents
				.iter()
				.map(|app| {
					let urls_struct_name =
						crate::pascal_case::to_pascal_case_with_suffix(&app.to_string(), "Urls");
					let urls_struct =
						proc_macro2::Ident::new(&urls_struct_name, proc_macro2::Span::call_site());
					quote! {
						pub fn #app(&self) -> #urls_struct<'_> {
							#urls_struct { resolver: self.resolver }
						}
					}
				})
				.collect();

			// ClientUrls gateway: urls.client().<app>().<handler>()
			//
			// Issue #4509: skipped entirely when `no_client_resolvers` is set —
			// the surrounding `__client_router_gate` mod is not emitted.
			let client_app_accessors: Vec<_> = if no_client_resolvers {
				Vec::new()
			} else {
				app_idents
					.iter()
					.map(|app| {
						let client_urls_struct_name =
							crate::pascal_case::to_pascal_case_with_suffix(
								&app.to_string(),
								"ClientUrls",
							);
						let client_urls_struct = proc_macro2::Ident::new(
							&client_urls_struct_name,
							proc_macro2::Span::call_site(),
						);
						quote! {
							pub fn #app(&self) -> #client_urls_struct<'_> {
								#client_urls_struct { resolver: self.resolver }
							}
						}
					})
					.collect()
			};

			// WsUrls gateway: urls.ws().<app>().<handler>()
			//
			// Issue #4509: skipped entirely when `no_ws_resolvers` is set — the
			// surrounding `__namespaced_ws_resolvers` mod is not emitted.
			let ws_app_accessors: Vec<_> = if no_ws_resolvers {
				Vec::new()
			} else {
				app_idents
					.iter()
					.map(|app| {
						let ws_urls_struct_name = crate::pascal_case::to_pascal_case_with_suffix(
							&app.to_string(),
							"WsUrls",
						);
						let ws_urls_struct = proc_macro2::Ident::new(
							&ws_urls_struct_name,
							proc_macro2::Span::call_site(),
						);
						quote! {
							pub fn #app(&self) -> #ws_urls_struct<'_> {
								#ws_urls_struct { resolver: self.resolver }
							}
						}
					})
					.collect()
			};

			// Issue #4509: build the WebSocket and client-router blocks as
			// conditional sub-quotes so REST-only apps under `server_only`
			// (or fine-grained `no_*_resolvers`) skip the module emission
			// entirely. Apps without `urls/ws_urls/` or `client_url_resolvers`
			// modules then compile cleanly without stub files.
			let ws_block = if no_ws_resolvers {
				quote! {}
			} else {
				quote! {
					// WebSocket per-app resolver structs (native-only, parallel to HTTP resolvers).
					#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
					#[doc(hidden)]
					mod __namespaced_ws_resolvers {
						// `unexpected_cfgs`: macro-generated `#[cfg(feature = "...")]` arms
						// may reference features that the downstream crate has not declared
						// when running under `-D unexpected_cfgs`.
						// `dead_code`: when an app exposes no `#[url_patterns(..., mode = ws)]`
						// surface, the per-app `ws_url_resolvers` re-exports and helpers
						// emitted here are intentionally unused.
						#![allow(unexpected_cfgs, dead_code)]
						pub use super::ResolvedUrls;

						#(#per_app_ws_code)*

						/// WebSocket URL gateway. Access via `urls.ws().<app>().<route>()`.
						pub struct WsUrls<'a> {
							resolver: &'a ResolvedUrls,
						}

						impl WsUrls<'_> {
							#(#ws_app_accessors)*
						}

						// urls.ws() accessor lives inside this module so that WsUrls.resolver
						// (a private field) is accessible via struct-literal syntax (E0451 fix).
						impl ResolvedUrls {
							/// Access WebSocket URL resolvers via `urls.ws().<app>().<route>()`.
							pub fn ws(&self) -> WsUrls<'_> {
								WsUrls { resolver: self }
							}
						}
					}
					#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
					pub use __namespaced_ws_resolvers::*;

					// WebSocketUrlResolver stub impl: allows the type chain to compile.
					// For actual WS URL resolution, call impl_ws_url_resolver!(ResolvedUrls)
					// after importing reinhardt-websockets.
					#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
					impl #reinhardt::WebSocketUrlResolver for ResolvedUrls {
						fn resolve_ws_url(&self, _name: &str, _params: &[(&str, &str)]) -> String {
							unimplemented!(
								"WebSocket URL resolution requires reinhardt-websockets. \
								 Call impl_ws_url_resolver!(ResolvedUrls) to enable it."
							)
						}
					}
				}
			};

			let client_block = if no_client_resolvers {
				quote! {}
			} else {
				quote! {
					// Client-side per-app resolvers are cross-platform (native + WASM).
					#[doc(hidden)]
					mod __client_router_gate {
						// `unexpected_cfgs`: the generated `#[cfg(feature = "client-router")]`
						// gate references a feature only declared on the consumer side, so
						// downstream builds with `-D unexpected_cfgs` would otherwise reject
						// the emitted module wrapper.
						// `deprecated`: emitted helper trait impls may forward through
						// items that carry `#[deprecated]` during RC transitions; the
						// generated forwards intentionally outlive the deprecation window.
						#![allow(unexpected_cfgs, deprecated)]

						#[cfg(feature = "client-router")]
						#[doc(hidden)]
						pub mod __namespaced_client_resolvers {
							pub use super::super::ResolvedUrls;

							#(#per_app_client_code)*

							/// Client URL gateway. Access via `urls.client().<app>().<route>()`.
							pub struct ClientUrls<'a> {
								resolver: &'a ResolvedUrls,
							}

							impl ClientUrls<'_> {
								#(#client_app_accessors)*
							}

							impl ResolvedUrls {
								/// Access client URL resolvers via `urls.client().<app>().<route>()`.
								pub fn client(&self) -> ClientUrls<'_> {
									ClientUrls { resolver: self }
								}
							}
						}
						#[cfg(feature = "client-router")]
						pub use __namespaced_client_resolvers::*;
					}
					pub use __client_router_gate::*;
				}
			};

			quote! {
				// Track state file for incremental compilation invalidation.
				// When installed_apps! rewrites the file, the compiler re-expands #[routes].
				//
				// Path is namespaced by `CARGO_CRATE_NAME` to match
				// `crate::macro_state::state_dir_path` — required to keep multiple
				// `[[test]]` binaries in one crate from racing on a shared state
				// file (Issue #4592).
				//
				// Both `env!()` calls hard-fail at compile time if the env var is
				// missing; `concat!()` cannot consume `option_env!()` with a
				// compile-time fallback. `macro_state::state_dir_path` is
				// symmetrically hard-fail (both vars are `?`-propagated) so the
				// two paths stay consistent — see the module-level doc-comment in
				// `macro_state.rs` for rationale.
				#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
				const _: &[u8] = include_bytes!(concat!(
					env!("CARGO_MANIFEST_DIR"),
					"/target/reinhardt/",
					env!("CARGO_CRATE_NAME"),
					"/.installed_apps",
				));

				// Server-side per-app resolvers are native-only.
				#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
				#[doc(hidden)]
				mod __namespaced_resolvers {
					#![allow(unexpected_cfgs, deprecated)]
					pub use super::ResolvedUrls;

					#(#per_app_code)*

					/// HTTP URL gateway. Access via `urls.server().<app>().<route>()`.
					pub struct ServerUrls<'a> {
						resolver: &'a ResolvedUrls,
					}

					impl ServerUrls<'_> {
						#(#server_app_accessors)*
					}

					impl ResolvedUrls {
						/// Access HTTP URL resolvers via `urls.server().<app>().<route>()`.
						pub fn server(&self) -> ServerUrls<'_> {
							ServerUrls { resolver: self }
						}
					}

					/// Prelude module re-exporting URL resolver types.
					pub mod url_prelude {
						pub use super::ResolvedUrls;
						#(#prelude_exports)*
					}
				}
				#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
				pub use __namespaced_resolvers::*;

				#ws_block

				#client_block
			}
		}
	};

	// Generate ResolvedUrls struct.
	// Native: holds both ServerRouter and ClientUrlReverser.
	// WASM: holds only ClientUrlReverser.
	// Gate using raw platform check because this code expands in consuming
	// crates that do not have the `native` cfg alias.
	//
	// Fallback expression used by `ResolvedUrls::from_global()` when the
	// global client reverser has not been registered. With `no_client_resolvers`
	// (Issue #4509), the per-app `client_url_resolvers` lookups are skipped, so
	// no client reverser is registered at startup; tests and binaries that only
	// exercise server-side routing must still be able to construct
	// `ResolvedUrls` without panicking. Fall back to an empty
	// `ClientUrlReverser` (which returns `None` from `reverse(...)` for every
	// name) in that case, and keep the explicit panic message otherwise so
	// projects that genuinely forgot to call `#[routes]` still get a clear
	// diagnostic. Fixes #4629.
	let client_reverser_fallback = if no_client_resolvers {
		// Cache the empty fallback in a per-call-site `OnceLock` so that
		// repeated `ResolvedUrls::from_global()` calls under
		// `no_client_resolvers` mode (e.g. once per request) do not allocate
		// a fresh `HashMap` + `Arc<ClientUrlReverser>` every time. The static
		// is scoped inside the `unwrap_or_else` closure block, so each
		// generated `from_global()` body gets its own one-shot cache. Fixes
		// #4635.
		quote! {
			{
				static EMPTY_REVERSER:
					::std::sync::OnceLock<::std::sync::Arc<#reinhardt::ClientUrlReverser>>
					= ::std::sync::OnceLock::new();
				::std::sync::Arc::clone(
					EMPTY_REVERSER.get_or_init(|| {
						::std::sync::Arc::new(#reinhardt::ClientUrlReverser::new(
							::std::collections::HashMap::new(),
						))
					}),
				)
			}
		}
	} else {
		quote! {
			panic!(
				"Global client reverser not registered. Ensure the #[routes] function has been called."
			)
		}
	};

	let url_resolver_code = quote! {
		#[doc(hidden)]
		pub mod __url_resolver_support {
			#![allow(unexpected_cfgs)]
			/// Type-safe URL resolver backed by the global `ServerRouter`
			/// and `ClientUrlReverser`.
			///
			/// Provides URL resolution methods via extension traits generated
			/// by view macros. Import `url_prelude::*` to bring all resolver
			/// methods into scope.
			#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
			pub struct ResolvedUrls {
				router: ::std::sync::Arc<#reinhardt::ServerRouter>,
				#[cfg(feature = "client-router")]
				client_reverser: ::std::sync::Arc<#reinhardt::ClientUrlReverser>,
			}

			/// WASM-only `ResolvedUrls` with client URL resolution only.
			#[cfg(all(target_family = "wasm", target_os = "unknown"))]
			pub struct ResolvedUrls {
				client_reverser: ::std::sync::Arc<#reinhardt::ClientUrlReverser>,
			}

			#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
			impl #reinhardt::UrlResolver for ResolvedUrls {
				fn resolve_url(&self, name: &str, params: &[(&str, &str)]) -> String {
					self.router
						.reverse(name, params)
						.unwrap_or_else(|| panic!("Route '{}' not found in router", name))
				}

				// Non-panicking lookup used by `UrlResolverUnprefixed`'s
				// override below to probe candidate `"<app>:<name>"`
				// namespaces without panicking. Refs Issue #4507.
				fn try_resolve_url(&self, name: &str, params: &[(&str, &str)]) -> Option<String> {
					self.router.reverse(name, params)
				}
			}

			#[cfg(feature = "client-router")]
			impl #reinhardt::ClientUrlResolver for ResolvedUrls {
				fn resolve_client_url(&self, name: &str, params: &[(&str, &str)]) -> String {
					self.client_reverser
						.reverse(name, params)
						.unwrap_or_else(|| panic!("Client route '{}' not found in router", name))
				}
			}

			#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
			impl ResolvedUrls {
				/// Create a `ResolvedUrls` from the globally registered router.
				///
				/// # Panics
				///
				/// Always panics if no global router has been registered via
				/// `#[routes]`. The behaviour for a missing global client
				/// reverser is conditional on the `#[routes(...)]` attribute
				/// flags that built this `ResolvedUrls`:
				///
				/// - When `#[routes(...)]` did **not** specify
				///   `no_client_resolvers` (or its alias `server_only`), a
				///   missing client reverser panics with the same diagnostic
				///   the router check uses.
				/// - When `#[routes(no_client_resolvers)]` (or `server_only`)
				///   *was* specified, no client reverser is ever registered
				///   by `#[routes]`; `from_global()` falls back to an empty
				///   `ClientUrlReverser` whose `reverse(...)` always returns
				///   `None`. This lets server-only crates construct a
				///   `ResolvedUrls` without panicking. See Issues #4509 and
				///   #4629.
				#[cfg(feature = "client-router")]
				pub fn from_global() -> Self {
					let router = #reinhardt::get_router()
						.expect("Global router not registered. Ensure the #[routes] function has been called.");
					let client_reverser = #reinhardt::get_client_reverser()
						.unwrap_or_else(|| #client_reverser_fallback);
					Self { router, client_reverser }
				}

				/// Create a `ResolvedUrls` from the globally registered router (without client router).
				///
				/// # Panics
				///
				/// Panics if no global router has been registered via `#[routes]`.
				#[cfg(not(feature = "client-router"))]
				pub fn from_global() -> Self {
					let router = #reinhardt::get_router()
						.expect("Global router not registered. Ensure the #[routes] function has been called.");
					Self { router }
				}

				/// Create a `ResolvedUrls` from explicit router and client reverser.
				#[cfg(feature = "client-router")]
				pub fn from_router(
					router: ::std::sync::Arc<#reinhardt::ServerRouter>,
					client_reverser: ::std::sync::Arc<#reinhardt::ClientUrlReverser>,
				) -> Self {
					Self { router, client_reverser }
				}

				/// Create a `ResolvedUrls` from an explicit router (without client router).
				#[cfg(not(feature = "client-router"))]
				pub fn from_router(
					router: ::std::sync::Arc<#reinhardt::ServerRouter>,
				) -> Self {
					Self { router }
				}

				/// Access streaming topic resolvers.
				///
				/// Extension traits generated by `#[producer]`/`#[consumer]` macros
				/// add typed accessor methods on the returned `StreamingRef`. Bring
				/// the app's streaming handlers into scope to access their methods.
				///
				/// # Example
				///
				/// ```rust,ignore
				/// let urls = ResolvedUrls::from_global();
				/// let topic = urls.streaming().topic_for("create_order"); // → "orders"
				/// ```
				#[cfg(feature = "streaming")]
				pub fn streaming(&self) -> StreamingRef<'_> {
					StreamingRef { _marker: ::core::marker::PhantomData }
				}
			}

			/// Streaming resolver returned by `ResolvedUrls::streaming()`.
			///
			/// Extension traits from `#[producer]`/`#[consumer]` macros add per-handler
			/// accessor methods that return the registered Kafka topic name.
			#[cfg(all(not(all(target_family = "wasm", target_os = "unknown")), feature = "streaming"))]
			pub struct StreamingRef<'a> {
				_marker: ::core::marker::PhantomData<&'a ()>,
			}

			#[cfg(all(not(all(target_family = "wasm", target_os = "unknown")), feature = "streaming"))]
			impl #reinhardt::streaming::StreamingTopicResolver for StreamingRef<'_> {
				fn resolve_topic(&self, name: &str) -> &'static str {
					#reinhardt::streaming::resolve_streaming_topic(name)
				}
			}

			#[cfg(all(not(all(target_family = "wasm", target_os = "unknown")), feature = "streaming"))]
			impl<'a> StreamingRef<'a> {
				/// Resolve a topic name by handler name (runtime lookup).
				pub fn topic_for(&self, name: &str) -> &'static str {
					#reinhardt::streaming::resolve_streaming_topic(name)
				}
			}

			#[cfg(all(target_family = "wasm", target_os = "unknown"))]
			impl ResolvedUrls {
				/// Create a `ResolvedUrls` from the globally registered client reverser.
				///
				/// # Panics
				///
				/// When `#[routes(...)]` did not specify `no_client_resolvers`
				/// (or its alias `server_only`), panics if no global client
				/// reverser has been registered via `#[routes]`. When the
				/// flag *was* specified — typically only meaningful on the
				/// native target, but the WASM impl honors the same fallback
				/// for symmetry — falls back to an empty
				/// `ClientUrlReverser` whose `reverse(...)` always returns
				/// `None`. See Issues #4509 and #4629.
				pub fn from_global() -> Self {
					let client_reverser = #reinhardt::get_client_reverser()
						.unwrap_or_else(|| #client_reverser_fallback);
					Self { client_reverser }
				}

				/// Create a `ResolvedUrls` from an explicit client reverser.
				pub fn from_reverser(
					client_reverser: ::std::sync::Arc<#reinhardt::ClientUrlReverser>,
				) -> Self {
					Self { client_reverser }
				}
			}

			#url_prelude_code
		}
		// Explicit re-export so consumers can write `<path>::ResolvedUrls`
		// without reaching into the underscore-prefixed implementation module.
		pub use __url_resolver_support::ResolvedUrls;
		pub use __url_resolver_support::*;
	};

	let combined = quote! {
		#expanded
		#url_resolver_code
	};

	Ok(combined)
}
