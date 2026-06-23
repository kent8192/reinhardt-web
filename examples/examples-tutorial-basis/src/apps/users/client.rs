//! Users-app client (WASM) modules.
//!
//! Holds the authentication UI (`components`) and page wrappers (`pages`).
//! The submodules are gated by
//! the `#[cfg(client)]` declaration on `pub mod client;` in `apps/users.rs`,
//! so this aggregator does not need its own per-target cfg.

pub mod components;
pub mod pages;
