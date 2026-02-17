//! Event Bus for Inter-Plugin Communication
//!
//! This module provides an event bus that enables asynchronous communication
//! between plugins using a publish-subscribe pattern.
//!
//! # Design
//!
//! - **Thread-safe**: Uses `parking_lot::RwLock` for concurrent access
//! - **Wildcard patterns**: Supports `*` and `prefix.*` patterns
//! - **Polling-based**: WASM plugins poll for events (no push notifications)
//! - **Per-subscription queues**: Each subscription maintains its own event queue

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Event data that can be transmitted between plugins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
	/// Event name (e.g., "user.created", "order.completed")
	pub name: String,
	/// MessagePack-serialized payload
	pub payload: Vec<u8>,
	/// Source plugin name that emitted the event
	pub source: String,
	/// Timestamp (Unix epoch milliseconds)
	pub timestamp: u64,
}

impl Event {
	/// Create a new event with the current timestamp.
	pub fn new(name: impl Into<String>, payload: Vec<u8>, source: impl Into<String>) -> Self {
		let timestamp = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("System time before UNIX epoch")
			.as_millis() as u64;

		Self {
			name: name.into(),
			payload,
			source: source.into(),
			timestamp,
		}
	}

	/// Create an event with a specific timestamp (for testing).
	#[cfg(test)]
	pub fn with_timestamp(
		name: impl Into<String>,
		payload: Vec<u8>,
		source: impl Into<String>,
		timestamp: u64,
	) -> Self {
		Self {
			name: name.into(),
			payload,
			source: source.into(),
			timestamp,
		}
	}
}

/// Subscription information for event filtering.
struct Subscription {
	/// Pattern to match event names (supports wildcards)
	pattern: String,
	/// Queue of events waiting to be polled
	queue: VecDeque<Event>,
	/// Plugin name that owns this subscription
	owner: String,
}

/// Event bus for inter-plugin communication.
///
/// The event bus provides a publish-subscribe mechanism that allows plugins
/// to communicate without direct references to each other.
///
/// # Example
///
/// ```ignore
/// use reinhardt_dentdelion::wasm::events::EventBus;
///
/// let bus = EventBus::new();
///
/// // Subscribe to user events
/// let sub_id = bus.subscribe("user.*", "my-plugin");
///
/// // Emit an event
/// bus.emit("user.created", vec![1, 2, 3], "auth-plugin");
///
/// // Poll for events
/// let events = bus.poll(sub_id, 10);
/// ```
pub struct EventBus {
	/// Active subscriptions keyed by subscription ID
	subscriptions: RwLock<HashMap<u64, Subscription>>,
	/// Counter for generating unique subscription IDs
	next_id: AtomicU64,
	/// Maximum queue size per subscription (prevents memory exhaustion)
	max_queue_size: usize,
}

impl EventBus {
	/// Create a new event bus with default settings.
	pub fn new() -> Self {
		Self::with_max_queue_size(10_000)
	}

	/// Create a new event bus with a custom maximum queue size.
	pub fn with_max_queue_size(max_queue_size: usize) -> Self {
		Self {
			subscriptions: RwLock::new(HashMap::new()),
			next_id: AtomicU64::new(1),
			max_queue_size,
		}
	}

	/// Emit an event to all matching subscribers.
	///
	/// # Arguments
	///
	/// * `name` - Event name (e.g., "user.created")
	/// * `payload` - MessagePack-serialized event data
	/// * `source` - Name of the plugin emitting the event
	///
	/// # Returns
	///
	/// The number of subscriptions that received the event.
	pub fn emit(&self, name: &str, payload: Vec<u8>, source: &str) -> usize {
		let event = Event::new(name, payload, source);
		let mut delivered = 0;

		let mut subs = self.subscriptions.write();
		for sub in subs.values_mut() {
			if Self::matches_pattern(&sub.pattern, &event.name) {
				// Enforce queue size limit (drop oldest if full)
				while sub.queue.len() >= self.max_queue_size {
					sub.queue.pop_front();
				}
				sub.queue.push_back(event.clone());
				delivered += 1;
			}
		}

		delivered
	}

