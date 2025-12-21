//! examples-twitter library
//!
//! This is a full-stack Twitter clone built with reinhardt-pages.
//! - Server-side: REST API with server functions
//! - Client-side: WASM frontend with reactive UI

// Conditional compilation for WASM vs Server

// ============================================================================
// Shared modules (available on both WASM and server)
// ============================================================================
pub mod shared {
	pub mod errors;
	pub mod types;
}

// ============================================================================
// Server-only modules (non-WASM)
// ============================================================================
// Re-export internal crates for macro-generated code
pub use reinhardt::core::async_trait;
pub use reinhardt::reinhardt_apps;
pub use reinhardt::reinhardt_core;
pub use reinhardt::reinhardt_http;
pub use reinhardt::reinhardt_migrations;
pub use reinhardt::reinhardt_params;

// Core modules
pub mod apps;
pub mod config;
pub mod migrations;

#[cfg(not(target_arch = "wasm32"))]
pub mod server {
	// New structure
	pub mod middleware;
	pub mod models;
	pub mod server_fn;

	// Re-export commonly used items
	pub use crate::config::settings::get_settings;
}

#[cfg(any(test, feature = "e2e-tests"))]
pub mod test_utils;

// ============================================================================
// Client-only modules (WASM)
// ============================================================================
#[cfg(target_arch = "wasm32")]
pub mod client {
	pub mod components;
	pub mod pages;
	pub mod router;
	pub mod state;

	// WASM entry point
	pub mod lib;
}

// ============================================================================
// Re-exports for convenience
// ============================================================================
#[cfg(not(target_arch = "wasm32"))]
pub use server::get_settings;
