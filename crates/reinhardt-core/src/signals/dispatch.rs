//! Django-style synchronous signal dispatcher
//!
//! This module provides a synchronous signal system compatible with Django's dispatch pattern.

use parking_lot::RwLock;
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Weak};

/// Receiver function type for synchronous signals
pub type SyncReceiverFn = Arc<
	dyn Fn(Option<Arc<dyn Any + Send + Sync>>, &HashMap<String, String>) -> String + Send + Sync,
>;

/// Type alias for signal receiver function
type ReceiverFn =
	dyn Fn(Option<Arc<dyn Any + Send + Sync>>, &HashMap<String, String>) -> String + Send + Sync;

/// Synchronous signal that mimics Django's Signal class
#[derive(Clone)]
pub struct SyncSignal {
	receivers: Arc<RwLock<Vec<SignalReceiver>>>,
	/// Caching flag reserved for future optimization
	///
	/// This field is intentionally excluded from current implementation.
	/// The caching mechanism is designed but not used in the current send() implementation.
	///
	/// Planned usage:
	/// - Cache resolved receiver functions to avoid repeated lookups
	/// - Optimize signal dispatch performance for frequently-fired signals
	/// - Implement cache invalidation on receiver registration/deregistration
	///
	/// Implementation requires:
	/// - Cache storage design (per-signal or global)
	/// - Cache invalidation strategy
	/// - Thread-safety considerations for cached function pointers
	// Allow dead_code: caching flag reserved for planned dispatch optimization; not yet used in current send() implementation
	#[allow(dead_code)]
	use_caching: bool,
}

struct SignalReceiver {
	receiver: Weak<ReceiverFn>,
	sender_type_id: Option<std::any::TypeId>,
	dispatch_uid: Option<String>,
	// Keep a strong reference to prevent premature deallocation (when caller transfers ownership)
	_strong_ref: Option<SyncReceiverFn>,
}

impl SyncSignal {
	/// Create a new synchronous signal
	pub fn new() -> Self {
		Self {
			receivers: Arc::new(RwLock::new(Vec::new())),
			use_caching: false,
		}
	}

	/// Create a new synchronous signal with caching
	pub fn new_with_caching() -> Self {
		Self {
			receivers: Arc::new(RwLock::new(Vec::new())),
			use_caching: true,
		}
	}

	/// Connect a receiver to this signal
	pub fn connect<F>(
		&self,
		receiver: Arc<F>,
		sender_type_id: Option<std::any::TypeId>,
		dispatch_uid: Option<String>,
	) -> Result<(), String>
	where
		F: Fn(Option<Arc<dyn Any + Send + Sync>>, &HashMap<String, String>) -> String
			+ Send
			+ Sync
			+ 'static,
	{
		// Check if caller has other references before converting
		let should_store_strong = Arc::strong_count(&receiver) == 1;

		// Store the Arc as a trait object
		let receiver_arc: SyncReceiverFn = receiver;
		let weak_receiver = Arc::downgrade(&receiver_arc);
		let mut receivers = self.receivers.write();

		// Remove existing receiver with same dispatch_uid
		if let Some(ref uid) = dispatch_uid {
			receivers.retain(|r| r.dispatch_uid.as_ref() != Some(uid));
		}

		// Prevent duplicate registrations
		let receiver_ptr = weak_receiver.as_ptr();
		receivers.retain(|r| !std::ptr::addr_eq(r.receiver.as_ptr(), receiver_ptr));

		receivers.push(SignalReceiver {
			receiver: weak_receiver,
			sender_type_id,
			dispatch_uid,
			// Only store strong ref if caller has no other references (ownership transfer)
			_strong_ref: if should_store_strong {
				Some(receiver_arc)
			} else {
				None
			},
		});

		Ok(())
	}

	/// Disconnect a receiver by dispatch_uid
	/// If dispatch_uid is None, disconnects all receivers
	pub fn disconnect(&self, dispatch_uid: Option<&str>) -> bool {
		let mut receivers = self.receivers.write();
		let original_len = receivers.len();

		if let Some(uid) = dispatch_uid {
			receivers.retain(|r| r.dispatch_uid.as_deref() != Some(uid));
		} else {
			// If no dispatch_uid provided, clear all receivers
			receivers.clear();
		}

		receivers.len() < original_len
	}

