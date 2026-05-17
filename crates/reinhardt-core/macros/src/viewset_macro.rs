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
			pub trait #list_trait_ident: #reinhardt_crate::UrlResolver {
				#[doc = #list_doc]
				fn #list_method_ident(&self) -> String {
					self.resolve_url(#list_route_name, &[])
				}
			}
			#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
			impl<T: #reinhardt_crate::UrlResolver> #list_trait_ident for T {}
		}
		#[doc(hidden)]
		pub use #list_mod_ident::*;

		#[doc(hidden)]
		pub mod #detail_mod_ident {
			#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
			#[doc = #detail_doc]
			pub trait #detail_trait_ident: #reinhardt_crate::UrlResolver {
				#[doc = #detail_doc]
				fn #detail_method_ident(&self, id: &str) -> String {
					self.resolve_url(#detail_route_name, &[("id", id)])
				}
			}
			#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
			impl<T: #reinhardt_crate::UrlResolver> #detail_trait_ident for T {}
		}
		#[doc(hidden)]
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

	quote! {
		#[doc(hidden)]
		macro_rules! #macro_name {
			($callback:ident, $app:ident) => { #body };
		}
		pub(crate) use #macro_name;
	}
}

/// Emit a per-fn manifest macro that fans out the list/detail metas.
///
/// `#[url_patterns]` in Phase 6 calls this manifest from inside its
/// `__for_each_url_resolver` arm so the corresponding `<App>Urls` struct
/// gets the typed methods.
///
/// Refs Issue #4507.
fn emit_per_fn_manifest(fn_name: &syn::Ident, basename: &str) -> TokenStream {
	let manifest_name = syn::Ident::new(
		&format!("__for_each_viewset_meta_{fn_name}"),
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
	quote! {
		#[doc(hidden)]
		macro_rules! #manifest_name {
			($callback:ident, $app:ident) => {
				#list_meta!($callback, $app);
				#detail_meta!($callback, $app);
			};
		}
		pub(crate) use #manifest_name;
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

/// Fn-form expansion of `#[viewset]`.
///
/// Pre-existing behaviour: extracts the basename from the function body
/// (`ModelViewSet::new("...")` / `GenericViewSet::new("...", ...)`),
/// generates the typed list/detail resolver traits, and emits the
/// per-fn meta + manifest macros consumed by `__for_each_url_resolver`.
fn viewset_fn_impl(_args: TokenStream, func: ItemFn) -> syn::Result<TokenStream> {
	let fn_name = &func.sig.ident;

	let basename = extract_basename(&func).ok_or_else(|| {
		syn::Error::new_spanned(
			&func.sig.ident,
			"#[viewset] could not extract basename. \
			 Expected ModelViewSet::new(\"basename\") or \
			 GenericViewSet::new(\"basename\", ...) in the function body.",
		)
	})?;

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

	Ok(quote! {
		#func
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
/// `__url_resolver_meta_action_<basename>_<fn>` macro_rules per action,
/// and a `__for_each_viewset_action_meta_<TypeNameSnake>` manifest that
/// fans them out. Phase 6 will splice the manifest into the existing
/// `__for_each_url_resolver` so typed methods land on `<App>Urls`.
///
/// Refs Issue #4507.
fn viewset_impl_impl(args: TokenStream, item_impl: syn::ItemImpl) -> syn::Result<TokenStream> {
	let basename = parse_impl_basename_arg(args)?;
	let (action_metas, fn_idents) = collect_actions(&item_impl, &basename)?;
	let type_snake = type_name_to_snake(&item_impl.self_ty)?;
	let manifest = emit_impl_action_manifest(&type_snake, &basename, &fn_idents);

	Ok(quote! {
		#item_impl
		#(#action_metas)*
		#manifest
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
/// and the bare method identifiers in declaration order. Non-`#[action]`
/// items in the impl block (helpers, associated consts, type aliases)
/// are skipped silently.
///
/// Refs Issue #4507.
fn collect_actions(
	item_impl: &syn::ItemImpl,
	basename: &str,
) -> syn::Result<(Vec<proc_macro2::TokenStream>, Vec<syn::Ident>)> {
	let mut metas = Vec::new();
	let mut fn_idents = Vec::new();
	for item in &item_impl.items {
		let syn::ImplItem::Fn(method) = item else {
			continue;
		};
		let Some(action_attr) = method.attrs.iter().find(|a| a.path().is_ident("action")) else {
			continue;
		};
		let meta = parse_action_meta_for_viewset(action_attr, &method.sig.ident, basename)?;
		metas.push(meta);
		fn_idents.push(method.sig.ident.clone());
	}
	Ok((metas, fn_idents))
}

/// TEMPORARY (Phase 4): real implementation lands in Phase 5 / Task 5.2.
/// This stub assumes `detail = true` for every action (always emits an "id" param)
/// and ignores the action's `url_path` placeholders. Phase 5 will parse the
/// full `#[action(...)]` attribute and replace this body. The macro_rules name
/// here uses `fn_ident`; Phase 5 will switch it to `url_name`.
///
/// Refs Issue #4507.
fn parse_action_meta_for_viewset(
	_attr: &syn::Attribute,
	fn_ident: &syn::Ident,
	basename: &str,
) -> syn::Result<proc_macro2::TokenStream> {
	let macro_name = syn::Ident::new(
		&format!("__url_resolver_meta_action_{basename}_{fn_ident}"),
		Span::call_site(),
	);
	let method_ident = fn_ident.clone();
	let route = format!("{basename}-{fn_ident}");
	Ok(quote! {
		#[doc(hidden)]
		macro_rules! #macro_name {
			($callback:ident, $app:ident) => {
				$callback!($app, #method_ident, #route, "id");
			};
		}
		pub(crate) use #macro_name;
	})
}

/// Emit the per-impl manifest macro that fans out every per-action meta.
///
/// The manifest is named `__for_each_viewset_action_meta_<TypeNameSnake>`
/// so Phase 6 can call it from `__for_each_url_resolver`'s arm without
/// needing to see the ViewSet's basename at the `#[url_patterns]` site.
///
/// Refs Issue #4507.
fn emit_impl_action_manifest(
	type_snake: &str,
	basename: &str,
	fn_idents: &[syn::Ident],
) -> proc_macro2::TokenStream {
	let manifest_name = syn::Ident::new(
		&format!("__for_each_viewset_action_meta_{type_snake}"),
		Span::call_site(),
	);
	let meta_calls: Vec<proc_macro2::TokenStream> = fn_idents
		.iter()
		.map(|fn_ident| {
			let meta_name = syn::Ident::new(
				&format!("__url_resolver_meta_action_{basename}_{fn_ident}"),
				Span::call_site(),
			);
			quote! { #meta_name!($callback, $app); }
		})
		.collect();
	quote! {
		#[doc(hidden)]
		macro_rules! #manifest_name {
			($callback:ident, $app:ident) => {
				#(#meta_calls)*
			};
		}
		pub(crate) use #manifest_name;
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

		// Assert: manifest macro that fans out to both meta macros exists.
		assert!(
			out_s.contains("__for_each_viewset_meta_viewset"),
			"fn-form must emit per-fn manifest macro; got: {out_s}"
		);
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
		let (metas, fn_idents) = collect_actions(&item_impl, "snippet").unwrap();

		// Assert
		assert_eq!(metas.len(), 2);
		assert_eq!(fn_idents.len(), 2);
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
		let (metas, fn_idents) = collect_actions(&item_impl, "snippet").unwrap();

		// Assert
		assert_eq!(metas.len(), 1);
		assert_eq!(fn_idents.len(), 1);
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
}
