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
//! - **Shared storage**: An emitted event is stored once and shared by every
//!   queue that receives it, so fan-out does not copy the payload
//! - **Byte budgets**: Event names, payloads, and queued bytes are bounded
//!   because both emitters and subscribers may be untrusted plugins

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
		// Use unwrap_or_default to avoid panic when system clock precedes Unix epoch.
		let timestamp = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap_or_default()
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

	/// Bytes charged against queue budgets while this event is queued.
	fn retained_bytes(&self) -> usize {
		self.name.len() + self.payload.len() + self.source.len()
	}
}

/// Subscription information for event filtering.
struct Subscription {
	/// Pattern to match event names (supports wildcards)
	pattern: String,
	/// Queue of events waiting to be polled.
	///
	/// Events are shared with every other queue that received the same emit.
	queue: VecDeque<Arc<Event>>,
	/// Bytes charged for the events currently in `queue`
	queued_bytes: usize,
	/// Plugin name that owns this subscription
	owner: String,
}

impl Subscription {
	/// Queue `event` unless doing so would exceed a count or byte budget.
	///
	/// Budgets are enforced by dropping this subscription's oldest events
	/// only. Events queued for other subscriptions are never evicted to make
	/// room, so one plugin cannot discard another plugin's pending events.
	/// When clearing this queue would still not make room, the event is not
	/// delivered to this subscription and `false` is returned.
	fn enqueue(
		&mut self,
		event: &Arc<Event>,
		bytes: usize,
		max_queue_size: usize,
		limits: &EventByteLimits,
		budget: &mut QueueBudget,
	) -> bool {
		let owner_bytes = budget.owner_bytes(&self.owner);
		let fits_after_clearing = max_queue_size > 0
			&& bytes <= limits.max_queue_bytes
			&& owner_bytes.saturating_sub(self.queued_bytes) + bytes
				<= limits.max_owner_queue_bytes
			&& budget.total_bytes.saturating_sub(self.queued_bytes) + bytes
				<= limits.max_total_queue_bytes;
		if !fits_after_clearing {
			return false;
		}

		while self.queue.len() >= max_queue_size
			|| self.queued_bytes + bytes > limits.max_queue_bytes
			|| budget.owner_bytes(&self.owner) + bytes > limits.max_owner_queue_bytes
			|| budget.total_bytes + bytes > limits.max_total_queue_bytes
		{
			if self.pop_oldest(budget).is_none() {
				// Unreachable while the budget counters are consistent: the
				// check above proved that an empty queue leaves enough room.
				return false;
			}
		}

		self.queue.push_back(Arc::clone(event));
		self.queued_bytes += bytes;
		budget.charge(&self.owner, bytes);
		true
	}

	/// Remove the oldest queued event and release its bytes.
	fn pop_oldest(&mut self, budget: &mut QueueBudget) -> Option<Arc<Event>> {
		let event = self.queue.pop_front()?;
		let bytes = event.retained_bytes();
		self.queued_bytes -= bytes;
		budget.release(&self.owner, bytes);
		Some(event)
	}
}

/// Bytes currently charged to subscription owners and to the whole bus.
#[derive(Default)]
struct QueueBudget {
	/// Queued bytes per subscription owner, across all of its subscriptions
	owner_bytes: HashMap<String, usize>,
	/// Queued bytes across all subscriptions
	total_bytes: usize,
}

impl QueueBudget {
	fn owner_bytes(&self, owner: &str) -> usize {
		self.owner_bytes.get(owner).copied().unwrap_or(0)
	}

	fn charge(&mut self, owner: &str, bytes: usize) {
		match self.owner_bytes.get_mut(owner) {
			Some(owner_bytes) => *owner_bytes += bytes,
			None => {
				self.owner_bytes.insert(owner.to_owned(), bytes);
			}
		}
		self.total_bytes += bytes;
	}

	fn release(&mut self, owner: &str, bytes: usize) {
		if let Some(owner_bytes) = self.owner_bytes.get_mut(owner) {
			*owner_bytes -= bytes;
			if *owner_bytes == 0 {
				self.owner_bytes.remove(owner);
			}
		}
		self.total_bytes -= bytes;
	}
}

/// Subscriptions and the byte budget that accounts for their queues.
///
/// Both live behind one lock so that queue contents and budget counters are
/// always updated together.
#[derive(Default)]
struct BusState {
	/// Active subscriptions keyed by subscription ID
	subscriptions: HashMap<u64, Subscription>,
	/// Bytes charged for queued events
	budget: QueueBudget,
}

/// Default maximum number of subscriptions per plugin.
const DEFAULT_MAX_SUBSCRIPTIONS_PER_PLUGIN: usize = 100;

