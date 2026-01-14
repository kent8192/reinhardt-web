//! examples-twitter library
//!
//! This is a full-stack Twitter clone built with reinhardt-pages.
//! - Server-side: REST API with server functions
//! - Client-side: WASM frontend with reactive UI

// ============================================================================
// Server-only modules (non-WASM)
// ============================================================================
#[cfg(not(target_arch = "wasm32"))]
mod server_only {
	// Re-export internal crates for macro-generated code
	pub use reinhardt::core::async_trait;
	pub use reinhardt::reinhardt_apps;
	pub use reinhardt::reinhardt_core;
	pub use reinhardt::reinhardt_http;
	pub use reinhardt::reinhardt_migrations;
	pub use reinhardt::reinhardt_params;
}
#[cfg(not(target_arch = "wasm32"))]
pub use server_only::*;

// ============================================================================
// Applications (shared between WASM and server with conditional modules)
// ============================================================================
pub mod apps;

// ============================================================================
// Server-only modules
// ============================================================================
#[cfg(not(target_arch = "wasm32"))]
pub mod config;
#[cfg(not(target_arch = "wasm32"))]
pub mod migrations;

// ============================================================================
// Client-only modules (WASM)
// ============================================================================
#[cfg(target_arch = "wasm32")]
pub mod core;

// ============================================================================
// Re-exports for convenience
// ============================================================================
#[cfg(not(target_arch = "wasm32"))]
pub use config::settings::get_settings;

// Test utilities (available for testing on server)
#[cfg(not(target_arch = "wasm32"))]
pub mod test_utils;
