//! Transaction Support - Signals tied to database transaction lifecycle
//!
//! This module provides signal support for database transaction lifecycle events,
//! allowing receivers to be notified of transaction begin, commit, and rollback events.
//!
//! # Examples
//!
//! ```rust,no_run
//! use reinhardt_core::signals::transaction::{on_commit, on_rollback, TransactionSignals};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to transaction commit
//! on_commit().connect(|ctx| async move {
//!     println!("Transaction committed: {:?}", ctx);
//!     Ok(())
//! });
//!
//! // Use TransactionSignals for manual control
//! let tx_signals = TransactionSignals::new("tx_1");
//! tx_signals.send_begin().await?;
//! tx_signals.send_commit().await?;
//! # Ok(())
//! # }
//! ```

use super::core::SignalName;
use super::error::SignalError;
use super::registry::get_signal;
use super::signal::Signal;
use serde::{Deserialize, Serialize};

/// Transaction context passed to signal receivers
///
/// # Examples
///
/// ```
/// use reinhardt_core::signals::transaction::TransactionContext;
///
/// let ctx = TransactionContext::new("tx_123");
/// assert_eq!(ctx.transaction_id, "tx_123");
/// assert_eq!(ctx.savepoint_depth, 0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionContext {
	/// Unique transaction identifier
	pub transaction_id: String,
	/// Savepoint depth for nested transactions
	pub savepoint_depth: usize,
	/// Optional savepoint name
	pub savepoint_name: Option<String>,
	/// Whether this is a nested transaction
	pub is_nested: bool,
}

impl TransactionContext {
	/// Create a new transaction context
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::signals::transaction::TransactionContext;
	///
	/// let ctx = TransactionContext::new("my_transaction");
	/// assert_eq!(ctx.transaction_id, "my_transaction");
	/// assert!(!ctx.is_nested);
	/// ```
	pub fn new(transaction_id: impl Into<String>) -> Self {
		Self {
			transaction_id: transaction_id.into(),
			savepoint_depth: 0,
			savepoint_name: None,
			is_nested: false,
		}
	}

	/// Create a nested transaction context
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::signals::transaction::TransactionContext;
	///
	/// let ctx = TransactionContext::nested("tx_1", 1, "sp_1");
	/// assert!(ctx.is_nested);
	/// assert_eq!(ctx.savepoint_depth, 1);
	/// assert_eq!(ctx.savepoint_name, Some("sp_1".to_string()));
	/// ```
	pub fn nested(
		transaction_id: impl Into<String>,
		depth: usize,
		savepoint_name: impl Into<String>,
	) -> Self {
		Self {
			transaction_id: transaction_id.into(),
			savepoint_depth: depth,
			savepoint_name: Some(savepoint_name.into()),
			is_nested: true,
		}
	}

	/// Enter a savepoint (increase nesting depth)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::signals::transaction::TransactionContext;
	///
	/// let mut ctx = TransactionContext::new("tx_1");
	/// ctx.enter_savepoint("checkpoint_1");
	/// assert_eq!(ctx.savepoint_depth, 1);
	/// assert_eq!(ctx.savepoint_name, Some("checkpoint_1".to_string()));
	/// ```
	pub fn enter_savepoint(&mut self, name: impl Into<String>) {
		self.savepoint_depth += 1;
		self.savepoint_name = Some(name.into());
		self.is_nested = true;
	}

	/// Exit a savepoint (decrease nesting depth)
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::signals::transaction::TransactionContext;
	///
	/// let mut ctx = TransactionContext::nested("tx_1", 2, "sp_2");
	/// ctx.exit_savepoint();
	/// assert_eq!(ctx.savepoint_depth, 1);
	/// ```
	pub fn exit_savepoint(&mut self) {
		if self.savepoint_depth > 0 {
			self.savepoint_depth -= 1;
		}
		if self.savepoint_depth == 0 {
			self.savepoint_name = None;
			self.is_nested = false;
		}
	}
}

/// Transaction signal manager
///
/// Manages transaction lifecycle signals for a specific transaction
///
/// # Examples
///
/// ```
/// use reinhardt_core::signals::transaction::TransactionSignals;
///
/// let signals = TransactionSignals::new("transaction_1");
/// ```
pub struct TransactionSignals {
	context: TransactionContext,
}

impl TransactionSignals {
	/// Create a new transaction signal manager
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::signals::transaction::TransactionSignals;
	///
	/// let signals = TransactionSignals::new("tx_001");
	/// ```
	pub fn new(transaction_id: impl Into<String>) -> Self {
		Self {
			context: TransactionContext::new(transaction_id),
		}
	}

	/// Create a nested transaction signal manager
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::signals::transaction::TransactionSignals;
	///
	/// let signals = TransactionSignals::nested("tx_001", 1, "savepoint_1");
	/// ```
	pub fn nested(
		transaction_id: impl Into<String>,
		depth: usize,
		savepoint_name: impl Into<String>,
	) -> Self {
		Self {
			context: TransactionContext::nested(transaction_id, depth, savepoint_name),
		}
	}

