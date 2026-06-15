//! WASM/server public API parity macro.

use std::collections::BTreeMap;

use proc_macro::TokenStream;
use quote::{ToTokens, quote};
use syn::spanned::Spanned;
use syn::{Attribute, Error, Item, ItemFn, ItemMod, Meta, Visibility, parse_macro_input};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TargetKind {
	Wasm,
	Server,
}

#[derive(Clone)]
struct Variant {
	function: ItemFn,
	attributes_fingerprint: String,
	visibility_fingerprint: String,
	signature_fingerprint: String,
}

#[derive(Default)]
struct Pair {
	wasm: Option<Variant>,
	server: Option<Variant>,
}

/// Main entry point for the `#[wasm_server_api]` macro.
pub(crate) fn wasm_server_api_impl(args: TokenStream, input: TokenStream) -> TokenStream {
	if !args.is_empty() {
		return Error::new(
			proc_macro2::TokenStream::from(args).span(),
			"`#[wasm_server_api]` does not accept arguments",
		)
		.to_compile_error()
		.into();
	}

	let module = parse_macro_input!(input as ItemMod);
	match expand_wasm_server_api(module) {
		Ok(tokens) => tokens.into(),
		Err(error) => error.to_compile_error().into(),
	}
}

fn expand_wasm_server_api(mut module: ItemMod) -> syn::Result<proc_macro2::TokenStream> {
	let Some((_brace, items)) = module.content.take() else {
		return Err(Error::new(
			module.ident.span(),
			"`#[wasm_server_api]` requires an inline module body",
		));
	};

	let mut pairs = BTreeMap::<String, Pair>::new();
	let mut expanded_items = Vec::with_capacity(items.len());

	for item in items {
		match split_target_item(item)? {
			TargetItem::Plain(item) => expanded_items.push(item),
			TargetItem::Function {
				target,
				mut function,
			} => {
				let key = function.sig.ident.to_string();
				let variant = build_variant(function.clone());
				let pair = pairs.entry(key.clone()).or_default();
				match target {
					TargetKind::Wasm => {
						if pair.wasm.is_some() {
							return Err(Error::new(
								function.sig.ident.span(),
								format!("duplicate `#[wasm]` variant for `{key}`"),
							));
						}
						function.attrs.insert(
							0,
							syn::parse_quote!(#[cfg(all(target_family = "wasm", target_os = "unknown"))]),
						);
						expanded_items.push(Item::Fn(function));
						pair.wasm = Some(variant);
					}
					TargetKind::Server => {
						if pair.server.is_some() {
							return Err(Error::new(
								function.sig.ident.span(),
								format!("duplicate `#[server]` variant for `{key}`"),
							));
						}
						function.attrs.insert(
							0,
							syn::parse_quote!(#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]),
						);
						expanded_items.push(Item::Fn(function));
						pair.server = Some(variant);
					}
				}
			}
		}
	}

	if pairs.is_empty() {
		return Err(Error::new(
			module.ident.span(),
			"`#[wasm_server_api]` must declare at least one `#[wasm]` / `#[server]` function pair",
		));
	}

	validate_pairs(&pairs)?;

	let attrs = &module.attrs;
	let vis = &module.vis;
	let ident = &module.ident;
	let unsafety = &module.unsafety;
	let mod_token = &module.mod_token;

	Ok(quote! {
		#(#attrs)*
		#vis #unsafety #mod_token #ident {
			#(#expanded_items)*
		}
	})
}

enum TargetItem {
	Plain(Item),
	Function {
		target: TargetKind,
		function: ItemFn,
	},
}

fn split_target_item(item: Item) -> syn::Result<TargetItem> {
	match item {
		Item::Fn(mut function) => {
			let (target, retained_attrs) = extract_target_marker(&function.attrs)?;
			function.attrs = retained_attrs;
			if let Some(target) = target {
				if !matches!(function.vis, Visibility::Public(_)) {
					return Err(Error::new(
						function.sig.ident.span(),
						"`#[wasm_server_api]` target variants must be public functions",
					));
				}
				Ok(TargetItem::Function { target, function })
			} else {
				Ok(TargetItem::Plain(Item::Fn(function)))
			}
		}
		other => {
			let (target, _) = extract_target_marker(other.attrs())?;
			if target.is_some() {
				return Err(Error::new(
					other.span(),
					"`#[wasm]` and `#[server]` markers may only annotate functions",
				));
			}
			Ok(TargetItem::Plain(other))
		}
	}
}

trait ItemAttributes {
	fn attrs(&self) -> &[Attribute];
}

impl ItemAttributes for Item {
	fn attrs(&self) -> &[Attribute] {
		match self {
			Item::Const(item) => &item.attrs,
			Item::Enum(item) => &item.attrs,
			Item::ExternCrate(item) => &item.attrs,
			Item::Fn(item) => &item.attrs,
			Item::ForeignMod(item) => &item.attrs,
			Item::Impl(item) => &item.attrs,
			Item::Macro(item) => &item.attrs,
			Item::Mod(item) => &item.attrs,
			Item::Static(item) => &item.attrs,
			Item::Struct(item) => &item.attrs,
			Item::Trait(item) => &item.attrs,
			Item::TraitAlias(item) => &item.attrs,
			Item::Type(item) => &item.attrs,
			Item::Union(item) => &item.attrs,
			Item::Use(item) => &item.attrs,
			Item::Verbatim(_) => &[],
			_ => &[],
		}
	}
}

fn extract_target_marker(attrs: &[Attribute]) -> syn::Result<(Option<TargetKind>, Vec<Attribute>)> {
	let mut target = None;
	let mut retained = Vec::with_capacity(attrs.len());

	for attr in attrs {
		let kind = if attr.path().is_ident("wasm") {
			Some(TargetKind::Wasm)
		} else if attr.path().is_ident("server") {
			Some(TargetKind::Server)
		} else {
			None
		};

		let Some(kind) = kind else {
			retained.push(attr.clone());
			continue;
		};

		if !matches!(attr.meta, Meta::Path(_)) {
			return Err(Error::new(
				attr.span(),
				"`#[wasm]` and `#[server]` markers do not accept arguments",
			));
		}

		if target.replace(kind).is_some() {
			return Err(Error::new(
				attr.span(),
				"only one target marker is allowed per function",
			));
		}
	}

	Ok((target, retained))
}

fn build_variant(function: ItemFn) -> Variant {
	Variant {
		attributes_fingerprint: fingerprint_attrs(&function.attrs),
		visibility_fingerprint: fingerprint_visibility(&function.vis),
		signature_fingerprint: function.sig.to_token_stream().to_string(),
		function,
	}
}

fn validate_pairs(pairs: &BTreeMap<String, Pair>) -> syn::Result<()> {
	for (name, pair) in pairs {
		let Some(wasm) = &pair.wasm else {
			let server = pair.server.as_ref().expect("server variant exists");
			return Err(Error::new(
				server.function.sig.ident.span(),
				format!("missing `#[wasm]` variant for `{name}`"),
			));
		};

		let Some(server) = &pair.server else {
			return Err(Error::new(
				wasm.function.sig.ident.span(),
				format!("missing `#[server]` variant for `{name}`"),
			));
		};

		validate_pair(name, wasm, server)?;
	}

	Ok(())
}

fn validate_pair(name: &str, wasm: &Variant, server: &Variant) -> syn::Result<()> {
	if wasm.visibility_fingerprint != server.visibility_fingerprint {
		return Err(Error::new(
			server.function.vis.span(),
			format!(
				"`#[wasm]` and `#[server]` variants for `{name}` must have matching visibility"
			),
		));
	}

	if wasm.attributes_fingerprint != server.attributes_fingerprint {
		return Err(Error::new(
			server.function.sig.ident.span(),
			format!(
				"`#[wasm]` and `#[server]` variants for `{name}` must have matching non-target attributes"
			),
		));
	}

	if wasm.signature_fingerprint != server.signature_fingerprint {
		return Err(Error::new(
			server.function.sig.ident.span(),
			format!(
				"`#[wasm]` and `#[server]` variants for `{name}` must have matching signatures"
			),
		));
	}

	Ok(())
}

fn fingerprint_attrs(attrs: &[Attribute]) -> String {
	attrs
		.iter()
		.map(|attr| attr.to_token_stream().to_string())
		.collect::<Vec<_>>()
		.join("\n")
}

fn fingerprint_visibility(vis: &Visibility) -> String {
	vis.to_token_stream().to_string()
}
