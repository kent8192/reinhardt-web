//! Procedural macros for reinhardt-pages-components
//!
//! This crate provides the `page!` and `form!` macros for declarative UI construction.

use proc_macro::TokenStream;
use quote::quote;

/// Placeholder for `page!` macro.
///
/// The implementation is not yet available; invoking this macro emits a
/// `compile_error!` so user code fails to compile cleanly rather than
/// triggering a procedural-macro panic.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_pages_components::*;
///
/// page! {
///     Container {
///         children: [Alert { message: "Hello" }],
///     }
/// }
/// ```
#[proc_macro]
pub fn page(_input: TokenStream) -> TokenStream {
	quote! {
		::core::compile_error!("page! macro is not yet implemented");
	}
	.into()
}

/// Placeholder for `form!` macro.
///
/// The implementation is not yet available; invoking this macro emits a
/// `compile_error!` so user code fails to compile cleanly rather than
/// triggering a procedural-macro panic.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_pages_components::*;
///
/// form! {
///     LoginForm {
///         action: "/login",
///         remember_me: true,
///     }
/// }
/// ```
#[proc_macro]
pub fn form(_input: TokenStream) -> TokenStream {
	quote! {
		::core::compile_error!("form! macro is not yet implemented");
	}
	.into()
}
