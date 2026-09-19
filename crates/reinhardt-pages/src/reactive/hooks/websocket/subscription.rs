use super::WebSocketMessage;
#[cfg(test)]
use serde::de::DeserializeOwned;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::rc::{Rc, Weak};

/// Errors reported while decoding or delivering a typed WebSocket event.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WebSocketEventError {
	/// The frame was not valid for the requested decoder.
	Decode,
	/// The frame kind is not supported by the requested decoder.
	UnsupportedFrame,
	/// The frame exceeded the subscription's byte limit.
	FrameTooLarge,
	/// The transport reported an error without a safe payload to expose.
	Transport,
}

/// Limits applied before a typed subscription decodes a frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WebSocketSubscriptionOptions {
	max_frame_bytes: NonZeroUsize,
}

impl WebSocketSubscriptionOptions {
	/// Creates subscription options with a maximum UTF-8 frame size.
	pub const fn new(max_frame_bytes: NonZeroUsize) -> Self {
		Self { max_frame_bytes }
	}

	fn accepts(&self, frame: &WebSocketMessage) -> bool {
		let size = match frame {
			WebSocketMessage::Text(text) => text.len(),
			WebSocketMessage::Binary(bytes) => bytes.len(),
		};
		size <= self.max_frame_bytes.get()
	}
}

/// Ordered, synchronous subscribers for one WebSocket handle.
pub(super) struct EventHub {
	next_id: Cell<u64>,
	entries: RefCell<BTreeMap<u64, Weak<SubscriptionEntry>>>,
}

struct SubscriptionEntry {
	live: Cell<bool>,
	// Browser callbacks consume these fields; native transport deliberately stays inert.
	#[cfg_attr(not(any(wasm, test)), allow(dead_code))]
	callback: Rc<dyn Fn(&WebSocketMessage)>,
	// Browser callbacks consume these fields; native transport deliberately stays inert.
	#[cfg_attr(not(any(wasm, test)), allow(dead_code))]
	error_callback: Option<Rc<dyn Fn(WebSocketEventError)>>,
}

/// RAII guard for one WebSocket event subscription.
///
/// Dropping the guard revokes delivery. The guard is intentionally not cloneable
/// so that ownership of cleanup remains explicit.
#[must_use = "retain the subscription guard while events should be delivered"]
pub struct WebSocketSubscription {
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

	// Raw subscription is a deterministic unit-test primitive; applications use the handle API.
	#[cfg(test)]
	pub(super) fn subscribe(
		self: &Rc<Self>,
		callback: Rc<dyn Fn(&WebSocketMessage)>,
	) -> WebSocketSubscription {
		self.subscribe_callbacks(callback, None)
	}

	fn subscribe_callbacks(
		self: &Rc<Self>,
		callback: Rc<dyn Fn(&WebSocketMessage)>,
		error_callback: Option<Rc<dyn Fn(WebSocketEventError)>>,
	) -> WebSocketSubscription {
		let id = self.next_id.get().wrapping_add(1);
		self.next_id.set(id);
		let entry = Rc::new(SubscriptionEntry {
			live: Cell::new(true),
			callback,
			error_callback,
		});
		self.entries.borrow_mut().insert(id, Rc::downgrade(&entry));
		WebSocketSubscription {
			hub: Rc::downgrade(self),
			id,
			entry: Some(entry),
		}
	}

