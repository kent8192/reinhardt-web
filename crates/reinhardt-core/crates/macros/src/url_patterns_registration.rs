//! URL patterns registration macro implementation
//!
//! This module implements the `register_url_patterns!()` macro that allows
//! projects to register their URL pattern functions for automatic discovery
//! by the framework.
//!
//! # Macro Syntax
//!
//! The macro accepts two forms:
//!
//! 1. **Standard projects (no admin)**:
//!    ```rust,ignore
//!    register_url_patterns!();
//!    ```
//!
//! 2. **Admin-enabled projects**:
//!    ```rust,ignore
//!    register_url_patterns!(admin);
//!    ```
//!
//! # Generated Code
//!
//! The macro generates an `inventory::submit!` call that registers function
//! pointers with the framework's URL patterns registry.
//!
//! ## Standard Project
//!
//! ```rust,ignore
//! // Input:
//! register_url_patterns!();
//!
//! // Generated output:
//! inventory::submit! {
//!     ::reinhardt_urls::routers::UrlPatternsRegistration {
//!         get_router: url_patterns,
//!         get_admin_router: None,
//!     }
//! }
//! ```
//!
//! ## Admin-Enabled Project
//!
//! ```rust,ignore
//! // Input:
//! register_url_patterns!(admin);
//!
//! // Generated output:
//! inventory::submit! {
//!     ::reinhardt_urls::routers::UrlPatternsRegistration {
//!         get_router: url_patterns,
//!         get_admin_router: Some(url_patterns_with_admin),
//!     }
//! }
//! ```

use crate::crate_paths::get_reinhardt_crate;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, parse_macro_input};

/// Input parser for register_url_patterns macro
///
/// Accepts either:
/// - Empty input (no admin)
/// - `admin` identifier (with admin)
struct RegisterInput {
	/// Whether the project has admin functionality
	has_admin: bool,
}

impl Parse for RegisterInput {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		// Empty input means no admin
		if input.is_empty() {
			return Ok(RegisterInput { has_admin: false });
		}

		// Parse the identifier
		let ident: Ident = input.parse()?;

		// Only accept "admin" as the identifier
		if ident == "admin" {
			Ok(RegisterInput { has_admin: true })
		} else {
			Err(syn::Error::new_spanned(
				ident,
				"Expected 'admin' or no arguments. Usage: register_url_patterns!() or register_url_patterns!(admin)",
			))
		}
	}
}

/// Implementation of the register_url_patterns macro
///
/// This function generates an `inventory::submit!` call with appropriate
/// function pointers based on whether admin functionality is enabled.
///
/// # Parameters
///
/// * `input` - Token stream from the macro invocation
///
/// # Returns
///
/// Generated code as a `TokenStream`
pub(crate) fn register_url_patterns_impl(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as RegisterInput);
	let reinhardt = get_reinhardt_crate();

	let registration = if input.has_admin {
		// Admin-enabled: register both url_patterns and url_patterns_with_admin
		quote! {
			#reinhardt::inventory::submit! {
				#reinhardt::UrlPatternsRegistration {
					get_router: url_patterns,
					get_admin_router: Some(url_patterns_with_admin),
				}
			}
		}
	} else {
		// Standard: register only url_patterns
		quote! {
			#reinhardt::inventory::submit! {
				#reinhardt::UrlPatternsRegistration {
					get_router: url_patterns,
					get_admin_router: None,
				}
			}
		}
	};

	registration.into()
}
