//! Runtime support for reinhardt-mobile.
//!
//! Provides WebView runtime, event handling, IPC bridge, and reactive
//! system integration for mobile applications.

mod events;
mod ipc;
mod reactive;
mod webview;

pub use events::{EventDispatcher, MobileEvent};
pub use ipc::IpcBridge;
pub use reactive::ReactiveRuntime;
pub use webview::{MobileWebView, WebViewConfig};
