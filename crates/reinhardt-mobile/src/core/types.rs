//! Core types for mobile runtime.

use serde::{Deserialize, Serialize};

/// IPC request from JavaScript to Rust.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcRequest {
	/// Command name
	pub command: String,

	/// Request payload as JSON
	pub payload: serde_json::Value,

	/// Request ID for response matching
	pub request_id: Option<String>,
}

/// IPC response from Rust to JavaScript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
	/// Request ID for response matching
	pub request_id: Option<String>,

	/// Success flag
	pub success: bool,

	/// Response data
	pub data: Option<serde_json::Value>,

	/// Error message if failed
	pub error: Option<String>,
}

/// Mobile application state.
#[derive(Debug, Clone, Default)]
pub struct AppState {
	/// Whether the app is initialized
	pub initialized: bool,

	/// Current route/page
	pub current_route: Option<String>,

	/// Custom state data
	pub custom_data: std::collections::HashMap<String, serde_json::Value>,
}

/// Event types for mobile events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MobileEventType {
	/// App lifecycle: created
	AppCreated,
	/// App lifecycle: resumed
	AppResumed,
	/// App lifecycle: paused
	AppPaused,
	/// App lifecycle: destroyed
	AppDestroyed,
	/// WebView ready
	WebViewReady,
	/// Navigation event
	Navigation,
	/// IPC message received
	IpcMessage,
}