/// Default maximum number of total subscriptions across all plugins.
const DEFAULT_MAX_TOTAL_SUBSCRIPTIONS: usize = 10_000;

/// Default maximum event name size in bytes.
const DEFAULT_MAX_EVENT_NAME_BYTES: usize = 256;

/// Default maximum event payload size in bytes (64 KiB).
const DEFAULT_MAX_EVENT_PAYLOAD_BYTES: usize = 64 * 1024;

/// Default maximum queued bytes per subscription (1 MiB).
const DEFAULT_MAX_QUEUE_BYTES: usize = 1024 * 1024;

/// Default maximum queued bytes per subscription owner (4 MiB).
const DEFAULT_MAX_OWNER_QUEUE_BYTES: usize = 4 * 1024 * 1024;

/// Default maximum queued bytes across the whole bus (64 MiB).
const DEFAULT_MAX_TOTAL_QUEUE_BYTES: usize = 64 * 1024 * 1024;

/// Byte budgets enforced by an [`EventBus`].
///
/// Event names and payloads come from plugins, so the bus bounds the size of
/// each emitted event and the bytes retained in subscription queues.
///
/// A queued event is charged `name + payload + source` bytes against the
/// receiving subscription, the plugin that owns the subscription, and the
/// whole bus. Every queue that receives an event shares a single copy of it,
/// so the charged total is an upper bound on the memory held by queues.
///
/// # Example
///
/// ```rust
/// use reinhardt_dentdelion::wasm::{EventBus, EventByteLimits};
///
/// let mut limits = EventByteLimits::default();
/// limits.max_payload_bytes = 16 * 1024;
///
/// let bus = EventBus::new().with_byte_limits(limits);
/// assert_eq!(bus.byte_limits().max_payload_bytes, 16 * 1024);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct EventByteLimits {
	/// Maximum event name size accepted by [`EventBus::try_emit`]
	pub max_name_bytes: usize,
	/// Maximum event payload size accepted by [`EventBus::try_emit`]
	pub max_payload_bytes: usize,
	/// Maximum bytes queued for a single subscription.
	///
	/// When a new event does not fit, the subscription's oldest events are
	/// dropped first.
	pub max_queue_bytes: usize,
	/// Maximum bytes queued across all subscriptions owned by one plugin.
	///
	/// When a new event does not fit, the receiving subscription's oldest
	/// events are dropped. If the owner's other subscriptions hold the
	/// budget, the event is not delivered to the receiving subscription.
	pub max_owner_queue_bytes: usize,
	/// Maximum bytes queued across the whole bus.
	///
	/// Enforced like `max_owner_queue_bytes`: only the receiving
	/// subscription's own events are dropped to make room.
	pub max_total_queue_bytes: usize,
}

impl Default for EventByteLimits {
	fn default() -> Self {
		Self {
			max_name_bytes: DEFAULT_MAX_EVENT_NAME_BYTES,
			max_payload_bytes: DEFAULT_MAX_EVENT_PAYLOAD_BYTES,
			max_queue_bytes: DEFAULT_MAX_QUEUE_BYTES,
			max_owner_queue_bytes: DEFAULT_MAX_OWNER_QUEUE_BYTES,
			max_total_queue_bytes: DEFAULT_MAX_TOTAL_QUEUE_BYTES,
		}
	}
}

/// Event bus for inter-plugin communication.
///
/// The event bus provides a publish-subscribe mechanism that allows plugins
/// to communicate without direct references to each other.
///
/// # Resource Limits
///
/// The event bus enforces count limits to prevent memory exhaustion:
/// - Per-plugin subscription limit (default: 100)
/// - Global total subscription limit (default: 10,000)
/// - Per-subscription queue size limit (default: 10,000 events)
///
/// It also enforces byte limits (see [`EventByteLimits`]):
/// - Event name size limit (default: 256 bytes)
/// - Event payload size limit (default: 64 KiB)
/// - Per-subscription queued bytes (default: 1 MiB)
/// - Per-owner queued bytes across all of a plugin's subscriptions (default: 4 MiB)
/// - Total queued bytes across the bus (default: 64 MiB)
///
/// Oversized events are rejected by [`try_emit`](Self::try_emit) before they
/// reach any queue. Queue budgets only ever drop the receiving subscription's
/// own oldest events; a subscription that cannot make room skips the event.
///
/// # Example
///
/// ```ignore
/// use reinhardt_dentdelion::wasm::events::EventBus;
///
/// let bus = EventBus::new();
///
/// // Subscribe to user events
/// let sub_id = bus.subscribe("user.*", "my-plugin")?;
///
/// // Emit an event
/// bus.emit("user.created", vec![1, 2, 3], "auth-plugin");
///
/// // Poll for events
/// let events = bus.poll(sub_id, 10);
/// ```
pub struct EventBus {
	/// Subscriptions and their queued-byte accounting
	state: RwLock<BusState>,
	/// Counter for generating unique subscription IDs
	next_id: AtomicU64,
	/// Maximum queue size per subscription (prevents memory exhaustion)
	max_queue_size: usize,
	/// Maximum number of subscriptions per plugin
	max_subscriptions_per_plugin: usize,
	/// Maximum total number of subscriptions across all plugins
	max_total_subscriptions: usize,
	/// Byte budgets for emitted and queued events
	byte_limits: EventByteLimits,
}

