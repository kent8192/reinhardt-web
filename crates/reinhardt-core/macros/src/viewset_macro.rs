use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::ItemFn;
#[cfg(test)]
use syn::parse2;

/// Extract basename string literal from a function body.
///
/// Scans tokens for `ModelViewSet::new("basename")` or `GenericViewSet::new("basename", ...)`
/// patterns and returns the first string literal found as the basename.
fn extract_basename(func: &ItemFn) -> Option<String> {
	let body_tokens: Vec<proc_macro2::TokenTree> = func
		.block
		.stmts
		.iter()
		.flat_map(|stmt| {
			let tokens: TokenStream = quote! { #stmt };
			tokens.into_iter().collect::<Vec<_>>()
		})
		.collect();

	let mut i = 0;
	while i < body_tokens.len() {
		// Look for: Ident("ModelViewSet" | "GenericViewSet") Punct(':') Punct(':')
		if i + 2 < body_tokens.len()
			&& let proc_macro2::TokenTree::Ident(type_ident) = &body_tokens[i]
			&& (type_ident == "ModelViewSet" || type_ident == "GenericViewSet")
			&& let proc_macro2::TokenTree::Punct(p1) = &body_tokens[i + 1]
			&& p1.as_char() == ':'
			&& let proc_macro2::TokenTree::Punct(p2) = &body_tokens[i + 2]
			&& p2.as_char() == ':'
		{
			// Check for `new` ident and parenthesized group
			if i + 4 < body_tokens.len()
				&& let proc_macro2::TokenTree::Ident(new_ident) = &body_tokens[i + 3]
				&& new_ident == "new"
				&& let proc_macro2::TokenTree::Group(group) = &body_tokens[i + 4]
				&& group.delimiter() == proc_macro2::Delimiter::Parenthesis
			{
				// Extract first string literal from the group
				for tt in group.stream() {
					if let proc_macro2::TokenTree::Literal(lit) = tt {
						let lit_str = lit.to_string();
						if lit_str.starts_with('"') && lit_str.ends_with('"') {
							return Some(lit_str[1..lit_str.len() - 1].to_string());
						}
					}
				}
			}
		}
		i += 1;
	}
	None
}

/// Convert a basename like "snippet" to PascalCase: "Snippet".
/// Handles underscored names: "auth_user" → "AuthUser".
fn to_pascal_case(s: &str) -> String {
	let mut result = String::new();
	for segment in s.split('_') {
		let mut chars = segment.chars();
		if let Some(first) = chars.next() {
			result.push(first.to_ascii_uppercase());
			result.extend(chars);
		}
	}
	result
}

/// Generate URL resolver modules for a ViewSet basename.
///
/// For basename "snippet", generates:
/// - `__url_resolver_snippet_list` with trait `ResolveSnippetList`
/// - `__url_resolver_snippet_detail` with trait `ResolveSnippetDetail`
///
/// These modules are emitted at the same level as the annotated function.
/// Because `mod` items are invalid inside `impl` blocks, `#[viewset]` must
/// be applied to a free (module-level) function, not a method.
fn generate_viewset_resolver_tokens(basename: &str) -> TokenStream {
	let pascal = to_pascal_case(basename);

	let list_mod_ident = syn::Ident::new(
		&format!("__url_resolver_{basename}_list"),
		Span::call_site(),
	);
	let detail_mod_ident = syn::Ident::new(
		&format!("__url_resolver_{basename}_detail"),
		Span::call_site(),
	);

	let list_trait_ident = syn::Ident::new(&format!("Resolve{pascal}List"), Span::call_site());
	let detail_trait_ident = syn::Ident::new(&format!("Resolve{pascal}Detail"), Span::call_site());

	let list_method_ident = syn::Ident::new(&format!("{basename}_list"), Span::call_site());
	let detail_method_ident = syn::Ident::new(&format!("{basename}_detail"), Span::call_site());

	let list_route_name = format!("{basename}-list");
	let detail_route_name = format!("{basename}-detail");

	let list_doc = format!("Resolve URL for route `{list_route_name}`.");
	let detail_doc = format!("Resolve URL for route `{detail_route_name}`.");

	let reinhardt_crate = crate::crate_paths::get_reinhardt_crate();

	quote! {
		#[doc(hidden)]
		pub mod #list_mod_ident {
			#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
			#[doc = #list_doc]
			pub trait #list_trait_ident: #reinhardt_crate::UrlResolverUnprefixed {
				#[doc = #list_doc]
				fn #list_method_ident(&self) -> String {
					// Supertrait `UrlResolverUnprefixed` brings the
					// namespace-aware lookup into scope; no extra `use`
					// statement needed. The trait is intentionally
					// deprecated; suppress the warning for this
					// macro-emitted call site only.
					#[allow(deprecated)]
					self.resolve_url_unprefixed(#list_route_name, &[])
				}
			}
			// The trait is intentionally deprecated; this is the legacy
			// compatibility surface and must opt out of the warning. The
			// supertrait bound is `UrlResolverUnprefixed` (not `UrlResolver`)
			// so the blanket impl does NOT collide with the
			// `#[routes]`-emitted `impl UrlResolverUnprefixed for ResolvedUrls`
			// (E0119 prevention). Refs Issue #4507.
			#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
			#[allow(deprecated)]
			impl<T: #reinhardt_crate::UrlResolverUnprefixed> #list_trait_ident for T {}
		}
		// Legacy deprecated re-export; the inner trait carries `#[deprecated]`
		// so the glob must opt out of the warning. Refs Issue #4507.
		#[doc(hidden)]
		#[allow(deprecated)]
		pub use #list_mod_ident::*;

		#[doc(hidden)]
		pub mod #detail_mod_ident {
			#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
			#[doc = #detail_doc]
			pub trait #detail_trait_ident: #reinhardt_crate::UrlResolverUnprefixed {
				#[doc = #detail_doc]
				fn #detail_method_ident(&self, id: &str) -> String {
					// Supertrait `UrlResolverUnprefixed` brings the
					// namespace-aware lookup into scope; no extra `use`
					// statement needed. The trait is intentionally
					// deprecated; suppress the warning for this
					// macro-emitted call site only.
					#[allow(deprecated)]
					self.resolve_url_unprefixed(#detail_route_name, &[("id", id)])
				}
			}
			// The trait is intentionally deprecated; this is the legacy
			// compatibility surface and must opt out of the warning. The
			// supertrait bound is `UrlResolverUnprefixed` (not `UrlResolver`)
			// so the blanket impl does NOT collide with the
			// `#[routes]`-emitted `impl UrlResolverUnprefixed for ResolvedUrls`
			// (E0119 prevention). Refs Issue #4507.
			#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
			#[allow(deprecated)]
			impl<T: #reinhardt_crate::UrlResolverUnprefixed> #detail_trait_ident for T {}
		}
		// Legacy deprecated re-export; the inner trait carries `#[deprecated]`
		// so the glob must opt out of the warning. Refs Issue #4507.
		#[doc(hidden)]
		#[allow(deprecated)]
		pub use #detail_mod_ident::*;
	}
}

