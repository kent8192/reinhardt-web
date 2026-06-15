//! Reinhardt Basis Tutorial Example - Polling Application with Pages
//!
//! This example demonstrates the concepts covered in the Reinhardt basis tutorial:
//! - Project setup and configuration
//! - Database models and ORM
//! - Views with reinhardt-pages (WASM + SSR)
//! - Forms and generic views
//! - Testing
//! - Static files
//! - Admin panel customization

// Server-only re-exports for macro-generated code
#[cfg(server)]
mod server_only {
	pub use reinhardt::core::async_trait;
	pub use reinhardt::reinhardt_apps;
	pub use reinhardt::reinhardt_core;
	pub use reinhardt::reinhardt_di::params;
	pub use reinhardt::reinhardt_http;
}
#[cfg(server)]
pub use server_only::*;

// Applications (declared on both targets; submodules cfg-gate themselves)
pub mod apps;

// Configuration (urls unconditional, rest server-only)
pub mod config;

// Cross-target page stubs live under `client::pages`; executable browser
// modules inside `client` remain cfg-gated.
pub mod client;

// Cross-target wrappers for client-side data loaders.
pub mod client_api;

// Native runtime wiring plus WASM no-op route-table shims.
pub mod native_runtime;

// Shared modules (both WASM and server)
//
// Server functions are now scoped under each app — they live alongside
// the app's models / views / urls in `apps::<app>::server_fn`, which
// keeps related code together and removes the top-level `server_fn`
// module that previously had to mirror the app list.
pub mod shared;

// Re-exports
#[cfg(server)]
pub use config::settings::get_settings;
