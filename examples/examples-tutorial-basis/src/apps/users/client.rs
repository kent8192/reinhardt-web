//! Users-app client (WASM) modules.
//!
//! Holds the authentication UI (`components`). Routed page functions use
//! `#[client_page]`, so the same module tree can be referenced while native
//! builds construct route tables.

pub mod components;
