//! Client-side (WASM) modules for the {{ app_name }} application.
//!
//! Reached only on the WASM target via `#[cfg(client)] pub mod client;`
//! in the parent app aggregator; contents below need no additional gates.
//!
//! The freshly generated app contains a placeholder component / page in
//! `components.rs` and `pages.rs`. Register the placeholder route in
//! `urls/client_router.rs` to see the SPA boot, then replace both stubs
//! with your real UI.

pub mod components;
pub mod pages;
