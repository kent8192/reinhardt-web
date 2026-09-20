//! Browser regressions for the public WebSocket hook's retry and ownership contract.
//!
//! The browser boundary is controlled synchronously: these tests exercise the real
//! Rust hook without a network server, wall-clock sleeps, or production test hooks.
//!
//! Run from the workspace root:
//! `wasm-pack test --headless --chrome crates/reinhardt-pages -- --test websocket_reconnect_wasm_test`

#![cfg(wasm)]

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use reinhardt_pages::reactive::hooks::{
	ConnectionState, UseWebSocketOptions, WebSocketHandle, WebSocketMessage, use_websocket,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen(inline_js = r#"
let originals;
let sockets = [];
let timers = new Map();
let nextTimer = 1;
let attempts = 0;
let failures = 0;

export function restore_browser() {
    if (!originals) return;
    globalThis.WebSocket = originals.WebSocket;
    globalThis.setTimeout = originals.setTimeout;
    globalThis.clearTimeout = originals.clearTimeout;
    originals = undefined;
    timers.clear();
    sockets = [];
}

export function install_browser() {
    // Also restore after a preceding WASM test traps before its Rust guard drops.
    restore_browser();
    originals = {
        WebSocket: globalThis.WebSocket,
        setTimeout: globalThis.setTimeout,
        clearTimeout: globalThis.clearTimeout,
    };
    sockets = [];
    timers = new Map();
    nextTimer = 1;
    attempts = 0;
    failures = 0;
    globalThis.WebSocket = class {
        static CONNECTING = 0;
        static OPEN = 1;
        static CLOSING = 2;
        static CLOSED = 3;
        constructor(url) {
            attempts++;
            if (failures > 0) {
                failures--;
                throw new Error("controlled constructor failure");
            }
            this.url = url;
            this.readyState = 0;
            this.binaryType = "blob";
            this.onopen = null;
            this.onmessage = null;
            this.onclose = null;
            this.onerror = null;
            this.closeCalls = 0;
            this.sent = [];
            sockets.push(this);
        }
        send(value) {
            if (this.readyState !== 1) throw new Error("socket is not open");
            this.sent.push(typeof value === "string" ? value : new Uint8Array(value).slice());
        }
        close() {
            this.closeCalls++;
            if (this.readyState !== 3) this.readyState = 2;
        }
    };
    globalThis.setTimeout = (callback, delay) => {
        const id = nextTimer++;
        timers.set(id, { callback, delay });
        return id;
    };
    globalThis.clearTimeout = id => { timers.delete(id); };
}

export function constructor_attempts() { return attempts; }
export function fail_constructors(count) { failures = count; }
export function timer_count() { return timers.size; }
export function next_delay() {
    const first = timers.values().next();
    if (first.done) throw new Error("no pending timer");
    return first.value.delay;
}
export function fire_timer() {
    const first = timers.entries().next();
    if (first.done) return false;
    const [id, timer] = first.value;
    timers.delete(id);
    timer.callback();
    return true;
}
export function open_socket(index) {
    const socket = sockets[index];
    socket.readyState = 1;
    socket.onopen?.(new Event("open"));
}
export function disconnect_socket(index) {
    const socket = sockets[index];
    socket.readyState = 3;
    socket.onclose?.(new CloseEvent("close"));
}
export function error_socket(index) {
    sockets[index].onerror?.(new ErrorEvent("error"));
}
export function receive_text(index, text) {
    sockets[index].onmessage?.(new MessageEvent("message", { data: text }));
}
export function receive_binary(index) {
    sockets[index].onmessage?.(new MessageEvent("message", {
        data: new Uint8Array([7, 8, 9]).buffer,
    }));
}
export function sent_binary(index, message) { return sockets[index].sent[message]; }
export function sent_text(index, message) { return sockets[index].sent[message]; }
export function close_calls(index) { return sockets[index].closeCalls; }
export function handlers_detached(index) {
    const socket = sockets[index];
    return socket.onopen === null && socket.onmessage === null
        && socket.onclose === null && socket.onerror === null;
}
"#)]
extern "C" {
	fn install_browser();
	fn restore_browser();
	fn constructor_attempts() -> u32;
	fn fail_constructors(count: u32);
	fn timer_count() -> u32;
	fn next_delay() -> u32;
	fn fire_timer() -> bool;
	fn open_socket(index: u32);
	fn disconnect_socket(index: u32);
	fn error_socket(index: u32);
	fn receive_text(index: u32, text: &str);
	fn receive_binary(index: u32);
	fn sent_binary(index: u32, message: u32) -> Vec<u8>;
	fn sent_text(index: u32, message: u32) -> String;
	fn close_calls(index: u32) -> u32;
	fn handlers_detached(index: u32) -> bool;
}

/// Restore browser globals after all handles in the test have been dropped.
struct BrowserFixture;

impl BrowserFixture {
	fn new() -> Self {
		install_browser();
		Self
	}
}

impl Drop for BrowserFixture {
	fn drop(&mut self) {
		restore_browser();
	}
}

const URL: &str = "wss://example.invalid/notifications";

#[wasm_bindgen_test]
fn reconnect_honors_delay_and_reuses_the_public_handle() {
	// Arrange
	let _browser = BrowserFixture::new();
	let handle = use_websocket(
		URL,
		UseWebSocketOptions {
			reconnect_delay: 37,
			..Default::default()
		},
	);
	open_socket(0);
	handle.send_text("first".into()).unwrap();

	// Act: a disconnect waits for the configured timer before opening a new socket.
	disconnect_socket(0);

	// Assert
	assert_eq!(handle.connection_state().get(), ConnectionState::Closed);
	assert_eq!(constructor_attempts(), 1);
	assert_eq!(timer_count(), 1);
	assert_eq!(next_delay(), 37);
	assert!(handlers_detached(0));
	assert!(fire_timer());
	assert_eq!(constructor_attempts(), 2);
	assert_eq!(handle.connection_state().get(), ConnectionState::Connecting);
	open_socket(1);
	assert!(handle.is_open());
	handle.send_text("second".into()).unwrap();
	assert_eq!(sent_text(0, 0), "first");
	assert_eq!(sent_text(1, 0), "second");
	receive_text(1, "current");
	receive_text(0, "stale");
	assert_eq!(
		handle.latest_message().get(),
		Some(WebSocketMessage::Text("current".into()))
	);
	handle.send_binary(vec![1, 2, 3]).unwrap();
	assert_eq!(sent_binary(1, 1), vec![1, 2, 3]);
	receive_binary(1);
	assert_eq!(
		handle.latest_message().get(),
		Some(WebSocketMessage::Binary(vec![7, 8, 9]))
	);
}

#[wasm_bindgen_test]
fn retries_stop_at_the_configured_budget() {
	// Arrange
	let _browser = BrowserFixture::new();
	let _handle = use_websocket(
		URL,
		UseWebSocketOptions {
			max_reconnect_attempts: 2,
			..Default::default()
		},
	);

	// Act
	for socket in 0..2 {
		disconnect_socket(socket);
		assert!(fire_timer());
	}
	disconnect_socket(2);

	// Assert: the initial connection is not counted as a retry.
	assert_eq!(constructor_attempts(), 3);
	assert_eq!(timer_count(), 0);
	assert!(!fire_timer());
}

#[wasm_bindgen_test]
fn successful_open_resets_the_retry_budget() {
	// Arrange
	let _browser = BrowserFixture::new();
	let _handle = use_websocket(
		URL,
		UseWebSocketOptions {
			max_reconnect_attempts: 1,
			..Default::default()
		},
	);
	disconnect_socket(0);
	assert!(fire_timer());
	open_socket(1);

	// Act
	disconnect_socket(1);

	// Assert
	assert_eq!(timer_count(), 1);
	assert!(fire_timer());
	assert_eq!(constructor_attempts(), 3);
}

#[wasm_bindgen_test]
fn disabled_reconnection_and_zero_budget_do_not_schedule_timers() {
	for (enabled, budget) in [(false, 5), (true, 0)] {
		// Arrange
		let _browser = BrowserFixture::new();
		let _handle = use_websocket(
			URL,
			UseWebSocketOptions {
				auto_reconnect: enabled,
				max_reconnect_attempts: budget,
				..Default::default()
			},
		);

		// Act
		disconnect_socket(0);

		// Assert
		assert_eq!(constructor_attempts(), 1);
		assert_eq!(timer_count(), 0);
	}
}

#[wasm_bindgen_test]
fn explicit_close_cancels_a_pending_retry() {
	// Arrange
	let _browser = BrowserFixture::new();
	let handle = use_websocket(URL, UseWebSocketOptions::default());
	disconnect_socket(0);
	assert_eq!(timer_count(), 1);

	// Act
	handle.close();
	handle.close();

	// Assert
	assert_eq!(timer_count(), 0);
	assert!(!fire_timer());
	assert_eq!(constructor_attempts(), 1);
	assert_eq!(handle.connection_state().get(), ConnectionState::Closed);
	assert!(handle.send_text("after close".into()).is_err());
}

#[wasm_bindgen_test]
fn explicit_close_of_a_live_socket_is_terminal_and_notifies_once() {
	// Arrange
	let _browser = BrowserFixture::new();
	let closed = Rc::new(Cell::new(0));
	let closed_callback = Rc::clone(&closed);
	let handle = use_websocket(
		URL,
		UseWebSocketOptions {
			on_close: Some(Rc::new(move || {
				closed_callback.set(closed_callback.get() + 1)
			})),
			..Default::default()
		},
	);
	open_socket(0);

	// Act
	handle.close();
	handle.close();

	// Assert
	assert_eq!(close_calls(0), 1);
	assert_eq!(handle.connection_state().get(), ConnectionState::Closing);
	disconnect_socket(0);
	assert_eq!(closed.get(), 1);
	assert_eq!(handle.connection_state().get(), ConnectionState::Closed);
	assert_eq!(timer_count(), 0);
	assert!(handlers_detached(0));
	open_socket(0);
	receive_text(0, "late");
	assert!(!handle.is_open());
	assert_eq!(handle.latest_message().get(), None);
}

#[wasm_bindgen_test]
fn last_handle_drop_cancels_pending_reconnection() {
	// Arrange
	let _browser = BrowserFixture::new();
	let handle = use_websocket(URL, UseWebSocketOptions::default());
	let last_handle = handle.clone();
	open_socket(0);

	// Act: dropping one clone must not close the shared connection.
	drop(handle);
	assert_eq!(close_calls(0), 0);
	last_handle.send_text("still owned".into()).unwrap();
	disconnect_socket(0);
	assert_eq!(timer_count(), 1);
	drop(last_handle);

	// Assert
	assert_eq!(timer_count(), 0);
	assert!(!fire_timer());
	assert_eq!(constructor_attempts(), 1);
}

#[wasm_bindgen_test]
fn last_handle_drop_closes_the_socket_and_detaches_callbacks() {
	// Arrange
	let _browser = BrowserFixture::new();
	let handle = use_websocket(URL, UseWebSocketOptions::default());
	open_socket(0);

	// Act
	drop(handle);

	// Assert
	assert_eq!(close_calls(0), 1);
	assert!(handlers_detached(0));
	assert_eq!(timer_count(), 0);
}

#[wasm_bindgen_test]
fn constructor_failures_are_reported_and_have_bounded_retries() {
	// Arrange
	let _browser = BrowserFixture::new();
	fail_constructors(2);
	let errors = Rc::new(Cell::new(0));
	let errors_callback = Rc::clone(&errors);
	let handle = use_websocket(
		URL,
		UseWebSocketOptions {
			max_reconnect_attempts: 1,
			on_error: Some(Rc::new(move |_| {
				errors_callback.set(errors_callback.get() + 1)
			})),
			..Default::default()
		},
	);

	// Act
	assert!(fire_timer());

	// Assert
	assert_eq!(constructor_attempts(), 2);
	assert_eq!(errors.get(), 2);
	assert_eq!(timer_count(), 0);
	assert!(matches!(
		handle.connection_state().get(),
		ConnectionState::Error(_)
	));
	assert!(handle.send_text("unavailable".into()).is_err());
}

#[wasm_bindgen_test]
fn on_close_can_stop_reconnection_without_a_refcell_panic() {
	// Arrange
	let _browser = BrowserFixture::new();
	let slot: Rc<RefCell<Option<Weak<WebSocketHandle>>>> = Rc::new(RefCell::new(None));
	let callback_slot = Rc::clone(&slot);
	let handle = Rc::new(use_websocket(
		URL,
		UseWebSocketOptions {
			on_close: Some(Rc::new(move || {
				let handle = callback_slot.borrow().as_ref().and_then(Weak::upgrade);
				if let Some(handle) = handle {
					handle.close();
				}
			})),
			..Default::default()
		},
	));
	*slot.borrow_mut() = Some(Rc::downgrade(&handle));

	// Act
	disconnect_socket(0);

	// Assert
	assert_eq!(timer_count(), 0);
	assert_eq!(handle.connection_state().get(), ConnectionState::Closed);
}

#[wasm_bindgen_test]
fn error_followed_by_close_schedules_only_one_retry() {
	// Arrange
	let _browser = BrowserFixture::new();
	let handle = use_websocket(URL, UseWebSocketOptions::default());

	// Act
	error_socket(0);
	assert!(matches!(
		handle.connection_state().get(),
		ConnectionState::Error(_)
	));
	assert_eq!(timer_count(), 0);
	disconnect_socket(0);

	// Assert
	assert_eq!(timer_count(), 1);
	assert!(fire_timer());
	assert_eq!(constructor_attempts(), 2);
	assert_eq!(timer_count(), 0);
}

#[wasm_bindgen_test]
fn reconnect_delay_is_clamped_to_the_browser_timer_range() {
	// Arrange
	let _browser = BrowserFixture::new();
	let _handle = use_websocket(
		URL,
		UseWebSocketOptions {
			reconnect_delay: u32::MAX,
			..Default::default()
		},
	);

	// Act
	disconnect_socket(0);

	// Assert: large unsigned delays must not wrap into immediate retries.
	assert_eq!(next_delay(), i32::MAX as u32);
}