/// Errors that can occur during event bus operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EventBusError {
	/// Per-plugin subscription limit exceeded
	#[error("plugin '{plugin}' has reached subscription limit ({limit})")]
	PerPluginLimitExceeded {
		/// Plugin that attempted to subscribe
		plugin: String,
		/// Maximum allowed subscriptions per plugin
		limit: usize,
	},

	/// Global subscription limit exceeded
	#[error("total subscription limit reached ({limit})")]
	TotalLimitExceeded {
		/// Maximum allowed total subscriptions
		limit: usize,
	},
}

/// Errors returned when an event is rejected before delivery.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EventEmitError {
	/// The event name exceeds [`EventByteLimits::max_name_bytes`]
	#[error("event name is {size} bytes, exceeding the {limit}-byte limit")]
	NameTooLarge {
		/// Size of the rejected event name in bytes
		size: usize,
		/// Maximum allowed event name size in bytes
		limit: usize,
	},

	/// The event payload exceeds [`EventByteLimits::max_payload_bytes`]
	#[error("event payload is {size} bytes, exceeding the {limit}-byte limit")]
	PayloadTooLarge {
		/// Size of the rejected payload in bytes
		size: usize,
		/// Maximum allowed payload size in bytes
		limit: usize,
	},
}

impl EventBus {
	/// Create a new event bus with default settings.
	pub fn new() -> Self {
		Self::with_limits(
			10_000,
			DEFAULT_MAX_SUBSCRIPTIONS_PER_PLUGIN,
			DEFAULT_MAX_TOTAL_SUBSCRIPTIONS,
		)
	}

	/// Create a new event bus with a custom maximum queue size.
	pub fn with_max_queue_size(max_queue_size: usize) -> Self {
		Self::with_limits(
			max_queue_size,
			DEFAULT_MAX_SUBSCRIPTIONS_PER_PLUGIN,
			DEFAULT_MAX_TOTAL_SUBSCRIPTIONS,
		)
	}

	/// Create a new event bus with custom resource limits.
	///
	/// Byte limits use [`EventByteLimits::default`]; override them with
	/// [`with_byte_limits`](Self::with_byte_limits).
	///
	/// # Arguments
	///
	/// * `max_queue_size` - Maximum number of events per subscription queue
	/// * `max_subscriptions_per_plugin` - Maximum subscriptions a single plugin can create
	/// * `max_total_subscriptions` - Maximum total subscriptions across all plugins
	pub fn with_limits(
		max_queue_size: usize,
		max_subscriptions_per_plugin: usize,
		max_total_subscriptions: usize,
	) -> Self {
		Self {
			state: RwLock::new(BusState::default()),
			next_id: AtomicU64::new(1),
			max_queue_size,
			max_subscriptions_per_plugin,
			max_total_subscriptions,
			byte_limits: EventByteLimits::default(),
		}
	}

	/// Replace the byte limits enforced by this event bus.
	///
	/// Call this before the bus is shared; events already queued are not
	/// re-checked against the new limits.
	pub fn with_byte_limits(mut self, byte_limits: EventByteLimits) -> Self {
		self.byte_limits = byte_limits;
		self
	}

	/// Get the byte limits enforced by this event bus.
	pub fn byte_limits(&self) -> EventByteLimits {
		self.byte_limits
	}

