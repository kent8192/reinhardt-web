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

/// Synchronous signal that mimics Django's Signal class
#[derive(Clone)]
pub struct SyncSignal {
	receivers: Arc<RwLock<Vec<SignalReceiver>>>,
	#[allow(dead_code)]
	use_caching: bool,
}

struct SignalReceiver {
	receiver: Weak<
		dyn Fn(Option<Arc<dyn Any + Send + Sync>>, &HashMap<String, String>) -> String
			+ Send
			+ Sync,
	>,
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
	pub fn send(
		&self,
		sender: Option<Arc<dyn Any + Send + Sync>>,
		kwargs: &HashMap<String, String>,
	) -> Vec<(String, String)> {
		self.clear_dead_receivers();
		let receivers = self.receivers.read();
		let mut results = Vec::new();

		for receiver_data in receivers.iter() {
			// Check sender type match
			if let Some(expected_type_id) = receiver_data.sender_type_id {
				if let Some(ref actual_sender) = sender {
					// Must explicitly dereference Arc to get the underlying TypeId
					if (**actual_sender).type_id() != expected_type_id {
						continue; // Type mismatch
					}
				} else {
					continue; // Receiver expects a specific sender, but None was provided
				}
			}

			if let Some(receiver) = receiver_data.receiver.upgrade() {
				let result = receiver(sender.clone(), kwargs);
				results.push(("receiver".to_string(), result));
			}
		}

		results
	}

	/// Send signal robustly (catching panics)
	pub fn send_robust(
		&self,
		sender: Option<Arc<dyn Any + Send + Sync>>,
		kwargs: &HashMap<String, String>,
	) -> Vec<(String, Result<String, String>)> {
		self.clear_dead_receivers();
		let receivers = self.receivers.read();
		let mut results = Vec::new();

		for receiver_data in receivers.iter() {
			if let Some(receiver) = receiver_data.receiver.upgrade() {
				let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
					receiver(sender.clone(), kwargs)
				}));

				match result {
					Ok(val) => results.push(("receiver".to_string(), Ok(val))),
					Err(_) => results.push(("receiver".to_string(), Err("panic".to_string()))),
				}
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
