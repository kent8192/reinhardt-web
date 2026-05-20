//! Users-app client (WASM) modules.
//!
//! Holds the authentication UI (`components`) and the typed URL helpers
//! used to navigate between users routes (`links`). Both submodules are
//! gated by the `#[cfg(wasm)]` declaration on `pub mod client;` in
//! `apps/users.rs`, so this aggregator does not need its own per-target
//! cfg.

pub mod components;
pub mod links;