	/// Emit an event to all matching subscribers.
	///
	/// Events that exceed a size limit are not delivered and `0` is returned.
	/// Use [`try_emit`](Self::try_emit) to distinguish a rejected event from
	/// an event without matching subscribers.
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
		match self.try_emit(name, payload, source) {
			Ok(delivered) => delivered,
			Err(error) => {
				tracing::warn!(source, %error, "event rejected by the event bus");
				0
			}
		}
	}

	/// Emit an event to all matching subscribers, rejecting oversized events.
	///
	/// The event is stored once and shared by every receiving queue. A
	/// subscription that cannot make room within its byte budgets by dropping
	/// its own oldest events does not receive the event.
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
	///
	/// # Errors
	///
	/// Returns an error without queuing anything if the event name or payload
	/// exceeds the configured [`EventByteLimits`].
	pub fn try_emit(
		&self,
		name: &str,
		payload: Vec<u8>,
		source: &str,
	) -> Result<usize, EventEmitError> {
		let limits = self.byte_limits;
		if name.len() > limits.max_name_bytes {
			return Err(EventEmitError::NameTooLarge {
				size: name.len(),
				limit: limits.max_name_bytes,
			});
		}
		if payload.len() > limits.max_payload_bytes {
			return Err(EventEmitError::PayloadTooLarge {
				size: payload.len(),
				limit: limits.max_payload_bytes,
			});
		}

		let event = Arc::new(Event::new(name, payload, source));
		let bytes = event.retained_bytes();
		let mut delivered = 0;

		let mut state = self.state.write();
		let BusState {
			subscriptions,
			budget,
		} = &mut *state;
		for sub in subscriptions.values_mut() {
			if Self::matches_pattern(&sub.pattern, &event.name)
				&& sub.enqueue(&event, bytes, self.max_queue_size, &limits, budget)
			{
				delivered += 1;
			}
		}

		Ok(delivered)
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
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The plugin has reached its per-plugin subscription limit
	/// - The global total subscription limit has been reached
	pub fn subscribe(&self, pattern: &str, owner: &str) -> Result<u64, EventBusError> {
		let mut state = self.state.write();
		let subs = &mut state.subscriptions;

		// Check global subscription limit
		if subs.len() >= self.max_total_subscriptions {
			return Err(EventBusError::TotalLimitExceeded {
				limit: self.max_total_subscriptions,
			});
		}

		// Check per-plugin subscription limit
		let plugin_count = subs.values().filter(|s| s.owner == owner).count();
		if plugin_count >= self.max_subscriptions_per_plugin {
			return Err(EventBusError::PerPluginLimitExceeded {
				plugin: owner.to_string(),
				limit: self.max_subscriptions_per_plugin,
			});
		}

		// Wrapping addition is the default behavior of AtomicU64::fetch_add.
		// With u64 range (~1.8 * 10^19), overflow is practically unreachable.
		// If it ever wraps, the subscription map insertion will overwrite any
		// stale entry with the same ID, which is acceptable.
		let id = self.next_id.fetch_add(1, Ordering::Relaxed);
		let subscription = Subscription {
			pattern: pattern.to_string(),
			queue: VecDeque::new(),
			queued_bytes: 0,
			owner: owner.to_string(),
		};
		if let Some(replaced) = subs.insert(id, subscription) {
			state.budget.release(&replaced.owner, replaced.queued_bytes);
		}
		Ok(id)
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
		let mut state = self.state.write();
		match state.subscriptions.remove(&id) {
			Some(removed) => {
				state.budget.release(&removed.owner, removed.queued_bytes);
				true
			}
			None => false,
		}
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
		let polled: Vec<Arc<Event>> = {
			let mut state = self.state.write();
			let BusState {
				subscriptions,
				budget,
			} = &mut *state;
			match subscriptions.get_mut(&id) {
				Some(sub) => {
					let count = limit.min(sub.queue.len());
					(0..count).filter_map(|_| sub.pop_oldest(budget)).collect()
				}
				None => Vec::new(),
			}
		};

		// Materialize outside the lock. The payload is copied only while
		// another subscription still shares the same event.
		polled.into_iter().map(Arc::unwrap_or_clone).collect()
	}

	/// Get the number of pending events for a subscription.
	pub fn pending_count(&self, id: u64) -> usize {
		self.state
			.read()
			.subscriptions
			.get(&id)
			.map(|s| s.queue.len())
			.unwrap_or(0)
	}

	/// Get the bytes charged for pending events of a subscription.
	pub fn pending_bytes(&self, id: u64) -> usize {
		self.state
			.read()
			.subscriptions
			.get(&id)
			.map(|s| s.queued_bytes)
			.unwrap_or(0)
	}

	/// Get the bytes charged for pending events across all subscriptions.
	pub fn total_pending_bytes(&self) -> usize {
		self.state.read().budget.total_bytes
	}

	/// Check if a subscription exists.
	pub fn has_subscription(&self, id: u64) -> bool {
		self.state.read().subscriptions.contains_key(&id)
	}

	/// Get the total number of active subscriptions.
	pub fn subscription_count(&self) -> usize {
		self.state.read().subscriptions.len()
	}

	/// Remove all subscriptions owned by a specific plugin.
	///
	/// This is useful for cleanup when a plugin is unloaded.
	pub fn remove_plugin_subscriptions(&self, plugin_name: &str) -> usize {
		let mut state = self.state.write();
		let BusState {
			subscriptions,
			budget,
		} = &mut *state;
		let initial_count = subscriptions.len();
		subscriptions.retain(|_, sub| {
			if sub.owner == plugin_name {
				budget.release(&sub.owner, sub.queued_bytes);
				false
			} else {
				true
			}
		});
		initial_count - subscriptions.len()
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
		let state = self.state.read();
		f.debug_struct("EventBus")
			.field("subscription_count", &state.subscriptions.len())
			.field("total_pending_bytes", &state.budget.total_bytes)
			.field("max_queue_size", &self.max_queue_size)
			.field(
				"max_subscriptions_per_plugin",
				&self.max_subscriptions_per_plugin,
			)
			.field("max_total_subscriptions", &self.max_total_subscriptions)
			.field("byte_limits", &self.byte_limits)
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
		// Arrange & Act
		let event = Event::new("user.created", vec![1, 2, 3], "test-plugin");

		// Assert
		assert_eq!(event.name, "user.created");
		assert_eq!(event.payload, vec![1, 2, 3]);
		assert_eq!(event.source, "test-plugin");
		assert!(event.timestamp > 0);
	}

	#[rstest]
	fn test_subscribe_and_unsubscribe() {
		// Arrange
		let bus = EventBus::new();

		// Act
		let id = bus.subscribe("user.*", "test-plugin").unwrap();

		// Assert
		assert!(bus.has_subscription(id));
		assert_eq!(bus.subscription_count(), 1);

		let removed = bus.unsubscribe(id);
		assert!(removed);
		assert!(!bus.has_subscription(id));
		assert_eq!(bus.subscription_count(), 0);
	}

	#[rstest]
	fn test_emit_and_poll() {
		// Arrange
		let bus = EventBus::new();
		let sub_id = bus.subscribe("user.*", "consumer").unwrap();

		// Act - emit events
		let delivered = bus.emit("user.created", vec![1], "producer");
		assert_eq!(delivered, 1);

		let delivered = bus.emit("user.updated", vec![2], "producer");
		assert_eq!(delivered, 1);

		// Non-matching event
		let delivered = bus.emit("order.created", vec![3], "producer");
		assert_eq!(delivered, 0);

		// Assert - poll events
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
		// Arrange
		let bus = EventBus::new();
		let sub1 = bus.subscribe("user.*", "plugin-a").unwrap();
		let sub2 = bus.subscribe("*", "plugin-b").unwrap();
		let sub3 = bus.subscribe("order.*", "plugin-c").unwrap();

		// Act - emit user event (matches sub1 and sub2)
		let delivered = bus.emit("user.created", vec![1], "producer");
		assert_eq!(delivered, 2);

		// Emit order event (matches sub2 and sub3)
		let delivered = bus.emit("order.created", vec![2], "producer");
		assert_eq!(delivered, 2);

		// Assert
		assert_eq!(bus.pending_count(sub1), 1);
		assert_eq!(bus.pending_count(sub2), 2);
		assert_eq!(bus.pending_count(sub3), 1);
	}

	#[rstest]
	fn test_queue_limit() {
		// Arrange
		let bus = EventBus::with_max_queue_size(3);
		let sub_id = bus.subscribe("*", "test").unwrap();

		// Act - emit 5 events
		for i in 0..5 {
			bus.emit(&format!("event.{}", i), vec![i as u8], "producer");
		}

		// Assert - only 3 should remain (oldest dropped)
		let events = bus.poll(sub_id, 10);
		assert_eq!(events.len(), 3);
		assert_eq!(events[0].name, "event.2");
		assert_eq!(events[1].name, "event.3");
		assert_eq!(events[2].name, "event.4");
	}

	#[rstest]
	fn test_remove_plugin_subscriptions() {
		// Arrange
		let bus = EventBus::new();
		bus.subscribe("a.*", "plugin-a").unwrap();
		bus.subscribe("b.*", "plugin-a").unwrap();
		bus.subscribe("c.*", "plugin-b").unwrap();
		assert_eq!(bus.subscription_count(), 3);

		// Act
		let removed = bus.remove_plugin_subscriptions("plugin-a");

		// Assert
		assert_eq!(removed, 2);
		assert_eq!(bus.subscription_count(), 1);
	}

	#[rstest]
	fn test_poll_nonexistent_subscription() {
		// Arrange
		let bus = EventBus::new();

		// Act
		let events = bus.poll(999, 10);

		// Assert
		assert!(events.is_empty());
	}

	#[rstest]
	fn test_poll_limit() {
		// Arrange
		let bus = EventBus::new();
		let sub_id = bus.subscribe("*", "test").unwrap();

		// Emit 10 events
		for i in 0..10 {
			bus.emit(&format!("event.{}", i), vec![], "producer");
		}

		// Act
		let events = bus.poll(sub_id, 3);

		// Assert
		assert_eq!(events.len(), 3);
		assert_eq!(bus.pending_count(sub_id), 7);
	}

	// ==========================================================================
	// Subscription Limit Tests (#685)
	// ==========================================================================

	#[rstest]
	fn test_per_plugin_subscription_limit() {
		// Arrange: allow only 2 subscriptions per plugin
		let bus = EventBus::with_limits(10_000, 2, 10_000);

		// Act
		bus.subscribe("a.*", "greedy-plugin").unwrap();
		bus.subscribe("b.*", "greedy-plugin").unwrap();
		let result = bus.subscribe("c.*", "greedy-plugin");

		// Assert
		assert!(matches!(
			result,
			Err(EventBusError::PerPluginLimitExceeded { limit: 2, .. })
		));
		assert_eq!(bus.subscription_count(), 2);
	}

	#[rstest]
	fn test_per_plugin_limit_is_per_plugin() {
		// Arrange: allow 2 subscriptions per plugin
		let bus = EventBus::with_limits(10_000, 2, 10_000);

		// Act - plugin-a can subscribe up to its limit
		bus.subscribe("a.*", "plugin-a").unwrap();
		bus.subscribe("b.*", "plugin-a").unwrap();

		// plugin-b has its own separate limit
		let result = bus.subscribe("c.*", "plugin-b");

		// Assert
		assert!(result.is_ok());
		assert_eq!(bus.subscription_count(), 3);
	}

	#[rstest]
	fn test_global_subscription_limit() {
		// Arrange: allow only 3 total subscriptions
		let bus = EventBus::with_limits(10_000, 100, 3);

		// Act
		bus.subscribe("a.*", "plugin-a").unwrap();
		bus.subscribe("b.*", "plugin-b").unwrap();
		bus.subscribe("c.*", "plugin-c").unwrap();
		let result = bus.subscribe("d.*", "plugin-d");

		// Assert
		assert!(matches!(
			result,
			Err(EventBusError::TotalLimitExceeded { limit: 3 })
		));
		assert_eq!(bus.subscription_count(), 3);
	}

	#[rstest]
	fn test_unsubscribe_frees_slot_for_new_subscription() {
		// Arrange: allow only 2 subscriptions per plugin
		let bus = EventBus::with_limits(10_000, 2, 10_000);
		let id1 = bus.subscribe("a.*", "plugin-a").unwrap();
		bus.subscribe("b.*", "plugin-a").unwrap();

		// Act: unsubscribe to free a slot
		bus.unsubscribe(id1);
		let result = bus.subscribe("c.*", "plugin-a");

		// Assert
		assert!(result.is_ok());
		assert_eq!(bus.subscription_count(), 2);
	}

	// ==========================================================================
	// Byte Limit Tests (GHSA-mwj5-gv8q-62wg)
	// ==========================================================================

	/// Bytes charged for a three-byte event name emitted by `p` with `payload_len` bytes.
	const fn charge(payload_len: usize) -> usize {
		"e.0".len() + payload_len + "p".len()
	}

	fn byte_limits(
		max_queue_bytes: usize,
		max_owner_queue_bytes: usize,
		max_total_queue_bytes: usize,
	) -> EventByteLimits {
		EventByteLimits {
			max_queue_bytes,
			max_owner_queue_bytes,
			max_total_queue_bytes,
			..EventByteLimits::default()
		}
	}

	#[rstest]
	fn test_try_emit_rejects_payload_over_limit() {
		// Arrange
		let limits = EventByteLimits {
			max_payload_bytes: 8,
			..EventByteLimits::default()
		};
		let bus = EventBus::new().with_byte_limits(limits);
		let sub_id = bus.subscribe("*", "consumer").unwrap();

		// Act
		let result = bus.try_emit("e.0", vec![0; 9], "producer");

		// Assert
		assert_eq!(
			result,
			Err(EventEmitError::PayloadTooLarge { size: 9, limit: 8 })
		);
		assert_eq!(bus.pending_count(sub_id), 0);
		assert_eq!(bus.total_pending_bytes(), 0);
	}

	#[rstest]
	fn test_try_emit_rejects_name_over_limit() {
		// Arrange
		let limits = EventByteLimits {
			max_name_bytes: 4,
			..EventByteLimits::default()
		};
		let bus = EventBus::new().with_byte_limits(limits);
		let sub_id = bus.subscribe("*", "consumer").unwrap();

		// Act
		let result = bus.try_emit("event", vec![], "producer");

		// Assert
		assert_eq!(
			result,
			Err(EventEmitError::NameTooLarge { size: 5, limit: 4 })
		);
		assert_eq!(bus.pending_count(sub_id), 0);
	}

	#[rstest]
	fn test_emit_returns_zero_for_rejected_event() {
		// Arrange
		let limits = EventByteLimits {
			max_payload_bytes: 8,
			..EventByteLimits::default()
		};
		let bus = EventBus::new().with_byte_limits(limits);
		let sub_id = bus.subscribe("*", "consumer").unwrap();

		// Act
		let delivered = bus.emit("e.0", vec![0; 9], "producer");

		// Assert
		assert_eq!(delivered, 0);
		assert_eq!(bus.pending_count(sub_id), 0);
	}

	#[rstest]
	fn test_default_limits_reject_payload_just_over_default() {
		// Arrange
		let bus = EventBus::new();
		bus.subscribe("*", "consumer").unwrap();

		// Act
		let accepted = bus.try_emit("e.0", vec![0; DEFAULT_MAX_EVENT_PAYLOAD_BYTES], "p");
		let rejected = bus.try_emit("e.1", vec![0; DEFAULT_MAX_EVENT_PAYLOAD_BYTES + 1], "p");

		// Assert
		assert_eq!(accepted, Ok(1));
		assert_eq!(
			rejected,
			Err(EventEmitError::PayloadTooLarge {
				size: DEFAULT_MAX_EVENT_PAYLOAD_BYTES + 1,
				limit: DEFAULT_MAX_EVENT_PAYLOAD_BYTES,
			})
		);
	}

	#[rstest]
	fn test_payload_at_limit_reaches_every_matching_subscriber() {
		// Arrange
		let limits = EventByteLimits {
			max_payload_bytes: 8,
			..EventByteLimits::default()
		};
		let bus = EventBus::new().with_byte_limits(limits);
		let sub1 = bus.subscribe("e.*", "plugin-a").unwrap();
		let sub2 = bus.subscribe("*", "plugin-b").unwrap();

		// Act
		let result = bus.try_emit("e.0", vec![7; 8], "p");

		// Assert
		assert_eq!(result, Ok(2));
		for sub_id in [sub1, sub2] {
			let events = bus.poll(sub_id, 10);
			assert_eq!(events.len(), 1);
			assert_eq!(events[0].payload, vec![7; 8]);
			assert_eq!(events[0].source, "p");
		}
		assert_eq!(bus.total_pending_bytes(), 0);
	}

	#[rstest]
	fn test_fanout_shares_one_event_allocation() {
		// Arrange
		let bus = EventBus::new();
		let ids: Vec<u64> = (0..3)
			.map(|i| bus.subscribe("*", &format!("plugin-{i}")).unwrap())
			.collect();

		// Act
		let delivered = bus.emit("e.0", vec![1; 1024], "p");

		// Assert
		assert_eq!(delivered, 3);
		let state = bus.state.read();
		let first = &state.subscriptions[&ids[0]].queue[0];
		assert_eq!(Arc::strong_count(first), 3);
		for id in &ids[1..] {
			assert!(Arc::ptr_eq(first, &state.subscriptions[id].queue[0]));
		}
	}

	#[rstest]
	fn test_queue_byte_limit_drops_oldest_events() {
		// Arrange: each subscription can hold two 10-byte payload events
		let limits = byte_limits(charge(10) * 2, usize::MAX, usize::MAX);
		let bus = EventBus::new().with_byte_limits(limits);
		let sub_id = bus.subscribe("*", "consumer").unwrap();

		// Act
		for i in 0..3u8 {
			bus.emit(&format!("e.{i}"), vec![i; 10], "p");
		}

		// Assert
		assert_eq!(bus.pending_bytes(sub_id), charge(10) * 2);
		let names: Vec<String> = bus.poll(sub_id, 10).into_iter().map(|e| e.name).collect();
		assert_eq!(names, vec!["e.1", "e.2"]);
	}

	#[rstest]
	fn test_owner_byte_limit_spans_subscriptions() {
		// Arrange: the owner can hold two events across both subscriptions
		let limits = byte_limits(usize::MAX, charge(10) * 2, usize::MAX);
		let bus = EventBus::new().with_byte_limits(limits);
		let sub_a = bus.subscribe("a.*", "hoarder").unwrap();
		let sub_b = bus.subscribe("b.*", "hoarder").unwrap();
		bus.emit("a.0", vec![0; 10], "p");
		bus.emit("a.1", vec![1; 10], "p");

		// Act: the owner budget is held by sub_a, so sub_b cannot make room
		let skipped = bus.emit("b.0", vec![2; 10], "p");
		// sub_a can make room by dropping its own oldest event
		let rotated = bus.emit("a.2", vec![3; 10], "p");

		// Assert
		assert_eq!(skipped, 0);
		assert_eq!(rotated, 1);
		assert_eq!(bus.pending_count(sub_b), 0);
		let names: Vec<String> = bus.poll(sub_a, 10).into_iter().map(|e| e.name).collect();
		assert_eq!(names, vec!["a.1", "a.2"]);
	}

	#[rstest]
	fn test_total_byte_limit_never_evicts_other_owners() {
		// Arrange: the bus can hold two events in total
		let limits = byte_limits(usize::MAX, usize::MAX, charge(10) * 2);
		let bus = EventBus::new().with_byte_limits(limits);
		let victim = bus.subscribe("v.*", "victim").unwrap();
		let hoarder = bus.subscribe("*", "hoarder").unwrap();
		bus.emit("v.0", vec![0; 10], "p");

		// Act: the bus is full; the hoarder may only drop its own events
		let delivered = bus.emit("h.0", vec![1; 10], "p");

		// Assert
		assert_eq!(delivered, 1);
		assert_eq!(bus.total_pending_bytes(), charge(10) * 2);
		let victim_events = bus.poll(victim, 10);
		assert_eq!(victim_events.len(), 1);
		assert_eq!(victim_events[0].name, "v.0");
		let names: Vec<String> = bus.poll(hoarder, 10).into_iter().map(|e| e.name).collect();
		assert_eq!(names, vec!["h.0"]);
	}

	#[rstest]
	fn test_event_larger_than_queue_budget_keeps_existing_events() {
		// Arrange
		let limits = byte_limits(charge(10), usize::MAX, usize::MAX);
		let bus = EventBus::new().with_byte_limits(limits);
		let sub_id = bus.subscribe("*", "consumer").unwrap();
		bus.emit("e.0", vec![0; 10], "p");

		// Act
		let delivered = bus.emit("e.1", vec![1; 11], "p");

		// Assert
		assert_eq!(delivered, 0);
		let events = bus.poll(sub_id, 10);
		assert_eq!(events.len(), 1);
		assert_eq!(events[0].name, "e.0");
	}

	#[rstest]
	fn test_poll_releases_byte_budget() {
		// Arrange
		let limits = byte_limits(usize::MAX, usize::MAX, charge(10));
		let bus = EventBus::new().with_byte_limits(limits);
		let sub_a = bus.subscribe("a.*", "plugin-a").unwrap();
		let sub_b = bus.subscribe("b.*", "plugin-b").unwrap();
		bus.emit("a.0", vec![0; 10], "p");
		let skipped_while_full = bus.emit("b.0", vec![1; 10], "p");

		// Act
		let polled = bus.poll(sub_a, 10);
		let delivered = bus.emit("b.1", vec![2; 10], "p");

		// Assert
		assert_eq!(skipped_while_full, 0);
		assert_eq!(polled.len(), 1);
		assert_eq!(delivered, 1);
		assert_eq!(bus.pending_bytes(sub_a), 0);
		assert_eq!(bus.pending_bytes(sub_b), charge(10));
		assert_eq!(bus.total_pending_bytes(), charge(10));
	}

	#[rstest]
	#[case::unsubscribe(true)]
	#[case::remove_plugin_subscriptions(false)]
	fn test_subscription_removal_releases_byte_budget(#[case] unsubscribe: bool) {
		// Arrange
		let limits = byte_limits(usize::MAX, charge(10), charge(10));
		let bus = EventBus::new().with_byte_limits(limits);
		let first = bus.subscribe("*", "plugin-a").unwrap();
		bus.emit("e.0", vec![0; 10], "p");
		assert_eq!(bus.total_pending_bytes(), charge(10));

		// Act
		if unsubscribe {
			bus.unsubscribe(first);
		} else {
			bus.remove_plugin_subscriptions("plugin-a");
		}
		let second = bus.subscribe("*", "plugin-a").unwrap();
		let delivered = bus.emit("e.1", vec![1; 10], "p");

		// Assert
		assert_eq!(delivered, 1);
		assert_eq!(bus.pending_bytes(second), charge(10));
		assert_eq!(bus.total_pending_bytes(), charge(10));
	}

	#[rstest]
	fn test_zero_queue_size_skips_delivery() {
		// Arrange
		let bus = EventBus::with_max_queue_size(0);
		let sub_id = bus.subscribe("*", "consumer").unwrap();

		// Act
		let delivered = bus.emit("e.0", vec![0], "p");

		// Assert
		assert_eq!(delivered, 0);
		assert_eq!(bus.pending_count(sub_id), 0);
	}
}
