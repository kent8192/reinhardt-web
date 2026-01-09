//! Server functions module
//!
//! This module contains server functions accessible from both WASM (client stubs)
//! and server (handlers). The `#[server_fn]` macro generates target-specific code.

pub mod auth;
#[cfg(not(target_arch = "wasm32"))]
pub mod dm;
pub mod profile;
pub mod relationship;
pub mod tweet;

// Re-export ServerFnError for macro-generated code
// The #[server_fn] macro generates code that references `crate::server_fn::ServerFnError`
#[cfg(target_arch = "wasm32")]
pub use reinhardt::pages::server_fn::ServerFnError;

#[cfg(not(target_arch = "wasm32"))]
pub use reinhardt::pages::server_fn::ServerFnError;
