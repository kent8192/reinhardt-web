//! Browser-WASM-side public re-exports for `reinhardt`.
//!
//! Reserved for items whose paths are only reachable on
//! `wasm32-unknown-unknown` browser builds. WASM-side **shims** for native-only
//! crates live in [`crate::compat`] and are re-exported at their canonical
//! root paths from `src/lib.rs` (e.g. `crate::reinhardt_apps`,
//! `crate::urls`, `crate::WebSocketRouter`).
//!
//! Currently no items are routed exclusively through this module — the wasm
//! facade is composed entirely of `crate::compat` shims (canonical paths) plus
//! the cross-target items in [`super::common`]. This module exists so future
//! browser-only additions have an obvious home that does not pollute the
//! cross-target layer.
