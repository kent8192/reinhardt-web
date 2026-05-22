//! Procedural macros for reinhardt-testkit.
//!
//! See `crates/reinhardt-testkit/src/fixtures/di_overrides.rs` for the
//! runtime types used by the macros in this crate.

mod with_di_overrides;

use proc_macro::TokenStream;

/// Sets up a test `InjectionContext` with one or more dependencies
/// overridden. See the crate README for usage.
#[proc_macro]
pub fn with_di_overrides(input: TokenStream) -> TokenStream {
	with_di_overrides::expand(input.into())
		.unwrap_or_else(|err| err.to_compile_error())
		.into()
}