	/// Send signal to all connected receivers
	///
	/// Receivers are collected under the lock, then the lock is released before
	/// invoking callbacks to prevent deadlock if a callback tries to modify
	/// the signal (connect/disconnect/send).
	pub fn send(
		&self,
		sender: Option<Arc<dyn Any + Send + Sync>>,
		kwargs: &HashMap<String, String>,
	) -> Vec<(String, String)> {
		self.clear_dead_receivers();

		// Collect live receivers under the lock, then release it before invocation
		let live_receivers: Vec<(Option<std::any::TypeId>, SyncReceiverFn)> = {
			let receivers = self.receivers.read();
			receivers
				.iter()
				.filter_map(|r| r.receiver.upgrade().map(|recv| (r.sender_type_id, recv)))
				.collect()
		};
		// Lock is now released; safe to invoke callbacks without deadlock risk

		let mut results = Vec::new();

		for (sender_type_id, receiver) in &live_receivers {
			// Check sender type match
			if let Some(expected_type_id) = sender_type_id {
				if let Some(ref actual_sender) = sender {
					// Must explicitly dereference Arc to get the underlying TypeId
					if (**actual_sender).type_id() != *expected_type_id {
						continue; // Type mismatch
					}
				} else {
					continue; // Receiver expects a specific sender, but None was provided
				}
			}

			let result = receiver(sender.clone(), kwargs);
			results.push(("receiver".to_string(), result));
		}

		results
	}

	/// Send signal robustly (catching panics)
	///
	/// Receivers are collected under the lock, then the lock is released before
	/// invoking callbacks to prevent deadlock if a callback tries to modify
	/// the signal (connect/disconnect/send).
	pub fn send_robust(
		&self,
		sender: Option<Arc<dyn Any + Send + Sync>>,
		kwargs: &HashMap<String, String>,
	) -> Vec<(String, Result<String, String>)> {
		self.clear_dead_receivers();

		// Collect live receivers under the lock, then release it before invocation
		let live_receivers: Vec<(Option<std::any::TypeId>, SyncReceiverFn)> = {
			let receivers = self.receivers.read();
			receivers
				.iter()
				.filter_map(|r| r.receiver.upgrade().map(|recv| (r.sender_type_id, recv)))
				.collect()
		};
		// Lock is now released; safe to invoke callbacks without deadlock risk

		let mut results = Vec::new();

		for (sender_type_id, receiver) in &live_receivers {
			// Check sender type match (same filtering as send)
			if let Some(expected_type_id) = sender_type_id {
				if let Some(ref actual_sender) = sender {
					// Must explicitly dereference Arc to get the underlying TypeId
					if (**actual_sender).type_id() != *expected_type_id {
						continue; // Type mismatch
					}
				} else {
					continue; // Receiver expects a specific sender, but None was provided
				}
			}

			let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
				receiver(sender.clone(), kwargs)
			}));

			match result {
				Ok(val) => results.push(("receiver".to_string(), Ok(val))),
				Err(_) => results.push(("receiver".to_string(), Err("panic".to_string()))),
			}
		}

		results
	}

	/// Check if signal has any listeners
	pub fn has_listeners(&self) -> bool {
		self.clear_dead_receivers();
		let receivers = self.receivers.read();
		!receivers.is_empty()
	}

	/// Get receiver count
	pub fn receivers_count(&self) -> usize {
		self.receivers.read().len()
	}

	/// Clear dead (garbage collected) receivers
	pub fn clear_dead_receivers(&self) {
		let mut receivers = self.receivers.write();
		receivers.retain(|r| r.receiver.strong_count() > 0);
	}
}

impl Default for SyncSignal {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	struct SenderA;
	struct SenderB;

	fn recv_echo(
		_sender: Option<Arc<dyn Any + Send + Sync>>,
		_kwargs: &HashMap<String, String>,
	) -> String {
		"received".to_string()
	}

