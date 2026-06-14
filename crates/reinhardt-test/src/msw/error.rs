//! Error type for MSW runtime startup and lifecycle failures.

use std::error::Error;
use std::fmt;

/// Error returned by fallible MSW lifecycle operations.
#[derive(Debug)]
pub enum MswError {
	/// The worker is already active.
	AlreadyStarted,
	/// The worker has not been started.
	NotStarted,
	/// Binding the native loopback listener failed.
	Bind(std::io::Error),
	/// Native MSW cannot pass unhandled requests through to a real upstream.
	NativePassthroughUnsupported,
	/// The native runtime task failed while shutting down.
	Shutdown(String),
}

impl fmt::Display for MswError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::AlreadyStarted => write!(f, "MockServiceWorker is already started"),
			Self::NotStarted => write!(f, "MockServiceWorker is not started"),
			Self::Bind(err) => write!(f, "failed to bind native MSW listener: {err}"),
			Self::NativePassthroughUnsupported => write!(
				f,
				"UnhandledPolicy::Passthrough is not supported on native MSW"
			),
			Self::Shutdown(message) => write!(f, "native MSW shutdown failed: {message}"),
		}
	}
}

impl Error for MswError {
	fn source(&self) -> Option<&(dyn Error + 'static)> {
		match self {
			Self::Bind(err) => Some(err),
			_ => None,
		}
	}
}

impl From<std::io::Error> for MswError {
	fn from(err: std::io::Error) -> Self {
		Self::Bind(err)
	}
}
