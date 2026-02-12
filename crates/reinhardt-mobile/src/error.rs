//! Error types for reinhardt-mobile.

use thiserror::Error;

/// Errors that can occur in reinhardt-mobile operations.
#[derive(Debug, Error)]
pub enum MobileError {
	/// WebView initialization failed
	#[error("WebView initialization failed: {0}")]
	WebViewInit(String),

	/// Asset loading failed
	#[error("Failed to load asset: {path}")]
	AssetLoad {
		path: String,
		#[source]
		source: std::io::Error,
	},

	/// IPC communication error
	#[error("IPC error: {0}")]
	Ipc(String),

	/// Platform-specific error
	#[error("Platform error ({platform}): {message}")]
	Platform {
		platform: &'static str,
		message: String,
	},

	/// Configuration error
	#[error("Configuration error: {0}")]
	Config(String),

	/// Build error
	#[error("Build error: {0}")]
	Build(String),

	/// Serialization error
	#[error("Serialization error: {0}")]
	Serialization(#[from] serde_json::Error),

	/// IR processing error
	#[error("IR processing error: {0}")]
	IrProcessing(String),
}

/// Result type alias for mobile operations.
pub type MobileResult<T> = Result<T, MobileError>;
