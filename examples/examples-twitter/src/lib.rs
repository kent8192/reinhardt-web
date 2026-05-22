//! examples-twitter library
//!
//! This is a full-stack Twitter clone built with reinhardt-pages.
//! - Server-side: REST API with server functions
//! - Client-side: WASM frontend with reactive UI
#[cfg(native)]
mod server_only {
	pub use reinhardt::core::async_trait;
	pub use reinhardt::reinhardt_apps;
	pub use reinhardt::reinhardt_core;
	pub use reinhardt::reinhardt_di::params;
	pub use reinhardt::reinhardt_http;
}
#[cfg(native)]
pub use server_only::*;
pub mod apps;
pub mod config;
#[cfg(wasm)]
pub mod core;
#[cfg(native)]
pub mod migrations;
#[cfg(native)]
pub use config::settings::get_settings;
#[cfg(native)]
pub mod test_utils;
