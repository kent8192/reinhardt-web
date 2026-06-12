//! Error types for provider integrations.

use thiserror::Error;

/// Result type for provider operations.
pub type Result<T> = std::result::Result<T, ProviderError>;

/// Error type for provider operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ProviderError {
	/// Provider configuration is invalid or incomplete.
	#[error("provider configuration error: {0}")]
	Config(String),

	/// The requested provider resource was not found.
	#[error("provider resource not found: {0}")]
	NotFound(String),

	/// The provider rejected the operation due to permissions.
	#[error("provider permission denied: {0}")]
	PermissionDenied(String),

	/// The provider returned an unsuccessful HTTP status.
	#[error("provider service error {status}: {message}")]
	Service {
		/// HTTP status code.
		status: u16,
		/// Provider response body or status text.
		message: String,
	},

	/// HTTP transport error.
	#[error("provider HTTP error: {0}")]
	Http(#[from] reqwest::Error),

	/// URL construction or parsing error.
	#[error("provider URL error: {0}")]
	Url(#[from] url::ParseError),

	/// Header construction or parsing error.
	#[error("provider header error: {0}")]
	Header(String),
}