	/// Get the transaction context
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_core::signals::transaction::TransactionSignals;
	///
	/// let signals = TransactionSignals::new("tx_001");
	/// let ctx = signals.context();
	/// assert_eq!(ctx.transaction_id, "tx_001");
	/// ```
	pub fn context(&self) -> &TransactionContext {
		&self.context
	}

	/// Send transaction begin signal
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_core::signals::transaction::TransactionSignals;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let signals = TransactionSignals::new("tx_001");
	/// signals.send_begin().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn send_begin(&self) -> Result<(), SignalError> {
		on_begin().send(self.context.clone()).await
	}

	/// Send transaction commit signal
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_core::signals::transaction::TransactionSignals;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let signals = TransactionSignals::new("tx_001");
	/// signals.send_commit().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn send_commit(&self) -> Result<(), SignalError> {
		on_commit().send(self.context.clone()).await
	}

	/// Send transaction rollback signal
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_core::signals::transaction::TransactionSignals;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let signals = TransactionSignals::new("tx_001");
	/// signals.send_rollback().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn send_rollback(&self) -> Result<(), SignalError> {
		on_rollback().send(self.context.clone()).await
	}

	/// Enter a savepoint and send signal
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_core::signals::transaction::TransactionSignals;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut signals = TransactionSignals::new("tx_001");
	/// signals.enter_savepoint("checkpoint_1").await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn enter_savepoint(&mut self, name: impl Into<String>) -> Result<(), SignalError> {
		self.context.enter_savepoint(name);
		on_savepoint().send(self.context.clone()).await
	}

	/// Exit a savepoint and send signal
	///
	/// # Examples
	///
	/// ```rust,no_run
	/// use reinhardt_core::signals::transaction::TransactionSignals;
	///
	/// # #[tokio::main]
	/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
	/// let mut signals = TransactionSignals::nested("tx_001", 1, "sp_1");
	/// signals.exit_savepoint().await?;
	/// # Ok(())
	/// # }
	/// ```
	pub async fn exit_savepoint(&mut self) -> Result<(), SignalError> {
		self.context.exit_savepoint();
		on_savepoint_release().send(self.context.clone()).await
	}
}

/// Get the transaction begin signal
///
/// # Examples
///
/// ```
/// use reinhardt_core::signals::transaction::on_begin;
///
/// let signal = on_begin();
/// ```
pub fn on_begin() -> Signal<TransactionContext> {
	get_signal::<TransactionContext>(SignalName::custom("transaction_begin"))
}

/// Get the transaction commit signal
///
/// # Examples
///
/// ```
/// use reinhardt_core::signals::transaction::on_commit;
///
/// let signal = on_commit();
/// ```
pub fn on_commit() -> Signal<TransactionContext> {
	get_signal::<TransactionContext>(SignalName::custom("transaction_commit"))
}

/// Get the transaction rollback signal
///
/// # Examples
///
/// ```
/// use reinhardt_core::signals::transaction::on_rollback;
///
/// let signal = on_rollback();
/// ```
pub fn on_rollback() -> Signal<TransactionContext> {
	get_signal::<TransactionContext>(SignalName::custom("transaction_rollback"))
}

/// Get the savepoint create signal
///
/// # Examples
///
/// ```
/// use reinhardt_core::signals::transaction::on_savepoint;
///
/// let signal = on_savepoint();
/// ```
pub fn on_savepoint() -> Signal<TransactionContext> {
	get_signal::<TransactionContext>(SignalName::custom("transaction_savepoint"))
}

/// Get the savepoint release signal
///
/// # Examples
///
/// ```
/// use reinhardt_core::signals::transaction::on_savepoint_release;
///
/// let signal = on_savepoint_release();
/// ```
pub fn on_savepoint_release() -> Signal<TransactionContext> {
	get_signal::<TransactionContext>(SignalName::custom("transaction_savepoint_release"))
}

#[cfg(test)]
mod tests {
	use super::*;
	use parking_lot::Mutex;
	use rstest::rstest;
	use std::sync::Arc;
	use std::sync::atomic::{AtomicUsize, Ordering};

	#[rstest]
	fn test_transaction_context_creation() {
		let ctx = TransactionContext::new("tx_1");
		assert_eq!(ctx.transaction_id, "tx_1");
		assert_eq!(ctx.savepoint_depth, 0);
		assert_eq!(ctx.savepoint_name, None);
		assert!(!ctx.is_nested);
	}

	#[rstest]
	fn test_transaction_context_nested() {
		let ctx = TransactionContext::nested("tx_1", 2, "sp_2");
		assert_eq!(ctx.transaction_id, "tx_1");
		assert_eq!(ctx.savepoint_depth, 2);
		assert_eq!(ctx.savepoint_name, Some("sp_2".to_string()));
		assert!(ctx.is_nested);
	}

	#[rstest]
	fn test_transaction_context_enter_savepoint() {
		let mut ctx = TransactionContext::new("tx_1");
		ctx.enter_savepoint("checkpoint_1");
		assert_eq!(ctx.savepoint_depth, 1);
		assert_eq!(ctx.savepoint_name, Some("checkpoint_1".to_string()));
		assert!(ctx.is_nested);
	}

