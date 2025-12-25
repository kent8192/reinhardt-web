//! Procedural Macros for Reinhardt Pages
//!
//! This crate provides procedural macros for the reinhardt-pages WASM frontend framework.
//!
//! ## Available Macros
//!
//! - `page!` - Anonymous component DSL macro
//! - `#[server_fn]` - Server Functions (RPC) macro
//!
//! ## page! Macro Example
//!
//! ```ignore
//! use reinhardt_pages::page;
//!
//! // Define an anonymous component with closure-style props
//! let counter = page!(|initial: i32| {
//!     div {
//!         class: "counter",
//!         h1 { "Counter" }
//!         span { format!("Count: {}", initial) }
//!         button {
//!             @click: |_| { /* handler */ },
//!             "+"
//!         }
//!     }
//! });
//!
//! // Use like a function
//! let view = counter(42);
//! ```
//!
//! ## server_fn Example
//!
//! ```ignore
//! use reinhardt_pages_macros::server_fn;
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

mod crate_paths;
mod page;
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
/// - `use_inject = true` - Enable dependency injection
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

/// Page component macro
///
/// Creates an anonymous component with a closure-style DSL for defining views.
/// The component is returned as a callable function that takes props and returns a View.
///
/// ## Syntax
///
/// ```text
/// page!(|prop1: Type1, prop2: Type2| {
///     element {
///         attr: "value",
///         @event: |e| { handler(e) },
///         child_element { ... }
///         "text content"
///     }
/// })
/// ```
///
/// ## Elements
///
/// HTML elements are written as `tag { ... }`:
///
/// ```ignore
/// page!(|| {
///     div {
///         h1 { "Title" }
///         p { "Paragraph" }
///     }
/// })
/// ```
///
/// ## Attributes
///
/// Attributes use `key: value` syntax:
///
/// ```ignore
/// page!(|| {
///     div {
///         class: "container",
///         id: "main",
///         data_testid: "test",  // Converts to data-testid
///     }
/// })
/// ```
///
/// ## Events
///
/// Events use `@event: handler` syntax:
///
/// ```ignore
/// page!(|| {
///     button {
///         @click: |_| { console_log!("Clicked!") },
///         "Click me"
///     }
/// })
/// ```
///
/// ## Conditional Rendering
///
/// Use `if` and `if/else`:
///
/// ```ignore
/// page!(|show: bool| {
///     div {
///         if show {
///             span { "Visible" }
///         } else {
///             span { "Hidden" }
///         }
///     }
/// })
/// ```
///
/// ## List Rendering
///
/// Use `for` loops:
///
/// ```ignore
/// page!(|items: Vec<String>| {
///     ul {
///         for item in items {
///             li { item }
///         }
///     }
/// })
/// ```
#[proc_macro]
pub fn page(input: TokenStream) -> TokenStream {
	page::page_impl(input)
}

// Note: For dependency injection parameters, use the tool attribute #[reinhardt::inject]
// instead of a bare #[inject]. This is because proc_macro_attribute doesn't support
// helper attributes (unlike proc_macro_derive). Tool attributes provide namespace
// clarity and prevent "cannot find attribute in scope" compiler errors.
// The #[server_fn] macro detects and processes #[reinhardt::inject] during expansion.
