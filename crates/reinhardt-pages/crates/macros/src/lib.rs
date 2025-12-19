//! Procedural Macros for Reinhardt Page
//!
//! This crate provides procedural macros for the reinhardt-pages WASM frontend framework.
//!
//! ## Available Macros
//!
//! - `#[server_fn]` - Server Functions (RPC) macro
//!
//! ## Example
//!
//! ```ignore
//! use reinhardt_pagess_macros::server_fn;
//!
//! #[server_fn]
//! async fn get_user(id: u32) -> Result<User, ServerFnError> {
//!     // Server-side code (automatically removed on WASM build)
//!     let user = User::find_by_id(id).await?;
//!     Ok(user)
//! }
//!
//! // On client (WASM), this expands to an HTTP request
//! // On server, this expands to a route handler
//! ```

use proc_macro::TokenStream;

mod server_fn;

/// Server Function macro
///
/// This macro generates client-side stub (WASM) and server-side handler (non-WASM)
/// for seamless RPC communication between frontend and backend.
///
/// ## Basic Usage
///
/// ```ignore
/// #[server_fn]
/// async fn get_user(id: u32) -> Result<User, ServerFnError> {
///     // Server-side implementation
///     let user = User::find_by_id(id).await?;
///     Ok(user)
/// }
/// ```
///
/// ## Options
///
/// - `use_inject = true` - Enable dependency injection (Phase 5.5, Week 4)
/// - `endpoint = "/custom/path"` - Custom endpoint path
/// - `codec = "json"` - Serialization codec (json, url, msgpack)
///
/// ```ignore
/// #[server_fn(endpoint = "/api/users/get")]
/// async fn get_user(id: u32) -> Result<User, ServerFnError> {
///     // ...
/// }
/// ```
#[proc_macro_attribute]
pub fn server_fn(args: TokenStream, input: TokenStream) -> TokenStream {
	server_fn::server_fn_impl(args, input)
}

/// Dependency injection marker attribute (Week 4 Day 1-2)
///
/// This attribute marks parameters for dependency injection.
/// It's processed by the `#[server_fn(use_inject = true)]` macro.
///
/// ## Usage
///
/// ```ignore
/// #[server_fn(use_inject = true)]
/// async fn handler(
///     id: u32,              // Regular parameter
///     #[inject] db: Database, // DI parameter
/// ) -> Result<User, Error>
/// ```
///
/// ## Implementation
///
/// This is a no-op attribute macro that allows the Rust compiler
/// to recognize `#[inject]` on function parameters. The actual
/// processing is done by the `#[server_fn]` macro.
#[proc_macro_attribute]
pub fn inject(_args: TokenStream, input: TokenStream) -> TokenStream {
	// No-op: just pass through the input unchanged
	// The actual processing is done by #[server_fn] macro
	input
}
