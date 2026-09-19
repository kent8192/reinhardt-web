use super::WebSocketMessage;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

/// Ordered, synchronous subscribers for one WebSocket handle.
pub(super) struct EventHub {
	next_id: Cell<u64>,
	entries: RefCell<BTreeMap<u64, Weak<SubscriptionEntry>>>,
}

struct SubscriptionEntry {
	live: Cell<bool>,
	callback: Rc<dyn Fn(&WebSocketMessage)>,
}

/// RAII guard for one WebSocket event subscription.
///
/// Dropping the guard revokes delivery. The guard is intentionally not cloneable
/// so that ownership of cleanup remains explicit.
#[must_use = "retain the subscription guard while events should be delivered"]
pub(super) struct WebSocketSubscription {
	hub: Weak<EventHub>,
	id: u64,
	entry: Option<Rc<SubscriptionEntry>>,
}

impl EventHub {
	pub(super) fn new() -> Rc<Self> {
		Rc::new(Self {
			next_id: Cell::new(0),
			entries: RefCell::new(BTreeMap::new()),
		})
	}

	pub(super) fn subscribe(
		self: &Rc<Self>,
		callback: Rc<dyn Fn(&WebSocketMessage)>,
	) -> WebSocketSubscription {
		let id = self.next_id.get().wrapping_add(1);
		self.next_id.set(id);
		let entry = Rc::new(SubscriptionEntry {
			live: Cell::new(true),
			callback,
		});
		self.entries.borrow_mut().insert(id, Rc::downgrade(&entry));
		WebSocketSubscription {
			hub: Rc::downgrade(self),
			id,
			entry: Some(entry),
		}
	}

	pub(super) fn dispatch(&self, frame: &WebSocketMessage) {
		let snapshot = self
			.entries
			.borrow()
			.values()
			.filter_map(Weak::upgrade)
			.collect::<Vec<_>>();
		for entry in snapshot {
			if entry.live.get() {
				(entry.callback)(frame);
			}
		}
	}
}

impl Drop for WebSocketSubscription {
	fn drop(&mut self) {
		let Some(entry) = self.entry.take() else {
			return;
		};
		entry.live.set(false);
		if let Some(hub) = self.hub.upgrade() {
			hub.entries.borrow_mut().remove(&self.id);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::reactive::ReactiveScope;
	use std::cell::{Cell, RefCell};
	use std::rc::Rc;

	#[rstest::rstest]
	fn equal_frames_are_individual_events() {
		ReactiveScope::run(|| {
			let hub = EventHub::new();
			let seen = Rc::new(RefCell::new(Vec::new()));
			let output = Rc::clone(&seen);
			let guard = hub.subscribe(Rc::new(move |frame| {
				output.borrow_mut().push(frame.clone());
			}));
			let frame = WebSocketMessage::Text("same".into());
			crate::reactive::batch(|| {
				hub.dispatch(&frame);
				hub.dispatch(&frame);
			});
			assert_eq!(*seen.borrow(), vec![frame.clone(), frame.clone()]);
			drop(guard);
			hub.dispatch(&frame);
			assert_eq!(seen.borrow().len(), 2);
		});
	}

	#[test]
	fn dropping_the_current_guard_stops_later_frames() {
		let hub = EventHub::new();
		let calls = Rc::new(Cell::new(0));
		let calls_for_callback = Rc::clone(&calls);
		let holder = Rc::new(RefCell::new(None));
		let holder_for_callback = Rc::clone(&holder);
		let guard = hub.subscribe(Rc::new(move |_| {
			calls_for_callback.set(calls_for_callback.get() + 1);
			holder_for_callback.borrow_mut().take();
		}));
		*holder.borrow_mut() = Some(guard);

		hub.dispatch(&WebSocketMessage::Text("first".into()));
		hub.dispatch(&WebSocketMessage::Text("second".into()));

		assert_eq!(calls.get(), 1);
	}

	#[test]
	fn dropping_the_next_guard_prevents_its_turn() {
		let hub = EventHub::new();
		let calls = Rc::new(RefCell::new(Vec::new()));
		let next = Rc::new(RefCell::new(None));

		let first_calls = Rc::clone(&calls);
		let next_for_first = Rc::clone(&next);
		let first = hub.subscribe(Rc::new(move |_| {
			first_calls.borrow_mut().push("first");
			next_for_first.borrow_mut().take();
		}));
		let second_calls = Rc::clone(&calls);
		let second = hub.subscribe(Rc::new(move |_| {
			second_calls.borrow_mut().push("second");
		}));
		*next.borrow_mut() = Some(second);

		hub.dispatch(&WebSocketMessage::Text("frame".into()));

		assert_eq!(*calls.borrow(), vec!["first"]);
		drop(first);
	}

	#[test]
	fn subscriber_added_during_dispatch_starts_on_the_next_frame() {
		let hub = EventHub::new();
		let calls = Rc::new(RefCell::new(Vec::new()));
		let added = Rc::new(Cell::new(false));
		let new_guard = Rc::new(RefCell::new(None));

		let first_calls = Rc::clone(&calls);
		let added_for_first = Rc::clone(&added);
		let hub_for_first = Rc::clone(&hub);
		let guard_for_first = Rc::clone(&new_guard);
		let first = hub.subscribe(Rc::new(move |_| {
			first_calls.borrow_mut().push("first");
			if !added_for_first.replace(true) {
				let calls_for_new = Rc::clone(&first_calls);
				*guard_for_first.borrow_mut() = Some(hub_for_first.subscribe(Rc::new(
					move |_| calls_for_new.borrow_mut().push("new"),
				)));
			}
		}));

		hub.dispatch(&WebSocketMessage::Text("first".into()));
		hub.dispatch(&WebSocketMessage::Text("second".into()));

		assert_eq!(*calls.borrow(), vec!["first", "first", "new"]);
		drop(first);
	}

	#[test]
	fn dropping_the_hub_before_the_guard_is_safe() {
		let hub = EventHub::new();
		let weak_hub = Rc::downgrade(&hub);
		let guard = hub.subscribe(Rc::new(|_| {}));
		drop(hub);

		assert!(weak_hub.upgrade().is_none());
		drop(guard);
	}

	#[test]
	fn dropping_a_guard_releases_callback_captures() {
		let hub = EventHub::new();
		let captured = Rc::new(());
		let weak_captured = Rc::downgrade(&captured);
		let captured_for_callback = Rc::clone(&captured);
		let guard = hub.subscribe(Rc::new(move |_| {
			let _ = &captured_for_callback;
		}));
		drop(captured);
		assert!(weak_captured.upgrade().is_some());

		drop(guard);
		assert!(weak_captured.upgrade().is_none());
	}
}
