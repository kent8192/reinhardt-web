//! Receiver context for signal handlers
//!
//! This module provides the `ReceiverContext` struct that can be used
//! to pass additional context information to signal receivers.
//!
//! # Usage
//!
//! ```rust,no_run
//! use reinhardt_core::signals::{Signal, SignalName, SignalError, InjectableSignal, ReceiverContext};
//! use std::sync::Arc;
//!
//! #[derive(Clone, Debug)]
//! struct User;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), SignalError> {
//!     let signal = Signal::<User>::new(SignalName::custom("user_created"));
//!
//!     signal.connect_with_context(|instance, ctx| async move {
//!         println!("Received signal for {:?}", instance);
//!         Ok(())
//!     });
//!
//!     let user = User;
//!     signal.send(user).await?;
//!     Ok(())
//! }
//! ```

/// Context passed to signal receivers
///
/// This struct allows receivers to access additional context information
/// when processing signals.
#[derive(Clone, Default)]
pub struct ReceiverContext {
	_phantom: std::marker::PhantomData<()>,
}

impl ReceiverContext {
	/// Create a new empty receiver context
	pub fn new() -> Self {
		Self {
			_phantom: std::marker::PhantomData,
		}
	}

	/// Check if DI context is available (always false, DI is not supported)
	pub fn has_di_context(&self) -> bool {
		false
	}
}

impl std::fmt::Debug for ReceiverContext {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ReceiverContext")
			.field("has_di_context", &false)
			.finish()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_receiver_context_new() {
		let ctx = ReceiverContext::new();
		assert!(!ctx.has_di_context());
	}

	#[rstest]
	fn test_receiver_context_default() {
		let ctx = ReceiverContext::default();
		assert!(!ctx.has_di_context());
	}

	#[rstest]
	fn test_receiver_context_debug() {
		let ctx = ReceiverContext::new();
		let debug_str = format!("{:?}", ctx);
		assert!(debug_str.contains("ReceiverContext"));
		assert!(debug_str.contains("has_di_context"));
	}
}
