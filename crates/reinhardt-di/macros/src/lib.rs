//! Procedural macros for Reinhardt dependency injection
//!
//! This crate provides FastAPI-style dependency injection macros:
//! - `#[injectable]` - Mark an async function as a dependency provider
//! - `#[injectable_factory]` - Deprecated compatibility name for `#[injectable]`
//! - `#[injectable_key]` - Mark a struct as a dependency provider key

#![warn(missing_docs)]

use proc_macro::TokenStream;
use syn::{ItemFn, ItemStruct, parse_macro_input};

mod crate_paths;
mod injectable_factory;
mod injectable_key;
mod utils;

/// Register an injectable provider function.
///
/// Provider functions must be async and return `FactoryOutput<K, T>`, where
/// `K` is an `InjectableKey` and `T` is the value consumed through
/// `Depends<K, T>`.
///
/// On `wasm32-unknown-unknown` the generated provider becomes an inert
/// same-name async stub and skips DI registration. This lets downstream crates
/// keep DI provider definitions in modules that are also compiled for WASM,
/// while dependency resolution remains native-only.
#[proc_macro_attribute]
pub fn injectable(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	injectable_factory::injectable_factory_impl(args.into(), input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// Deprecated compatibility name for `#[injectable]`.
///
/// Use `#[injectable]` on provider functions instead.
#[deprecated(note = "use #[injectable] on provider functions instead")]
#[proc_macro_attribute]
pub fn injectable_factory(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	injectable_factory::injectable_factory_impl(args.into(), input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}

/// Mark a type as a dependency provider key.
///
/// The key type is emitted on every target, but the `InjectableKey` impl is
/// skipped on `wasm32-unknown-unknown` because keyed DI resolution is
/// native-only.
#[proc_macro_attribute]
pub fn injectable_key(args: TokenStream, input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemStruct);

	injectable_key::injectable_key_impl(args.into(), input)
		.unwrap_or_else(|e| e.to_compile_error())
		.into()
}