	/// Subscribe to events matching a pattern.
	///
	/// # Pattern Syntax
	///
	/// - `*` - Matches all events
	/// - `user.*` - Matches events starting with "user." (e.g., "user.created", "user.deleted")
	/// - `user.created` - Matches only "user.created"
	///
	/// # Arguments
	///
	/// * `pattern` - Event name pattern
	/// * `owner` - Name of the plugin creating the subscription
	///
	/// # Returns
	///
	/// A unique subscription ID that can be used for polling and unsubscribing.
	pub fn subscribe(&self, pattern: &str, owner: &str) -> u64 {
		let id = self.next_id.fetch_add(1, Ordering::SeqCst);
		let subscription = Subscription {
			pattern: pattern.to_string(),
			queue: VecDeque::new(),
			owner: owner.to_string(),
		};
		self.subscriptions.write().insert(id, subscription);
		id
	}

	/// Unsubscribe from a subscription.
	///
	/// # Arguments
	///
	/// * `id` - Subscription ID returned by `subscribe`
	///
	/// # Returns
	///
	/// `true` if the subscription existed and was removed, `false` otherwise.
	pub fn unsubscribe(&self, id: u64) -> bool {
		self.subscriptions.write().remove(&id).is_some()
	}

	/// Poll for pending events on a subscription.
	///
	/// Events are removed from the queue once polled.
	///
	/// # Arguments
	///
	/// * `id` - Subscription ID
	/// * `limit` - Maximum number of events to return
	///
	/// # Returns
	///
	/// A vector of events, up to `limit` events.
	pub fn poll(&self, id: u64, limit: usize) -> Vec<Event> {
		let mut subs = self.subscriptions.write();
		if let Some(sub) = subs.get_mut(&id) {
			let count = limit.min(sub.queue.len());
			sub.queue.drain(..count).collect()
		} else {
			Vec::new()
		}
	}

	/// Get the number of pending events for a subscription.
	pub fn pending_count(&self, id: u64) -> usize {
		self.subscriptions
			.read()
			.get(&id)
			.map(|s| s.queue.len())
			.unwrap_or(0)
	}

	/// Check if a subscription exists.
	pub fn has_subscription(&self, id: u64) -> bool {
		self.subscriptions.read().contains_key(&id)
	}

	/// Get the total number of active subscriptions.
	pub fn subscription_count(&self) -> usize {
		self.subscriptions.read().len()
	}

	/// Remove all subscriptions owned by a specific plugin.
	///
	/// This is useful for cleanup when a plugin is unloaded.
	pub fn remove_plugin_subscriptions(&self, plugin_name: &str) -> usize {
		let mut subs = self.subscriptions.write();
		let initial_count = subs.len();
		subs.retain(|_, sub| sub.owner != plugin_name);
		initial_count - subs.len()
	}

	/// Check if an event name matches a pattern.
	///
	/// # Pattern Matching Rules
	///
	/// - `*` matches any event name
	/// - `prefix.*` matches any event name starting with "prefix."
	/// - Exact match otherwise
	fn matches_pattern(pattern: &str, event_name: &str) -> bool {
		if pattern == "*" {
			return true;
		}

		if let Some(prefix) = pattern.strip_suffix(".*") {
			// Pattern like "user.*" matches "user.created", "user.deleted", etc.
			// The event must start with the prefix followed by a dot
			event_name.starts_with(prefix)
				&& event_name
					.get(prefix.len()..)
					.is_some_and(|s| s.starts_with('.'))
		} else {
			// Exact match
			pattern == event_name
		}
	}
}

impl Default for EventBus {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Debug for EventBus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let subs = self.subscriptions.read();
		f.debug_struct("EventBus")
			.field("subscription_count", &subs.len())
			.field("max_queue_size", &self.max_queue_size)
			.finish()
	}
}

/// Shared event bus instance type.
pub type SharedEventBus = Arc<EventBus>;

#[cfg(test)]
mod tests {
	use super::*;
	use rstest::rstest;

	#[rstest]
	fn test_event_creation() {
		let event = Event::new("user.created", vec![1, 2, 3], "test-plugin");

		assert_eq!(event.name, "user.created");
		assert_eq!(event.payload, vec![1, 2, 3]);
		assert_eq!(event.source, "test-plugin");
		assert!(event.timestamp > 0);
	}

	#[rstest]
	fn test_subscribe_and_unsubscribe() {
		let bus = EventBus::new();

		let id = bus.subscribe("user.*", "test-plugin");
		assert!(bus.has_subscription(id));
		assert_eq!(bus.subscription_count(), 1);

		let removed = bus.unsubscribe(id);
		assert!(removed);
		assert!(!bus.has_subscription(id));
		assert_eq!(bus.subscription_count(), 0);
	}

