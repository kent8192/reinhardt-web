//! WASM shim for `WebSocketRouter` (Issue #4161).
//!
//! `#[url_patterns(.., mode = ws)]` expansions call `.with_namespace(...)`
//! on the function's return value, and the function's return type
//! references `WebSocketRouter`. The real type lives in
//! `reinhardt-websockets`, which depends on `tokio-tungstenite` and is
//! native-only. This stub matches the surface the macro emits and the
//! user-facing imports (`use reinhardt::WebSocketRouter`) so that wasm
//! consumers compile, including the typical
//! `WebSocketRouter::new().consumer(my_ws).consumer(other_ws)` body
//! pattern.
//!
//! Re-exported at `crate::WebSocketRouter` from `src/lib.rs` (wasm-only) to
//! preserve the canonical user-facing name.

pub struct WebSocketRouter {
	_private: (),
}

impl WebSocketRouter {
	pub fn new() -> Self {
		Self { _private: () }
	}

	pub fn with_namespace(self, _namespace: impl Into<String>) -> Self {
		self
	}

	/// Inert wasm counterpart of `WebSocketRouter::consumer`.
	///
	/// The native variant requires `C: WebSocketEndpointInfo`, but that
	/// trait lives behind `#[cfg(native)]` in `reinhardt-core::ws`. To
	/// keep `#[url_patterns(.., mode = ws)]` user bodies such as
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
