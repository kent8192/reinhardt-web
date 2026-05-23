//! Polls-app client (WASM) modules.
//!
//! Holds the polls-specific UI (`components`). Typed URL helpers are now
//! emitted by `#[url_patterns]` directly — call sites resolve URLs via
//! `crate::apps::polls::urls::client_router::urls::*` (see issue #4656
//! for the removal of the legacy hand-rolled `links` wrapper). The
//! submodule is gated by the `#[cfg(wasm)]` declaration on
//! `pub mod client;` in `apps/polls.rs`, so this aggregator does not need
//! its own per-target cfg.

pub mod components;
