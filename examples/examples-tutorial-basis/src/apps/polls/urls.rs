//! URL configuration for the polls application.
//!
//! Aggregates server-side and client-side route definitions from sibling
//! submodules. Submodules are cfg-gated by target:
//!
//! - `server_urls` — `ServerRouter` mounted by `config/urls.rs` (native target)
//! - `client_router` — `ClientRouter` driven by the WASM entry point (wasm target)

#[cfg(native)]
pub mod server_urls;

#[cfg(wasm)]
pub mod client_router;

#[cfg(native)]
pub use server_urls::routes;

#[cfg(wasm)]
pub use client_router::{init_global_router, with_router};
