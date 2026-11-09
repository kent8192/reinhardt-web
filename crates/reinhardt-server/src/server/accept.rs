//! What a listener does when accepting a connection fails.
//!
//! Running out of file descriptors (`EMFILE`, `ENFILE`) is the usual cause,
//! and it ends once connections close, so by default a listener waits and
//! accepts again instead of ending the server.

use std::io;
use std::time::Duration;

/// What a listener does after a failed `accept`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AcceptErrorAction {
	/// Wait this long, then accept again. A shutdown ends the wait.
	///
	/// Connections already accepted keep being served while the listener waits.
	RetryAfter(Duration),
	/// Stop accepting, drain the accepted connections, and return the error.
	Stop,
}

/// Decides what a listener does when accepting a connection fails.
///
/// Install one with [`HttpServer::with_accept_error_handler`](super::HttpServer::with_accept_error_handler).
/// The listener calls it from its accept loop, so it must not block.
pub trait AcceptErrorHandler: Send + Sync {
	/// Called after each failed `accept`.
	fn on_error(&self, error: &io::Error) -> AcceptErrorAction;

	/// Called after each successful `accept`, so a handler can tell when
	/// failures have ended. Does nothing by default.
	fn on_accepted(&self) {}
}

/// The default handler: log a warning and accept again after a delay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryAcceptErrors {
	delay: Duration,
}

impl RetryAcceptErrors {
	/// The wait between tries that the default handler uses.
	pub const DEFAULT_DELAY: Duration = Duration::from_secs(1);

	/// Retry after `delay` instead of [`Self::DEFAULT_DELAY`].
	pub const fn after(delay: Duration) -> Self {
		Self { delay }
	}
}

impl Default for RetryAcceptErrors {
	fn default() -> Self {
		Self::after(Self::DEFAULT_DELAY)
	}
}

impl AcceptErrorHandler for RetryAcceptErrors {
	fn on_error(&self, error: &io::Error) -> AcceptErrorAction {
		tracing::warn!(
			%error,
			retry_after_ms = self.delay.as_millis() as u64,
			"accepting a connection failed; retrying"
		);
		AcceptErrorAction::RetryAfter(self.delay)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn default_handler_retries_after_one_second() {
		// Arrange
		let handler = RetryAcceptErrors::default();
		let error = io::Error::from_raw_os_error(24);

		// Act
		let action = handler.on_error(&error);

		// Assert
		assert_eq!(
			action,
			AcceptErrorAction::RetryAfter(Duration::from_secs(1))
		);
	}

	#[rstest]
	fn custom_delay_is_used() {
		// Arrange
		let handler = RetryAcceptErrors::after(Duration::from_millis(50));
		let error = io::Error::other("accept failed");

		// Act
		let action = handler.on_error(&error);

		// Assert
		assert_eq!(
			action,
			AcceptErrorAction::RetryAfter(Duration::from_millis(50))
		);
	}
}
