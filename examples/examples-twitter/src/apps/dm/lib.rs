//! dm application module
//!
//! Direct message models for examples-twitter

#[cfg(native)]
use reinhardt::app_config;

#[cfg(native)]
pub mod admin;
#[cfg(native)]
pub mod models;
pub mod shared;
pub mod urls;

#[cfg(wasm)]
pub mod client;

#[cfg(native)]
pub mod server;

#[cfg(test)]
pub mod tests;

// DM WebSocket routes are intentionally NOT registered via a `urls/ws_urls.rs`
// submodule (kent8192/reinhardt-web#3918). The DM app uses `DMHandler`, a
// `WebSocketConsumer` wired through middleware in `src/config/middleware.rs`.
// This shows an alternative WS integration path that bypasses the URL system.

#[cfg(native)]
#[app_config(name = "dm", label = "dm", verbose_name = "Direct Messages")]
pub struct DmConfig;