	fn recv_a(
		_sender: Option<Arc<dyn Any + Send + Sync>>,
		_kwargs: &HashMap<String, String>,
	) -> String {
		"a".to_string()
	}

	fn recv_b(
		_sender: Option<Arc<dyn Any + Send + Sync>>,
		_kwargs: &HashMap<String, String>,
	) -> String {
		"b".to_string()
	}

	#[rstest]
	fn test_send_robust_filters_by_sender_type_id() {
		// Arrange
		let signal = SyncSignal::new();
		signal
			.connect(
				Arc::new(recv_echo),
				Some(std::any::TypeId::of::<SenderA>()),
				Some("typed_receiver".to_string()),
			)
			.unwrap();

		let kwargs = HashMap::new();

		// Act: send_robust with matching sender type
		let sender_a: Arc<dyn Any + Send + Sync> = Arc::new(SenderA);
		let results = signal.send_robust(Some(sender_a), &kwargs);

		// Assert: receiver should be invoked
		assert_eq!(results.len(), 1);
		assert_eq!(results[0].1, Ok("received".to_string()));
	}

	#[rstest]
	fn test_send_robust_skips_mismatched_sender_type() {
		// Arrange
		let signal = SyncSignal::new();
		signal
			.connect(
				Arc::new(recv_echo),
				Some(std::any::TypeId::of::<SenderA>()),
				Some("typed_receiver".to_string()),
			)
			.unwrap();

		let kwargs = HashMap::new();

		// Act: send_robust with different sender type
		let sender_b: Arc<dyn Any + Send + Sync> = Arc::new(SenderB);
		let results = signal.send_robust(Some(sender_b), &kwargs);

		// Assert: receiver should be skipped due to type mismatch
		assert_eq!(results.len(), 0);
	}

	#[rstest]
	fn test_send_robust_skips_when_sender_is_none_but_receiver_expects_type() {
		// Arrange
		let signal = SyncSignal::new();
		signal
			.connect(
				Arc::new(recv_echo),
				Some(std::any::TypeId::of::<SenderA>()),
				Some("typed_receiver".to_string()),
			)
			.unwrap();

		let kwargs = HashMap::new();

		// Act: send_robust with None sender
		let results = signal.send_robust(None, &kwargs);

		// Assert: receiver should be skipped (expects specific sender, but None provided)
		assert_eq!(results.len(), 0);
	}

	#[rstest]
	fn test_send_robust_invokes_unfiltered_receiver_with_any_sender() {
		// Arrange
		let signal = SyncSignal::new();
		signal
			.connect(
				Arc::new(recv_echo),
				None,
				Some("unfiltered_receiver".to_string()),
			)
			.unwrap();

		let kwargs = HashMap::new();

		// Act: send_robust with any sender
		let sender_b: Arc<dyn Any + Send + Sync> = Arc::new(SenderB);
		let results = signal.send_robust(Some(sender_b), &kwargs);

		// Assert: receiver should be invoked (no sender type filter)
		assert_eq!(results.len(), 1);
		assert_eq!(results[0].1, Ok("received".to_string()));
	}

	#[rstest]
	fn test_send_and_send_robust_have_consistent_filtering() {
		// Arrange
		let signal = SyncSignal::new();
		signal
			.connect(
				Arc::new(recv_a),
				Some(std::any::TypeId::of::<SenderA>()),
				Some("receiver_a".to_string()),
			)
			.unwrap();
		signal
			.connect(Arc::new(recv_b), None, Some("receiver_b".to_string()))
			.unwrap();

		let kwargs = HashMap::new();
		let sender_b: Arc<dyn Any + Send + Sync> = Arc::new(SenderB);

		// Act
		let send_results = signal.send(Some(sender_b.clone()), &kwargs);
		let send_robust_results = signal.send_robust(Some(sender_b), &kwargs);

		// Assert: both should skip receiver_a (type mismatch) and invoke receiver_b
		assert_eq!(send_results.len(), 1);
		assert_eq!(send_results[0].1, "b");
		assert_eq!(send_robust_results.len(), 1);
		assert_eq!(send_robust_results[0].1, Ok("b".to_string()));
	}
}
