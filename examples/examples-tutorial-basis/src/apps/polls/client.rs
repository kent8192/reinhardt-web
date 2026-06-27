//! Polls-app client (WASM) modules.
//!
//! Holds the polls-specific UI (`components`) and page wrappers (`pages`).
//! The submodules are gated by
//! the `#[cfg(client)]` declaration on `pub mod client;` in `apps/polls.rs`,
//! so this aggregator does not need its own per-target cfg.

pub mod components;
pub mod pages;
