//! Desktop application framework for Reinhardt using wry/tao.
//!
//! This crate enables building cross-platform desktop applications from the same
//! `reinhardt-manouche` DSL used for web applications.
//!
//! ## Architecture
//!
//! - [`tao`] provides window management and event loop
//! - [`wry`] embeds a WebView for rendering
//! - Custom protocol (`reinhardt://`) serves bundled assets
//! - IPC bridge enables Rust ↔ JavaScript communication
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_desktop::{DesktopApp, WindowConfig};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let app = DesktopApp::builder()
//!         .title("My App")
//!         .size(800, 600)
//!         .build()?;
//!
//!     app.run()
//! }
//! ```

#![warn(missing_docs)]

mod app;
mod config;
mod error;
mod ipc;
mod protocol;
mod webview;
mod window;

pub use app::{DesktopApp, DesktopAppBuilder};
pub use config::WindowConfig;
pub use error::{DesktopError, Result};
pub use ipc::{IpcHandler, IpcMessage, IpcResponse};
pub use protocol::ProtocolHandler;
pub use webview::WebViewManager;
pub use window::WindowManager;
