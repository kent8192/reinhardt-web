use thiserror::Error;

/// Errors that can occur in the streaming layer.
#[derive(Debug, Error)]
pub enum StreamingError {
	/// The client could not connect to a streaming backend.
	#[error("connection failed: {0}")]
	Connection(String),
	/// A payload could not be serialized or deserialized.
	#[error("serialization error: {0}")]
	Serialization(String),
	/// The requested topic is not known to the backend.
	#[error("topic not found: {0}")]
	TopicNotFound(String),
	/// The operation failed with a condition that may succeed on retry.
	#[error("retryable error: {0}")]
	Retryable(String),
	/// The operation failed with a non-retryable backend condition.
	#[error("fatal error: {0}")]
	Fatal(String),
	/// Backend-specific error that does not fit a narrower category.
	#[error("backend error: {0}")]
	Backend(String),
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::*;

	#[rstest]
	fn retryable_formats_message() {
		let err = StreamingError::Retryable("timeout".to_owned());
		assert_eq!(err.to_string(), "retryable error: timeout");
	}

	#[rstest]
	fn fatal_formats_message() {
		let err = StreamingError::Fatal("disk full".to_owned());
		assert_eq!(err.to_string(), "fatal error: disk full");
	}

	#[rstest]
	fn connection_formats_message() {
		let err = StreamingError::Connection("refused".to_owned());
		assert_eq!(err.to_string(), "connection failed: refused");
	}
}