	#[rstest]
	fn test_transaction_context_exit_savepoint() {
		let mut ctx = TransactionContext::nested("tx_1", 2, "sp_2");
		ctx.exit_savepoint();
		assert_eq!(ctx.savepoint_depth, 1);

		ctx.exit_savepoint();
		assert_eq!(ctx.savepoint_depth, 0);
		assert_eq!(ctx.savepoint_name, None);
		assert!(!ctx.is_nested);
	}

	#[rstest]
	#[tokio::test]
	async fn test_on_commit_signal() {
		let counter = Arc::new(AtomicUsize::new(0));
		let counter_clone = Arc::clone(&counter);

		on_commit().connect(move |_ctx| {
			let counter = Arc::clone(&counter_clone);
			async move {
				counter.fetch_add(1, Ordering::SeqCst);
				Ok(())
			}
		});

		let ctx = TransactionContext::new("tx_1");
		on_commit().send(ctx).await.unwrap();

		assert_eq!(counter.load(Ordering::SeqCst), 1);
	}

	#[rstest]
	#[tokio::test]
	async fn test_on_rollback_signal() {
		let counter = Arc::new(AtomicUsize::new(0));
		let counter_clone = Arc::clone(&counter);

		on_rollback().connect(move |_ctx| {
			let counter = Arc::clone(&counter_clone);
			async move {
				counter.fetch_add(1, Ordering::SeqCst);
				Ok(())
			}
		});

		let ctx = TransactionContext::new("tx_1");
		on_rollback().send(ctx).await.unwrap();

		assert_eq!(counter.load(Ordering::SeqCst), 1);
	}

	#[rstest]
	#[tokio::test]
	#[serial_test::serial]
	async fn test_transaction_signals_flow() {
		// Clean up before test
		on_begin().disconnect_all();
		on_commit().disconnect_all();

		let events = Arc::new(Mutex::new(Vec::new()));

		let e1 = events.clone();
		on_begin().connect(move |ctx| {
			let e = e1.clone();
			async move {
				e.lock().push(format!("begin:{}", ctx.transaction_id));
				Ok(())
			}
		});

		let e2 = events.clone();
		on_commit().connect(move |ctx| {
			let e = e2.clone();
			async move {
				e.lock().push(format!("commit:{}", ctx.transaction_id));
				Ok(())
			}
		});

		let signals = TransactionSignals::new("tx_test");
		signals.send_begin().await.unwrap();
		signals.send_commit().await.unwrap();

		let event_log = events.lock();
		assert_eq!(event_log.len(), 2);
		assert_eq!(event_log[0], "begin:tx_test");
		assert_eq!(event_log[1], "commit:tx_test");

		// Clean up after test
		on_begin().disconnect_all();
		on_commit().disconnect_all();
	}

	#[rstest]
	#[tokio::test]
	#[serial_test::serial]
	async fn test_savepoint_signals() {
		// Clean up before test
		on_savepoint().disconnect_all();
		on_savepoint_release().disconnect_all();

		let counter = Arc::new(AtomicUsize::new(0));

		let c1 = counter.clone();
		on_savepoint().connect(move |_| {
			let c = c1.clone();
			async move {
				c.fetch_add(1, Ordering::SeqCst);
				Ok(())
			}
		});

		let c2 = counter.clone();
		on_savepoint_release().connect(move |_| {
			let c = c2.clone();
			async move {
				c.fetch_add(10, Ordering::SeqCst);
				Ok(())
			}
		});

		let mut signals = TransactionSignals::new("tx_1");
		signals.enter_savepoint("sp_1").await.unwrap();
		signals.exit_savepoint().await.unwrap();

		assert_eq!(counter.load(Ordering::SeqCst), 11); // 1 + 10

		// Clean up after test
		on_savepoint().disconnect_all();
		on_savepoint_release().disconnect_all();
	}

	#[rstest]
	#[tokio::test]
	#[serial_test::serial]
	async fn test_nested_transaction_signals() {
		// Clean up before test
		on_savepoint().disconnect_all();

		let events = Arc::new(Mutex::new(Vec::new()));

		let e = events.clone();
		on_savepoint().connect(move |ctx| {
			let e = e.clone();
			async move {
				e.lock().push(format!(
					"savepoint:{}:depth:{}",
					ctx.savepoint_name.as_deref().unwrap_or(""),
					ctx.savepoint_depth
				));
				Ok(())
			}
		});

		let mut signals = TransactionSignals::new("tx_nested");
		signals.enter_savepoint("level_1").await.unwrap();
		signals.enter_savepoint("level_2").await.unwrap();

		let event_log = events.lock();
		assert_eq!(event_log.len(), 2);
		assert_eq!(event_log[0], "savepoint:level_1:depth:1");
		assert_eq!(event_log[1], "savepoint:level_2:depth:2");

		// Clean up after test
		on_savepoint().disconnect_all();
	}
}