/// Emit a meta macro that dispatches the standard fan-out callback used by
/// `__for_each_url_resolver` (see routes_registration.rs::gen_resolver_callback_arms).
///
/// `kind` is either "list" or "detail"; for "detail" the callback receives
/// an additional "id" param literal so the generated accessor's signature
/// gains an `id: &str` argument.
///
/// Phase 6.2 (Issue #4507): the macro carries `#[macro_export]` so it lands
/// at the user crate's root. This sidesteps E0364 ("cannot re-export
/// `pub(crate)` macro_rules! as `pub`") which blocked the previous attempt
/// to thread the manifest through a `pub use` chain into `url_resolvers`.
/// Names are pre-collision-resistant: every macro carries `<fn>` /
/// `<basename>` / `<kind>` in its identifier, so multiple `#[viewset]`
/// invocations in the same crate produce distinct crate-root names.
///
/// Refs Issue #4507.
fn emit_meta_macro(
	fn_name: &syn::Ident,
	basename: &str,
	kind: &str,
	include_id: bool,
) -> TokenStream {
	let macro_name = syn::Ident::new(
		&format!("__url_resolver_meta_{fn_name}_{basename}_{kind}"),
		Span::call_site(),
	);
	let method_ident = syn::Ident::new(&format!("{basename}_{kind}"), Span::call_site());
	let route_literal = format!("{basename}-{kind}");

	let body = if include_id {
		quote! { $callback!($app, #method_ident, #route_literal, "id"); }
	} else {
		quote! { $callback!($app, #method_ident, #route_literal, ); }
	};

	// `#[macro_export]` already publishes the macro at the user crate root;
	// no additional `pub use` is required (and adding one triggers E0255
	// "defined multiple times").
	quote! {
		#[doc(hidden)]
		#[macro_export]
		macro_rules! #macro_name {
			($callback:ident, $app:ident) => { #body };
		}
	}
}

/// Emit a per-fn manifest macro that fans out the list/detail metas.
///
/// `#[url_patterns]` in Phase 6 calls this manifest from inside its
/// `__for_each_url_resolver` arm so the corresponding `<App>Urls` struct
/// gets the typed methods.
///
/// Phase 6.2 (Issue #4507): the manifest carries `#[macro_export]` so it
/// lands at the user crate's root. The body invokes the per-fn meta macros
/// via the same `$crate::` channel because they too are `#[macro_export]`'d
/// (see `emit_meta_macro`).
///
/// Issue #4523 fix: the manifest macro name embeds **both** `<fn>` and
/// `<basename>` so that two `#[viewset]` functions in different modules
/// sharing the same function identifier (e.g. the conventional
/// `pub fn viewset()` in two sibling modules) do not collide on a single
/// crate-root name. The consumer side (`url_patterns::build_viewset_meta_forwarder`)
/// does not know the basename from the call site, so it reaches the
/// manifest through the scope-respecting bundle module
/// `__viewset_resolvers_<fn>`, which re-exports the manifest under the
/// fixed local alias `__for_each_meta` (see `viewset_fn_impl`).
///
/// Refs Issue #4507, Fixes Issue #4523.
fn emit_per_fn_manifest(fn_name: &syn::Ident, basename: &str) -> TokenStream {
	let manifest_name = syn::Ident::new(
		&format!("__for_each_viewset_meta_{fn_name}_{basename}"),
		Span::call_site(),
	);
	let list_meta = syn::Ident::new(
		&format!("__url_resolver_meta_{fn_name}_{basename}_list"),
		Span::call_site(),
	);
	let detail_meta = syn::Ident::new(
		&format!("__url_resolver_meta_{fn_name}_{basename}_detail"),
		Span::call_site(),
	);
	// `#[macro_export]` already publishes the macro at the user crate root;
	// no additional sibling `pub use` is required (and adding one in the
	// same module triggers E0255 "defined multiple times"). A `pub use` from
	// a *child* module (the bundle module) is fine and is used in
	// `viewset_fn_impl` to expose the manifest under a fixed alias.
	quote! {
		#[doc(hidden)]
		#[macro_export]
		macro_rules! #manifest_name {
			($callback:ident, $app:ident) => {
				$crate::#list_meta!($callback, $app);
				$crate::#detail_meta!($callback, $app);
			};
		}
	}
}

/// Dispatcher for the `#[viewset]` attribute macro.
///
/// Branches on whether `input` parses as a free function (`ItemFn` — the
/// classic fn-form that builds a `ModelViewSet`/`GenericViewSet`) or as an
/// `impl` block (`ItemImpl` — the new impl-form that hosts `#[action]`
/// methods).
///
/// Refs Issue #4507.
pub(crate) fn viewset_macro_impl(
	args: TokenStream,
	input: TokenStream,
) -> syn::Result<TokenStream> {
	if let Ok(item_fn) = syn::parse2::<ItemFn>(input.clone()) {
		return viewset_fn_impl(args, item_fn);
	}
	if let Ok(item_impl) = syn::parse2::<syn::ItemImpl>(input.clone()) {
		return viewset_impl_impl(args, item_impl);
	}
	Err(syn::Error::new(
		Span::call_site(),
		"#[viewset] must be applied to a `pub fn ... -> ModelViewSet<...>` \
		 or to an `impl YourViewSet` block",
	))
}

/// Parse the optional `basename = "..."` argument of the fn-form
/// `#[viewset]`.
///
/// Returns:
/// * `Ok(Some(s))` when the caller wrote `#[viewset(basename = "...")]`
///   — the explicit, recommended form (Issue #4549).
/// * `Ok(None)` when the attribute argument list is empty
///   (`#[viewset]`), in which case the caller falls back to the legacy
///   `extract_basename` token walker and the macro emits a deprecation
///   marker pointing the user at the explicit form.
/// * `Err(_)` when a non-empty argument list does not match the
///   `basename = "..."` shape — surfaced as a compile error at the
///   attribute call site.
///
/// Refs Issue #4549.
fn parse_optional_basename_arg(args: TokenStream) -> syn::Result<Option<String>> {
	if args.is_empty() {
		return Ok(None);
	}
	let parser = |input: syn::parse::ParseStream<'_>| -> syn::Result<String> {
		let key: syn::Ident = input.parse()?;
		if key != "basename" {
			return Err(syn::Error::new(
				key.span(),
				"#[viewset] only accepts `basename = \"...\"`. \
				 Example: #[viewset(basename = \"snippet\")]",
			));
		}
		input.parse::<syn::Token![=]>()?;
		let lit: syn::LitStr = input.parse()?;
		Ok(lit.value())
	};
	syn::parse::Parser::parse2(parser, args).map(Some)
}

/// Emit a `#[deprecated]` marker that fires a rustc `deprecated` lint
/// warning at the call site of `#[viewset]` when the legacy body-token
/// `extract_basename` fallback was used.
///
/// The marker is a self-contained `mod` that defines a deprecated
/// `const REASON: () = ()` and a sibling `const _: () = REASON;` that
/// reads it — reading a `#[deprecated]` item is the lint trigger
/// rustc understands on stable. The module is uniquely named per
/// fn so multiple `#[viewset]` invocations in the same module do not
/// produce E0428 ("the name `__viewset_basename_inferred_<fn>` is
/// defined multiple times"). The marker is also gated off on `wasm32`
/// to match the gating applied to the rest of the fn-form expansion
/// — the deprecation has nothing useful to say on the client target.
///
/// `#[doc(hidden)]` keeps the noise out of `cargo doc` output.
///
/// Refs Issue #4549.
fn emit_basename_fallback_deprecation(fn_name: &syn::Ident, basename: &str) -> TokenStream {
	let module_ident = syn::Ident::new(
		&format!("__viewset_basename_inferred_{fn_name}"),
		Span::call_site(),
	);
	let note = format!(
		"#[viewset] inferred basename = \"{basename}\" from the function body. \
		 Prefer the explicit form #[viewset(basename = \"{basename}\")]; the body-walker \
		 fallback will be removed in v0.2.0 (see Issue #4549)."
	);
	quote! {
		#[doc(hidden)]
		#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
		#[allow(non_snake_case)]
		mod #module_ident {
			#[deprecated(note = #note)]
			pub const REASON: () = ();
			// Reading the `#[deprecated]` const is what causes rustc to
			// emit a `deprecated` lint at the `#[viewset]` call site.
			#[allow(deprecated_in_future, clippy::no_effect)]
			const _: () = REASON;
		}
	}
}

/// Fn-form expansion of `#[viewset]`.
///
/// Accepts an optional `basename = "..."` attribute argument (Issue
/// #4549). When the argument is provided, the legacy body-token walker
/// (`extract_basename`) is bypassed and the supplied literal is used
/// directly. When the argument is absent, the macro falls back to the
/// body-walker for one release for backwards compatibility and emits a
/// `#[deprecated]` marker so callers see a deprecation warning at the
/// macro call site. The walker fallback is scheduled for removal in
/// v0.2.0.
///
/// In addition to the resolver-trait emission, the macro emits the
/// per-fn meta + manifest macros consumed by `__for_each_url_resolver`.
fn viewset_fn_impl(args: TokenStream, func: ItemFn) -> syn::Result<TokenStream> {
	let fn_name = &func.sig.ident;

	let explicit_basename = parse_optional_basename_arg(args)?;
	let used_fallback = explicit_basename.is_none();
	let basename = match explicit_basename {
		Some(b) => b,
		None => extract_basename(&func).ok_or_else(|| {
			syn::Error::new_spanned(
				&func.sig.ident,
				"#[viewset] could not extract basename. \
				 Pass it explicitly via #[viewset(basename = \"...\")], or \
				 ensure the function body contains ModelViewSet::new(\"basename\") \
				 or GenericViewSet::new(\"basename\", ...).",
			)
		})?,
	};

	let resolver_tokens = generate_viewset_resolver_tokens(&basename);

	let list_meta = emit_meta_macro(fn_name, &basename, "list", false);
	let detail_meta = emit_meta_macro(fn_name, &basename, "detail", true);
	let manifest = emit_per_fn_manifest(fn_name, &basename);

	// Generate a well-known bundle module that #[url_patterns] can reference
	// by function name (which it CAN see from tokens).
	let bundle_mod_ident =
		syn::Ident::new(&format!("__viewset_resolvers_{fn_name}"), Span::call_site());
	let list_mod_ident = syn::Ident::new(
		&format!("__url_resolver_{basename}_list"),
		Span::call_site(),
	);
	let detail_mod_ident = syn::Ident::new(
		&format!("__url_resolver_{basename}_detail"),
		Span::call_site(),
	);
	// Issue #4523 fix: the manifest macro name embeds `<basename>` to avoid
	// crate-root collisions between two `#[viewset] pub fn viewset()` in
	// different modules. The consumer reaches it via the scope-respecting
	// bundle module below under the fixed alias `__for_each_meta`.
	let manifest_name = syn::Ident::new(
		&format!("__for_each_viewset_meta_{fn_name}_{basename}"),
		Span::call_site(),
	);

	// Issue #4549: emit a `#[deprecated]`-driven warning when the caller
	// relied on the legacy body-token `extract_basename` fallback instead
	// of passing `basename = "..."` explicitly. The marker uses the
	// const-read-of-deprecated-item pattern because rustc only fires the
	// `deprecated` lint at the *use site* of a `#[deprecated]` item,
	// not at its definition. The marker is scoped to a module so multiple
	// `#[viewset]` invocations do not collide on the const identifier.
	let deprecation_marker = if used_fallback {
		emit_basename_fallback_deprecation(fn_name, &basename)
	} else {
		TokenStream::new()
	};

	Ok(quote! {
		#func
		#deprecation_marker
		#resolver_tokens
		#list_meta
		#detail_meta
		#manifest

		#[doc(hidden)]
		pub mod #bundle_mod_ident {
			#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
			pub use super::#list_mod_ident::*;
			#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
			pub use super::#detail_mod_ident::*;
			// Scope-respecting alias for the manifest macro (Issue #4523).
			// `pub use` of a `#[macro_export]`'d macro from a child module
			// is supported in Rust 2018+ and does not trigger E0255.
			#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
			pub use crate::#manifest_name as __for_each_meta;
		}
	})
}

