//! Polls-app client (WASM) modules.
//!
//! Holds the polls-specific UI (`components`) and the typed URL helpers
//! used to navigate between polls routes (`links`). Both submodules are
//! gated by the `#[cfg(wasm)]` declaration on `pub mod client;` in
//! `apps/polls.rs`, so this aggregator does not need its own per-target
//! cfg.

pub mod components;
pub mod links;
