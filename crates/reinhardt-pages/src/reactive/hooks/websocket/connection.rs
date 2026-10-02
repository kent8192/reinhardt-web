//! Browser-owned socket, callbacks, and cancellable reconnection timer.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gloo_timers::callback::Timeout;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

use super::{
	ConnectionState, EventHub, Signal, UseWebSocketOptions, WebSocketEventError, WebSocketMessage,
	invoke_in_owner_scope,
};
use reinhardt_core::reactive::ScopeId;

/// Detach callbacks before releasing their Rust closures or closing the socket.
struct ActiveSocket {
	socket: WebSocket,
	_onopen: Closure<dyn FnMut(JsValue)>,
	_onmessage: Closure<dyn FnMut(MessageEvent)>,
	_onclose: Closure<dyn FnMut(CloseEvent)>,
	_onerror: Closure<dyn FnMut(ErrorEvent)>,
}

impl Drop for ActiveSocket {
	fn drop(&mut self) {
		self.socket.set_onopen(None);
		self.socket.set_onmessage(None);
		self.socket.set_onclose(None);
		self.socket.set_onerror(None);
		if matches!(
			self.socket.ready_state(),
			WebSocket::CONNECTING | WebSocket::OPEN
		) {
			let _ = self.socket.close();
		}
	}
}

/// Only public handles own this state; callbacks and timers hold weak references.
pub(super) struct Connection {
	url: String,
	options: UseWebSocketOptions,
	owner_scope: ScopeId,
	event_hub: Rc<EventHub>,
	connection_state: Signal<ConnectionState>,
	latest_message: Signal<Option<WebSocketMessage>>,
	socket: RefCell<Option<ActiveSocket>>,
	retry_timer: RefCell<Option<Timeout>>,
	retries: Cell<usize>,
	generation: Cell<u64>,
	stopped: Cell<bool>,
}

impl Connection {
	pub(super) fn new(
		url: &str,
		options: UseWebSocketOptions,
		owner_scope: ScopeId,
		connection_state: Signal<ConnectionState>,
		latest_message: Signal<Option<WebSocketMessage>>,
		event_hub: Rc<EventHub>,
	) -> Rc<Self> {
		Rc::new(Self {
			url: url.to_owned(),
			options,
			owner_scope,
			event_hub,
			connection_state,
			latest_message,
			socket: RefCell::new(None),
			retry_timer: RefCell::new(None),
			retries: Cell::new(0),
			generation: Cell::new(0),
			stopped: Cell::new(false),
		})
	}

	pub(super) fn start(self: &Rc<Self>) {
		self.connect();
	}

	fn is_current(&self, generation: u64) -> bool {
		self.generation.get() == generation && self.socket.borrow().is_some()
	}

	fn accepts_events(&self, generation: u64) -> bool {
		!self.stopped.get() && self.is_current(generation)
	}

