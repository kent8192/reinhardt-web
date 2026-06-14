//! WASM shim for `WebSocketRouter` (Issue #4161).
//!
//! WebSocket route declarations call `.with_namespace(...)` on the
//! function's return value, and the function's return type references
//! `WebSocketRouter`. The real type lives in `reinhardt-websockets`,
//! which depends on `tokio-tungstenite` and is native-only. This stub
//! matches the surface the user-facing imports
//! (`use reinhardt::WebSocketRouter`) so that wasm consumers compile,
//! including the typical
//! `WebSocketRouter::new().consumer(my_ws).consumer(other_ws)` body
//! pattern.

/// WASM-only no-op stand-in for native `WebSocketRouter`.
pub struct WebSocketRouter {
	_private: (),
}

impl WebSocketRouter {
	/// Creates an empty no-op WebSocket router.
	pub fn new() -> Self {
		Self { _private: () }
	}

	/// Accepts a route namespace and returns the unchanged no-op router.
	pub fn with_namespace(self, _namespace: impl Into<String>) -> Self {
		self
	}

	/// Inert wasm counterpart of `WebSocketRouter::consumer`.
	///
	/// The native variant requires `C: WebSocketEndpointInfo`, but that
	/// trait lives behind `#[cfg(native)]` in `reinhardt-core::ws`. To
	/// keep WebSocket route declarations such as
	/// `.consumer(chat_ws)` compiling on wasm, this stub accepts any
	/// factory `Fn() -> C` with no further bounds and discards it.
	pub fn consumer<C, F>(self, _f: F) -> Self
	where
		F: Fn() -> C,
	{
		self
	}
}

impl Default for WebSocketRouter {
	fn default() -> Self {
		Self::new()
	}
}
