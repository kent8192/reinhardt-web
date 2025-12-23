//! Helper functions for dynamic crate path resolution using proc_macro_crate

use proc_macro2::TokenStream;
use quote::quote;

/// Resolves the path to the reinhardt_openapi crate dynamically.
///
/// This supports different crate naming scenarios (reinhardt-openapi, renamed crates, etc.)
pub(crate) fn get_reinhardt_openapi_crate() -> TokenStream {
	use proc_macro_crate::{FoundCrate, crate_name};

	// First, try to find reinhardt-openapi directly
	match crate_name("reinhardt-openapi") {
		Ok(FoundCrate::Itself) => quote!(crate),
		Ok(FoundCrate::Name(name)) => {
			let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
			quote!(::#ident)
		}
		Err(_) => {
			// If reinhardt-openapi is not found directly, try to find it via reinhardt crate
			match crate_name("reinhardt-web") {
				Ok(FoundCrate::Itself) => {
					// reinhardt-web is the current crate
					quote!(crate::rest::openapi)
				}
				Ok(FoundCrate::Name(name)) => {
					let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
					quote!(::#ident::rest::openapi)
				}
				Err(_) => {
					// Also try renamed "reinhardt" crate
					match crate_name("reinhardt") {
						Ok(FoundCrate::Itself) => {
							// reinhardt is the current crate
							quote!(crate::rest::openapi)
						}
						Ok(FoundCrate::Name(name)) => {
							let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
							quote!(::#ident::rest::openapi)
						}
						Err(_) => {
							// Fallback: assume reinhardt_openapi is available
							quote!(::reinhardt_openapi)
						}
					}
				}
			}
		}
	}
}
