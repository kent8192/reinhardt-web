use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, ReturnType, Type, parse_macro_input};

use crate::crate_paths::get_reinhardt_pages_crate;

pub(crate) fn client_page_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);
	expand_client_page(input)
		.unwrap_or_else(|err| err.to_compile_error())
		.into()
}

fn expand_client_page(input: ItemFn) -> syn::Result<proc_macro2::TokenStream> {
	if input.sig.asyncness.is_some() {
		return Err(syn::Error::new_spanned(
			input.sig.asyncness,
			"#[client_page] functions must not be async",
		));
	}

	match &input.sig.output {
		ReturnType::Type(_, ty) if is_page_type(ty) => {}
		_ => {
			return Err(syn::Error::new_spanned(
				&input.sig,
				"#[client_page] functions must return Page",
			));
		}
	}

	let pages_crate = get_reinhardt_pages_crate();
	let attrs = &input.attrs;
	let vis = &input.vis;
	let sig = &input.sig;
	let block = &input.block;

	Ok(quote! {
		#[cfg(all(target_family = "wasm", target_os = "unknown"))]
		#(#attrs)*
		#vis #sig #block

		#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
		#(#attrs)*
		// Native stubs preserve the client page signature but intentionally
		// ignore arguments while building route tables.
		#[allow(unused_variables)]
		#vis #sig {
			#pages_crate::Page::empty()
		}
	})
}

fn is_page_type(ty: &Type) -> bool {
	let Type::Path(type_path) = ty else {
		return false;
	};
	type_path
		.path
		.segments
		.last()
		.is_some_and(|segment| segment.ident == "Page")
}