	pub(super) fn subscribe_typed<T: 'static>(
		self: &Rc<Self>,
		options: WebSocketSubscriptionOptions,
		decode: impl Fn(&WebSocketMessage) -> Result<T, WebSocketEventError> + 'static,
		on_event: impl Fn(T) + 'static,
		on_error: impl Fn(WebSocketEventError) + 'static,
	) -> WebSocketSubscription {
		let decode = Rc::new(decode);
		let on_event = Rc::new(on_event);
		let on_error = Rc::new(on_error);
		let callback_error = Rc::clone(&on_error);
		let callback = Rc::new(move |frame: &WebSocketMessage| {
			if !options.accepts(frame) {
				callback_error(WebSocketEventError::FrameTooLarge);
				return;
			}
			match decode(frame) {
				Ok(value) => on_event(value),
				Err(error) => callback_error(error),
			}
		});
		self.subscribe_callbacks(callback, Some(on_error))
	}

	// Direct JSON hub access is used by tests; applications use the handle method.
	#[cfg(test)]
	pub(super) fn subscribe_json<T: DeserializeOwned + 'static>(
		self: &Rc<Self>,
		options: WebSocketSubscriptionOptions,
		on_event: impl Fn(T) + 'static,
		on_error: impl Fn(WebSocketEventError) + 'static,
	) -> WebSocketSubscription {
		self.subscribe_typed(
			options,
			|frame| match frame {
				WebSocketMessage::Text(text) => {
					serde_json::from_str(text).map_err(|_| WebSocketEventError::Decode)
				}
				WebSocketMessage::Binary(_) => Err(WebSocketEventError::UnsupportedFrame),
			},
			on_event,
			on_error,
		)
	}

	// Browser message callbacks and native tests are the only dispatch sources.
	#[cfg_attr(not(any(wasm, test)), allow(dead_code))]
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

	// Browser error callbacks and native tests are the only error dispatch sources.
	#[cfg_attr(not(any(wasm, test)), allow(dead_code))]
	pub(super) fn dispatch_error(&self, error: WebSocketEventError) {
		let snapshot = self
			.entries
			.borrow()
			.values()
			.filter_map(Weak::upgrade)
			.collect::<Vec<_>>();
		for entry in snapshot {
			if entry.live.get()
				&& let Some(callback) = &entry.error_callback
			{
				callback(error);
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
	use serde::Deserialize;
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

	#[derive(Debug, Deserialize, PartialEq)]
	struct Payload {
		value: u32,
	}

	#[test]
	fn json_subscription_reports_one_decode_error_and_continues() {
		let hub = EventHub::new();
		let values = Rc::new(RefCell::new(Vec::new()));
		let errors = Rc::new(RefCell::new(Vec::new()));
		let values_for_callback = Rc::clone(&values);
		let errors_for_callback = Rc::clone(&errors);
		let _guard = hub.subscribe_json(
			WebSocketSubscriptionOptions::new(
				std::num::NonZeroUsize::new(128).expect("non-zero test limit"),
			),
			move |payload: Payload| values_for_callback.borrow_mut().push(payload),
			move |error| errors_for_callback.borrow_mut().push(error),
		);

		hub.dispatch(&WebSocketMessage::Text(r#"{"value":1}"#.into()));
		hub.dispatch(&WebSocketMessage::Text(
			"{\"secret\":\"must-not-escape\"".into(),
		));
		hub.dispatch(&WebSocketMessage::Text(r#"{"value":1}"#.into()));

		assert_eq!(
			*values.borrow(),
			vec![Payload { value: 1 }, Payload { value: 1 }]
		);
		assert_eq!(*errors.borrow(), vec![WebSocketEventError::Decode]);
		assert_eq!(format!("{:?}", errors.borrow()[0]), "Decode");
	}

	#[test]
	fn json_subscription_rejects_binary_frames() {
		let hub = EventHub::new();
		let errors = Rc::new(RefCell::new(Vec::new()));
		let errors_for_callback = Rc::clone(&errors);
		let _guard = hub.subscribe_json::<Payload>(
			WebSocketSubscriptionOptions::new(
				std::num::NonZeroUsize::new(128).expect("non-zero test limit"),
			),
			|_: Payload| {},
			move |error| errors_for_callback.borrow_mut().push(error),
		);

		hub.dispatch(&WebSocketMessage::Binary(vec![1, 2, 3]));

		assert_eq!(*errors.borrow(), vec![WebSocketEventError::UnsupportedFrame]);
	}

	#[test]
	fn typed_subscription_decodes_binary_and_enforces_utf8_bytes() {
		let hub = EventHub::new();
		let values = Rc::new(RefCell::new(Vec::new()));
		let errors = Rc::new(RefCell::new(Vec::new()));
		let values_for_callback = Rc::clone(&values);
		let errors_for_callback = Rc::clone(&errors);
		let _guard = hub.subscribe_typed(
			WebSocketSubscriptionOptions::new(
				std::num::NonZeroUsize::new(1).expect("non-zero test limit"),
			),
			|frame| match frame {
				WebSocketMessage::Binary(bytes) => Ok(bytes[0]),
				WebSocketMessage::Text(_) => Err(WebSocketEventError::Decode),
			},
			move |value| values_for_callback.borrow_mut().push(value),
			move |error| errors_for_callback.borrow_mut().push(error),
		);

		hub.dispatch(&WebSocketMessage::Binary(vec![7]));
		hub.dispatch(&WebSocketMessage::Text("é".into()));
		hub.dispatch(&WebSocketMessage::Text("abc".into()));

		assert_eq!(*values.borrow(), vec![7]);
		assert_eq!(
			*errors.borrow(),
			vec![
				WebSocketEventError::FrameTooLarge,
				WebSocketEventError::FrameTooLarge,
			]
		);
	}
}
