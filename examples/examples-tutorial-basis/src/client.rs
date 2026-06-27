//! Cross-app client shell.
//!
//! Each application's UI lives under `apps::<app>::client::*`. This
//! module only hosts cross-app concerns: the WASM entry point
//! (`lib::main`) and the `components::nav` shell.

pub mod lib;

pub mod components;
