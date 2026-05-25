//! Polls-app client (WASM) modules.
//!
//! Holds the polls-specific UI (`components`). The submodule is gated by
//! the `#[cfg(wasm)]` declaration on `pub mod client;` in `apps/polls.rs`,
//! so this aggregator does not need its own per-target cfg.

pub mod components;