	fn connect(self: &Rc<Self>) {
		if self.stopped.get() {
			return;
		}
		self.generation.set(self.generation.get().wrapping_add(1));
		let generation = self.generation.get();
		let _ = self.connection_state.try_set(ConnectionState::Connecting);
		// A reactive observer may call close() while processing the state change.
		if self.stopped.get() {
			return;
		}
		let socket = match WebSocket::new(&self.url) {
			Ok(socket) => socket,
			Err(error) => {
				self.report_error(format!("Failed to create WebSocket: {error:?}"));
				self.schedule_retry();
				return;
			}
		};
		socket.set_binary_type(web_sys::BinaryType::Arraybuffer);

		let weak = Rc::downgrade(self);
		let onopen = Closure::wrap(Box::new(move |_: JsValue| {
			let Some(connection) = weak.upgrade() else {
				return;
			};
			if !connection.accepts_events(generation) {
				return;
			}
			connection.retries.set(0);
			let _ = connection.connection_state.try_set(ConnectionState::Open);
			if connection.accepts_events(generation)
				&& let Some(callback) = &connection.options.on_open
			{
				let _ = invoke_in_owner_scope(connection.owner_scope, || callback());
			}
		}) as Box<dyn FnMut(JsValue)>);
		socket.set_onopen(Some(onopen.as_ref().unchecked_ref()));

		let weak = Rc::downgrade(self);
		let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
			let Some(connection) = weak.upgrade() else {
				return;
			};
			if !connection.accepts_events(generation) {
				return;
			}
			let data = event.data();
			let message = if let Some(text) = data.as_string() {
				Some(WebSocketMessage::Text(text))
			} else if let Ok(buffer) = data.dyn_into::<js_sys::ArrayBuffer>() {
				Some(WebSocketMessage::Binary(
					js_sys::Uint8Array::new(&buffer).to_vec(),
				))
			} else {
				None
			};
			if let Some(message) = message {
				connection.event_hub.dispatch(&message);
				if connection.accepts_events(generation) {
					let _ = connection.latest_message.try_set(Some(message));
				}
			}
		}) as Box<dyn FnMut(MessageEvent)>);
		socket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

		let weak = Rc::downgrade(self);
		let onclose = Closure::wrap(Box::new(move |_: CloseEvent| {
			let Some(connection) = weak.upgrade() else {
				return;
			};
			if !connection.accepts_events(generation) {
				return;
			}
			let socket = connection.socket.borrow_mut().take();
			drop(socket);
			let _ = connection.connection_state.try_set(ConnectionState::Closed);
			if !connection.stopped.get()
				&& let Some(callback) = &connection.options.on_close
			{
				let _ = invoke_in_owner_scope(connection.owner_scope, || callback());
			}
			// No RefCell borrow crosses the callback, which may stop this connection.
			connection.schedule_retry();
		}) as Box<dyn FnMut(CloseEvent)>);
		socket.set_onclose(Some(onclose.as_ref().unchecked_ref()));

		let weak = Rc::downgrade(self);
		let onerror = Closure::wrap(Box::new(move |_: ErrorEvent| {
			let Some(connection) = weak.upgrade() else {
				return;
			};
			if connection.accepts_events(generation) {
				connection.report_error("WebSocket error occurred".to_owned());
			}
			// The subsequent close event owns retry scheduling, avoiding two timers.
		}) as Box<dyn FnMut(ErrorEvent)>);
		socket.set_onerror(Some(onerror.as_ref().unchecked_ref()));

		*self.socket.borrow_mut() = Some(ActiveSocket {
			socket,
			_onopen: onopen,
			_onmessage: onmessage,
			_onclose: onclose,
			_onerror: onerror,
		});
	}

	fn report_error(&self, message: String) {
		let _ = self
			.connection_state
			.try_set(ConnectionState::Error(message.clone()));
		if !self.stopped.get() {
			self.event_hub
				.dispatch_error(WebSocketEventError::Transport);
		}
		if !self.stopped.get()
			&& let Some(callback) = &self.options.on_error
		{
			let _ = invoke_in_owner_scope(self.owner_scope, || callback(message));
		}
	}

	fn schedule_retry(self: &Rc<Self>) {
		if self.stopped.get()
			|| !self.options.auto_reconnect
			|| self.retries.get() >= self.options.max_reconnect_attempts
			|| self.retry_timer.borrow().is_some()
		{
			return;
		}
		self.retries.set(self.retries.get() + 1);
		let generation = self.generation.get();
		let weak = Rc::downgrade(self);
		// Browser timeout delays are signed 32-bit values; avoid overflow to zero.
		let delay = self.options.reconnect_delay.min(i32::MAX as u32);
		let timer = Timeout::new(delay, move || {
			let Some(connection) = weak.upgrade() else {
				return;
			};
			let timer = connection.retry_timer.borrow_mut().take();
			if !connection.stopped.get() && connection.generation.get() == generation {
				connection.connect();
			}
			drop(timer);
		});
		*self.retry_timer.borrow_mut() = Some(timer);
	}

	pub(super) fn send(&self, message: WebSocketMessage) -> Result<(), String> {
		if self.stopped.get() {
			return Err("WebSocket has been closed".to_owned());
		}
		let socket = self
			.socket
			.borrow()
			.as_ref()
			.map(|active| active.socket.clone());
		let socket = socket.ok_or("WebSocket not connected")?;
		if socket.ready_state() != WebSocket::OPEN {
			return Err("WebSocket is not open".to_owned());
		}
		match message {
			WebSocketMessage::Text(text) => socket
				.send_with_str(&text)
				.map_err(|error| format!("Failed to send text: {error:?}")),
			WebSocketMessage::Binary(data) => socket
				.send_with_u8_array(&data)
				.map_err(|error| format!("Failed to send binary: {error:?}")),
		}
	}

	pub(super) fn close(&self) {
		if self.stopped.replace(true) {
			return;
		}
		self.generation.set(self.generation.get().wrapping_add(1));
		let timer = self.retry_timer.borrow_mut().take();
		drop(timer);
		drop(self.socket.borrow_mut().take());
		let _ = self.connection_state.try_set(ConnectionState::Closed);
	}
}

impl Drop for Connection {
	fn drop(&mut self) {
		self.stopped.set(true);
		drop(self.retry_timer.get_mut().take());
		drop(self.socket.get_mut().take());
	}
}
