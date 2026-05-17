//! WebSocket routing — re-exported from `reinhardt-core::ws`, plus
//! the inventory-based registration glue introduced for #4453.
//!
//! The foundational types (`WebSocketRoute`, `WebSocketRouter`, etc.) live in
//! `reinhardt-core::ws` so that `reinhardt-urls` can depend on them without
//! creating a circular dependency through `reinhardt-pages`. The
//! `WsRouterRegistration` type — the symmetric counterpart to
//! `reinhardt-urls`'s `UrlPatternsRegistration` and
//! `ClientRouterRegistration` — is anchored *here* (instead of in
//! `reinhardt-urls`) so adding it does not introduce a
//! `reinhardt-urls → reinhardt-websockets` dependency edge. The macro
//! and the `RunServerCommand` consumer access it through the facade
//! re-export instead.

use std::sync::Arc;

pub use reinhardt_core::ws::{
	RouteError, RouteResult, WebSocketRoute, WebSocketRouter, clear_websocket_router,
	get_websocket_router, register_websocket_router, reverse_websocket_url,
};

/// Build-time WebSocket router registration submitted by `#[routes]`
/// expansion on native targets with the `websockets` feature enabled
/// (Refs #4453).
///
/// The macro emits one `inventory::submit!` per `#[routes]` function:
///
/// ```ignore
/// #[cfg(all(not(target_family = "wasm"), feature = "websockets"))]
/// const _: () = {
///     fn __get_websocket_router() -> ::std::sync::Arc<WebSocketRouter> {
///         ::std::sync::Arc::new(routes().into_websocket())
///     }
///     inventory::submit! {
///         WsRouterRegistration::__macro_new(__get_websocket_router)
///     }
/// };
/// ```
///
/// The consumer side iterates registrations via [`inventory::iter`],
/// merges the resulting `WebSocketRouter`s through
/// [`collect_websocket_router_from_inventory`], and installs the
/// process-wide instance via [`register_websocket_router`]. This
/// mirrors the [`UrlPatternsRegistration`] / [`ClientRouterRegistration`]
/// pattern in `reinhardt-urls`.
///
/// [`UrlPatternsRegistration`]: reinhardt_urls::routers::UrlPatternsRegistration
/// [`ClientRouterRegistration`]: reinhardt_urls::routers::ClientRouterRegistration
pub struct WsRouterRegistration {
	get_websocket_router: fn() -> Arc<WebSocketRouter>,
}

impl WsRouterRegistration {
	/// Internal constructor invoked by the `#[routes]` proc-macro
	/// expansion. **Not** intended for direct user calls — the macro
	/// is the only stable caller.
	#[doc(hidden)]
	pub const fn __macro_new(get_websocket_router: fn() -> Arc<WebSocketRouter>) -> Self {
		Self {
			get_websocket_router,
		}
	}

	/// Materialize the registered `WebSocketRouter` by invoking the
	/// macro-supplied factory.
	pub fn websocket_router(&self) -> Arc<WebSocketRouter> {
		(self.get_websocket_router)()
	}
}

inventory::collect!(WsRouterRegistration);

/// Iterate every `#[routes]`-registered WebSocket router and fold them
/// into a single [`WebSocketRouter`] via [`WebSocketRouter::merge`].
///
/// Returns `None` if no registrations exist. Each factory is invoked
/// exactly once; the resulting `Arc<WebSocketRouter>` is unwrapped
/// (preferring zero-cost extraction when the `Arc` is unique, falling
/// back to a [`Clone`] otherwise — `WebSocketRouter` derives `Clone`).
///
/// Used by `RunServerCommand::register_websocket_routes_from_inventory`
/// (Refs #4453). An empty inventory is **not** an error here: WebSocket
/// routes are optional, unlike HTTP routes.
pub fn collect_websocket_router_from_inventory() -> Option<WebSocketRouter> {
	let mut iter = inventory::iter::<WsRouterRegistration>().map(|r| r.websocket_router());
	let first = iter.next()?;
	let mut merged = Arc::try_unwrap(first).unwrap_or_else(|arc| (*arc).clone());
	for arc in iter {
		let other = Arc::try_unwrap(arc).unwrap_or_else(|arc| (*arc).clone());
		merged = merged.merge(other);
	}
	Some(merged)
}