	#[rstest]
	fn test_emit_and_poll() {
		let bus = EventBus::new();

		let sub_id = bus.subscribe("user.*", "consumer");

		// Emit some events
		let delivered = bus.emit("user.created", vec![1], "producer");
		assert_eq!(delivered, 1);

		let delivered = bus.emit("user.updated", vec![2], "producer");
		assert_eq!(delivered, 1);

		// Non-matching event
		let delivered = bus.emit("order.created", vec![3], "producer");
		assert_eq!(delivered, 0);

		// Poll events
		let events = bus.poll(sub_id, 10);
		assert_eq!(events.len(), 2);
		assert_eq!(events[0].name, "user.created");
		assert_eq!(events[1].name, "user.updated");

		// Queue should be empty now
		let events = bus.poll(sub_id, 10);
		assert!(events.is_empty());
	}

	#[rstest]
	fn test_pattern_matching() {
		// Wildcard matches all
		assert!(EventBus::matches_pattern("*", "anything"));
		assert!(EventBus::matches_pattern("*", "user.created"));

		// Prefix wildcard
		assert!(EventBus::matches_pattern("user.*", "user.created"));
		assert!(EventBus::matches_pattern("user.*", "user.deleted"));
		assert!(EventBus::matches_pattern("user.*", "user.profile.updated"));
		assert!(!EventBus::matches_pattern("user.*", "user"));
		assert!(!EventBus::matches_pattern("user.*", "users.created"));

		// Exact match
		assert!(EventBus::matches_pattern("user.created", "user.created"));
		assert!(!EventBus::matches_pattern("user.created", "user.deleted"));
	}

	#[rstest]
	fn test_multiple_subscribers() {
		let bus = EventBus::new();

		let sub1 = bus.subscribe("user.*", "plugin-a");
		let sub2 = bus.subscribe("*", "plugin-b");
		let sub3 = bus.subscribe("order.*", "plugin-c");

		// Emit user event - should match sub1 and sub2
		let delivered = bus.emit("user.created", vec![1], "producer");
		assert_eq!(delivered, 2);

		// Emit order event - should match sub2 and sub3
		let delivered = bus.emit("order.created", vec![2], "producer");
		assert_eq!(delivered, 2);

		// Check individual queues
		assert_eq!(bus.pending_count(sub1), 1);
		assert_eq!(bus.pending_count(sub2), 2);
		assert_eq!(bus.pending_count(sub3), 1);
	}

	#[rstest]
	fn test_queue_limit() {
		let bus = EventBus::with_max_queue_size(3);
		let sub_id = bus.subscribe("*", "test");

		// Emit 5 events
		for i in 0..5 {
			bus.emit(&format!("event.{}", i), vec![i as u8], "producer");
		}

		// Only 3 should remain (oldest dropped)
		let events = bus.poll(sub_id, 10);
		assert_eq!(events.len(), 3);
		assert_eq!(events[0].name, "event.2");
		assert_eq!(events[1].name, "event.3");
		assert_eq!(events[2].name, "event.4");
	}

	#[rstest]
	fn test_remove_plugin_subscriptions() {
		let bus = EventBus::new();

		bus.subscribe("a.*", "plugin-a");
		bus.subscribe("b.*", "plugin-a");
		bus.subscribe("c.*", "plugin-b");

		assert_eq!(bus.subscription_count(), 3);

		let removed = bus.remove_plugin_subscriptions("plugin-a");
		assert_eq!(removed, 2);
		assert_eq!(bus.subscription_count(), 1);
	}

	#[rstest]
	fn test_poll_nonexistent_subscription() {
		let bus = EventBus::new();
		let events = bus.poll(999, 10);
		assert!(events.is_empty());
	}

	#[rstest]
	fn test_poll_limit() {
		let bus = EventBus::new();
		let sub_id = bus.subscribe("*", "test");

		// Emit 10 events
		for i in 0..10 {
			bus.emit(&format!("event.{}", i), vec![], "producer");
		}

		// Poll with limit of 3
		let events = bus.poll(sub_id, 3);
		assert_eq!(events.len(), 3);

		// 7 remaining
		assert_eq!(bus.pending_count(sub_id), 7);
	}
}