/// Parse the `basename = "..."` argument required by the impl-form
/// of `#[viewset]`.
///
/// Returns the basename string on success, or a compile error pointing
/// to the call site when the argument list is empty or shaped incorrectly.
///
/// Refs Issue #4507.
fn parse_impl_basename_arg(args: TokenStream) -> syn::Result<String> {
	let parser = |input: syn::parse::ParseStream<'_>| -> syn::Result<String> {
		let key: syn::Ident = input.parse()?;
		if key != "basename" {
			return Err(syn::Error::new(
				key.span(),
				"#[viewset] on impl block requires basename = \"...\". \
				 Example: #[viewset(basename = \"snippet\")]",
			));
		}
		input.parse::<syn::Token![=]>()?;
		let lit: syn::LitStr = input.parse()?;
		Ok(lit.value())
	};

	if args.is_empty() {
		return Err(syn::Error::new(
			Span::call_site(),
			"#[viewset] on impl block requires basename = \"...\". \
			 Example: #[viewset(basename = \"snippet\")]",
		));
	}
	syn::parse::Parser::parse2(parser, args)
}

/// Impl-form expansion of `#[viewset]`.
///
/// Walks the `impl` block for `#[action]`-decorated methods, emits a
/// `__url_resolver_meta_action_<basename>_<url_name>` macro_rules per action,
/// and a `__for_each_viewset_action_meta_<TypeNameSnake>` manifest that
/// fans them out. Phase 6 splices the manifest into the existing
/// `__for_each_url_resolver` so typed methods land on `<App>Urls`.
///
/// Phase 5.1 (Issue #4507): also emits `inventory::submit!` blocks that
/// register each action's runtime metadata under the marker type's
/// `std::any::type_name`. `viewset_with_actions::<V, M>(...)` then bridges
/// these marker-keyed entries into the concrete `type_name::<V>()` slot so
/// `ViewSet::get_extra_actions` (and the dispatcher's route registration
/// in `DefaultRouter::register_viewset`) finds them. This eliminates the
/// previous requirement for users to manually call `register_action(...)`
/// at startup to make `#[action]`-decorated methods discoverable.
///
/// Refs Issue #4507.
fn viewset_impl_impl(args: TokenStream, item_impl: syn::ItemImpl) -> syn::Result<TokenStream> {
	let basename = parse_impl_basename_arg(args)?;
	let (action_metas, url_names) = collect_actions(&item_impl, &basename)?;
	let type_snake = type_name_to_snake(&item_impl.self_ty)?;
	let manifest = emit_impl_action_manifest(&type_snake, &basename, &url_names);
	let runtime_registrations = emit_runtime_action_registrations(&item_impl)?;

	Ok(quote! {
		#item_impl
		#(#action_metas)*
		#manifest
		#runtime_registrations
	})
}

