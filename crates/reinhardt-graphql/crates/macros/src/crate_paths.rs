//! Helper functions for dynamic crate path resolution using proc_macro_crate

use proc_macro2::TokenStream;
use quote::quote;

/// Resolves the path to the reinhardt_di crate dynamically.
///
/// This supports different crate naming scenarios (reinhardt-di, renamed crates, etc.)
pub(crate) fn get_reinhardt_di_crate() -> TokenStream {
	use proc_macro_crate::{FoundCrate, crate_name};

	match crate_name("reinhardt-di") {
		Ok(FoundCrate::Itself) => quote!(::reinhardt_di),
		Ok(FoundCrate::Name(name)) => {
			let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
			quote!(::#ident)
		}
		Err(_) => quote!(::reinhardt_di), // Fallback
	}
}

/// Resolves the path to the reinhardt_graphql crate dynamically.
///
/// This supports different crate naming scenarios (reinhardt-graphql, renamed crates, etc.)
pub(crate) fn get_reinhardt_graphql_crate() -> TokenStream {
	use proc_macro_crate::{FoundCrate, crate_name};

	match crate_name("reinhardt-graphql") {
		Ok(FoundCrate::Itself) => quote!(::reinhardt_graphql),
		Ok(FoundCrate::Name(name)) => {
			let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
			quote!(::#ident)
		}
		Err(_) => quote!(::reinhardt_graphql), // Fallback
	}
}
