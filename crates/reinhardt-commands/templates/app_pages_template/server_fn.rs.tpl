//! Server functions for the {{ app_name }} application.
//!
//! Server functions are callable from the WASM client. The `#[server_fn]`
//! attribute generates both native and WASM-side bindings, so **do not add
//! `#[cfg(server)]` to the `#[server_fn]` items themselves** — the macro
//! handles target dispatch internally.
//!
//! Server-only imports and helper functions, however, must be gated with
//! `#[cfg(server)]` because they reference types (database connection,
//! session middleware, models, ...) that are not compiled on the WASM
//! target.

use reinhardt::pages::server_fn::{ServerFnError, server_fn};

// Example server-only imports (uncomment when you add real handlers):
// #[cfg(server)]
// use {
//     crate::apps::{{ app_name }}::models::{{ camel_case_app_name }},
//     reinhardt::DatabaseConnection,
//     reinhardt::Model,
// };

// -----------------------------------------------------------------------------
// PLACEHOLDER: delete or replace before shipping.
//
// Registers a no-op `placeholder` server function via inventory so the
// freshly generated module compiles end-to-end. Remove this once you have
// real `#[server_fn]` items — otherwise the WASM client can still call
// `placeholder()` as a dead endpoint.
// -----------------------------------------------------------------------------
#[server_fn]
pub async fn placeholder() -> std::result::Result<(), ServerFnError> {
    Ok(())
}
