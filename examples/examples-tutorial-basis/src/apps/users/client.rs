//! Users-app client (WASM) modules.
//!
//! Holds the authentication UI (`components`). The submodule is gated by
//! the `#[cfg(wasm)]` declaration on `pub mod client;` in `apps/users.rs`,
//! so this aggregator does not need its own per-target cfg.

pub mod components;
