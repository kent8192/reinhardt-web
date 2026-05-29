//! Canonical Todo example for `reinhardt-pages`.
//!
//! The crate keeps the domain model, server functions, and pages UI small so
//! new Reinhardt users can see the full client/server loop in one place.

#[cfg(native)]
mod server_only {
	pub use reinhardt::reinhardt_apps;
	pub use reinhardt::reinhardt_core;
	pub use reinhardt::reinhardt_http;
}
#[cfg(native)]
pub use server_only::*;

#[cfg(wasm)]
pub mod client;
#[cfg(native)]
pub mod config;
pub mod server_fn;
pub mod todo;
pub mod ui;

#[cfg(native)]
pub use config::settings::get_settings;