/// Emit a `#[ctor]`-driven startup function that registers each
/// `#[action]`-decorated method's runtime metadata under the marker type's
/// `std::any::type_name`.
///
/// `#[ctor]` runs at process startup (before `main` for binaries, before the
/// first test for `cargo test`), which sidesteps the unstable
/// `const_type_name` feature: `type_name::<T>()` is callable in regular
/// (non-const) function context, but `inventory::submit!`'s `static`
/// initializer cannot use it yet (rust-lang/rust#63084 — `const_type_name`
/// is still unstable as of Rust 1.94).
///
/// The handler is intentionally left as the `ActionMetadata::new`-default
/// no-op: the user's method has a free-form signature (e.g.
/// `async fn highlight(_id: String) -> ViewResult<Response>`) that does not
/// match `FunctionActionHandler`'s required
/// `fn(Request) -> Pin<Box<dyn Future<Output = Result<Response>>>>` shape,
/// so we can't automatically wire it. URL reversal — the primary consumer
/// of these entries via `urls.server().<app>().<action>(...)` — only needs
/// the route name and path metadata, which `ActionMetadata::new` carries
/// faithfully. Future work on Issue #4507 may extend `#[action]` to emit a
/// signature-adapted wrapper that the macro can plug in here.
///
/// Registration is keyed on the marker type (e.g. `SnippetViewSet`). The
/// `viewset_with_actions::<V, M>` runtime helper bridges these
/// marker-keyed entries into the concrete `type_name::<V>()` slot.
///
/// Refs Issue #4507.
fn emit_runtime_action_registrations(
	item_impl: &syn::ItemImpl,
) -> syn::Result<proc_macro2::TokenStream> {
	let marker_ty = &item_impl.self_ty;
	let views_crate = crate::crate_paths::get_reinhardt_views_crate();
	let hyper_crate = crate::crate_paths::get_hyper_crate();
	let type_snake = type_name_to_snake(marker_ty)?;
	let ctor_fn_ident = syn::Ident::new(
		&format!("__reinhardt_register_viewset_actions_{type_snake}"),
		Span::call_site(),
	);
	let mut registrations: Vec<proc_macro2::TokenStream> = Vec::new();

	for item in &item_impl.items {
		let syn::ImplItem::Fn(method) = item else {
			continue;
		};
		let Some(action_attr) = method.attrs.iter().find(|a| a.path().is_ident("action")) else {
			continue;
		};
		let attr_tokens = match &action_attr.meta {
			syn::Meta::List(ml) => ml.tokens.clone(),
			syn::Meta::Path(_) => proc_macro2::TokenStream::new(),
			_ => {
				return Err(syn::Error::new_spanned(
					action_attr,
					"unexpected #[action] form",
				));
			}
		};
		let parsed =
			crate::action::parse_action_args_with_defaults(attr_tokens, &method.sig.ident)?;

		// Build the `ActionMetadata::new(...).with_...` chain as runtime tokens.
		// `with_url_path` is only emitted when the attribute supplied a non-empty
		// `url_path`; `ActionMetadata::get_url_path` already defaults to
		// `name.replace('_', "-")` when absent. The handler is left as the
		// constructor default (see fn-doc above).
		let url_name_lit = syn::LitStr::new(&parsed.url_name, Span::call_site());
		let detail = parsed.detail;
		let with_url_path = if parsed.url_path.is_empty() {
			quote! {}
		} else {
			let lit = syn::LitStr::new(&parsed.url_path, Span::call_site());
			quote! { .with_url_path(#lit) }
		};

		// Method-list literal. `hyper::Method::from_bytes` is case-sensitive
		// per RFC 7230 and rejects lowercase input (e.g. `b"post"`) at
		// runtime; the impl-form `#[viewset]` validation path
		// (`parse_action_args_with_defaults`) does not currently canonicalize
		// or reject lowercase method names, so a `#[action(methods = "post",
		// ...)]` would otherwise compile and then panic via `.expect()` when
		// the registration ctor runs. Uppercase here so the generated bytes
		// always match `from_bytes`'s contract, regardless of how the user
		// spelled them.
		let method_lits: Vec<proc_macro2::TokenStream> = parsed
			.methods
			.iter()
			.map(|m| {
				let normalized = m.to_ascii_uppercase();
				let lit = syn::LitStr::new(&normalized, Span::call_site());
				quote! {
					#hyper_crate::Method::from_bytes(#lit.as_bytes())
						.expect("#[action] methods are uppercased at macro expansion")
				}
			})
			.collect();

		registrations.push(quote! {
			#views_crate::viewsets::register_action(
				::std::any::type_name::<#marker_ty>(),
				#views_crate::viewsets::ActionMetadata::new(#url_name_lit)
					.with_detail(#detail)
					.with_url_name(#url_name_lit)
					.with_methods(vec![#(#method_lits),*])
					#with_url_path,
			);
		});
	}

	if registrations.is_empty() {
		return Ok(proc_macro2::TokenStream::new());
	}

	Ok(quote! {
		#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
		#[doc(hidden)]
		#[::ctor::ctor]
		fn #ctor_fn_ident() {
			#(#registrations)*
		}
	})
}

/// Convert the last segment of a type path to snake_case.
///
/// `SnippetViewSet` becomes `snippet_view_set`; `views::SnippetViewSet`
/// also becomes `snippet_view_set` (path qualifiers are stripped).
///
/// Refs Issue #4507.
fn type_name_to_snake(ty: &syn::Type) -> syn::Result<String> {
	let path = match ty {
		syn::Type::Path(tp) => &tp.path,
		_ => return Err(syn::Error::new_spanned(ty, "expected a type path")),
	};
	let ident = &path
		.segments
		.last()
		.ok_or_else(|| syn::Error::new_spanned(ty, "empty type path"))?
		.ident;
	Ok(camel_to_snake(&ident.to_string()))
}

/// Convert a `CamelCase` identifier to `snake_case`.
///
/// Inserts an underscore before every ASCII uppercase character after
/// position 0, then lowercases the entire string. Non-ASCII characters
/// pass through unchanged on the boundary check.
///
/// Refs Issue #4507.
fn camel_to_snake(s: &str) -> String {
	let mut out = String::with_capacity(s.len() + 4);
	for (i, c) in s.chars().enumerate() {
		if c.is_ascii_uppercase() && i != 0 {
			out.push('_');
		}
		out.push(c.to_ascii_lowercase());
	}
	out
}

/// Collect all `#[action]`-decorated methods from an `impl` block.
///
/// Returns a pair of vectors: the per-action meta macro token streams,
/// and the corresponding `url_name` values (defaulting to the method's
/// identifier when the `#[action]` attribute omits `url_name`).
/// Non-`#[action]` items in the impl block (helpers, associated consts,
/// type aliases) are skipped silently.
///
/// Refs Issue #4507.
fn collect_actions(
	item_impl: &syn::ItemImpl,
	basename: &str,
) -> syn::Result<(Vec<proc_macro2::TokenStream>, Vec<String>)> {
	let mut metas = Vec::new();
	let mut url_names = Vec::new();
	for item in &item_impl.items {
		let syn::ImplItem::Fn(method) = item else {
			continue;
		};
		let Some(action_attr) = method.attrs.iter().find(|a| a.path().is_ident("action")) else {
			continue;
		};
		// Re-parse the attribute to recover the (defaulted) url_name. Phase 5
		// keeps the parser inside `action.rs` and reuses it here so the
		// manifest body and the per-action meta macro names stay in lock-step.
		let attr_tokens = match &action_attr.meta {
			syn::Meta::List(ml) => ml.tokens.clone(),
			syn::Meta::Path(_) => proc_macro2::TokenStream::new(),
			_ => {
				return Err(syn::Error::new_spanned(
					action_attr,
					"unexpected #[action] form",
				));
			}
		};
		let parsed =
			crate::action::parse_action_args_with_defaults(attr_tokens, &method.sig.ident)?;
		let meta = parse_action_meta_for_viewset(action_attr, &method.sig.ident, basename)?;
		metas.push(meta);
		url_names.push(parsed.url_name);
	}
	Ok((metas, url_names))
}

/// Parse a single `#[action(...)]` attribute and emit its
/// `__url_resolver_meta_action_<basename>_<url_name>` macro.
///
/// The emitted macro fans `$callback` out with the route name
/// (`"<basename>-<url_name>"`) and the parameter list:
/// `"id"` first when `detail = true`, then every placeholder extracted
/// from `url_path` (e.g. `"/children/{child_id}"` contributes `"child_id"`).
///
/// Replaces the Phase 4 stub which always emitted `"id"` and ignored
/// `url_path` / `url_name`.
///
/// Refs Issue #4507.
fn parse_action_meta_for_viewset(
	attr: &syn::Attribute,
	fn_ident: &syn::Ident,
	basename: &str,
) -> syn::Result<proc_macro2::TokenStream> {
	let attr_tokens = match &attr.meta {
		syn::Meta::List(ml) => ml.tokens.clone(),
		syn::Meta::Path(_) => proc_macro2::TokenStream::new(),
		_ => return Err(syn::Error::new_spanned(attr, "unexpected #[action] form")),
	};
	let meta = crate::action::parse_action_args_with_defaults(attr_tokens, fn_ident)?;

	let macro_name = syn::Ident::new(
		&format!("__url_resolver_meta_action_{basename}_{}", meta.url_name),
		Span::call_site(),
	);
	let method_ident = syn::Ident::new(&meta.url_name, Span::call_site());
	let route_literal = format!("{basename}-{}", meta.url_name);

	let mut param_literals: Vec<proc_macro2::TokenStream> = Vec::new();
	if meta.detail {
		param_literals.push(quote! { "id" });
	}
	for p in crate::url_patterns::extract_url_params_pub(&meta.url_path) {
		let lit = syn::LitStr::new(&p, Span::call_site());
		param_literals.push(quote! { #lit });
	}

	let body = if param_literals.is_empty() {
		quote! { $callback!($app, #method_ident, #route_literal, ); }
	} else {
		quote! { $callback!($app, #method_ident, #route_literal, #(#param_literals),*); }
	};

	// Phase 6.2 (Issue #4507): `#[macro_export]` puts the per-action meta
	// at the user crate root so the per-impl manifest can call it via
	// `$crate::__url_resolver_meta_action_<basename>_<url_name>!`. Names
	// are pre-collision-resistant via `<basename>` + `<url_name>`. No
	// additional `pub use` is needed (it would conflict with the
	// `#[macro_export]` re-export and trigger E0255).
	Ok(quote! {
		#[doc(hidden)]
		#[macro_export]
		macro_rules! #macro_name {
			($callback:ident, $app:ident) => { #body };
		}
	})
}

/// Emit the per-impl manifest macro that fans out every per-action meta.
///
/// The manifest is named `__for_each_viewset_action_meta_<TypeNameSnake>`
/// so Phase 6 can call it from `__for_each_url_resolver`'s arm without
/// needing to see the ViewSet's basename at the `#[url_patterns]` site.
///
/// Each entry references the per-action meta macro by its `url_name`-derived
/// name (post-Phase-5), not the bare method identifier.
///
/// Refs Issue #4507.
fn emit_impl_action_manifest(
	type_snake: &str,
	basename: &str,
	url_names: &[String],
) -> proc_macro2::TokenStream {
	let manifest_name = syn::Ident::new(
		&format!("__for_each_viewset_action_meta_{type_snake}"),
		Span::call_site(),
	);
	let meta_calls: Vec<proc_macro2::TokenStream> = url_names
		.iter()
		.map(|n| {
			let meta_name = syn::Ident::new(
				&format!("__url_resolver_meta_action_{basename}_{n}"),
				Span::call_site(),
			);
			// Phase 6.2 (Issue #4507): per-action meta macros live at the
			// user crate root via `#[macro_export]`, so the manifest body
			// reaches them via `$crate::`.
			quote! { $crate::#meta_name!($callback, $app); }
		})
		.collect();
	// Phase 6.2 (Issue #4507): `#[macro_export]` puts the manifest at the
	// user crate root so the `#[url_patterns]`-generated forwarder can
	// reach it via `$crate::__for_each_viewset_action_meta_<TypeSnake>!`.
	// No additional `pub use` is required.
	quote! {
		#[doc(hidden)]
		#[macro_export]
		macro_rules! #manifest_name {
			($callback:ident, $app:ident) => {
				#(#meta_calls)*
			};
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn extract_basename_model_viewset() {
		let func: ItemFn = parse2(quote! {
			pub fn viewset() -> ModelViewSet<Snippet, SnippetSerializer> {
				ModelViewSet::new("snippet")
					.with_pagination(PaginationConfig::page_number(10, Some(100)))
			}
		})
		.unwrap();

		assert_eq!(extract_basename(&func), Some("snippet".to_string()));
	}

	#[test]
	fn extract_basename_generic_viewset() {
		let func: ItemFn = parse2(quote! {
			pub fn viewset() -> GenericViewSet<User> {
				GenericViewSet::new("user", ())
			}
		})
		.unwrap();

		assert_eq!(extract_basename(&func), Some("user".to_string()));
	}

	#[test]
	fn extract_basename_not_found() {
		let func: ItemFn = parse2(quote! {
			pub fn viewset() -> ServerRouter {
				ServerRouter::new()
			}
		})
		.unwrap();

		assert_eq!(extract_basename(&func), None);
	}

	#[test]
	fn to_pascal_case_single() {
		assert_eq!(to_pascal_case("snippet"), "Snippet");
	}

	#[test]
	fn to_pascal_case_multi() {
		assert_eq!(to_pascal_case("auth_user"), "AuthUser");
	}

	#[test]
	fn generate_resolver_tokens_contains_expected_identifiers() {
		let tokens = generate_viewset_resolver_tokens("snippet");
		let output = tokens.to_string();
		assert!(output.contains("__url_resolver_snippet_list"));
		assert!(output.contains("__url_resolver_snippet_detail"));
		assert!(output.contains("ResolveSnippetList"));
		assert!(output.contains("ResolveSnippetDetail"));
		assert!(output.contains("snippet_list"));
		assert!(output.contains("snippet_detail"));
	}

	#[test]
	fn fn_version_emits_list_and_detail_meta_macros() {
		// Arrange
		let input = quote! {
			pub fn viewset() -> ModelViewSet<Snippet, SnippetSerializer> {
				ModelViewSet::new("snippet")
			}
		};

		// Act
		let out = viewset_macro_impl(quote! {}, input).expect("should expand");
		let out_s = out.to_string();

		// Assert: list meta macro exists with the expected callback shape.
		assert!(
			out_s.contains("__url_resolver_meta_viewset_snippet_list"),
			"list meta macro must be emitted; got: {out_s}"
		);
		assert!(
			out_s.contains("\"snippet-list\""),
			"list route-name literal must be emitted"
		);

		// Detail meta + "id" param literal
		assert!(out_s.contains("__url_resolver_meta_viewset_snippet_detail"));
		assert!(out_s.contains("\"snippet-detail\""));
		assert!(out_s.contains("\"id\""));
	}

	#[test]
	fn fn_version_emits_for_each_viewset_meta_manifest() {
		// Arrange
		let input = quote! {
			pub fn viewset() -> ModelViewSet<Snippet, SnippetSerializer> {
				ModelViewSet::new("snippet")
			}
		};

		// Act
		let out_s = viewset_macro_impl(quote! {}, input).unwrap().to_string();

		// Assert: manifest macro that fans out to both meta macros exists,
		// keyed on both fn_name AND basename (Issue #4523).
		assert!(
			out_s.contains("__for_each_viewset_meta_viewset_snippet"),
			"fn-form must emit per-fn manifest macro keyed on <fn>_<basename>; got: {out_s}"
		);
		// And the bundle module must re-export it under the fixed alias
		// `__for_each_meta` so the consumer can reach it through the
		// scope-respecting module path (Issue #4523).
		assert!(
			out_s.contains(
				"pub use crate :: __for_each_viewset_meta_viewset_snippet as __for_each_meta"
			),
			"bundle module must re-export the manifest under fixed alias `__for_each_meta`; got: {out_s}"
		);
	}

	#[test]
	fn fn_version_manifest_does_not_collide_across_basenames() {
		// Arrange: two #[viewset] fns sharing the conventional `viewset`
		// fn name but with different basenames must yield distinct
		// crate-root manifest macro names (Issue #4523).
		let snippet_input = quote! {
			pub fn viewset() -> ModelViewSet<Snippet, SnippetSerializer> {
				ModelViewSet::new("snippet")
			}
		};
		let post_input = quote! {
			pub fn viewset() -> ModelViewSet<Post, PostSerializer> {
				ModelViewSet::new("post")
			}
		};

		// Act
		let snippet_out = viewset_macro_impl(quote! {}, snippet_input)
			.unwrap()
			.to_string();
		let post_out = viewset_macro_impl(quote! {}, post_input)
			.unwrap()
			.to_string();

		// Assert: the two manifests have non-overlapping crate-root names.
		assert!(snippet_out.contains("__for_each_viewset_meta_viewset_snippet"));
		assert!(post_out.contains("__for_each_viewset_meta_viewset_post"));
		assert!(!snippet_out.contains("__for_each_viewset_meta_viewset_post"));
		assert!(!post_out.contains("__for_each_viewset_meta_viewset_snippet"));
	}

	#[test]
	fn impl_version_requires_basename_arg() {
		// Arrange
		let args = quote! {};
		let input = quote! {
			impl SnippetViewSet {
				#[action(methods = "POST", detail = true, url_name = "highlight")]
				async fn highlight(&self) -> () {}
			}
		};

		// Act
		let err = viewset_macro_impl(args, input).unwrap_err().to_string();

		// Assert
		assert!(err.contains("requires basename"), "got: {err}");
	}

	#[test]
	fn impl_version_accepts_basename_arg() {
		// Arrange
		let args = quote! { basename = "snippet" };
		let input = quote! {
			impl SnippetViewSet {
				#[action(methods = "POST", detail = true, url_name = "highlight")]
				async fn highlight(&self) -> () {}
			}
		};

		// Act
		let out = viewset_macro_impl(args, input).expect("impl form should expand");
		let out_s = out.to_string();

		// Assert
		assert!(
			out_s.contains("SnippetViewSet"),
			"impl block should be preserved; got: {out_s}"
		);
	}

	#[test]
	fn type_name_to_snake_camel_case() {
		// Arrange
		let ty: syn::Type = syn::parse_quote! { SnippetViewSet };
		// Act + Assert
		assert_eq!(type_name_to_snake(&ty).unwrap(), "snippet_view_set");
	}

	#[test]
	fn type_name_to_snake_path() {
		// Arrange
		let ty: syn::Type = syn::parse_quote! { views::SnippetViewSet };
		// Act + Assert
		assert_eq!(type_name_to_snake(&ty).unwrap(), "snippet_view_set");
	}

	#[test]
	fn collect_actions_two_actions() {
		// Arrange
		let item_impl: syn::ItemImpl = syn::parse_quote! {
			impl SnippetViewSet {
				#[action(methods = "POST", detail = true, url_name = "highlight")]
				async fn highlight(&self) -> () {}
				#[action(methods = "GET", detail = false, url_name = "export")]
				async fn export(&self) -> () {}
			}
		};

		// Act
		let (metas, url_names) = collect_actions(&item_impl, "snippet").unwrap();

		// Assert
		assert_eq!(metas.len(), 2);
		assert_eq!(
			url_names,
			vec!["highlight".to_string(), "export".to_string()]
		);
		let combined = metas.iter().map(|t| t.to_string()).collect::<String>();
		assert!(combined.contains("__url_resolver_meta_action_snippet_highlight"));
		assert!(combined.contains("__url_resolver_meta_action_snippet_export"));
	}

	#[test]
	fn collect_actions_skips_non_action_methods() {
		// Arrange
		let item_impl: syn::ItemImpl = syn::parse_quote! {
			impl SnippetViewSet {
				#[action(methods = "POST", detail = true, url_name = "highlight")]
				async fn highlight(&self) -> () {}
				async fn helper(&self) -> () {}
			}
		};

		// Act
		let (metas, url_names) = collect_actions(&item_impl, "snippet").unwrap();

		// Assert
		assert_eq!(metas.len(), 1);
		assert_eq!(url_names.len(), 1);
	}

	#[test]
	fn impl_form_manifest_references_each_action_meta() {
		// Arrange
		let args = quote! { basename = "snippet" };
		let input = quote! {
			impl SnippetViewSet {
				#[action(methods = "POST", detail = true, url_name = "highlight")]
				async fn highlight(&self) -> () {}
				#[action(methods = "GET", detail = false, url_name = "export")]
				async fn export(&self) -> () {}
			}
		};

		// Act
		let out_s = viewset_macro_impl(args, input).unwrap().to_string();

		// Assert
		assert!(out_s.contains("__for_each_viewset_action_meta_snippet_view_set"));
		assert!(out_s.contains("__url_resolver_meta_action_snippet_highlight"));
		assert!(out_s.contains("__url_resolver_meta_action_snippet_export"));
	}

	#[test]
	fn impl_action_meta_without_id_when_detail_false() {
		// Arrange
		let args = quote! { basename = "snippet" };
		let input = quote! {
			impl SnippetViewSet {
				#[action(methods = "GET", detail = false, url_name = "export")]
				async fn export(&self) -> () {}
			}
		};

		// Act
		let out_s = viewset_macro_impl(args, input).unwrap().to_string();

		// Assert
		let pos = out_s
			.find("__url_resolver_meta_action_snippet_export")
			.expect("export meta must be present");
		// Look at a window of the expansion around the export meta definition.
		let snippet = &out_s[pos..(pos + 600).min(out_s.len())];
		assert!(
			!snippet.contains("\"id\""),
			"detail=false meta must omit \"id\" param; got: {snippet}"
		);
	}

	#[test]
	fn impl_action_meta_with_url_path_params() {
		// Arrange
		let args = quote! { basename = "snippet" };
		let input = quote! {
			impl SnippetViewSet {
				#[action(
					methods = "GET",
					detail = true,
					url_name = "child",
					url_path = "/children/{child_id}"
				)]
				async fn child(&self) -> () {}
			}
		};

		// Act
		let out_s = viewset_macro_impl(args, input).unwrap().to_string();

		// Assert
		let pos = out_s
			.find("__url_resolver_meta_action_snippet_child")
			.unwrap();
		let snippet = &out_s[pos..(pos + 800).min(out_s.len())];
		assert!(snippet.contains("\"id\""), "detail=true must include id");
		assert!(
			snippet.contains("\"child_id\""),
			"url_path placeholders must be added"
		);
	}

	#[test]
	fn fn_form_marks_legacy_blanket_trait_as_deprecated() {
		// Arrange
		let input = quote! {
			pub fn viewset() -> ModelViewSet<Snippet, S> {
				ModelViewSet::new("snippet")
			}
		};

		// Act
		let out_s = viewset_macro_impl(quote! {}, input).unwrap().to_string();

		// Assert: no #[deprecated] markers remain on the legacy blanket traits
		// since they were removed in Issue #4520.
		let occurrences =
			out_s.matches("# [deprecated").count() + out_s.matches("#[deprecated").count();
		assert_eq!(
			occurrences, 0,
			"expected 0 #[deprecated] markers (removed in #4520), got {occurrences}; out={out_s}"
		);
	}

	#[test]
	fn fn_form_legacy_trait_uses_url_resolver_unprefixed_supertrait() {
		// Arrange
		let input = quote! {
			pub fn viewset() -> ModelViewSet<Snippet, S> {
				ModelViewSet::new("snippet")
			}
		};

		// Act
		let out_s = viewset_macro_impl(quote! {}, input).unwrap().to_string();

		// Assert: the emitted trait's supertrait must be the unprefixed
		// resolver variant (rather than the base UrlResolver) so that the
		// blanket impl this macro emits is keyed against the same supertrait
		// as the corresponding implementation the routes macro emits for
		// ResolvedUrls. Mismatched supertraits would produce two overlapping
		// blanket impls on the same target and trigger E0119 at the consumer
		// crate root.
		assert!(
			out_s.contains("UrlResolverUnprefixed"),
			"trait must reference UrlResolverUnprefixed as supertrait; got: {out_s}"
		);
	}

	#[test]
	fn fn_form_emits_deprecation_when_basename_arg_absent() {
		// Arrange: bare `#[viewset]` triggers the legacy body-walker
		// fallback path that the Issue #4549 deprecation flow targets.
		let input = quote! {
			pub fn viewset() -> ModelViewSet<Snippet, S> {
				ModelViewSet::new("snippet")
			}
		};

		// Act
		let out_s = viewset_macro_impl(quote! {}, input).unwrap().to_string();

		// Assert: the per-fn deprecation marker module is emitted, and it
		// contains the `#[deprecated]` const that triggers the rustc
		// `deprecated` lint at the macro call site.
		assert!(
			out_s.contains("__viewset_basename_inferred_viewset"),
			"fallback path must emit per-fn deprecation marker module; got: {out_s}"
		);
		// The note steers users toward the explicit form.
		assert!(
			out_s.contains("#[viewset(basename = \\\"snippet\\\")]"),
			"deprecation note must show the recommended explicit form; got: {out_s}"
		);
		// The marker reads its own deprecated const so rustc fires the lint.
		assert!(
			out_s.contains("const _ : () = REASON ;") || out_s.contains("const _: () = REASON;"),
			"deprecation marker must read the deprecated const; got: {out_s}"
		);
	}

	#[test]
	fn fn_form_skips_deprecation_when_basename_arg_provided() {
		// Arrange: explicit `basename = "..."` bypasses the body-walker
		// and the associated deprecation marker (Issue #4549).
		let args = quote! { basename = "snippet" };
		let input = quote! {
			pub fn viewset() -> ModelViewSet<Snippet, S> {
				ModelViewSet::new("snippet")
			}
		};

		// Act
		let out_s = viewset_macro_impl(args, input).unwrap().to_string();

		// Assert: explicit basename callers must not see the deprecation
		// marker module nor a reference to it.
		assert!(
			!out_s.contains("__viewset_basename_inferred_viewset"),
			"explicit basename must NOT emit the deprecation marker; got: {out_s}"
		);
	}

	#[test]
	fn fn_form_explicit_basename_overrides_body_walker() {
		// Arrange: the fn body uses `ModelViewSet::new("auto_snippet")`,
		// but the attribute requests `basename = "snippet"`. The explicit
		// argument must win (Issue #4549) so the generated resolver
		// modules use `snippet`, not `auto_snippet`.
		let args = quote! { basename = "snippet" };
		let input = quote! {
			pub fn viewset() -> ModelViewSet<Snippet, S> {
				ModelViewSet::new("auto_snippet")
			}
		};

		// Act
		let out_s = viewset_macro_impl(args, input).unwrap().to_string();

		// Assert
		assert!(
			out_s.contains("__url_resolver_snippet_list"),
			"explicit basename must drive the list resolver mod name; got: {out_s}"
		);
		assert!(
			!out_s.contains("__url_resolver_auto_snippet_list"),
			"body-walker result must be ignored when basename is explicit; got: {out_s}"
		);
	}

	#[test]
	fn fn_form_rejects_unknown_attribute_arg() {
		// Arrange: only `basename = "..."` is accepted by the fn-form
		// attribute argument list. Anything else is a hard compile error
		// at the call site (Issue #4549).
		let args = quote! { foo = "bar" };
		let input = quote! {
			pub fn viewset() -> ModelViewSet<Snippet, S> {
				ModelViewSet::new("snippet")
			}
		};

		// Act
		let err = viewset_macro_impl(args, input).unwrap_err().to_string();

		// Assert
		assert!(
			err.contains("only accepts `basename = \"...\"`"),
			"unknown attr arg must produce the explicit error; got: {err}"
		);
	}

	#[test]
	fn impl_form_manifest_uses_url_name_in_meta_calls() {
		// Arrange
		let args = quote! { basename = "snippet" };
		let input = quote! {
			impl SnippetViewSet {
				#[action(methods = "POST", detail = true, url_name = "highlight_code")]
				async fn highlight(&self) -> () {}
			}
		};

		// Act
		let out_s = viewset_macro_impl(args, input).unwrap().to_string();

		// Assert
		assert!(
			out_s.contains("__url_resolver_meta_action_snippet_highlight_code"),
			"manifest body must reference the url_name-derived meta name; got: {out_s}"
		);
	}
}
