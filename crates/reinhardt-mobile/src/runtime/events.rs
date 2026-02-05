//! Event handling for mobile applications.
//!
//! Provides event types and dispatching for mobile WebView events.

use std::collections::HashMap;
use std::sync::Arc;

use crate::MobileEventType;

/// Mobile event data.
#[derive(Debug, Clone)]
pub struct MobileEvent {
	/// Event type
	pub event_type: MobileEventType,

	/// Event target (element ID or name)
	pub target: Option<String>,

	/// Event data payload
	pub data: Option<serde_json::Value>,

	/// Timestamp
	pub timestamp: u64,
}

impl MobileEvent {
	/// Creates a new mobile event.
	pub fn new(event_type: MobileEventType) -> Self {
		Self {
			event_type,
			target: None,
			data: None,
			timestamp: std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.map(|d| d.as_millis() as u64)
				.unwrap_or(0),
		}
	}

	/// Sets the target.
	pub fn with_target(mut self, target: impl Into<String>) -> Self {
		self.target = Some(target.into());
		self
	}

	/// Sets the data.
	pub fn with_data(mut self, data: serde_json::Value) -> Self {
		self.data = Some(data);
		self
	}
}

/// Event handler function type.
pub(super) type EventHandler = Arc<dyn Fn(&MobileEvent) + Send + Sync>;

/// Event dispatcher for mobile events.
pub struct EventDispatcher {
	handlers: HashMap<MobileEventType, Vec<EventHandler>>,
}

impl EventDispatcher {
	/// Creates a new event dispatcher.
	pub fn new() -> Self {
		Self {
			handlers: HashMap::new(),
		}
	}

	/// Registers an event handler.
	pub fn on<F>(&mut self, event_type: MobileEventType, handler: F)
	where
		F: Fn(&MobileEvent) + Send + Sync + 'static,
	{
		self.handlers
			.entry(event_type)
			.or_default()
			.push(Arc::new(handler));
	}

	/// Dispatches an event to all registered handlers.
	pub fn dispatch(&self, event: &MobileEvent) {
		if let Some(handlers) = self.handlers.get(&event.event_type) {
			for handler in handlers {
				handler(event);
			}
		}
	}

	/// Removes all handlers for an event type.
	pub fn off(&mut self, event_type: MobileEventType) {
		self.handlers.remove(&event_type);
	}

	/// Removes all handlers.
	pub fn clear(&mut self) {
		self.handlers.clear();
	}
}

impl Default for EventDispatcher {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::atomic::{AtomicBool, Ordering};

	#[test]
	fn test_event_dispatch() {
		let mut dispatcher = EventDispatcher::new();
		let called = Arc::new(AtomicBool::new(false));
		let called_clone = called.clone();

		dispatcher.on(MobileEventType::WebViewReady, move |_| {
			called_clone.store(true, Ordering::SeqCst);
		});

		let event = MobileEvent::new(MobileEventType::WebViewReady);
		dispatcher.dispatch(&event);

		assert!(called.load(Ordering::SeqCst));
	}
}
