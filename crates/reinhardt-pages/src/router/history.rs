//! History API Integration for client-side routing.
//!
//! This module is a thin re-export shim over the canonical
//! implementation in `reinhardt_urls::routers::client_router::history`.
//! The two crates used to keep parallel copies of these primitives,
//! which produced the SPA navigation regression chain
//! (#4075 / #4088 / #4122 / #4203 / #4213) every time a fix landed on
//! only one side. Issue #4217 collapses them to a single source of
//! truth.

pub use reinhardt_urls::routers::client_router::history::{HistoryState, NavigationType};

// `setup_popstate_listener` is wasm-only: its return type
// (`Closure<dyn FnMut(PopStateEvent)>`) has no meaningful native
// counterpart. The previous native stub silently no-op'd, which made the
// signature appear cross-platform while in fact requiring `cfg` at every
// call site.
#[cfg(wasm)]
pub use reinhardt_urls::routers::client_router::history::setup_popstate_listener;
