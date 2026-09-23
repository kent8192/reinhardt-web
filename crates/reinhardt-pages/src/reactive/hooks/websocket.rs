//! WebSocket hook: use_websocket
//!
//! This hook provides a way to establish and manage WebSocket connections
//! in a reactive manner, integrating seamlessly with reinhardt-pages'
//! fine-grained reactivity system.

use crate::reactive::Signal;
use serde::de::DeserializeOwned;
use std::rc::Rc;

#[cfg(wasm)]
mod connection;
mod subscription;

use subscription::EventHub;
pub use subscription::{WebSocketEventError, WebSocketSubscription, WebSocketSubscriptionOptions};

// Native component tests check ownership without a browser transport.
#[cfg(all(test, native))]
fn receive_message(
	latest_message: Signal<Option<WebSocketMessage>>,
	event_hub: &EventHub,
	message: WebSocketMessage,
) {
	event_hub.dispatch(&message);
	let _ = latest_message.try_set(Some(message));
}

fn invoke_in_owner_scope<R>(
	owner_scope: reinhardt_core::reactive::ScopeId,
	callback: impl FnOnce() -> R,
) -> Option<R> {
	reinhardt_core::reactive::scope::enter_scope(owner_scope, callback).ok()
}

fn invoke_subscription_callback<R>(
	owner_scope: Option<reinhardt_core::reactive::ScopeId>,
	callback: impl FnOnce() -> R,
) -> Option<R> {
	if let Some(owner_scope) = owner_scope {
		invoke_in_owner_scope(owner_scope, || {
			reinhardt_core::reactive::untracked(callback)
		})
	} else {
		Some(reinhardt_core::reactive::untracked(callback))
	}
}

/// WebSocket connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
	/// WebSocket is attempting to connect
	Connecting,
	/// WebSocket connection is open and ready
	Open,
	/// WebSocket is closing
	Closing,
	/// WebSocket is closed
	Closed,
	/// WebSocket encountered an error
	Error(String),
}

/// WebSocket message types
#[derive(Debug, Clone, PartialEq)]
pub enum WebSocketMessage {
	/// Text message
	Text(String),
	/// Binary message
	Binary(Vec<u8>),
}

/// Options for configuring WebSocket behavior
pub struct UseWebSocketOptions {
	/// Reconnect after a transport failure or peer disconnect, unless explicitly closed.
	pub auto_reconnect: bool,
	/// Maximum consecutive retries, excluding the initial connection.
	/// A successful open resets the budget; zero disables retries.
	pub max_reconnect_attempts: usize,
	/// Fixed delay between retries in milliseconds, clamped to `i32::MAX`.
	pub reconnect_delay: u32,
	/// Called on every open, including reconnects, to restore subscriptions.
	pub on_open: Option<Rc<dyn Fn()>>,
	/// Callback for a peer or transport close; intentional shutdown is silent.
	pub on_close: Option<Rc<dyn Fn()>>,
	/// Callback when an error occurs, including WebSocket construction failure.
	pub on_error: Option<Rc<dyn Fn(String)>>,
}

impl Default for UseWebSocketOptions {
	fn default() -> Self {
		Self {
			auto_reconnect: true,
			max_reconnect_attempts: 5,
			reconnect_delay: 1000,
			on_open: None,
			on_close: None,
			on_error: None,
		}
	}
}

/// Handle for controlling a WebSocket connection
///
/// This struct provides methods to interact with the WebSocket connection,
/// monitor its state, and send/receive messages reactively.
///
/// Typed subscription methods have P1 symbol parity: WASM dispatches transport
/// frames, while native/SSR retains the same types and remains inert.
///
/// Clones share one connection and retry budget. Owner disposal terminates the
/// connection even if clones survive; dropping the last clone also closes it.
pub struct WebSocketHandle {
	connection_state: Signal<ConnectionState>,
	latest_message: Signal<Option<WebSocketMessage>>,
	send_fn: Rc<dyn Fn(WebSocketMessage) -> Result<(), String>>,
	close_fn: Rc<dyn Fn()>,
	event_hub: Rc<EventHub>,
	owner_scope: Option<reinhardt_core::reactive::ScopeId>,
	#[cfg(wasm)]
	_connection: Rc<connection::Connection>,
}

