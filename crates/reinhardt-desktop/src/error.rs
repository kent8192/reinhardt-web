//! Error types for reinhardt-desktop.

use thiserror::Error;

/// Result type alias for reinhardt-desktop operations.
pub type Result<T> = std::result::Result<T, DesktopError>;

/// Errors that can occur in reinhardt-desktop.
#[derive(Debug, Error)]
pub enum DesktopError {
	/// Failed to create event loop.
	#[error("failed to create event loop: {0}")]
	EventLoopCreation(String),

	/// Failed to create window.
	#[error("failed to create window: {0}")]
	WindowCreation(String),

	/// Failed to create WebView.
	#[error("failed to create webview: {0}")]
	WebViewCreation(String),

	/// Failed to register custom protocol.
	#[error("failed to register protocol: {0}")]
	ProtocolRegistration(String),

	/// IPC communication error.
	#[error("IPC error: {0}")]
	Ipc(String),

	/// Asset not found.
	#[error("asset not found: {0}")]
	AssetNotFound(String),

	/// Serialization error.
	#[error("serialization error: {0}")]
	Serialization(#[from] serde_json::Error),

	/// I/O error.
	#[error("I/O error: {0}")]
	Io(#[from] std::io::Error),
}
