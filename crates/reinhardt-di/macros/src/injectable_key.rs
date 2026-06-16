//! Implementation of the `#[injectable_key]` macro.

use crate::crate_paths::get_reinhardt_di_crate;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemStruct, Result};

pub(crate) fn injectable_key_impl(_args: TokenStream, input: ItemStruct) -> Result<TokenStream> {
	let ident = &input.ident;
	let generics = &input.generics;
	let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
	let di_crate = get_reinhardt_di_crate();

	Ok(quote! {
		#input

		impl #impl_generics #di_crate::InjectableKey for #ident #ty_generics #where_clause {}
	})
}
