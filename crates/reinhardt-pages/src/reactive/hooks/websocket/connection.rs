//! Browser-owned socket, callbacks, and cancellable reconnection timer.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gloo_timers::callback::Timeout;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{CloseEvent, ErrorEvent, MessageEvent, WebSocket};

use super::{ConnectionState, Signal, UseWebSocketOptions, WebSocketMessage};

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
		connection_state: Signal<ConnectionState>,
		latest_message: Signal<Option<WebSocketMessage>>,
	) -> Rc<Self> {
		let connection = Rc::new(Self {
			url: url.to_owned(),
			options,
			connection_state,
			latest_message,
			socket: RefCell::new(None),
			retry_timer: RefCell::new(None),
			retries: Cell::new(0),
			generation: Cell::new(0),
			stopped: Cell::new(false),
		});
		connection.connect();
		connection
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
		self.connection_state.set(ConnectionState::Connecting);
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
			connection.connection_state.set(ConnectionState::Open);
			if connection.accepts_events(generation)
				&& let Some(callback) = &connection.options.on_open
			{
				callback();
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
			if let Some(text) = data.as_string() {
				connection
					.latest_message
					.set(Some(WebSocketMessage::Text(text)));
			} else if let Ok(buffer) = data.dyn_into::<js_sys::ArrayBuffer>() {
				let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
				connection
					.latest_message
					.set(Some(WebSocketMessage::Binary(bytes)));
			}
		}) as Box<dyn FnMut(MessageEvent)>);
		socket.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

		let weak = Rc::downgrade(self);
		let onclose = Closure::wrap(Box::new(move |_: CloseEvent| {
			let Some(connection) = weak.upgrade() else {
				return;
			};
			// A manually closed socket still delivers its final close notification.
			if !connection.is_current(generation) {
				return;
			}
			let socket = connection.socket.borrow_mut().take();
			drop(socket);
			connection.connection_state.set(ConnectionState::Closed);
			if let Some(callback) = &connection.options.on_close {
				callback();
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
		self.connection_state
			.set(ConnectionState::Error(message.clone()));
		if !self.stopped.get()
			&& let Some(callback) = &self.options.on_error
		{
			callback(message);
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
		let timer = self.retry_timer.borrow_mut().take();
		drop(timer);
		let socket = self
			.socket
			.borrow()
			.as_ref()
			.map(|active| active.socket.clone());
		let Some(socket) = socket else {
			self.connection_state.set(ConnectionState::Closed);
			return;
		};
		match socket.ready_state() {
			WebSocket::CONNECTING | WebSocket::OPEN => {
				self.connection_state.set(ConnectionState::Closing);
				if let Err(error) = socket.close() {
					let active = self.socket.borrow_mut().take();
					drop(active);
					self.connection_state.set(ConnectionState::Error(format!(
						"Failed to close WebSocket: {error:?}"
					)));
				}
			}
			WebSocket::CLOSING => self.connection_state.set(ConnectionState::Closing),
			_ => self.connection_state.set(ConnectionState::Closed),
		}
	}
}

impl Drop for Connection {
	fn drop(&mut self) {
		self.stopped.set(true);
		drop(self.retry_timer.get_mut().take());
		drop(self.socket.get_mut().take());
	}
}
