//! Client-side code (WASM)
//!
//! This module contains all client-side code that runs in the browser.
//! Client-side routing lives under `crate::apps::polls::urls::client_router`.

//! Cross-app client shell.
//!
//! Each application's UI lives under `apps::<app>::client::*`. This
//! module only hosts cross-app concerns: the WASM entry point
//! (`lib::main`) and the `components::nav` shell.

pub mod lib;

pub mod components;
