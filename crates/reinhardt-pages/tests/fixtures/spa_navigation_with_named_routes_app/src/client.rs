//! Client-side (WASM) modules for the Tier 4 fixture.
//!
//! - `lib`    — `#[wasm_bindgen(start)]` entry + `__diag_*_js` exports
//! - `router` — `init_router` returning a [`Router`] with named routes
//! - `pages`  — page builders sharing a persistent layout shell
//!
//! The scaffolded `components` submodule was dropped: a regression
//! fixture only needs the components inlined into `pages.rs`.

pub mod lib;

pub mod router;

pub mod pages;
