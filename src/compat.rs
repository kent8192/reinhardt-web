//! Compatibility shims for WASM consumers (Issue #4362, Stage 1).
//!
//! Contains module bodies that the root crate re-exports at their canonical
//! paths so that the public API surface is preserved on
//! `wasm32-unknown-unknown`:
//!
//! - [`apps`] — body re-exported at `crate::reinhardt_apps`
//! - [`urls`] — body re-exported at `crate::urls`
//! - [`websockets`] — `WebSocketRouter` re-exported at `crate::WebSocketRouter`
//!
//! These shims compile but never execute. They mirror the surface that
//! macro-emitted code and downstream SPA consumers reference; they exist
//! because the real backing crates (`reinhardt-apps`, `reinhardt-websockets`)
//! depend on `tokio` and are native-only.

#[doc(hidden)]
pub mod apps;

pub mod urls;

pub mod websockets;
