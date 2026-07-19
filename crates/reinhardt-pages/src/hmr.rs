//! Hot Module Replacement (HMR) support for reinhardt-pages.
//!
//! Provides file watching and WebSocket-based browser notification for live
//! development reloading. CSS changes are applied without a full page reload,
//! while Rust source and template changes trigger a full reload.
//!
//! ## Features
//!
//! - File watching with debounce to avoid redundant reloads
//! - Change type classification (CSS hot-swap vs full reload)
//! - WebSocket server for push notifications to connected browsers
//! - Auto-injected client script for development builds
//!
//! ## Usage
//!
//! ```ignore
//! use reinhardt_pages::hmr::{HmrConfig, HmrServer};
//!
//! let config = HmrConfig::builder()
//!     .watch_path("src/")
//!     .watch_path("templates/")
//!     .debounce_ms(300)
//!     .ws_port(35729)
//!     .build();
//!
//! let server = HmrServer::new(config);
//! server.start().await;
//! ```

#[cfg(native)]
mod change_kind;
#[cfg(native)]
mod client_script;
#[cfg(native)]
mod config;
#[cfg(native)]
mod message;
pub mod protocol;
pub use protocol::*;
#[cfg(wasm)]
pub mod bridge;
#[cfg(wasm)]
pub use bridge::HmrBridge;
#[cfg(wasm)]
pub mod diagnostics;
#[cfg(wasm)]
pub mod overlay;
#[cfg(wasm)]
pub mod patch_transaction;
#[cfg(native)]
mod server;
#[cfg(wasm)]
pub mod template_instance;
#[cfg(wasm)]
pub mod template_registry;
#[cfg(native)]
mod watcher;

#[cfg(native)]
pub use change_kind::ChangeKind;
#[cfg(native)]
pub use client_script::{HMR_CLIENT_SCRIPT, hmr_script_tag};
#[cfg(native)]
pub use config::{HmrConfig, HmrConfigBuilder};
#[cfg(native)]
pub use message::HmrMessage;
#[cfg(native)]
pub use server::HmrServer;
#[cfg(native)]
pub use watcher::FileWatcher;
