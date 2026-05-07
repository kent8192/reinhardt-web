//! History API Integration for client-side routing.
//!
//! This module is a thin re-export shim over the canonical
//! implementation in `reinhardt_urls::routers::client_router::history`.
//! The two crates used to keep parallel copies of these primitives,
//! which produced the SPA navigation regression chain
//! (#4075 / #4088 / #4122 / #4203 / #4213) every time a fix landed on
//! only one side. Issue #4217 collapses them to a single source of
//! truth.

pub use reinhardt_urls::routers::client_router::history::{
	HistoryState, NavigationType, setup_popstate_listener,
};

// Internal helpers consumed by `super::core`. Tightened to `pub(super)`
// so they remain hidden from the crate's public API surface.
pub(super) use reinhardt_urls::routers::client_router::history::{
	current_path, push_state, replace_state,
};