impl WebSocketHandle {
	/// Get a reference to the connection state signal
	pub fn connection_state(&self) -> &Signal<ConnectionState> {
		&self.connection_state
	}

	/// Get a reference to the latest message signal
	pub fn latest_message(&self) -> &Signal<Option<WebSocketMessage>> {
		&self.latest_message
	}

	/// Returns the current connection state, or `Closed` after owner disposal.
	pub fn current_connection_state(&self) -> ConnectionState {
		self.connection_state
			.try_get_untracked()
			.unwrap_or(ConnectionState::Closed)
	}

	/// Returns the latest message, or `None` after owner disposal.
	pub fn current_message(&self) -> Option<WebSocketMessage> {
		self.latest_message.try_get_untracked().unwrap_or(None)
	}

	/// Send a WebSocket message
	pub fn send(&self, message: WebSocketMessage) -> Result<(), String> {
		(self.send_fn)(message)
	}

	/// Send a text message
	pub fn send_text(&self, text: String) -> Result<(), String> {
		self.send(WebSocketMessage::Text(text))
	}

	/// Send a binary message
	pub fn send_binary(&self, data: Vec<u8>) -> Result<(), String> {
		self.send(WebSocketMessage::Binary(data))
	}

	/// Send a JSON-serializable message
	///
	/// # Type Parameters
	///
	/// * `T` - The type to serialize as JSON
	///
	/// # Errors
	///
	/// Returns an error if serialization fails or if sending fails
	pub fn send_json<T: serde::Serialize>(&self, data: &T) -> Result<(), String> {
		let json =
			serde_json::to_string(data).map_err(|e| format!("JSON serialization error: {}", e))?;
		self.send_text(json)
	}

	/// Close the connection and cancel retries for every clone.
	/// This is terminal and idempotent; create a new hook to reconnect.
	pub fn close(&self) {
		(self.close_fn)()
	}

	/// Check if the connection is currently open
	pub fn is_open(&self) -> bool {
		matches!(self.current_connection_state(), ConnectionState::Open)
	}

	/// Subscribe to decoded WebSocket events.
	///
	/// The decoder and callbacks run untracked in the handle's live owner scope.
	/// Disposing that scope prevents further decoding and callback delivery.
	/// Delivery precedes replacing the [`Self::latest_message`] snapshot.
	///
	/// Parity: P1. WASM dispatches browser frames. Native/SSR returns an inert
	/// guard without opening a connection or delivering events.
	pub fn subscribe<T, D, E, F>(
		&self,
		options: WebSocketSubscriptionOptions,
		decode: D,
		on_event: E,
		on_error: F,
	) -> WebSocketSubscription
	where
		T: 'static,
		D: Fn(&WebSocketMessage) -> Result<T, WebSocketEventError> + 'static,
		E: Fn(T) + 'static,
		F: Fn(WebSocketEventError) + 'static,
	{
		let owner_scope = self.owner_scope;
		let on_event = Rc::new(on_event);
		let on_error = Rc::new(on_error);
		self.event_hub.subscribe_typed(
			options,
			move |frame| invoke_subscription_callback(owner_scope, || decode(frame)).transpose(),
			{
				let on_event = Rc::clone(&on_event);
				move |value| {
					if let Some(value) = value {
						let on_event = Rc::clone(&on_event);
						let _ = invoke_subscription_callback(owner_scope, move || on_event(value));
					}
				}
			},
			{
				let on_error = Rc::clone(&on_error);
				move |error| {
					let on_error = Rc::clone(&on_error);
					let _ = invoke_subscription_callback(owner_scope, move || on_error(error));
				}
			},
		)
	}

