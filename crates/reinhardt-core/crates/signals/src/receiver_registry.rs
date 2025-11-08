//! Receiver function registry for automatic signal connection
//!
//! This module provides infrastructure for the `#[receiver]` macro to automatically
//! register receiver functions and connect them to signals at runtime.

use crate::error::SignalError;
use std::any::TypeId;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for receiver function
pub type ReceiverFunction = dyn Fn(
		Arc<dyn std::any::Any + Send + Sync>,
	) -> Pin<Box<dyn Future<Output = Result<(), SignalError>> + Send>>
	+ Send
	+ Sync;

/// Entry in the receiver registry collected by inventory
///
/// This struct holds metadata about a receiver function that should be
/// automatically connected to a signal at runtime.
pub struct ReceiverRegistryEntry {
	/// Name of the signal to connect to
	pub signal_name: &'static str,

	/// Name of the receiver function (for debugging)
	pub receiver_name: &'static str,

	/// Optional sender type filter (computed lazily)
	pub sender_type_fn: Option<fn() -> TypeId>,

	/// Optional dispatch UID for this receiver
	pub dispatch_uid: Option<&'static str>,

	/// Priority for receiver execution (higher values execute first)
	pub priority: i32,

	/// Factory function to create the receiver
	pub receiver_factory: fn() -> Arc<ReceiverFunction>,
}

impl ReceiverRegistryEntry {
	/// Create a new receiver registry entry
	pub const fn new(
		signal_name: &'static str,
		receiver_name: &'static str,
		receiver_factory: fn() -> Arc<ReceiverFunction>,
	) -> Self {
		Self {
			signal_name,
			receiver_name,
			sender_type_fn: None,
			dispatch_uid: None,
			priority: 0,
			receiver_factory,
		}
	}

	/// Set sender type filter
	pub const fn with_sender_type(mut self, sender_type_fn: fn() -> TypeId) -> Self {
		self.sender_type_fn = Some(sender_type_fn);
		self
	}

	/// Set dispatch UID
	pub const fn with_dispatch_uid(mut self, dispatch_uid: &'static str) -> Self {
		self.dispatch_uid = Some(dispatch_uid);
		self
	}

	/// Set priority
	pub const fn with_priority(mut self, priority: i32) -> Self {
		self.priority = priority;
		self
	}
}

// Collect all ReceiverRegistryEntry instances via inventory
inventory::collect!(ReceiverRegistryEntry);

/// Automatically connect all registered receivers to their signals
///
/// This function should be called during application initialization to connect
/// all receivers that were registered via the `#[receiver]` macro.
///
/// # Examples
///
/// ```ignore
/// use reinhardt_signals::auto_connect_receivers;
///
/// #[tokio::main]
/// async fn main() {
///     // Connect all registered receivers
///     auto_connect_receivers();
///
///     // Your application code here
/// }
/// ```
pub fn auto_connect_receivers() {
	for entry in inventory::iter::<ReceiverRegistryEntry> {
		// Create the receiver function
		let receiver = (entry.receiver_factory)();

		// Connect the receiver to the signal using type-erased approach
		connect_receiver_to_signal(entry, receiver);
	}
}

/// Connect a receiver to its signal using dynamic dispatch
///
/// This function uses type erasure to connect receivers to signals at runtime,
/// since we don't know the concrete signal data type at compile time.
fn connect_receiver_to_signal(entry: &ReceiverRegistryEntry, receiver: Arc<ReceiverFunction>) {
	use crate::registry::get_signal_with_string;

	// Use type-erased data type (Arc<dyn Any + Send + Sync>)
	// This allows us to handle any signal data type at runtime
	type AnyData = Arc<dyn std::any::Any + Send + Sync>;

	// Get the signal using string-based lookup
	let signal = get_signal_with_string::<AnyData>(entry.signal_name);

	// Create a receiver wrapper that calls the actual receiver function
	let receiver_wrapper = move |data: Arc<AnyData>| {
		let receiver = Arc::clone(&receiver);
		receiver(data)
	};

	// Compute sender type ID if filter function is provided
	let sender_type_id = entry.sender_type_fn.map(|f| f());

	// Connect with options (priority, dispatch_uid, sender_type)
	signal.connect_with_options(
		receiver_wrapper,
		sender_type_id,
		entry.dispatch_uid.map(|s| s.to_string()),
		entry.priority,
	);
}
