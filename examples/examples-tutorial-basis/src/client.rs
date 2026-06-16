//! Client-side code (WASM)
//!
//! This module contains all client-side code that runs in the browser.
//! Client-side routing lives under `crate::apps::polls::urls::client_router`.

//! Cross-app client shell.
//!
//! Each application's UI lives under `apps::<app>::client::*`. This
//! module only hosts cross-app concerns: the WASM entry point
//! (`lib::main`), the SPA `pages` aggregator that wraps every routed page
//! with the shared nav bar, and the `components::nav` shell itself.

#[cfg(client)]
pub mod lib;

pub mod pages;

#[cfg(client)]
pub mod components;