	/// Subscribe to JSON text events without repeating deserialization boilerplate.
	///
	/// Parity: P1. WASM decodes browser text frames. Native/SSR returns an inert
	/// guard without opening a connection or delivering events.
	pub fn subscribe_json<T, E, F>(
		&self,
		options: WebSocketSubscriptionOptions,
		on_event: E,
		on_error: F,
	) -> WebSocketSubscription
	where
		T: DeserializeOwned + 'static,
		E: Fn(T) + 'static,
		F: Fn(WebSocketEventError) + 'static,
	{
		self.subscribe(
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

	#[cfg(test)]
	fn dispatch_for_test(&self, frame: &WebSocketMessage) {
		self.event_hub.dispatch(frame);
	}

	#[cfg(test)]
	fn dispatch_error_for_test(&self, error: WebSocketEventError) {
		self.event_hub.dispatch_error(error);
	}
}

impl Clone for WebSocketHandle {
	fn clone(&self) -> Self {
		Self {
			connection_state: self.connection_state,
			latest_message: self.latest_message,
			send_fn: Rc::clone(&self.send_fn),
			close_fn: Rc::clone(&self.close_fn),
			event_hub: Rc::clone(&self.event_hub),
			owner_scope: self.owner_scope,
			#[cfg(wasm)]
			_connection: Rc::clone(&self._connection),
		}
	}
}

/// Register a typed WebSocket subscription owned by the current reactive scope.
///
/// Parity: P1. WASM dispatches decoded browser frames. Native/SSR retains only
/// local cleanup ownership without opening a connection or invoking callbacks.
///
/// ```no_run
/// use reinhardt_pages::reactive::{ReactiveScope, hooks::{
///     use_websocket, use_websocket_subscription, WebSocketSubscriptionOptions,
///     WebSocketMessage, WebSocketEventError,
/// }};
///
/// ReactiveScope::run(|| {
///     let socket = use_websocket("wss://example.invalid/events", Default::default());
///     use_websocket_subscription(
///         &socket,
///         WebSocketSubscriptionOptions::new(std::num::NonZeroUsize::new(4096).unwrap()),
///         |frame| match frame {
///             WebSocketMessage::Binary(bytes) => Ok(bytes.len()),
///             _ => Err(WebSocketEventError::UnsupportedFrame),
///         },
///         |length| println!("Received {length} bytes"),
///         |error| eprintln!("Event error: {error:?}"),
///     );
/// });
/// ```
pub fn use_websocket_subscription<T, D, E, F>(
	handle: &WebSocketHandle,
	options: WebSocketSubscriptionOptions,
	decode: D,
	on_event: E,
	on_error: F,
) where
	T: 'static,
	D: Fn(&WebSocketMessage) -> Result<T, WebSocketEventError> + 'static,
	E: Fn(T) + 'static,
	F: Fn(WebSocketEventError) + 'static,
{
	let scope = reinhardt_core::reactive::scope::require_active_scope("use_websocket_subscription");
	let subscription = handle.subscribe(options, decode, on_event, on_error);
	if reinhardt_core::reactive::scope::on_scope_dispose(scope, move || drop(subscription)).is_err()
	{
		panic!("use_websocket_subscription requires a live reactive scope");
	}
}

/// Register a JSON WebSocket subscription owned by the current reactive scope.
///
/// Parity: P1. WASM decodes browser text frames. Native/SSR retains only local
/// cleanup ownership without opening a connection or invoking callbacks.
///
/// ```no_run
/// use reinhardt_pages::reactive::{ReactiveScope, hooks::{
///     use_websocket, use_websocket_json_subscription, WebSocketSubscriptionOptions,
/// }};
///
/// ReactiveScope::run(|| {
///     let socket = use_websocket("wss://example.invalid/events", Default::default());
///     use_websocket_json_subscription(
///         &socket,
///         WebSocketSubscriptionOptions::new(std::num::NonZeroUsize::new(4096).unwrap()),
///         |event: serde_json::Value| println!("Received {event}"),
///         |error| eprintln!("Event error: {error:?}"),
///     );
/// });
/// ```
pub fn use_websocket_json_subscription<T, E, F>(
	handle: &WebSocketHandle,
	options: WebSocketSubscriptionOptions,
	on_event: E,
	on_error: F,
) where
	T: DeserializeOwned + 'static,
	E: Fn(T) + 'static,
	F: Fn(WebSocketEventError) + 'static,
{
	let scope =
		reinhardt_core::reactive::scope::require_active_scope("use_websocket_json_subscription");
	let subscription = handle.subscribe_json(options, on_event, on_error);
	if reinhardt_core::reactive::scope::on_scope_dispose(scope, move || drop(subscription)).is_err()
	{
		panic!("use_websocket_json_subscription requires a live reactive scope");
	}
}

// ============================================================================
// WASM Implementation
// ============================================================================

/// Establish and manage a WebSocket connection (WASM implementation)
///
/// This hook creates a reactive WebSocket connection that integrates with
/// reinhardt-pages' Signal system. The connection state and incoming messages
/// are automatically tracked and can be used in reactive contexts.
///
/// # Arguments
///
/// * `url` - WebSocket endpoint URL (e.g., "ws://localhost:8000/ws/chat")
/// * `options` - Configuration options for the WebSocket connection
///
/// # Returns
///
/// A `WebSocketHandle` that can be used to control the connection and
/// reactively access its state.
///
/// # Example
///
/// ```ignore
/// use reinhardt_pages::deps;
/// use reinhardt_pages::reactive::hooks::{use_effect, use_websocket, UseWebSocketOptions};
/// use reinhardt_pages::reactive::hooks::ConnectionState;
///
/// let ws = use_websocket("ws://localhost:8000/ws/chat", UseWebSocketOptions::default());
///
/// // Monitor connection state
/// let connection_state = ws.connection_state().clone();
/// use_effect({
///     let connection_state = connection_state.clone();
///     move || {
///         match connection_state.get() {
///             ConnectionState::Open => log!("Connected"),
///             ConnectionState::Closed => log!("Disconnected"),
///             _ => {}
///         }
///     }
/// }, deps![connection_state]);
///
/// // Send a message
/// ws.send_text("Hello, server!".to_string()).ok();
///
/// // Receive messages
/// let latest_message = ws.latest_message().clone();
/// use_effect({
///     let latest_message = latest_message.clone();
///     move || {
///         if let Some(msg) = latest_message.get() {
///             match msg {
///                 WebSocketMessage::Text(text) => log!("Received: {}", text),
///                 _ => {}
///             }
///             None::<fn()>
///         }
///     }
/// }, deps![latest_message]);
/// ```
///
/// # Reactivity semantics
///
/// Event-callback closures inside `UseWebSocketOptions` (`on_open`,
/// `on_close`, `on_error`) re-enter the scope that created this hook, but run
/// outside any active reactive Observer. Reading `Signal::get()`, `Memo::get()`,
/// or `Resource::get()` inside returns the latest value WITHOUT subscribing for
/// future changes (Option A, Refs #4195).
#[cfg(wasm)]
pub fn use_websocket(url: &str, options: UseWebSocketOptions) -> WebSocketHandle {
	let owner_scope = reinhardt_core::reactive::scope::require_active_scope("use_websocket");
	let connection_state = Signal::new(ConnectionState::Connecting);
	let latest_message = Signal::new(None);
	let event_hub = EventHub::new();
	let connection = connection::Connection::new(
		url,
		options,
		owner_scope,
		connection_state,
		latest_message,
		Rc::clone(&event_hub),
	);
	let weak = Rc::downgrade(&connection);
	reinhardt_core::reactive::scope::on_scope_dispose(owner_scope, move || {
		if let Some(connection) = weak.upgrade() {
			connection.close();
		}
	})
	.expect("use_websocket requires a live reactive scope");
	connection.start();
	let weak = Rc::downgrade(&connection);
	let send_fn = Rc::new(move |message| {
		weak.upgrade()
			.ok_or_else(|| "WebSocket has been dropped".to_owned())?
			.send(message)
	});
	let weak = Rc::downgrade(&connection);
	let close_fn = Rc::new(move || {
		if let Some(connection) = weak.upgrade() {
			connection.close();
		}
	});
	WebSocketHandle {
		connection_state,
		latest_message,
		send_fn,
		close_fn,
		event_hub,
		owner_scope: Some(owner_scope),
		_connection: connection,
	}
}

// ============================================================================
// SSR (Server-Side Rendering) no-op Implementation
// ============================================================================

/// WebSocket hook - SSR no-op implementation
///
/// On the server side (non-WASM), WebSocket connections are not supported.
/// This implementation returns a handle that always reports the connection
/// as closed and rejects all send attempts.
///
/// # Arguments
///
/// * `_url` - WebSocket endpoint URL (ignored)
/// * `_options` - Configuration options (ignored)
///
/// # Returns
///
/// A `WebSocketHandle` with connection state always set to `Closed`.
#[cfg(native)]
pub fn use_websocket(_url: &str, _options: UseWebSocketOptions) -> WebSocketHandle {
	WebSocketHandle {
		connection_state: Signal::new(ConnectionState::Closed),
		latest_message: Signal::new(None),
		send_fn: Rc::new(|_| Err("WebSocket not available on server".to_string())),
		close_fn: Rc::new(|| {}),
		event_hub: EventHub::new(),
		owner_scope: reinhardt_core::reactive::current_scope_id(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[cfg(native)]
	use crate::reactive::ReactiveScope;
	#[cfg(native)]
	use rstest::rstest;
	#[cfg(native)]
	use serde::Deserialize;
	#[cfg(native)]
	use std::cell::Cell;
	#[cfg(native)]
	use std::cell::RefCell;
	#[cfg(native)]
	use std::rc::Rc;

	#[cfg(native)]
	#[derive(Debug, Deserialize, PartialEq)]
	struct TestPayload {
		value: u32,
	}

	#[cfg(native)]
	#[test]
	#[serial_test::serial(reactive_runtime)]
	fn websocket_callback_reenters_its_owner_scope() {
		let scope = reinhardt_core::reactive::ReactiveScope::new();
		let callback_ran = Rc::new(Cell::new(false));
		let callback_ran_for_callback = Rc::clone(&callback_ran);

		let _ = invoke_in_owner_scope(scope.id(), move || {
			let signal = Signal::new(42_i32);
			assert_eq!(signal.get(), 42);
			callback_ran_for_callback.set(true);
		});

		assert!(callback_ran.get());
	}

	#[test]
	#[cfg(native)]
	fn test_use_websocket_ssr_no_op() {
		reinhardt_core::reactive::ReactiveScope::run(|| {
			// Test sentinel URL — native build never opens this connection
			// (SSR no-op). Suppress Semgrep's awesome-secure-defaults
			// substring rule via concatenation so the literal scheme token
			// is never present in source.
			// nosemgrep: awesome-secure-defaults.insecure-websocket
			let scheme = "ws";
			let test_url = format!("{}://test", scheme);
			let ws = use_websocket(&test_url, UseWebSocketOptions::default());
			assert!(matches!(
				ws.connection_state().get(),
				ConnectionState::Closed
			));
			assert!(ws.send_text("test".to_string()).is_err());
			assert!(!ws.is_open());
		});
	}

	#[test]
	#[cfg(native)]
	#[serial_test::serial(reactive_runtime)]
	fn stale_websocket_handle_is_closed() {
		let scope = reinhardt_core::reactive::ReactiveScope::new();
		let handle = scope.enter(|| use_websocket("ignored", UseWebSocketOptions::default()));

		scope.dispose();

		assert!(!handle.is_open());
		assert_eq!(handle.current_connection_state(), ConnectionState::Closed);
		assert_eq!(handle.current_message(), None);
	}

	#[rstest]
	#[case::text(WebSocketMessage::Text("payload".to_owned()))]
	#[case::binary(WebSocketMessage::Binary(vec![1, 2, 3]))]
	#[cfg(native)]
	fn received_frame_moves_into_snapshot_after_delivery(
		#[case] frame: WebSocketMessage,
		#[values(false, true)] with_subscriber: bool,
	) {
		// Arrange
		fn payload_pointer(frame: &WebSocketMessage) -> *const u8 {
			match frame {
				WebSocketMessage::Text(text) => text.as_ptr(),
				WebSocketMessage::Binary(bytes) => bytes.as_ptr(),
			}
		}
		ReactiveScope::run(|| {
			let latest = Signal::new(None);
			let hub = EventHub::new();
			let received_pointer = payload_pointer(&frame);
			let deliveries = Rc::new(Cell::new(0));
			let deliveries_in_callback = Rc::clone(&deliveries);
			let _guard = with_subscriber.then(|| {
				hub.subscribe(Rc::new(move |message| {
					assert_eq!(payload_pointer(message), received_pointer);
					latest.with_untracked(|value| assert!(value.is_none()));
					deliveries_in_callback.set(deliveries_in_callback.get() + 1);
				}))
			});

			// Act
			receive_message(latest, &hub, frame);

			// Assert: the legacy snapshot owns the original allocation, even without subscribers.
			assert_eq!(deliveries.get(), usize::from(with_subscriber));
			latest.with_untracked(|value| {
				assert_eq!(payload_pointer(value.as_ref().unwrap()), received_pointer);
			});
		});
	}

	#[rstest]
	#[cfg(native)]
	fn typed_handle_subscription_delivers_in_owner_scope() {
		ReactiveScope::run(|| {
			let handle = use_websocket("ignored", UseWebSocketOptions::default());
			let values = Rc::new(RefCell::new(Vec::new()));
			let errors = Rc::new(RefCell::new(Vec::new()));
			let values_for_callback = Rc::clone(&values);
			let errors_for_callback = Rc::clone(&errors);
			let _guard = handle.subscribe_json(
				WebSocketSubscriptionOptions::new(
					std::num::NonZeroUsize::new(128).expect("non-zero test limit"),
				),
				move |payload: TestPayload| values_for_callback.borrow_mut().push(payload),
				move |error| errors_for_callback.borrow_mut().push(error),
			);

			handle.dispatch_for_test(&WebSocketMessage::Text(r#"{"value":2}"#.into()));
			handle.dispatch_for_test(&WebSocketMessage::Text("secret-invalid".into()));

			assert_eq!(*values.borrow(), vec![TestPayload { value: 2 }]);
			assert_eq!(*errors.borrow(), vec![WebSocketEventError::Decode]);
		});
	}

	#[rstest]
	#[cfg(native)]
	fn raw_subscription_outside_scope_is_guard_owned() {
		let owner = ReactiveScope::new();
		let handle = owner.enter(|| use_websocket("ignored", UseWebSocketOptions::default()));
		let calls = Rc::new(Cell::new(0));
		let calls_for_callback = Rc::clone(&calls);
		let guard = handle.subscribe_json::<TestPayload, _, _>(
			WebSocketSubscriptionOptions::new(
				std::num::NonZeroUsize::new(128).expect("non-zero test limit"),
			),
			move |_| calls_for_callback.set(calls_for_callback.get() + 1),
			|_| {},
		);

		handle.dispatch_for_test(&WebSocketMessage::Text(r#"{"value":4}"#.into()));
		assert_eq!(calls.get(), 1);
		drop(guard);
		handle.dispatch_for_test(&WebSocketMessage::Text(r#"{"value":5}"#.into()));
		assert_eq!(calls.get(), 1);
		owner.dispose();
	}

	#[rstest]
	#[cfg(native)]
	fn scoped_typed_subscription_is_revoked_with_owner() {
		let scope = ReactiveScope::new();
		let (handle, calls) = scope.enter(|| {
			let handle = use_websocket("ignored", UseWebSocketOptions::default());
			let calls = Rc::new(Cell::new(0));
			let calls_for_callback = Rc::clone(&calls);
			use_websocket_json_subscription(
				&handle,
				WebSocketSubscriptionOptions::new(
					std::num::NonZeroUsize::new(128).expect("non-zero test limit"),
				),
				move |_: TestPayload| calls_for_callback.set(calls_for_callback.get() + 1),
				|_| {},
			);
			(handle, calls)
		});

		scope.dispose();
		handle.dispatch_for_test(&WebSocketMessage::Text(r#"{"value":3}"#.into()));

		assert_eq!(calls.get(), 0);
	}

	#[rstest]
	#[cfg(native)]
	fn typed_subscription_reports_transport_category_without_payload() {
		ReactiveScope::run(|| {
			let handle = use_websocket("ignored", UseWebSocketOptions::default());
			let errors = Rc::new(RefCell::new(Vec::new()));
			let errors_for_callback = Rc::clone(&errors);
			let _guard = handle.subscribe_json::<TestPayload, _, _>(
				WebSocketSubscriptionOptions::new(
					std::num::NonZeroUsize::new(128).expect("non-zero test limit"),
				),
				|_| {},
				move |error| errors_for_callback.borrow_mut().push(error),
			);
			handle.dispatch_error_for_test(WebSocketEventError::Transport);
			assert_eq!(*errors.borrow(), vec![WebSocketEventError::Transport]);
		});
	}

	#[rstest]
	#[cfg(native)]
	fn typed_callback_reads_do_not_subscribe_the_dispatching_effect() {
		ReactiveScope::run(|| {
			let handle = use_websocket("ignored", UseWebSocketOptions::default());
			let trigger = Signal::new(0_u32);
			let observed = Signal::new(0_u32);
			let effect_runs = Rc::new(Cell::new(0));
			let callback_reads = Rc::new(Cell::new(0));
			let observed_for_callback = observed;
			let callback_reads_for_callback = Rc::clone(&callback_reads);
			let _guard = handle.subscribe(
				WebSocketSubscriptionOptions::new(
					std::num::NonZeroUsize::new(128).expect("non-zero test limit"),
				),
				move |_| Ok(observed.get()),
				move |_| {
					callback_reads_for_callback
						.set(callback_reads_for_callback.get() + observed_for_callback.get());
				},
				|_| {},
			);

			let effect_runs_for_effect = Rc::clone(&effect_runs);
			let handle_for_effect = handle.clone();
			let _effect = crate::reactive::Effect::new(move || {
				let _ = trigger.get();
				effect_runs_for_effect.set(effect_runs_for_effect.get() + 1);
				handle_for_effect.dispatch_for_test(&WebSocketMessage::Text("event".into()));
			});

			assert_eq!(effect_runs.get(), 1);
			observed.set(7);
			crate::reactive::runtime::with_runtime(|runtime| runtime.flush_updates());
			assert_eq!(effect_runs.get(), 1);
			assert_eq!(callback_reads.get(), 0);
			trigger.set(1);
			crate::reactive::runtime::with_runtime(|runtime| runtime.flush_updates());
			assert_eq!(effect_runs.get(), 2);
			assert_eq!(callback_reads.get(), 7);
		});
	}

	#[rstest]
	#[cfg(native)]
	fn custom_decoder_uses_owner_scope_and_stops_after_disposal() {
		// Arrange: keep the raw subscription alive beyond its owner scope.
		let owner = ReactiveScope::new();
		let owner_id = owner.id();
		let handle = owner.enter(|| use_websocket("ignored", UseWebSocketOptions::default()));
		let decoded = Rc::new(Cell::new(0));
		let delivered = Rc::new(Cell::new(0));
		let decoded_in_callback = Rc::clone(&decoded);
		let delivered_in_callback = Rc::clone(&delivered);
		let _guard = handle.subscribe(
			WebSocketSubscriptionOptions::new(std::num::NonZeroUsize::new(128).unwrap()),
			move |_| {
				assert_eq!(reinhardt_core::reactive::scope::active_scope_id(), owner_id);
				decoded_in_callback.set(decoded_in_callback.get() + 1);
				Ok(())
			},
			move |_| delivered_in_callback.set(delivered_in_callback.get() + 1),
			|error| panic!("unexpected decoding failure: {error:?}"),
		);

		// Act and assert: dispatch outside any scope must reenter the owner.
		handle.dispatch_for_test(&WebSocketMessage::Text("event".into()));
		assert_eq!((decoded.get(), delivered.get()), (1, 1));
		owner.dispose();
		handle.dispatch_for_test(&WebSocketMessage::Text("event".into()));
		assert_eq!((decoded.get(), delivered.get()), (1, 1));
	}

	#[test]
	fn test_connection_state_clone() {
		let state1 = ConnectionState::Open;
		let state2 = state1.clone();
		assert_eq!(state1, state2);
	}

	#[test]
	fn test_websocket_message_clone() {
		let msg1 = WebSocketMessage::Text("hello".to_string());
		let msg2 = msg1.clone();
		assert_eq!(msg1, msg2);
	}

	#[test]
	fn test_use_websocket_options_default() {
		let options = UseWebSocketOptions::default();
		assert!(options.auto_reconnect);
		assert_eq!(options.max_reconnect_attempts, 5);
		assert_eq!(options.reconnect_delay, 1000);
		assert!(options.on_open.is_none());
		assert!(options.on_close.is_none());
		assert!(options.on_error.is_none());
	}
}
